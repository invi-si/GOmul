"""Local native sessions. ROM bytes never travel back out to another service."""
import atexit
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import selectors
import shutil
import struct
import subprocess
import threading
import time
import uuid
import zipfile
import zlib
from alz_archive import read_alz

ROOT = Path(__file__).resolve().parents[2]
RPC_TIMEOUT = 120
RPC_LIMIT = 64 * 1024 * 1024


def validated_settings(values):
    if not isinstance(values,dict):raise ValueError('올바르지 않은 게임 설정입니다.')
    speed=values.get('speed',1000);width=values.get('width',240);height=values.get('height',320);phone=values.get('phone','')
    if type(speed) is not int or not 250<=speed<=2000 or type(width) is not int or type(height) is not int or not 64<=width<=1024 or not 64<=height<=1024 or not isinstance(phone,str) or (phone and not re.fullmatch('[0-9]{11}',phone)):
        raise ValueError('화면 크기, 실행 속도 또는 전화번호를 확인하세요.')
    return dict(speed=speed,width=width,height=height,custom=bool(values.get('custom')),full=bool(values.get('full')),phone=phone,autoStartup=bool(values.get('autoStartup')))


def archive_files(data):
    result = {}
    total = 0
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for info in archive.infolist():
            name = info.filename
            if name.startswith('/') or '\\' in name or '..' in PurePosixPath(name).parts:
                raise ValueError('안전하지 않은 ZIP 경로입니다.')
            if info.is_dir():
                continue
            total += info.file_size
            if total > 128 * 1024 * 1024 or name in result:
                raise ValueError('게임 ZIP 크기 또는 중복 항목 오류입니다.')
            result[name] = archive.read(info)
    return result


def prepare_archive(data):
    files = read_alz(data) if data.startswith(b'ALZ\x01') else archive_files(data)
    roots = {name.rpartition('/')[0] for name in files if PurePosixPath(name).name in ('app_info', '__adf__')}
    if len(roots) > 1:
        raise ValueError('한 게임만 포함된 ZIP을 선택하세요.')
    prefix = next(iter(roots), '')
    if prefix:
        prefix += '/'
        files = {name[len(prefix):]:value for name,value in files.items() if name.startswith(prefix)}
    # ZIP central directories also permit prepended data (e.g. image wrappers).
    # Canonicalize those and single-folder carrier bundles before native sniffing.
    if prefix or not data.startswith(b'PK\x03\x04'):
        result = io.BytesIO()
        with zipfile.ZipFile(result, 'w', zipfile.ZIP_DEFLATED) as archive:
            for name, value in sorted(files.items()):
                archive.writestr(zipfile.ZipInfo(name), value, compress_type=zipfile.ZIP_DEFLATED)
        data = result.getvalue()
    return data, files


def companion(save, files, phone):
    # Preserve all earned data, including legacy unmarked saves.
    if save.exists() and any(p.is_file() and p.relative_to(save).as_posix() not in ('phone-number.txt','display-options') for p in save.rglob('*')):
        return None
    info = files.get('app_info', b'').decode('euc-kr', 'replace')
    match = re.search(r'^PID:([A-Za-z0-9_-]+)\s*$', info, re.M)
    if not match: return None
    pid = match[1]
    records = {}
    parent = None
    for name, data in files.items():
        leaf = PurePosixPath(name).name
        if leaf == 'gomul.properties':
            declared = re.search(r'^phoneNumber\s*[=:]\s*([0-9]{11})\s*$', data.decode('latin1'), re.M)
            if declared: phone = declared[1]
        if not re.fullmatch(r'savedata|locdata|ranker|it[0-9]+|mk[0-9]+', leaf):continue
        folder = str(PurePosixPath(name).parent)
        if parent is not None and folder != parent:raise ValueError('초기 데이터 폴더가 둘 이상입니다.')
        parent = folder
        if leaf in records:raise ValueError('중복된 초기 데이터입니다.')
        records[leaf] = data
    if not records:return None
    if sum(map(len, records.values())) > 16 * 1024 * 1024:raise ValueError('초기 데이터가 너무 큽니다.')
    phone = phone or ('01055145031' if pid == 'PD121120' else None)
    if not phone: return {'needsPhone':True}
    if not re.fullmatch('[0-9]{11}', phone):raise ValueError('가상 전화번호 11자리를 입력하세요.')
    save.parent.mkdir(parents=True, exist_ok=True)
    temp = save.with_name(save.name + '.import-' + uuid.uuid4().hex)
    temp.mkdir()
    try:
        if save.exists():shutil.copytree(save, temp, dirs_exist_ok=True)
        for name, data in records.items():
            dest = temp / pid / 'db' / name / '1'; dest.parent.mkdir(parents=True,exist_ok=True); dest.write_bytes(data)
        (temp/'phone-number.txt').write_text(phone+'\n')
        (temp/'companion-imported').write_text('1')
        backup = save.with_name(save.name + '.before-import-' + uuid.uuid4().hex)
        if save.exists():save.rename(backup)
        try:temp.rename(save)
        except Exception:
            if backup.exists():backup.rename(save)
            raise
        return {'imported':True,'phone':phone}
    finally:
        if temp.exists():shutil.rmtree(temp)


