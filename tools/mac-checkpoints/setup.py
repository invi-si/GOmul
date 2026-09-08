#!/usr/bin/env python3
"""Import user-supplied LGT data and create a local protected startup checkpoint."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile
import zipfile
from checkpoints import Checkpoints


def package_info(game):
    data=game.read_bytes()
    if game.suffix.lower()!='.zip':
        raise ValueError('Use the complete LGT ZIP with app_info and the application JAR.')
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        info=archive.read('app_info').decode('euc-kr',errors='replace')
        pid=next((line[4:].strip() for line in info.splitlines() if line.startswith('PID:')),None)
        if not pid or not re.fullmatch(r'[A-Za-z0-9_-]+',pid):raise ValueError('Missing or invalid LGT PID.')
        if not any(name.endswith('.jar') for name in archive.namelist()):raise ValueError('The ZIP contains no application JAR.')
    return hashlib.sha256(data).hexdigest(),pid,data


def build_import(game,data_directory,phone):
    game_id,pid,game_bytes=package_info(game)
    if not re.fullmatch(r'[0-9]{11}',phone):raise ValueError('Enter the 11-digit emulated phone number without hyphens.')
    files=list(data_directory.iterdir())
    if not files:raise ValueError('The data directory is empty.')
    output=io.BytesIO()
    with tarfile.open(fileobj=output,mode='w') as archive:
        def add(name,data):
            info=tarfile.TarInfo(name);info.size=len(data);info.mode=0o600
            archive.addfile(info,io.BytesIO(data))
        add('files/games/'+game_id+'/'+game.name,game_bytes)
        save='files/saves/'+game_id+'/'
        for path in files:
            if not path.is_file() or path.is_symlink() or not re.fullmatch(r'[A-Za-z0-9_.-]{1,31}',path.name) or path.name in ('.','..'):
                raise ValueError('Choose the flat data folder, containing only its original data files.')
            add(save+pid+'/db/'+path.name+'/1',path.read_bytes())
        add(save+'phone-number.txt',(phone+'\n').encode())
    return game_id,output.getvalue()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=['import','pin','status','clear'])
    parser.add_argument('--config',required=True,type=Path)
    parser.add_argument('--game',required=True,type=Path)
    parser.add_argument('--data',type=Path)
    parser.add_argument('--phone-number',help='Identity specified for your supplied data; no number is bundled')
    args=parser.parse_args()
    config=json.loads(args.config.read_text())
    game_id,_,_=package_info(args.game)
    controller=Checkpoints(config['adb'],config['avd'],config['stateDirectory'],game=game_id)
    if args.action=='import':
        if not args.data or not args.phone_number:parser.error('import needs --data and --phone-number')
        _,data=build_import(args.game,args.data,args.phone_number)
        controller.connect()
        # Reject replacement instead of guessing whether a user's existing progress is disposable.
        existing=controller.command('-s',controller.serial,'shell','run-as','local.wie.nativeapp','ls','files')
        if 'saves' in existing.split() and game_id in controller.command('-s',controller.serial,'shell','run-as','local.wie.nativeapp','ls','files/saves').split():
            raise RuntimeError('This game already has saved data. Back it up and remove it explicitly before importing a replacement.')
        controller.command('-s',controller.serial,'shell','am','force-stop','local.wie.nativeapp')
        controller.device_bytes('shell','run-as','local.wie.nativeapp','tar','-xf','-',data=data)
        controller.command('-s',controller.serial,'shell','am','start','-n','local.wie.nativeapp/.MainActivity')
        print('Imported locally. Open the imported game, reach the working menu, then run this command with action pin.')
    else:
        if args.action=='pin':
            controller.connect()
            active=controller.device_bytes('exec-out','run-as','local.wie.nativeapp','cat','files/active-game.txt').decode().strip()
            if active!=game_id:raise RuntimeError('Open this exact game and reach its working menu before pinning.')
        print(json.dumps(controller.perform(args.action),indent=2))

if __name__=='__main__':main()
