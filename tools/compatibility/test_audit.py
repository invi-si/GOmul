import io
import json
import pathlib
import hashlib
import subprocess
import tarfile
import tempfile
from unittest.mock import patch
import audit
import struct
import unittest
import zipfile
from audit import classify, native_members, scan_archive, ktf_members


class AuditTests(unittest.TestCase):
    def test_probe_keys_match_native_bridge_and_reject_silent_noops(self):
        import argparse
        self.assertEqual(audit.parse_keys('OK,RSOFT,LSOFT,1,*,#'), 'OK,R,L,1,*,#')
        self.assertEqual(audit.parse_keys(' left, down ,CLR,CALL'), 'LEFT,DOWN,CLR,CALL')
        self.assertEqual(audit.parse_keys('CLR,WAIT:500,OK'), 'CLR,WAIT:500,OK')
        for value in ['', 'OK,,1', 'RIGTH', 'WAIT:-1', 'WAIT:30001', 'WAIT:oops']:
            with self.assertRaises(argparse.ArgumentTypeError):
                audit.parse_keys(value)

    def test_resumed_sweep_retains_cached_tail_in_aggregate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            cases = [{'game': 'files/games/' + key + '/game.zip', 'classification': 'smoke-only-no-detected-error', 'log_capture_verified': True, 'speed': 3000, 'keys': 'OK,DOWN,OK,1', 'boot_samples': 1} for key in ['a', 'b']]
            for case in cases:
                folder = root / case['game'].split('/')[2]
                folder.mkdir()
                (folder / 'result.json').write_text(json.dumps(case))
            (root / 'boot.json').write_text(json.dumps(cases[:1]))
            def fake_adb(words, **kwargs):
                return '\n'.join(c['game'] for c in cases) if 'find files/games' in words[-1] else ''
            with patch('sys.argv', ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot', '--speed', '3000']), patch('subprocess.check_output', side_effect=fake_adb):
                audit.main()
            self.assertEqual(json.loads((root / 'boot.json').read_text()), cases)

    def test_verified_cached_case_is_reused_with_same_options(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            case = root / 'a'
            case.mkdir()
            saved = {'game': 'files/games/a/game.zip', 'classification': 'smoke-only-no-detected-error',
                     'log_capture_verified': True, 'speed': 3000, 'keys': 'OK,DOWN,OK,1', 'boot_samples': 1,
                     'capture_rescue': True, 'capture_boot_rescue': True, 'restart_after_stop': True}
            result_path = case / 'result.json'
            result_path.write_text(json.dumps(saved))
            result_mtime = result_path.stat().st_mtime_ns
            def fake_adb(words, **kwargs):
                return 'files/games/a/game.zip' if 'find files/games' in words[-1] else ''
            arguments = ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot', '--speed', '3000',
                         '--capture-rescue', '--capture-boot-rescue', '--restart-after-stop']
            with patch('sys.argv', arguments), patch('subprocess.check_output', side_effect=fake_adb) as adb:
                audit.main()
            self.assertEqual(json.loads(result_path.read_text()), saved)
            self.assertEqual(result_path.stat().st_mtime_ns, result_mtime)
            self.assertFalse(any('am instrument' in str(call) for call in adb.call_args_list))

    def test_cached_case_with_option_mismatch_refuses_reuse_and_preserves_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            case = root / 'a'
            case.mkdir()
            old = {'game': 'files/games/a/game.zip', 'classification': 'cached', 'log_capture_verified': True,
                   'speed': 3000, 'keys': 'OK,DOWN,OK,1', 'boot_samples': 1,
                   'capture_rescue': False, 'capture_boot_rescue': False, 'restart_after_stop': False}
            (case / 'result.json').write_text(json.dumps(old))
            artifact = case / 'instrumentation.txt'
            artifact.write_text('preserve me')
            original_result = (case / 'result.json').read_bytes()
            original_artifact = artifact.read_bytes()
            def fake_adb(words, **kwargs):
                return 'files/games/a/game.zip' if 'find files/games' in words[-1] else ''
            for option in ['--capture-rescue', '--restart-after-stop']:
                with self.subTest(option=option):
                    arguments = ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot', '--speed', '3000', option]
                    with patch('sys.argv', arguments), patch('subprocess.check_output', side_effect=fake_adb) as adb:
                        with self.assertRaisesRegex(RuntimeError, 'incomplete or different options'):
                            audit.main()
                    self.assertFalse(any('am instrument' in str(call) for call in adb.call_args_list))
                    self.assertEqual((case / 'result.json').read_bytes(), original_result)
                    self.assertEqual(artifact.read_bytes(), original_artifact)

    def test_incomplete_cached_result_refuses_reuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            case = root / 'a'
            case.mkdir()
            incomplete = {'game': 'files/games/a/game.zip', 'classification': 'cached', 'log_capture_verified': False,
                          'speed': 1000, 'keys': 'OK,DOWN,OK,1', 'boot_samples': 1}
            result_path = case / 'result.json'
            result_path.write_text(json.dumps(incomplete))
            original = result_path.read_bytes()
            def fake_adb(words, **kwargs):
                return 'files/games/a/game.zip' if 'find files/games' in words[-1] else ''
            with patch('sys.argv', ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot']), patch('subprocess.check_output', side_effect=fake_adb) as adb:
                with self.assertRaisesRegex(RuntimeError, 'incomplete or different options'):
                    audit.main()
            self.assertFalse(any('am instrument' in str(call) for call in adb.call_args_list))
            self.assertEqual(result_path.read_bytes(), original)

    def test_partial_case_artifacts_without_result_refuse_reuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            case = root / 'a'
            case.mkdir()
            artifact = case / 'boot.png'
            artifact.write_bytes(b'partial')
            def fake_adb(words, **kwargs):
                return 'files/games/a/game.zip' if 'find files/games' in words[-1] else ''
            with patch('sys.argv', ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot']), patch('subprocess.check_output', side_effect=fake_adb) as adb:
                with self.assertRaisesRegex(RuntimeError, 'artifacts but no result'):
                    audit.main()
            self.assertFalse(any('am instrument' in str(call) for call in adb.call_args_list))
            self.assertEqual(artifact.read_bytes(), b'partial')

    def test_early_rescue_and_save_survive_process_failure(self):
        def make_tar(filename, payload):
            archive = io.BytesIO()
            with tarfile.open(fileobj=archive, mode='w') as saved:
                item = tarfile.TarInfo(filename)
                item.size = len(payload)
                saved.addfile(item, io.BytesIO(payload))
            return archive.getvalue()
        boot_rescue_tar = make_tar('rescue-trace', b'pre-crash-rescue')
        isolated_save_tar = make_tar('save-state', b'disposable-save')
        marker = {'phase': 'after-async-start', 'attempts': 2, 'rescuePath': '/cache/boot-rescue'}
        calls = []

        def fake_adb(words, **kwargs):
            calls.append(words)
            if words[3] == 'exec-out':
                command = words[4]
                if ' -C cache/compatibility-audit/boot-rescue ' in command:
                    return boot_rescue_tar
                if ' -C cache/compatibility-audit/saves/case ' in command:
                    return isolated_save_tar
                self.fail(f'Unexpected exec-out source: {command}')
            if words[3] == 'logcat':
                return ''
            command = words[4]
            if 'find files/games' in command:
                return 'files/games/a/game.zip\n'
            if 'am instrument' in command:
                raise subprocess.CalledProcessError(1, words)
            if 'cat cache/compatibility-audit/process-id' in command:
                return '4321'
            if 'test -f cache/compatibility-audit/boot-rescue.json' in command:
                return 'yes\n'
            if 'cat cache/compatibility-audit/boot-rescue.json' in command:
                return json.dumps(marker)
            if 'test -d cache/compatibility-audit/boot-rescue' in command:
                return 'yes\n'
            return ''

        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            arguments = ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot', '--speed', '3000', '--capture-boot-rescue']
            with patch('sys.argv', arguments), patch('subprocess.check_output', side_effect=fake_adb):
                audit.main()
            case = root / 'a'
            report = json.loads((case / 'result.json').read_text())
            self.assertEqual(report['classification'], 'harness-or-process-failure')
            self.assertEqual(report['boot_rescue_metadata'], marker)
            self.assertEqual(report['boot_rescue_sha256'], hashlib.sha256(boot_rescue_tar).hexdigest())
            self.assertEqual(report['isolated_save_sha256'], hashlib.sha256(isolated_save_tar).hexdigest())
            self.assertEqual((case / 'boot-rescue.tar').read_bytes(), boot_rescue_tar)
            self.assertEqual((case / 'isolated-save.tar').read_bytes(), isolated_save_tar)
            self.assertNotEqual((case / 'boot-rescue.tar').read_bytes(), (case / 'isolated-save.tar').read_bytes())
            exec_out_sources = [words[4] for words in calls if words[3] == 'exec-out']
            self.assertIn('run-as local.wie.nativeapp tar -cf - -C cache/compatibility-audit/boot-rescue .', exec_out_sources)
            self.assertIn('run-as local.wie.nativeapp tar -cf - -C cache/compatibility-audit/saves/case .', exec_out_sources)
            instrumentation = next(words[4] for words in calls if words[3] == 'shell' and 'am instrument' in words[4])
            self.assertIn('captureBootRescue true', instrumentation)

    def test_background_failure_overrides_paints(self):
        samples = [{'status': 'Running', 'paints': 20}, {'status': 'Running', 'paints': 40}]
        self.assertEqual(classify(samples, ['Exception in background thread']), 'logged-error-review')
        self.assertEqual(classify(samples, []), 'smoke-only-no-detected-error')
        self.assertEqual(classify([{'status': 'Running', 'paints': 0}], []), 'no-paints-review')
        self.assertEqual(classify([{'status': 'Running', 'paints': 41}, {'status': 'Game stopped; Quick Load available.', 'paints': 42}], []), 'stopped-review')

    def test_symbol_scan_does_not_invent_method_owner(self):
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, 'w') as z:
            z.writestr('binary.mod', b'java/lang/Thread\0run\0()V\0')
        result = scan_archive(archive.getvalue(), {'java/lang/Thread': []})[0]
        self.assertEqual(result['class_symbols'], ['java/lang/Thread'])
        self.assertEqual(result['native_member_candidates'], [])

    def test_ktf_encoded_names_do_not_inherit_lgt_slot_guesses(self):
        data = b'\0\x03(I)V+set\0noise+notAMethod\0'
        references = ktf_members(data, {('set', '(I)V'): ['led.rs']})
        self.assertEqual(len(references), 1)
        self.assertEqual(references[0]['tag'], 3)
        self.assertEqual(references[0]['matching_source_prototypes'], ['led.rs'])
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, 'w') as z:
            z.writestr('client.bin1108', data)
            z.writestr('broken.jar', b'not a zip')
        found = scan_archive(archive.getvalue(), {}, supported={('set', '(I)V'): ['led.rs']})
        self.assertEqual(found[0]['native_member_candidates'], references)
        self.assertIn('error', found[1])

    def test_pointer_backed_member_and_invalid_pointer(self):
        data = bytearray(256)
        data[:6] = b'\x7fELF\x01\x01'
        struct.pack_into('<I', data, 32, 52)
        struct.pack_into('<HH', data, 46, 40, 1)
        struct.pack_into('<10I', data, 52, 0, 1, 0, 0x1000, 96, 160, 0, 0, 0, 0)
        struct.pack_into('<IIIHHIII', data, 96, 0, 0x1040, 0x1050, 1, 1, 0, 0, 0)
        data[160:164] = b'run\0'
        data[176:180] = b'()V\0'
        abi = {'java/lang/Thread': [{'name': 'run', 'descriptor': '()V', 'index': 11}]}
        records = native_members(data, abi)
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]['matching_explicit_mappings'], [{'class': 'java/lang/Thread', 'index': 11}])
        struct.pack_into('<I', data, 104, 0xffff0000)
        self.assertEqual(native_members(data, abi), [])


if __name__ == '__main__':
    unittest.main()
