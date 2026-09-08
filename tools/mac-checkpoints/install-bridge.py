#!/usr/bin/env python3
"""Provision GOmul's optional Mac AVD checkpoint helper."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import re
import secrets
import shutil
import subprocess
import sys
from checkpoints import Checkpoints


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--avd',required=True,help='Name of the running Android Virtual Device')
    parser.add_argument('--adb',default=shutil.which('adb'))
    args=parser.parse_args()
    if sys.platform!='darwin':parser.error('The checkpoint helper currently supports macOS only.')
    if not args.adb:parser.error('Install Android platform-tools and put adb on PATH, or pass --adb.')
    if not re.fullmatch(r'[A-Za-z0-9_.-]+',args.avd):parser.error('Invalid AVD name.')
    directory=Path.home()/'Library/Application Support/GOmul'/args.avd
    directory.mkdir(parents=True,exist_ok=True)
    path=directory/'bridge.json'
    config=json.loads(path.read_text()) if path.exists() else {'port':18764,'token':secrets.token_hex(32)}
    config.update(adb=str(Path(args.adb).resolve()),avd=args.avd,stateDirectory=str(directory))
    path.write_text(json.dumps(config));path.chmod(0o600)
    source=Path(__file__).resolve().parent
    for name in ('bridge.py','checkpoints.py'):shutil.copy2(source/name,directory/name)
    controller=Checkpoints(config['adb'],args.avd,directory);controller.connect()
    controller.device_bytes('shell','run-as','local.wie.nativeapp','sh','-c',"'cat > files/mac-checkpoints.json'",data=json.dumps({'port':config['port'],'token':config['token']}).encode())
    label='io.github.invi-si.gomul.checkpoints'
    plist=Path.home()/'Library/LaunchAgents'/(label+'.plist');plist.parent.mkdir(parents=True,exist_ok=True)
    with plist.open('wb') as stream:plistlib.dump({'Label':label,'ProgramArguments':[sys.executable,str(directory/'bridge.py'),'--config',str(path)],'RunAtLoad':True,'KeepAlive':True,'StandardErrorPath':str(directory/'bridge.log')},stream)
    service='gui/'+str(os.getuid())
    subprocess.run(['launchctl','bootout',service+'/'+label],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    subprocess.run(['launchctl','bootstrap',service,str(plist)],check=True)
    print('Installed. Setup commands should use --config '+str(path))

if __name__=='__main__':main()