def frame_png(frame):
    if len(frame) < 17:raise ValueError('No frame')
    width,height = struct.unpack_from('<II', frame)
    if not 0 < width <= 1024 or not 0 < height <= 1024 or len(frame)!=17+width*height*4:raise ValueError('Invalid frame')
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            b,g,r,a = frame[17+4*(y*width+x):21+4*(y*width+x)]
            raw.extend((r,g,b,255))
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')


class Native:
    def __init__(self, catalog, data=None, binary=None):
        self.catalog = catalog
        self.root = data or Path(os.environ.get('XDG_DATA_HOME',Path.home()/'.local/share'))/'gomul-web'
        self.binary = binary or ROOT/'wie-web/native/gomul-local'
        self.lock = threading.RLock()
        self.process = None
        self.token = None
        self.game = None
        atexit.register(self.close)

    def rpc(self, command):
        if not self.process or self.process.poll() is not None:
            raise ValueError('게임 세션이 종료되었습니다. 게임을 다시 여세요.')
        try:
            self.process.stdin.write((json.dumps(command)+'\n').encode())
            self.process.stdin.flush()
            deadline = time.monotonic() + RPC_TIMEOUT
            data = bytearray()
            with selectors.DefaultSelector() as selector:
                selector.register(self.process.stdout, selectors.EVENT_READ)
                while True:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0 or not selector.select(remaining):
                        raise ValueError('게임 명령 대기 시간이 초과되었습니다. 게임을 다시 여세요.')
                    chunk = os.read(self.process.stdout.fileno(), 65536)
                    if not chunk:
                        raise ValueError('네이티브 게임 세션이 종료되었습니다.')
                    data.extend(chunk)
                    if len(data) > RPC_LIMIT:
                        raise ValueError('게임 응답 크기 제한을 초과했습니다.')
                    if b'\n' in chunk:break
            result = json.loads(data)
            if not isinstance(result, dict):
                raise ValueError('올바르지 않은 게임 응답입니다.')
        except (OSError, ValueError):
            # Drop the pipe after partial/invalid replies, so they can never be
            # mistaken for a subsequent command's response.
            self.close()
            raise ValueError('게임 통신이 중단되었습니다. 게임을 다시 여세요.') from None
        if 'error' in result:
            # A complete command error (e.g. incompatible checkpoint) preserves
            # the current runtime and its recoverable state.
            raise ValueError(result['error'])
        return result

    def close(self):
        with self.lock:
            if self.process:
                try:
                    self.process.stdin.close()
                    self.process.wait(timeout=3)
                except (OSError,subprocess.TimeoutExpired):
                    self.process.kill();self.process.wait()
                self.process.stdout.close()
                self.process = None
            self.token = None

    def settings(self, game):
        path = self.root/'settings'/f'{game}.json'
        return validated_settings(json.loads(path.read_text()) if path.exists() else {})

    def write_settings(self, values):
        path = self.root/'settings'/f'{self.game}.json';path.parent.mkdir(parents=True,exist_ok=True)
        temp = path.with_suffix('.tmp');temp.write_text(json.dumps(values));temp.replace(path)

    def paths(self):
        return self.root/'saves'/self.game, self.root/'checkpoints'/self.game

    def launch(self, game_id, index, expected_identity=None):
        files = self.catalog.attachments(game_id)
        if type(index) is not int or not 0 <= index < len(files):raise ValueError('첨부파일을 찾을 수 없습니다.')
        raw = self.catalog.rom(game_id,index)
        identity = hashlib.sha256(raw).hexdigest()
        if expected_identity is not None and identity != expected_identity:
            raise ValueError('게임 파일이 변경되었습니다. 목록에서 다시 실행하세요.')
        data, entries = prepare_archive(raw)
        name = PurePosixPath(files[index]['name']).name
        if name.lower().endswith('.alz'):name = name[:-4] + '.zip'
        with self.lock:
            if not self.binary.is_file():raise ValueError('네이티브 실행기를 먼저 빌드하세요: tools/web-catalog/build-native.sh')
            self.close()
            self.game = identity
            self.startup_error = None
            self.source = (game_id,index)
            self.entries = entries
            self.archive = self.root/'games'/identity/name
            self.archive.parent.mkdir(parents=True,exist_ok=True)
            if not self.archive.exists():self.archive.write_bytes(data)
            settings = self.settings(identity)
            save,slots = self.paths()
            setup = companion(save,entries,settings.get('phone',''))
            self.boot(settings)
            self.token = uuid.uuid4().hex
            startup = slots/'startup'
            if startup.is_dir() and settings.get('autoStartup',False):
                self.load_startup(slots)
            return {'session':self.token,'settings':settings,'setup':setup,'native':True}

    def boot(self, settings):
        save,_ = self.paths();save.mkdir(parents=True,exist_ok=True)
        if settings.get('phone'):(save/'phone-number.txt').write_text(settings['phone']+'\n')
        display = save/'display-options'
        if settings.get('custom'):display.write_text(f"{settings['width']} {settings['height']} {int(settings['full'])}")
        else:display.unlink(missing_ok=True)
        self.process = subprocess.Popen([str(self.binary),str(self.archive),str(save)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,bufsize=0)
        self.rpc({'op':'speed','value':settings['speed']})

    def checkpoint_startup(self, action):
        self.rpc({'op':'checkpoint','action':'save-startup' if action=='store' else 'load-startup'})

    def load_startup(self,slots):
        try:self.checkpoint_startup('load')
        except ValueError as error:
            if self.process is None:raise
            self.startup_error=str(error)

    def command(self, payload):
        with self.lock:
            if not self.token or payload.get('session') != self.token:raise ValueError('다른 창에서 게임이 바뀌었습니다. 게임을 다시 여세요.')
            op=payload.get('op')
            if op=='stop':self.close();return {}
            save,slots=self.paths()
            if op=='settings':
                settings=validated_settings(payload.get('values',{}))
                speed=settings['speed'];phone=settings['phone']
                previous=self.settings(self.game)
                restart=any(settings.get(k)!=previous.get(k) for k in ('width','height','custom','full','phone'))
                self.write_settings(settings)
                if restart:
                    token=self.token;self.close()
                    if previous.get('phone') and not phone:(save/'phone-number.txt').unlink(missing_ok=True)
                    companion(save,self.entries,phone);self.boot(settings);self.token=token
                else:self.rpc({'op':'speed','value':speed})
                return {'settings':settings,'restarted':restart}
            if op=='startup-save':
                self.checkpoint_startup('store')
                settings=self.settings(self.game);settings['autoStartup']=True;self.write_settings(settings)
                return {'settings':settings,'message':'시작 상태를 보호 저장했습니다. 다음 실행부터 자동으로 불러옵니다.'}
            if op=='reset':
                token=self.token;self.close()
                backup=self.root/'reset-backups'/f'{self.game}-{uuid.uuid4().hex}';backup.mkdir(parents=True)
                identity=(save/'phone-number.txt').read_bytes() if (save/'phone-number.txt').exists() else None
                for folder in (save,slots):
                    if folder.exists():folder.rename(backup/folder.parent.name)
                if identity is not None:save.mkdir(parents=True,exist_ok=True);(save/'phone-number.txt').write_bytes(identity)
                settings=self.settings(self.game);settings['autoStartup']=False;self.write_settings(settings)
                companion(save,self.entries,settings.get('phone',''));self.boot(settings);self.token=token
                return {'settings':settings,'message':'저장 데이터를 초기화했습니다. 이전 데이터는 Mac에 백업했습니다.'}
            if op in ('rescue','rescue-latest'):
                if op=='rescue':path=Path(self.rpc({'op':'rescue'})['path'])
                else:path=self.root/'rescues'/self.game/'latest'
                if not path.is_dir():raise ValueError('저장된 오류 보고서가 없습니다. 수동 보고서를 만드세요.')
                archive=io.BytesIO()
                with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as z:
                    for file in path.rglob('*'):
                        if file.is_file() and not file.is_symlink():z.write(file,file.relative_to(path).as_posix())
                    if (path/'frame').exists():z.writestr('screenshot.png',frame_png((path/'frame').read_bytes()))
                return archive.getvalue()
            if op not in ('poll','key','pause','korean','checkpoint','speed'):raise ValueError('지원하지 않는 명령입니다.')
            result=self.rpc(payload)
            if op=='poll':
                result['quick']=(slots/'quick').is_dir() or (slots/'quick-old').is_dir()
                result['recovery']=(slots/'recovery').is_dir() or (slots/'recovery-old').is_dir()
                result['startup']=(slots/'startup').is_dir()
                result['rescue']=(self.root/'rescues'/self.game/'latest').is_dir()
                if getattr(self,'startup_error',None):result['startupError']=self.startup_error;self.startup_error=None
            return result
