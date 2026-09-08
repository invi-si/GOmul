#!/usr/bin/env python3
"""Whole-AVD checkpoints for the local Mac controller; never edits game files."""
import argparse
import datetime
import fcntl
import json
import os
import re
import io
import tarfile
from pathlib import Path
import subprocess
import uuid


class Checkpoints:
    def __init__(self, adb, avd, directory, runner=None, game=None):
        self.adb, self.avd, self.directory = adb, avd, Path(directory)
        self.runner = runner or subprocess.run
        self.serial = None
        if game is not None and not re.fullmatch(r'[0-9a-f]{64}', game):
            raise ValueError('Invalid game identifier.')
        self.game = game
        self.path = self.directory / ('slots-' + game + '.json' if game else 'slots.json')

    def command(self, *args):
        result = self.runner([self.adb, *args], capture_output=True, text=True, timeout=120)
        output = result.stdout.strip()
        if result.returncode or any(line.startswith('KO:') for line in output.splitlines()):
            raise RuntimeError((output + '\n' + result.stderr).strip() or 'ADB command failed')
        return output

    def connect(self):
        devices = self.command('devices')
        for line in devices.splitlines()[1:]:
            parts = line.split()
            if len(parts) == 2 and parts[1] == 'device' and parts[0].startswith('emulator-'):
                name = self.command('-s', parts[0], 'emu', 'avd', 'name').splitlines()[0]
                if name == self.avd:
                    self.serial = parts[0]
                    return
        raise RuntimeError('Open the Android emulator ' + self.avd + ' first. Physical phones are not supported by this controller.')

    def snapshot(self, action, *args):
        return self.command('-s', self.serial, 'emu', 'avd', 'snapshot', action, *args)

    def read(self):
        if not self.path.exists():
            return {}
        value = json.loads(self.path.read_text())
        if value.get('avd') != self.avd:
            raise RuntimeError('Checkpoint metadata belongs to a different Android virtual device.')
        return value

    def write(self, value):
        value['avd'] = self.avd
        temporary = self.path.with_suffix('.tmp')
        with temporary.open('w') as stream:
            json.dump(value, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, self.path)

    def create(self, label):
        tag = 'wie-' + label + '-' + uuid.uuid4().hex
        self.snapshot('save', tag)
        if tag not in self.snapshot('list').split():
            raise RuntimeError('Android did not list the new snapshot; the previous checkpoint is retained.')
        return {'tag': tag, 'saved': datetime.datetime.now().astimezone().isoformat(timespec='seconds')}

    def available(self, entry, listing):
        return bool(entry and entry.get('tag') in listing.split())

    def perform(self, action):
        self.directory.mkdir(parents=True, exist_ok=True)
        with (self.directory / 'controller.lock').open('a') as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise RuntimeError('Another checkpoint operation is running. Please wait.')
            self.connect()
            state = self.read()
            listing = self.snapshot('list')
            if action == 'pin':
                if state.get('startup'):
                    raise RuntimeError('A protected startup checkpoint already exists.')
                state['startup'] = self.create('startup')
                self.write(state)
                message = 'Protected startup checkpoint saved.'
            elif action == 'clear':
                for key in ('quick', 'recovery'):
                    state.pop(key, None)
                self.write(state)
                message = 'Quick Load and Undo Load slots cleared. Startup retained.'
            elif action == 'boot':
                target = state.get('startup')
                if not target:
                    return {'startNormally': True}
                if not self.available(target, listing):
                    raise RuntimeError('The protected startup snapshot is unavailable.')
                preserved = self.preserve_other_games()
                self.snapshot('load', target['tag'])
                self.restore_other_games(preserved)
                message = 'Protected startup restored.'
            elif action == 'save':
                old = state.get('quick')
                state['quick'] = self.create('quick')
                self.write(state)
                self.prune(old, state)
                message = 'Quick Save complete.'
            elif action in ('load', 'recover'):
                key = 'quick' if action == 'load' else 'recovery'
                target = state.get(key)
                if not self.available(target, listing):
                    raise RuntimeError('No ' + ('Quick Save' if key == 'quick' else 'pre-load recovery') + ' checkpoint is available.')
                old_recovery = state.get('recovery')
                state['recovery'] = self.create('recovery')
                # Publish recovery BEFORE loading: a failed/interrupted load is recoverable.
                self.write(state)
                preserved = self.preserve_other_games() if self.game else None
                self.snapshot('load', target['tag'])
                if preserved is not None:
                    self.restore_other_games(preserved)
                self.prune(old_recovery, state)
                message = 'Quick Load complete.' if action == 'load' else 'Previous session restored.'
            else:
                message = 'Connected to ' + self.avd + '.'
            listing = self.snapshot('list')
            return {'message': message, 'quick': state.get('quick'), 'recovery': state.get('recovery'),
                    'canLoad': self.available(state.get('quick'), listing),
                    'canRecover': self.available(state.get('recovery'), listing)}

    def device_bytes(self, *args, data=None):
        result = self.runner([self.adb, '-s', self.serial, *args], input=data,
                             capture_output=True, timeout=120)
        if result.returncode:
            raise RuntimeError(result.stderr.decode(errors='replace') or 'Game data transfer failed.')
        return result.stdout

    def preserve_other_games(self):
        # Only the selected game's runtime is active. Other games have disk state only.
        data = self.device_bytes('exec-out', 'run-as', 'local.wie.nativeapp',
                                 'tar', '-cf', '-', 'files/saves', 'files/games', 'files/mac-checkpoints.json')
        output = io.BytesIO()
        with tarfile.open(fileobj=io.BytesIO(data)) as source, tarfile.open(fileobj=output, mode='w') as target:
            for member in source:
                parts = member.name.rstrip('/').split('/')
                if parts[:3] == ['files', 'saves', self.game]:
                    continue
                target.addfile(member, source.extractfile(member) if member.isfile() else None)
        archive = self.directory / ('preserved-' + self.game + '.tar')
        with archive.open('wb') as stream:
            stream.write(output.getvalue()); stream.flush(); os.fsync(stream.fileno())
        return archive

    def restore_other_games(self, archive):
        self.command('-s', self.serial, 'wait-for-device')
        # Remove snapshot-era directories, including saves deleted since that snapshot.
        names = self.command('-s', self.serial, 'shell', 'run-as', 'local.wie.nativeapp', 'ls', 'files/saves').splitlines()
        for name in names:
            if re.fullmatch(r'[0-9a-f]{64}', name) and name != self.game:
                self.command('-s', self.serial, 'shell', 'run-as', 'local.wie.nativeapp', 'rm', '-rf', 'files/saves/' + name)
        self.command('-s', self.serial, 'shell', 'run-as', 'local.wie.nativeapp', 'rm', '-rf', 'files/games')
        self.device_bytes('shell', 'run-as', 'local.wie.nativeapp', 'tar', '-xf', '-', data=archive.read_bytes())
        # Keep the host archive for recovery if a subsequent disk operation fails.

    def prune(self, old, state):
        if old and old['tag'].startswith('wie-') and old['tag'] not in [v.get('tag') for v in state.values() if isinstance(v, dict)]:
            try:
                self.snapshot('delete', old['tag'])
            except (RuntimeError, subprocess.TimeoutExpired):
                pass  # A leftover snapshot is preferable to losing a committed checkpoint.


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['status', 'save', 'load', 'recover'])
    parser.add_argument('--config', required=True)
    args = parser.parse_args()
    try:
        config = json.loads(Path(args.config).read_text())
        result = Checkpoints(config['adb'], config['avd'], config['stateDirectory']).perform(args.action)
        print(json.dumps(result))
    except Exception as error:
        print(json.dumps({'error': str(error)}))
        raise SystemExit(1)


if __name__ == '__main__':
    main()
