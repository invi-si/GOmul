#!/usr/bin/env python3
"""Frozen-control-only normal/skill alternation with recorded macOS host activation."""
import argparse,datetime,hashlib,json,pathlib,re,statistics,subprocess,time
from avd_host import activate
p=argparse.ArgumentParser(description=__doc__)
for name in ['adb','control','normal','skill','out']:p.add_argument('--'+name,required=True)
p.add_argument('--serial',default='emulator-5554');p.add_argument('--avd-host-pid',required=True,type=int)
p.add_argument('--pairs',type=int,default=6);p.add_argument('--pair-gap',type=float,default=10)
a=p.parse_args();out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=False)
base='/data/local/tmp/gomul-control-stability'
def adb(*args):return subprocess.check_output([a.adb,'-s',a.serial,*args],text=True,stderr=subprocess.STDOUT)
adb('shell','mkdir -p '+base);manifest={}
for name in ['control','normal','skill']:
 path=pathlib.Path(getattr(a,name));manifest[name]=hashlib.sha256(path.read_bytes()).hexdigest()
 target=base+'/'+name+('.json' if name!='control' else '')
 adb('push',str(path),target)
 if name=='control':adb('shell','chmod 755 '+target)
manifest['device_hashes']=adb('shell','sha256sum '+base+'/control '+base+'/normal.json '+base+'/skill.json')
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
rows=[]
for pair in range(a.pairs):
 for workload in ['normal','skill']:
  host=activate(a.avd_host_pid);start=time.monotonic()
  text=adb('shell',f'time {base}/control {base}/{workload}.json 3')
  (out/f'{pair:02d}-{workload}.txt').write_text(text)
  values=[int(n)/1e6 for n in re.findall(r'cpu_ns=(\d+)',text)]
  if len(values)!=3 or text.count('validated=true')!=3 or text.count('timing_eligible=true')!=3:raise ValueError('Unexpected validation/instrumentation output')
  row={'pair':pair,'workload':workload,'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'cpu_ms':values,'median_ms':statistics.median(values),'process_wall_sec':time.monotonic()-start,'host':host}
  rows.append(row);(out/'rounds.json').write_text(json.dumps(rows,indent=2)+'\n');print(workload,row['median_ms'],flush=True)
 if pair+1<a.pairs:time.sleep(a.pair_gap)
summary={}
for w in ['normal','skill']:
 v=[r['median_ms'] for r in rows if r['workload']==w]
 summary[w]={'process_medians_ms':v,'median_ms':statistics.median(v),'min_ms':min(v),'max_ms':max(v),'cv_percent':100*statistics.stdev(v)/statistics.mean(v) if len(v)>1 else None}
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
