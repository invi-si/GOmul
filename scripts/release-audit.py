#!/usr/bin/env python3
"""Scan release files and reachable Git history without printing secret values."""
import pathlib
import re
import subprocess
import sys

ROOT=pathlib.Path(__file__).resolve().parents[1]
ALLOWED_ARCHIVES={'wie-ktf/tests/data/helloworld_ktf.zip','wie-lgt/tests/data/helloworld_lgt.zip','wie-android/android/gradle/wrapper/gradle-wrapper.jar'}
PATTERNS={
 'personal home path': re.compile(rb'/(?:Users|home)/[A-Za-z0-9_.-]+/'),
 'GitHub credential': re.compile(rb'\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b'),
 'private key': re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'),
}

def forbidden_path(name):
    path=pathlib.PurePosixPath(name)
    if any(part in ('snapshots','saves','runs','node_modules','.gradle','__pycache__') for part in path.parts):return True
    if path.name in ('bridge.json','mac-checkpoints.json','phone-number.txt','local.properties') or path.name.startswith('slots-'):return True
    if path.suffix.lower() in ('.apk','.aab','.sav','.tar','.log','.keystore','.jks','.mod'):return True
    if path.suffix.lower() in ('.jar','.zip') and name not in ALLOWED_ARCHIVES:return True
    return False

def main():
    problems=[];checked=0
    names=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=ROOT).decode().split('\0')
    for name in names:
        if not name:continue
        if forbidden_path(name):problems.append(('working tree',name,'excluded file type/path'))
        path=ROOT/name
        if path.is_file():
            data=path.read_bytes();checked+=1
            for label,pattern in PATTERNS.items():
                if pattern.search(data):problems.append(('working tree',name,label))
    objects=subprocess.check_output(['git','rev-list','--objects','HEAD'],cwd=ROOT).decode().splitlines()
    batch=subprocess.Popen(['git','cat-file','--batch'],cwd=ROOT,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
    for line in objects:
        parts=line.split(' ',1)
        if len(parts)!=2:continue
        oid,name=parts
        if forbidden_path(name):problems.append(('history',name,'excluded file type/path'))
        batch.stdin.write((oid+'\n').encode());batch.stdin.flush()
        header=batch.stdout.readline().split();size=int(header[2]);data=batch.stdout.read(size);batch.stdout.read(1)
        if header[1]!=b'blob':continue
        checked+=1
        for label,pattern in PATTERNS.items():
            if pattern.search(data):problems.append(('history',name,label))
    batch.stdin.close();batch.wait()
    for item in sorted(set(problems)):print(': '.join(item))
    print('Audited',checked,'files/history blobs;',len(set(problems)),'findings.')
    return bool(problems)

if __name__=='__main__':sys.exit(main())
