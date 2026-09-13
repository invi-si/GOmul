import io
import subprocess
import sys
import time
from unittest.mock import patch
import struct
import tempfile
import unittest
import zipfile
import zlib
from pathlib import Path
from native import Native, archive_files, companion, frame_png, prepare_archive


def archive(entries):
    result = io.BytesIO()
    with zipfile.ZipFile(result, 'w') as z:
        for name, data in entries: z.writestr(name, data)
    return result.getvalue()


class NativeTests(unittest.TestCase):
    def test_ktf_wrapper_and_prefixed_zip_are_normalized(self):
        for prefix in (b'', b'\xff\xd8image-prefix'):
            wrapped = prefix + archive([('folder/__adf__', b'metadata'), ('folder/game.jar', b'code'), ('folder/P/data', b'resource')])
            normalized, files = prepare_archive(wrapped)
            self.assertTrue(normalized.startswith(b'PK\x03\x04'))
            self.assertEqual(set(files), {'__adf__','game.jar','P/data'})
            self.assertEqual(archive_files(normalized),files)
            with zipfile.ZipFile(io.BytesIO(normalized)) as canonical:
                self.assertTrue(all(item.date_time == (1980, 1, 1, 0, 0, 0) for item in canonical.infolist()))
            reordered = prefix + archive([('folder/P/data', b'resource'), ('folder/game.jar', b'code'), ('folder/__adf__', b'metadata')])
            self.assertEqual(prepare_archive(reordered)[0], normalized)
        with self.assertRaises(ValueError):
            prepare_archive(archive([('a/__adf__',b'a'),('b/__adf__',b'b')]))

    def test_wrapped_game_is_normalized_but_raw_game_is_preserved(self):
        raw = archive([('app_info', b'PID:test'), ('game.jar', b'content')])
        self.assertEqual(prepare_archive(raw)[0], raw)
        normalized, files = prepare_archive(archive([('folder/app_info', b'PID:test'), ('folder/game.jar', b'content')]))
        self.assertEqual(set(archive_files(normalized)), {'app_info', 'game.jar'})
        self.assertEqual(files['game.jar'], b'content')

    def test_rejects_unsafe_and_ambiguous_archives(self):
        for entries in ([('../outside', b'a')], [('/absolute', b'a')], [('a\\b', b'a')], [('a/app_info', b'a'), ('b/app_info', b'b')]):
            with self.assertRaises(ValueError): prepare_archive(archive(entries))

    def test_companion_imports_known_identity_and_preserves_earned_data(self):
        with tempfile.TemporaryDirectory() as temp:
            save = Path(temp)/'saves'/'game'
            files = {'app_info': b'PID:PD121120\r\n', 'data/savedata': b'initial', 'data/it0016': b'item'}
            result = companion(save, files, '')
            self.assertEqual(result['phone'], '01055145031')
            record = save/'PD121120/db/savedata/1'
            self.assertEqual(record.read_bytes(), b'initial')
            record.write_bytes(b'earned')
            (save/'companion-imported').unlink()
            self.assertIsNone(companion(save, files, ''))
            self.assertEqual(record.read_bytes(), b'earned')

    def test_unknown_companion_requires_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            save = Path(temp)/'game'
            files = {'app_info': b'PID:OTHER\n', 'data/savedata': b'initial'}
            self.assertEqual(companion(save, files, ''), {'needsPhone': True})
            self.assertFalse(save.exists())
            self.assertTrue(companion(save, files, '01012345678')['imported'])

    def test_rescue_png_has_correct_channel_order(self):
        frame = struct.pack('<IIQB', 1, 1, 7, 0) + bytes([30, 20, 10, 255])
        png = frame_png(frame)
        self.assertTrue(png.startswith(b'\x89PNG\r\n\x1a\n'))
        offset = png.index(b'IDAT')
        length = struct.unpack('>I', png[offset-4:offset])[0]
        self.assertEqual(zlib.decompress(png[offset+4:offset+4+length]), bytes([0,10,20,30,255]))
        with self.assertRaises(ValueError): frame_png(frame[:-1])

    def test_stale_session_cannot_control_another_game(self):
        native = Native(None)
        native.token = 'current'
        with self.assertRaises(ValueError): native.command({'session':'old','op':'stop'})
        self.assertEqual(native.token, 'current')
        native.close()

    def test_settings_are_game_specific(self):
        with tempfile.TemporaryDirectory() as temp:
            native = Native(None, Path(temp))
            native.game = 'game-a'
            native.write_settings({'speed':2000, 'width':240, 'height':380})
            self.assertEqual(native.settings('game-a')['speed'], 2000)
            self.assertEqual(native.settings('game-b')['speed'], 1000)
            native.close()

class NativeTransportTests(unittest.TestCase):
    def helper(self, script):
        native = Native(None)
        native.process = subprocess.Popen([sys.executable, '-u', '-c', script], stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0)
        self.addCleanup(native.close)
        return native

    def test_fragmented_reply_is_reassembled(self):
        native = self.helper('''import sys,time; input(); sys.stdout.write('{"ok":'); sys.stdout.flush(); time.sleep(.02); print('true}')''')
        self.assertEqual(native.rpc({'op':'poll'}), {'ok':True})

    def test_partial_reply_has_a_deadline_and_invalidates_the_session(self):
        native = self.helper('''import sys,time; input(); sys.stdout.write('{"ok":'); sys.stdout.flush(); time.sleep(2)''')
        native.token = 'old'
        start = time.monotonic()
        with patch('native.RPC_TIMEOUT', .08), self.assertRaises(ValueError):native.rpc({'op':'poll'})
        self.assertLess(time.monotonic()-start, 4)
        self.assertIsNone(native.process)
        self.assertIsNone(native.token)

    def test_valid_command_error_preserves_pipe_for_next_command(self):
        native = self.helper('''input(); print('{"error":"incompatible checkpoint"}'); input(); print('{"ok":true}')''')
        with self.assertRaisesRegex(ValueError, 'incompatible checkpoint'):native.rpc({'op':'checkpoint'})
        self.assertIsNotNone(native.process)
        self.assertEqual(native.rpc({'op':'poll'}), {'ok':True})

    def test_oversized_reply_is_rejected(self):
        native = self.helper('''input(); print('{"value":"' + 'x'*100 + '"}')''')
        with patch('native.RPC_LIMIT', 40), self.assertRaises(ValueError):native.rpc({'op':'poll'})
        self.assertIsNone(native.process)

if __name__ == '__main__': unittest.main()
