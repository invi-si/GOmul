import io
import json
import pathlib
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
        for value in ['', 'OK,,1', 'RIGTH', 'WAIT:500']:
            with self.assertRaises(argparse.ArgumentTypeError):
                audit.parse_keys(value)

    def test_resumed_sweep_retains_cached_tail_in_aggregate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            cases = [{'game': 'files/games/' + key + '/game.zip', 'classification': 'smoke-only-no-detected-error', 'log_capture_verified': True} for key in ['a', 'b']]
            for case in cases:
                folder = root / case['game'].split('/')[2]
                folder.mkdir()
                (folder / 'result.json').write_text(json.dumps(case))
            (root / 'boot.json').write_text(json.dumps(cases[:1]))
            def fake_adb(words, **kwargs):
                return '\n'.join(c['game'] for c in cases) if 'find files/games' in words[-1] else ''
            with patch('sys.argv', ['audit.py', '--serial', 'test', '--output', directory, '--phase', 'boot']), patch('subprocess.check_output', side_effect=fake_adb):
                audit.main()
            self.assertEqual(json.loads((root / 'boot.json').read_text()), cases)

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
