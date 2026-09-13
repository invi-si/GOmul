"""Browser-owned save bundles; capability-scoped, disposable native workers."""
import atexit
import base64
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import shutil
import tempfile
import threading
import time
import zipfile
from native import Native

BUNDLE_LIMIT = 128 * 1024 * 1024
EXPANDED_LIMIT = 256 * 1024 * 1024
SLOTS = {'quick', 'quick-old', 'recovery', 'recovery-old', 'startup'}


def storage_path(name, identity):
    parts = PurePosixPath(name).parts
    if not name or name.startswith('/') or '\\' in name or any(p in ('.', '..') for p in name.split('/')):
        raise ValueError('올바르지 않은 저장 경로입니다.')
    if len(parts) >= 2 and parts[:2] == ('saves', identity):return True
    if len(parts) >= 3 and parts[:2] == ('checkpoints', identity) and parts[2] in SLOTS:return True
    if parts == ('settings', identity + '.json'):return True
    raise ValueError('다른 게임 또는 지원하지 않는 저장 데이터입니다.')


def restore_bundle(root, identity, encoded):
    if not isinstance(encoded, str):raise ValueError('올바르지 않은 브라우저 저장 데이터입니다.')
    data = base64.b64decode(encoded, validate=True)
    if len(data) > BUNDLE_LIMIT:raise ValueError('브라우저 저장 크기 제한을 초과했습니다.')
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        total=0;seen=set()
        for info in archive.infolist():
            name=info.filename.rstrip('/') if info.is_dir() else info.filename
            storage_path(name, identity)
            if name in seen or len(seen)>=20000:raise ValueError('중복 또는 너무 많은 저장 파일입니다.')
            seen.add(name);total+=info.file_size
            if total>EXPANDED_LIMIT:raise ValueError('저장 데이터 압축 해제 제한을 초과했습니다.')
            # Never extract symlinks, devices, or archive-controlled permissions.
            mode=(info.external_attr >> 16) & 0o170000
            if mode not in (0,0o100000,0o040000):raise ValueError('지원하지 않는 저장 파일 형식입니다.')
            path=root/name
            if info.is_dir():path.mkdir(parents=True,exist_ok=True)
            else:
                path.parent.mkdir(parents=True,exist_ok=True)
                path.write_bytes(archive.read(info))


def signature(native):
    roots=[native.root/'saves'/native.game,native.root/'checkpoints'/native.game,native.root/'settings']
    return tuple((str(p.relative_to(native.root)),p.stat().st_mtime_ns,p.stat().st_size)
                 for root in roots if root.exists() for p in sorted(root.rglob('*')) if p.is_file())


