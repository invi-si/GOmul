import unittest
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
