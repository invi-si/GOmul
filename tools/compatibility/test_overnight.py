import unittest
import json
import pathlib
import subprocess
import tempfile
from unittest.mock import patch
from overnight import priority

class OvernightTests(unittest.TestCase):
    def case(self,**kw):
        return {'samples':[dict(phase='settled:'+str(i),status='Running',frameHash='x',**kw) for i in range(3)]}
    def test_repainting_black_screen_is_still_suspicious(self):
        self.assertTrue(priority(self.case(blackFraction=1,paints=100)).startswith('high:'))
    def test_white_screen_is_suspicious(self):
        self.assertTrue(priority(self.case(whiteFraction=.99)).startswith('high:'))
    def test_static_menu_is_not_a_crash(self):
        self.assertTrue(priority(self.case(blackFraction=.4)).startswith('review:'))
    def test_recovery_from_splash_is_not_blank_failure(self):
        c=self.case(blackFraction=.1);c['samples'].insert(0,dict(phase='boot:0',blackFraction=1))
        self.assertFalse(priority(c).startswith('high:'))
    def test_hidden_error_with_paints_is_low_priority(self):
        c=self.case(blackFraction=.1);c['errors']=['background Exception'];self.assertTrue(priority(c).startswith('low:'))
    def test_missing_frame_is_distinct(self):
        c={'samples':[{'phase':'settled:'+str(i)} for i in range(3)]};self.assertIn('no framebuffer',priority(c))

    def test_optional_rescue_error_is_not_game_failure(self):
        c=self.case(blackFraction=.1);c['rescueCaptureError']='Session recording is busy'
        self.assertTrue(priority(c).startswith('review:'))

    def test_reopened_final_samples_determine_blank_status(self):
        c=self.case(blackFraction=.1);c['samples']+=self.case(blackFraction=1)['samples']
        self.assertTrue(priority(c).startswith('high:'))

    def test_timeout_failure_records_speed_and_resumes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            plan = root / 'plan.json'
            plan.write_text(json.dumps({'games': [{'id': 'game-a', 'title': 'Game A', 'game': 'files/games/game-a/game.zip'}]}))
            output = root / 'output'
            commands = []

            def fake_run(command, **kwargs):
                commands.append(command)
                if 'audit.py' in command[1]:
                    raise subprocess.TimeoutExpired(command, 150)
                return subprocess.CompletedProcess(command, 0)

            args = ['overnight.py', '--plan', str(plan), '--output', str(output), '--serial', 'test',
                    '--restore-apk', 'ordinary.apk', '--speed', '1750']
            with patch('sys.argv', args), patch('subprocess.run', side_effect=fake_run):
                import overnight
                overnight.main()

            result_path = output / 'first' / 'game-a' / 'result.json'
            failure = json.loads(result_path.read_text())
            self.assertEqual(failure['classification'], 'harness-or-process-failure')
            self.assertEqual(failure['speed'], 1750)
            self.assertEqual(failure['keys'], 'OK,DOWN,OK,RSOFT,1')
            self.assertTrue((output / 'first' / 'game-a' / 'driver.log').exists())

            commands.clear()
            with patch('sys.argv', args), patch('subprocess.run', side_effect=fake_run):
                overnight.main()

            self.assertEqual(json.loads(result_path.read_text()), failure)
            self.assertFalse(any('audit.py' in command[1] for command in commands))
            resumed = json.loads((output / 'results.json').read_text())
            self.assertEqual(resumed[0]['id'], 'game-a')
            self.assertEqual(resumed[0]['priority'], 'high: process/harness failure (inspect evidence)')