def export_bundle(native):
    # The worker snapshots at a command boundary, without releasing held keys,
    # changing pause state, or replacing Quick Save with an autosave.
    native.rpc({'op':'browser-storage'})
    snapshot=native.root/'browser-export'
    settings=native.root/'settings'/f'{native.game}.json'
    if settings.exists():
        dest=snapshot/'settings'/settings.name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(settings,dest)
    result=io.BytesIO();total=0;count=0
    with zipfile.ZipFile(result,'w',zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(snapshot.rglob('*')):
            if path.is_symlink():raise ValueError('지원하지 않는 저장 링크입니다.')
            name=path.relative_to(snapshot).as_posix()
            # Parent containers are implicit; preserve empty guest directories.
            if path.is_dir() and len(path.relative_to(snapshot).parts)<3:continue
            storage_path(name,native.game)
            count+=1
            if count>20000:raise ValueError('너무 많은 브라우저 저장 파일입니다.')
            if path.is_dir():archive.writestr(zipfile.ZipInfo(name+'/'),b'')
            else:
                data=path.read_bytes();total+=len(data)
                if total>EXPANDED_LIMIT:raise ValueError('브라우저 저장 크기 제한을 초과했습니다.')
                archive.writestr(zipfile.ZipInfo(name),data,compress_type=zipfile.ZIP_DEFLATED)
    data=result.getvalue()
    if len(data)>BUNDLE_LIMIT:raise ValueError('브라우저 저장 크기 제한을 초과했습니다.')
    return data


class BrowserSessions:
    def __init__(self,catalog,binary=None,max_sessions=8,idle_seconds=300):
        self.catalog=catalog;self.binary=binary;self.max_sessions=max_sessions;self.idle_seconds=idle_seconds
        self.lock=threading.RLock();self.sessions={};self.pending=0;self.closed=False
        self.done=threading.Event()
        self.reaper=threading.Thread(target=self._reap,daemon=True);self.reaper.start()
        atexit.register(self.close)

    def identity(self,payload):
        if type(payload.get('index')) is not int or payload['index'] < 0:raise ValueError('첨부파일을 선택하세요.')
        raw=self.catalog.rom(str(payload.get('game','')),payload.get('index'))
        return {'identity':hashlib.sha256(raw).hexdigest()}

    def launch(self,payload):
        identity=self.identity(payload)['identity']
        if payload.get('identity')!=identity:raise ValueError('게임 파일이 변경되었습니다. 다시 실행하세요.')
        with self.lock:
            if self.closed or len(self.sessions)+self.pending>=self.max_sessions:raise ValueError('현재 실행 가능한 게임 수를 초과했습니다. 잠시 후 다시 시도하세요.')
            self.pending+=1
        temp=tempfile.TemporaryDirectory(prefix='gomul-browser-')
        native=Native(self.catalog,Path(temp.name),self.binary)
        try:
            if payload.get('storage') is not None:restore_bundle(native.root,identity,payload['storage'])
            result=native.launch(str(payload.get('game','')),payload.get('index'),identity)
            with self.lock:
                if self.closed:raise ValueError('서버가 종료되었습니다.')
                self.sessions[result['session']]={'native':native,'temp':temp,'seen':time.monotonic(),'signature':None,'revision':None}
            return dict(result,identity=identity)
        except Exception:
            native.close();temp.cleanup();raise
        finally:
            atexit.unregister(native.close)
            with self.lock:self.pending-=1

    def command(self,payload):
        token=payload.get('session')
        if not isinstance(token,str):raise ValueError('게임 세션이 필요합니다.')
        with self.lock:
            item=self.sessions.get(token)
            if item is None:raise ValueError('게임 세션이 만료되었습니다. 브라우저에 저장된 데이터로 다시 실행하세요.')
            item['seen']=time.monotonic()
        native=item['native']
        with native.lock:
            if payload.get('op')=='storage':
                current=signature(native)
                if current==item['signature'] and payload.get('revision')==item['revision']:
                    return {'revision':item['revision']}
                data=export_bundle(native)
                revision=hashlib.sha256(data).hexdigest()
                item['signature']=current;item['revision']=revision
                return {'revision':revision,'storage':base64.b64encode(data).decode()}
            if payload.get('op')=='stop':
                with self.lock:self.sessions.pop(token,None)
                native.close();item['temp'].cleanup();return {}
            result=native.command(payload)
            if payload.get('op')=='reset':
                result['message']='저장 데이터를 초기화했습니다. 브라우저 저장도 새 상태로 교체됩니다.'
            return result

    def _reap(self):
        while not self.done.wait(30):
            with self.lock:expired=[token for token,item in self.sessions.items() if time.monotonic()-item['seen']>self.idle_seconds]
            for token in expired:
                with self.lock:
                    item=self.sessions.get(token)
                    if item is None:continue
                # Do not expire a checkpoint/export currently being processed.
                with item['native'].lock:
                    with self.lock:
                        if time.monotonic()-item['seen']<=self.idle_seconds:continue
                        self.sessions.pop(token,None)
                    item['native'].close();item['temp'].cleanup()

    def close(self):
        self.done.set()
        with self.lock:
            self.closed=True;items=list(self.sessions.values());self.sessions.clear()
        for item in items:
            with item['native'].lock:item['native'].close();item['temp'].cleanup()
