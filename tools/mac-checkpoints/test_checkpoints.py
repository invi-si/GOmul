import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from checkpoints import Checkpoints


class FakeADB:
    def __init__(self):
        self.tags = set()
        self.calls = []
        self.fail = None

    def __call__(self, argv, **kwargs):
        args = argv[1:]
        self.calls.append(args)
        output = 'OK'
        if args == ['devices']:
            output = 'List of devices attached\nphysical123\tdevice\nemulator-5554\tdevice\n'
        elif args[-3:] == ['emu', 'avd', 'name']:
            output = 'test-avd\nOK'
        else:
            i = args.index('snapshot')
            action = args[i + 1]
            if action == self.fail:
                output = 'KO: simulated failure'  # Console errors can have exit status zero.
            elif action == 'save':
                self.tags.add(args[i + 2])
            elif action == 'delete':
                self.tags.remove(args[i + 2])
            elif action == 'list':
                output = '\n'.join('-- ' + tag + ' 72M' for tag in self.tags) + '\nOK'
        return SimpleNamespace(returncode=0, stdout=output, stderr='')


class CheckpointTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.fake = FakeADB()
        self.controller = Checkpoints('adb', 'test-avd', self.temp.name, self.fake)

    def test_failed_save_preserves_previous_checkpoint(self):
        first = self.controller.perform('save')['quick']
        self.fake.fail = 'save'
        with self.assertRaises(RuntimeError):
            self.controller.perform('save')
        self.assertEqual(self.controller.read()['quick'], first)
        self.assertIn(first['tag'], self.fake.tags)

    def test_missing_snapshot_does_not_load_or_create_recovery(self):
        self.controller.perform('save')
        self.fake.tags.clear()
        with self.assertRaises(RuntimeError):
            self.controller.perform('load')
        self.assertNotIn('recovery', self.controller.read())
        self.assertFalse(any('load' in call for call in self.fake.calls))

    def test_failed_recovery_save_prevents_load(self):
        self.controller.perform('save')
        self.fake.fail = 'save'
        with self.assertRaises(RuntimeError):
            self.controller.perform('load')
        self.assertFalse(any('load' in call for call in self.fake.calls))

    def test_failed_load_keeps_recovery(self):
        self.controller.perform('save')
        self.fake.fail = 'load'
        with self.assertRaises(RuntimeError):
            self.controller.perform('load')
        self.assertIn(self.controller.read()['recovery']['tag'], self.fake.tags)

    def test_undo_and_slot_rotation_preserve_quick_save(self):
        quick = self.controller.perform('save')['quick']['tag']
        before = self.controller.perform('load')['recovery']['tag']
        self.controller.perform('recover')
        self.assertEqual([call[-1] for call in self.fake.calls if 'load' in call][-1], before)
        self.assertIn(quick, self.fake.tags)
        self.assertEqual(len(self.fake.tags), 2)
        self.assertFalse(any('physical123' in call for call in self.fake.calls))


if __name__ == '__main__':
    unittest.main()

class PerGameTests(unittest.TestCase):
    def test_slots_and_recovery_are_independent(self):
        with tempfile.TemporaryDirectory() as directory:
            fake = FakeADB()
            a = Checkpoints('adb', 'test-avd', directory, fake, game='a'*64)
            b = Checkpoints('adb', 'test-avd', directory, fake, game='b'*64)
            a.preserve_other_games = lambda: 'archive'
            a.restore_other_games = lambda archive: None
            a_quick = a.perform('save')['quick']
            b_quick = b.perform('save')['quick']
            a.perform('load')
            self.assertEqual(b.read()['quick'], b_quick)
            self.assertNotIn('recovery', b.read())
            a.perform('save')
            self.assertIn(b_quick['tag'], fake.tags)
            self.assertNotEqual(a.read()['quick'], a_quick)
            self.assertFalse((Path(directory)/'slots.json').exists())

    def test_invalid_game_id_rejected(self):
        with self.assertRaises(ValueError):
            Checkpoints('adb', 'test-avd', '/unused', game='../other')

    def test_preservation_excludes_active_game_and_keeps_other_data(self):
        import io
        import tarfile
        with tempfile.TemporaryDirectory() as directory:
            source = io.BytesIO()
            entries = {'files/saves/'+'a'*64+'/state': b'old-a',
                       'files/saves/'+'b'*64+'/state': b'new-b',
                       'files/games/b/game.jar': b'archive',
                       'files/mac-checkpoints.json': b'config'}
            with tarfile.open(fileobj=source, mode='w') as tar:
                for name, data in entries.items():
                    info = tarfile.TarInfo(name); info.size = len(data)
                    tar.addfile(info, io.BytesIO(data))
            controller = Checkpoints('adb','test-avd',directory,game='a'*64)
            controller.device_bytes = lambda *args, **kwargs: source.getvalue()
            archive = controller.preserve_other_games()
            with tarfile.open(archive) as tar:
                self.assertNotIn('files/saves/'+'a'*64+'/state',tar.getnames())
                self.assertEqual(tar.extractfile('files/saves/'+'b'*64+'/state').read(), b'new-b')
                self.assertIn('files/games/b/game.jar',tar.getnames())
            commands=[]; transfers=[]
            def command(*args):
                commands.append(args)
                return '\n'.join(['a'*64,'b'*64,'c'*64]) if args[-2:]==('ls','files/saves') else ''
            controller.command=command
            controller.device_bytes=lambda *args, **kwargs: transfers.append(kwargs['data'])
            controller.restore_other_games(archive)
            removed=[args[-1] for args in commands if 'rm' in args]
            self.assertNotIn('files/saves/'+'a'*64,removed)
            self.assertIn('files/saves/'+'c'*64,removed)
            self.assertEqual(transfers,[archive.read_bytes()])

class StartupTests(unittest.TestCase):
    def test_startup_is_protected_from_save_clear_and_repin(self):
        with tempfile.TemporaryDirectory() as directory:
            fake=FakeADB()
            controller=Checkpoints('adb','test-avd',directory,fake,game='a'*64)
            controller.preserve_other_games=lambda: 'archive'
            controller.restore_other_games=lambda archive: None
            self.assertTrue(controller.perform('boot')['startNormally'])
            controller.perform('pin')
            startup=controller.read()['startup']
            controller.perform('save')
            controller.perform('load')
            controller.perform('clear')
            self.assertEqual(controller.read()['startup'],startup)
            self.assertNotIn('quick',controller.read())
            self.assertNotIn('recovery',controller.read())
            with self.assertRaises(RuntimeError):controller.perform('pin')
            controller.perform('boot')
            self.assertEqual([call[-1] for call in fake.calls if 'load' in call][-1],startup['tag'])
            self.assertIn(startup['tag'],fake.tags)
