#!/usr/bin/env python3
"""Queue ledger: wall lifetimes, active future polls, event order and key rank.
No scheduling or presentation assertions. Edge/ambiguous lifecycle links stay unknown.
"""
import csv,json,sys
from collections import defaultdict,Counter
from pathlib import Path
path=Path(sys.argv[1]);points=defaultdict(list);starts={};spans=[]
with path.open() as f:
    header=next(f)
    if header.split('lost=')[-1].strip()!='0':raise ValueError('Lost records: reject capture')
    rows=[(int(t),int(tid),int(k),ph,int(i),int(v)) for t,tid,k,ph,i,v in csv.reader(f)]
for t,tid,k,ph,i,v in sorted(rows):
    if ph=='B':starts[(tid,k,i)]=(t,v)
    elif ph=='E':
        begin=starts.pop((tid,k,i),None)
        if begin:spans.append(dict(a=begin[0],b=t,tid=tid,k=k,id=i,value=begin[1],result=v))
    else:points[k].append((t,tid,i,v))
span_ids={s['id']:s for s in spans}
roles={i:('EventLoopRunner' if v==1 else 'unknown') for t,tid,i,v in points[68]}
iteration_tasks={i:v for t,tid,i,v in points[67]}
returns={i:v for t,tid,i,v in points[82]};branches={i:v for t,tid,i,v in points[83]}
wrappers=[s for s in spans if s['k']==89]
dispatch_wrappers=[s for s in spans if s['k']==88]
def contains(outer,inner):return outer['tid']==inner['tid'] and outer['a']<=inner['a']<=inner['b']<=outer['b']
iteration_event={};ambiguous=0
for s in spans:
    if s['k']==71 and s['id'] in returns:
        candidates=[w for w in wrappers if contains(w,s)]
        if len(candidates)==1:iteration_event[candidates[0]['value']]=returns[s['id']]
        else:ambiguous+=1
for s in spans:
    if s['k']==78:
        ws=[w for w in dispatch_wrappers if contains(w,s)]
        if len(ws)==1:
            s['event_id']=iteration_event.get(ws[0]['value'])
            s['task_id']=iteration_tasks.get(ws[0]['value'])
        else:ambiguous+=1
names={1:'Redraw',2:'KeyDown',3:'KeyUp',4:'KeyRepeat',5:'Timer',6:'Notify'}
kinds={i:v for t,tid,i,v in points[60]};kinds.update({i:v for t,tid,i,v in points[63]})
physical={i:v for t,tid,i,v in points[61] if v}
enqueues=defaultdict(list)
for t,tid,i,v in points[62]:enqueues[i].append((t,v))
pops=defaultdict(list)
for t,tid,i,v in points[63]:pops[i].append(t)
labels={72:'pre-pop serviceRepaints',74:'callSerially callback',75:'timer callback',76:'timer yield',77:'empty-queue sleep',78:'dispatch',79:'pre-pop other'}
work=[s for s in spans if s['k'] in labels]
active=defaultdict(list)
for s in spans:
    if s['k']==84:active[s['value']].append((s['a'],s['b']))
def overlap(a,b,c,d):return max(0,min(b,d)-max(a,c))
def summarize_work(a,b):
    relevant=[s for s in work if overlap(a,b,s['a'],s['b'])]
    edges=sorted({a,b}|{max(a,s['a']) for s in relevant}|{min(b,s['b']) for s in relevant})
    out=defaultdict(int)
    for x,y in zip(edges,edges[1:]):
        containing=[s for s in relevant if s['a']<=x and s['b']>=y]
        # Prefer the most specific (shortest) lifetime. Pre-pop-other encloses stages.
        s=min(containing,key=lambda s:s['b']-s['a']) if containing else None
        label='unattributed / executor gap'
        if s:
            label=labels[s['k']]
            if s['k']==78:label+=' '+names.get(kinds.get(s.get('event_id')),'unknown event')
        out[label]+=y-x
    return {k:v/1e6 for k,v in out.items()}
probes=[]
for eid,input_id in physical.items():
    if eid not in enqueues or eid not in pops:continue
    a,depth=enqueues[eid][0];b=pops[eid][0]
    before=[(t,i,v) for t,tid,i,v in points[63] if a<t<b]
    rank=[{'ms':0,'rank':depth+1}]+[{'ms':(t-a)/1e6,'rank':v} for t,tid,i,v in points[90] if i==eid and a<=t<=b]+[{'ms':(b-a)/1e6,'rank':0}]
    task_overlap=defaultdict(int)
    for task_span in spans:
        if task_span['k']==4:
            task=task_span['value'];duration=overlap(a,b,task_span['a'],task_span['b'])
            if duration:task_overlap[f'{task}: {roles.get(task,"unknown")}']+=duration
    probes.append({'retained_task_overlap_ms':{k:v/1e6 for k,v in task_overlap.items()},'event_id':eid,'input_id':input_id,'type':names[kinds[eid]],'wait_ms':(b-a)/1e6,'pops_before_key':dict(Counter(names[v] for t,i,v in before)),'rank':rank,'wall_ledger_ms':summarize_work(a,b)})
stage_active={i:v for t,tid,i,v in points[92]}
stages={}
for k,label in labels.items():
    selected=[s for s in spans if s['k']==k];wall=[s['b']-s['a'] for s in selected]
    active_ns=[stage_active.get(s['id'],0) for s in selected]
    stages[label]={'count':len(wall),'inclusive_wall_total_ms':sum(wall)/1e6,'wall_max_ms':max(wall,default=0)/1e6,'active_poll_total_ms':sum(active_ns)/1e6 if k in (72,74,75,76,77) else None}
press_links={i:v for t,tid,i,v in points[26]}
input_times={i:v for t,tid,i,v in points[10] if v>0}
press_durations=[]
for i,parent in press_links.items():
    if i!=parent and i in input_times and parent in input_times:
        press_durations.append({'press_input_id':parent,'release_input_id':i,'duration_ms':(input_times[i]-input_times[parent])/1e6})
result={'phase_markers':points[94],'press_durations':press_durations,'task_roles':roles,'event_types_popped':dict(Counter(names[v] for t,tid,i,v in points[63])),'timer_due_counts':dict(Counter('due' if v else 'future' for t,tid,i,v in points[80])),'ambiguous_iteration_links':ambiguous,'stages':stages,'physical_keys':probes,'limits':['Wall categories include suspension; active future polls are separate.','Active polls include preemption; not on-CPU measurements.','Unknown capture-edge events remain unknown.','No class identity capture or first-affected-frame oracle.','Do not sum inclusive stage totals.']}
print(json.dumps(result,indent=2))
