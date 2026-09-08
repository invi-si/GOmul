#!/usr/bin/env python3
"""Join exported Perfetto worker states/frame tokens to native monotonic records.
Requires zero BOOTTIME–MONOTONIC offset, as checked on this unsuspended AVD.
"""
import csv,json,sys
from pathlib import Path
from collections import defaultdict
root=Path(sys.argv[1]);points=defaultdict(list);starts={};tasks=[]
with (root/'inputs.csv').open() as f:
    header=next(f)
    if 'lost=0' not in header:raise ValueError('Lost native records')
    for t,tid,k,phase,i,v in csv.reader(f):
        t,tid,k,i,v=map(int,(t,tid,k,i,v))
        if phase=='I':points[k].append((t,tid,i,v))
        if k==4:
            if phase=='B':starts[i]=(t,v)
            elif phase=='E' and i in starts:
                a,task=starts.pop(i);tasks.append((a,t,task))
assert len(points[99])==2 and all(i-v==0 for _,_,i,v in points[99]),'Clock conversion required'
begin,end=points[99][0][0],points[99][-1][0]
states=[(int(r['ts']),int(r['ts'])+int(r['dur']),r['state']) for r in csv.DictReader((root/'worker-states.csv').open()) if int(r['dur'])>=0]
def states_between(a,b):
    out=defaultdict(int)
    for x,y,state in states:
        if y>a and x<b:out[state]+=max(0,min(y,b)-max(x,a))
    return {k:v/1e6 for k,v in out.items()}
pub={i:t for t,_,i,v in points[15]};draws={i:(t,v) for t,_,i,v in points[22]};metrics={i for _,_,i,v in points[27]}
frames=[];seen=set()
for r in csv.DictReader((root/'presentation.csv').open()):
    _,di,pi=r['draw'].split(':');di,pi=int(di),int(pi)
    if pi not in pub or di not in draws:continue
    assert di not in seen,'Ambiguous draw-to-frame mapping';seen.add(di)
    assert r['surface_frame_token']!='[NULL]' and r['display_ts']!='[NULL]'
    token=int(r['name'].split()[-1]);assert token==int(r['surface_frame_token'])
    assert r['present_type']!='Dropped Frame'
    presented=int(r['display_ts'])+int(r['display_dur'])
    assert presented>=draws[di][0]>=pub[pi]
    frames.append({'paint':pi,'draw':di,'presented':presented,'pub_to_present_ms':(presented-pub[pi])/1e6,'draw_to_present_ms':(presented-draws[di][0])/1e6,'metric_token_seen':token in metrics})
def dist(values):
    v=sorted(values);return {'n':len(v),'median_ms':v[(len(v)-1)//2],'p95_ms':v[int((len(v)-1)*.95)],'max_ms':v[-1]} if v else {'n':0}
inputs=defaultdict(dict)
for k in (25,12,14,17):
    for t,tid,i,v in points[k]:inputs[i][k]=(t,v)
probes=[]
for i,p in inputs.items():
    if not all(k in p for k in (25,12,14,17)):continue
    listener=p[25][1];dequeue=p[12][0];pop=p[14][0]
    overlap=defaultdict(int)
    for a,b,task in tasks:
        if b>dequeue and a<pop:overlap[task]+=max(0,min(pop,b)-max(dequeue,a))
    probes.append({'input':i,'queued_ahead':p[17][1],'listener_to_worker_ms':(dequeue-listener)/1e6,'worker_to_pop_ms':(pop-dequeue)/1e6,'worker_states_during_queue_wait_ms':states_between(dequeue,pop),'long_task_overlap_ms':{k:v/1e6 for k,v in overlap.items()}})
result={'duration_s':(end-begin)/1e9,'matched_presentations':len(frames),'frame_metrics_token_matches':sum(f['metric_token_seen'] for f in frames),'publication_to_platform_presentation':dist([f['pub_to_present_ms'] for f in frames]),'draw_to_platform_presentation':dist([f['draw_to_present_ms'] for f in frames]),'presented_paint_generations_per_second':len(set(f['paint'] for f in frames if begin<=f['presented']<=end))/((end-begin)/1e9),'worker_states_ms':states_between(begin,end),'probes':probes,'limits':['FrameTimeline presentation is in the AVD guest display stack, not the Mac panel.','No first-affected-frame oracle.','Per-task CPU/host totals cannot be precisely intersected with scheduling after aggregation.','Input probes include holds and untracked auto-repeats, not 25 isolated taps.']}
(root/'correlation.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
