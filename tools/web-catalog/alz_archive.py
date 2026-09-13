"""Bounded ALZ decoding through unar; members stream to memory, not host paths."""
import json
import os
from pathlib import Path, PurePosixPath
import selectors
import shutil
import subprocess
import tempfile
import time

LIMIT=128*1024*1024


def output(command,limit):
    process=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
    data=bytearray();deadline=time.monotonic()+30
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout,selectors.EVENT_READ)
            while True:
                remaining=deadline-time.monotonic()
                if remaining<=0 or not selector.select(remaining):raise ValueError('ALZ 처리 시간이 초과되었습니다.')
                chunk=os.read(process.stdout.fileno(),min(65536,limit+1-len(data)))
                if not chunk:break
                data.extend(chunk)
                if len(data)>limit:raise ValueError('ALZ 압축 해제 크기 제한을 초과했습니다.')
        if process.wait(timeout=max(.01,deadline-time.monotonic()))!=0:raise ValueError('ALZ 파일을 읽지 못했습니다.')
        return bytes(data)
    finally:
        if process.poll() is None:process.kill();process.wait()
        process.stdout.close()


def members(manifest):
    entries=manifest.get('lsarContents')
    if not isinstance(entries,list) or len(entries)>20000:raise ValueError('올바르지 않은 ALZ 파일 목록입니다.')
    total=0;seen=set();result=[]
    for entry in entries:
        name=entry.get('XADFileName','');size=entry.get('XADFileSize',0);index=entry.get('XADIndex')
        if not isinstance(name,str) or not name or name.startswith('/') or '\\' in name or '..' in PurePosixPath(name).parts or name in seen:
            raise ValueError('안전하지 않거나 중복된 ALZ 경로입니다.')
        seen.add(name)
        if entry.get('XADIsLink') or entry.get('XADLinkDestination') or entry.get('XADIsEncrypted'):
            raise ValueError('링크 또는 암호화된 ALZ는 지원하지 않습니다.')
        if entry.get('XADIsDirectory'):continue
        if type(size) is not int or size<0 or type(index) is not int or index<0:raise ValueError('올바르지 않은 ALZ 파일 정보입니다.')
        total+=size
        if total>LIMIT:raise ValueError('ALZ 압축 해제 크기 제한을 초과했습니다.')
        result.append((name,size,index))
    return result


def read_alz(data):
    lsar=shutil.which('lsar');unar=shutil.which('unar')
    if not lsar or not unar:raise ValueError('ALZ 게임을 실행하려면 서버에 unar를 설치해야 합니다.')
    with tempfile.TemporaryDirectory(prefix='gomul-alz-') as temp:
        source=Path(temp)/'source.alz';source.write_bytes(data)
        manifest=json.loads(output([lsar,'-j',str(source)],4*1024*1024))
        files={}
        for name,size,index in members(manifest):
            body=output([unar,'-q','-nr','-o','-','-i',str(source),str(index)],size)
            if len(body)!=size:raise ValueError('ALZ 파일 크기가 올바르지 않습니다.')
            files[name]=body
        return files
