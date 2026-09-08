import tempfile
import unittest
from pathlib import Path
from native_callback_samples import active_polls, samples

class NativeSampleTest(unittest.TestCase):
    def test_active_poll_excludes_suspension_and_preserves_thread(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'trace';p.write_text('# lost=0\n0,7,75,B,100,9\n1,7,84,B,101,100\n300001,7,84,E,101,0\n900000,7,84,B,102,100\n1200000,7,84,E,102,1\n1200001,7,75,E,100,0\n')
            self.assertEqual(active_polls(p),{7:[(1,300001),(900000,1200000)]})
    def test_leaf_sample_does_not_get_overwritten_by_nested_callchain(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'sample';p.write_text('sample:\n  time: 12\n  thread_id: 7\n  symbol: leaf\n    symbol: caller\n')
            self.assertEqual(samples(p)[0]['symbol'],'leaf')
if __name__=='__main__':unittest.main()
