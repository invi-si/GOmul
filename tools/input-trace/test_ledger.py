import json,subprocess,sys,tempfile,unittest
from pathlib import Path
class LedgerTest(unittest.TestCase):
 def test_rank_and_timer_wait_do_not_double_count_wall_work(self):
  # Key queues behind a timer and redraw; pre-pop callback occupies a nested interval.
  rows=[(0,60,'I',1,5),(0,60,'I',2,1),(1000000,60,'I',3,2),(1000000,61,'I',3,7),(1000000,62,'I',3,2),
        (2000000,63,'I',1,5),(2000000,90,'I',3,2),(2000000,75,'B',100,1),(12000000,75,'E',100,0),
        (12000000,79,'B',101,0),(13000000,74,'B',102,101),(18000000,74,'E',102,0),(22000000,63,'I',2,1),
        (22000000,79,'E',101,0),(22000000,90,'I',3,1),(31000000,63,'I',3,2)]
  with tempfile.TemporaryDirectory() as d:
   f=Path(d)/'trace.csv';f.write_text('# monotonic_ns,tid,kind,phase,id,value; lost=0\n'+''.join(f'{t},1,{k},{p},{i},{v}\n' for t,k,p,i,v in rows))
   r=json.loads(subprocess.check_output([sys.executable,str(Path(__file__).with_name('ledger.py')),str(f)]))
  key=r['physical_keys'][0]
  self.assertEqual(key['pops_before_key'],{'Timer':1,'Redraw':1})
  self.assertEqual([x['rank'] for x in key['rank']],[3,2,1,0])
  self.assertEqual(sum(key['wall_ledger_ms'].values()),30)
  self.assertEqual(key['wall_ledger_ms']['timer callback'],10)
  self.assertEqual(key['wall_ledger_ms']['callSerially callback'],5)
  self.assertEqual(key['wall_ledger_ms']['pre-pop other'],5)
if __name__=='__main__':unittest.main()
