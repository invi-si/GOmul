#!/usr/bin/env python3
"""Permanent Android transcript gate: both correctness checks before any timing.
Run with the gameplay app paused. Fixtures/results contain private guest state.
"""
import argparse, hashlib, json, pathlib, re, statistics, subprocess, sys
from avd_host import activate
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--avd-host-pid',type=int,help='Required for a macOS AVD: activate its host before each process and record host state')
p.add_argument('--adb',required=True);p.add_argument('--serial',default='emulator-5554');p.add_argument('--out',required=True)
for name in ['control','candidate','control-verify','candidate-verify','normal','skill']: p.add_argument('--'+name,required=True)
a=p.parse_args()
if sys.platform=='darwin' and a.serial.startswith('emulator-') and not a.avd_host_pid: p.error('Supply --avd-host-pid for controlled macOS AVD activation')
out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
base='/data/local/tmp/gomul-transcript-gate'
def adb(*args): return subprocess.run([a.adb,'-s',a.serial,*args],capture_output=True,text=True,check=True).stdout
def execute(variant,workload,rounds,log,verify):
 if a.avd_host_pid:
  (out/(log+'.host.json')).write_text(json.dumps(activate(a.avd_host_pid),indent=2)+'\n')
 text=adb('shell',f'{base}/{variant} {base}/{workload}.json {rounds}')
 (out/log).write_text(text)
 lines=[line for line in text.splitlines() if line.startswith('round=')]
 if len(lines)!=rounds or any('validated=true' not in line for line in lines): raise ValueError('Missing validated rounds')
 expected='false' if verify else 'true'
 if any('timing_eligible='+expected not in line for line in lines): raise ValueError('Wrong instrumentation build')
 return [int(re.search(r'cpu_ns=(\d+)',line)[1])/1e6 for line in lines]
adb('shell','mkdir -p '+base)
manifest={}
for name in ['control','candidate','control-verify','candidate-verify','normal','skill']:
 path=pathlib.Path(getattr(a,name.replace('-','_')))
 manifest[name]={'path':str(path.resolve()),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
 target=base+'/'+name+('.json' if name in ['normal','skill'] else '')
 adb('push',str(path),target)
 if name not in ['normal','skill']: adb('shell','chmod 755 '+target)
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
for workload in ['normal','skill']:
 for variant in ['control-verify','candidate-verify']:
  execute(variant,workload,1,f'{workload}-{variant}.txt',True)
  print(f'{workload} {variant}: passed',flush=True)
summary={}
for workload in ['normal','skill']:
 groups={'control':[],'candidate':[]}
 for i,variant in enumerate(['control','candidate','candidate','control']*2):
  groups[variant].append(execute(variant,workload,3,f'{workload}-{i:02}-{variant}.txt',False))
 summary[workload]={}
 for variant,values in groups.items():
  all_values=sum(values,[])
  summary[workload][variant]={'median_ms':statistics.median(all_values),'min_ms':min(all_values),'max_ms':max(all_values),'process_medians_ms':[statistics.median(v) for v in values]}
 s=summary[workload];s['reduction_percent']=100*(1-s['candidate']['median_ms']/s['control']['median_ms'])
 print(workload,json.dumps(s),flush=True)
summary['meets_median_threshold']=all(summary[w]['reduction_percent']>=5 for w in ['normal','skill'])
summary['consistent_process_medians']=all(max(summary[w]['candidate']['process_medians_ms']) < min(summary[w]['control']['process_medians_ms']) for w in ['normal','skill'])
summary['eligible_for_live_validation']=summary['meets_median_threshold'] and summary['consistent_process_medians']
summary['note']='Threshold and non-overlapping process medians are a screening gate, not proof of live gameplay or physical-device benefit.'
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
