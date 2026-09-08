import json,subprocess,sys,tempfile,unittest
from pathlib import Path
class CensusTest(unittest.TestCase):
 def test_pending_duplicates_and_callback_successor_are_distinguished(self):
  rows=[]
  def point(t,k,i,v):rows.append((t,k,'I',i,v))
  def register(t,r,e,parent=0):
   for k,v in [(113,4096),(114,8193),(115,9),(116,20),(117,20),(118,0),(125,parent)]:point(t,k,r,v)
   point(t,110,e,r);point(t,60,e,5)
  register(0,10,100);register(1,11,101)
  point(2,61,200,7);point(2,60,200,2)
  point(3,119,50,4096)
  point(4,63,100,5);point(4,80,100,1)
  rows.append((4,120,'B',300,10));register(3000004,12,102,10)
  rows.append((33000004,120,'E',300,0))
  with tempfile.TemporaryDirectory() as d:
   f=Path(d)/'trace.csv';f.write_text('# monotonic_ns,tid,kind,phase,id,value; lost=0\n'+''.join(f'{t},1,{k},{ph},{i},{v}\n' for t,k,ph,i,v in rows))
   r=json.loads(subprocess.check_output([sys.executable,str(Path(__file__).with_name('timer_census.py')),str(f)]))
  self.assertEqual([e['ptr'] for e in r['keys'][0]['queue_ahead']],[4096,4096])
  self.assertEqual(r['logical_timer_objects'],1)
  self.assertEqual(r['peak_instances_per_ptr']['4096']['queued'],2)
  self.assertEqual(r['peak_instances_per_ptr']['4096']['in_flight'],1)
  self.assertEqual(next(o for o in r['operations'] if o['operation']=='Unset')['existing_instances'],{'queued':2})
  self.assertEqual(r['cancelled_registrations'],[10,11])
  self.assertEqual(r['callbacks_started_after_cancel'][0]['registration'],10)
  self.assertEqual(r['cancelled_registrations_rearming'],[12])
  child=r['callbacks'][0]['replacement_sets'][0]
  self.assertEqual((child['offset_ms'],child['callback_remaining_ms']),(3,30))
if __name__=='__main__':unittest.main()
