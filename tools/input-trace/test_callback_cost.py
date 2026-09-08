import tempfile
import unittest
from pathlib import Path
from callback_cost import analyze

class CallbackCostTest(unittest.TestCase):
    def test_completed_stage_join_and_cancelled_stage_exclusion(self):
        rows = ['0,1,110,I,9,8','0,1,113,I,8,4096','0,1,116,I,8,60',
                '1,1,75,B,7,9','1000001,1,75,E,7,0']
        rows += [f'1000001,1,{k},I,7,{v}' for k,v in [(130,500000),(131,200000),(132,2),(133,100),(134,1),(135,1),(136,0)]]
        rows += ['1000002,1,75,B,10,11','1000003,1,75,E,10,0','1000004,1,126,I,11,0']
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'trace.csv';p.write_text('# lost=0\n'+'\n'.join(rows))
            result=analyze(p)
        self.assertEqual(len(result['callbacks']),1)
        self.assertEqual(result['callbacks'][0]['registration'],8)
        self.assertEqual(result['callbacks'][0]['timer']['timeout_ms'],60)
        self.assertEqual(result['summary']['cpu_exclusive_ns']['median'],0.5)

if __name__ == '__main__': unittest.main()
