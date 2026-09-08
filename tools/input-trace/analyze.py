#!/usr/bin/env python3
"""Summarize diagnostic wall times; never interpret next paint as causal response."""
import argparse, bisect, csv, json, re
from collections import defaultdict
p=argparse.ArgumentParser();p.add_argument('trace');args=p.parse_args()
def distribution(ns):
    s=sorted(ns)
    if not s:return {'n':0}
    return {'n':len(s),'p50_ms':s[(len(s)-1)//2]/1e6,'p95_ms':s[int((len(s)-1)*.95)]/1e6,'max_ms':s[-1]/1e6}
with open(args.trace) as f:
    header=f.readline();lost=int(re.search(r'lost=(\d+)',header)[1])
    rows=[(int(t),int(tid),int(k),phase,int(i),int(v)) for t,tid,k,phase,i,v in csv.reader(f)]
rows.sort(key=lambda r:r[0])
starts={};spans=[];unmatched=0;points=defaultdict(list)
for t,tid,k,phase,i,v in rows:
    if phase=='B':starts[(tid,k,i)]=(t,v)
    elif phase=='E':
        begin=starts.pop((tid,k,i),None)
        if begin:spans.append((begin[0],t,tid,k,i,begin[1],v))
        else:unmatched+=1
    else:points[k].append((t,tid,i,v))
names={1:'tick',2:'executor',3:'pass',4:'task_poll',5:'cpu_run',6:'host_active_poll',7:'paint',8:'frame_copy_including_lock'}
summary={'lost_records':lost,'usable':lost==0,'unmatched_spans':unmatched+len(starts),'span_wall_times':{name:distribution([b-a for a,b,_,k,*_ in spans if k==kind]) for kind,name in names.items()}}
# Exclusive attribution of nested scopes using each thread's complete intervals.
# Wall time includes descheduling; OTHER is not automatically JVM time.
bounds=defaultdict(list)
for a,b,tid,k,i,*_ in spans:
    if k in (4,5,6):bounds[tid].extend([(a,1,k,i),(b,0,k,i)])
exclusive=defaultdict(int)
for events in bounds.values():
    active={};last=None
    for t,start,k,i in sorted(events):
        if last is not None and any(kind==4 for kind in active.values()):
            category='OTHER'
            if active:
                inner=active[next(reversed(active))]
                category={5:'GUEST_CPU_wall',6:'HOST_API_wall'}.get(inner,'OTHER')
            exclusive[category]+=t-last
        if start:active[i]=k
        else:active.pop(i,None)
        last=t
summary['exclusive_within_task_ms']={k:v/1e6 for k,v in exclusive.items()}
inputs=defaultdict(dict)
for k in (10,25,12,13,14):
    for t,tid,i,v in points[k]:inputs[i].setdefault(k,v if k in (10,25) else t)
for label,a,b in [('event_to_listener',10,25),('listener_to_worker',25,12),('worker_to_queue_pop',12,14),('listener_to_queue_pop',25,14)]:
    summary[label]=distribution([d[b]-d[a] for d in inputs.values() if d.get(a,0)>0 and b in d and d[b]>=d[a]])
summary['input_count']=len(inputs);summary['inputs_without_queue_pop']=sum(14 not in d for d in inputs.values())
publish={i:t for t,_,i,_ in points[15]};pickup={i:t for t,_,i,_ in points[16]}
summary['paint_to_pickup']=distribution([t-publish[i] for i,t in pickup.items() if i in publish and t>=publish[i]])
summary['paint_to_draw']=distribution([t-publish[paint] for t,_,draw,paint in points[22] if paint in publish and t>=publish[paint]])
summary['step_attempts']=sum(result for _,_,_,k,_,_,result in spans if k==5)
summary['host_polls_ready']=sum(k==6 and result==1 for _,_,_,k,_,_,result in spans)
summary['host_polls_pending']=sum(k==6 and result==0 for _,_,_,k,_,_,result in spans)
cpu_by_tid=defaultdict(list);svc_by_tid=defaultdict(list)
for a,b,tid,k,*_ in spans:
    if k==5:cpu_by_tid[tid].append(a)
for t,tid,i,v in points[18]:
    if v>=0x100000000:svc_by_tid[tid].append(t)
for d in (cpu_by_tid,svc_by_tid):
    for seq in d.values():seq.sort()
longest=[]
for a,b,tid,k,i,task,result in spans:
    if k==4:
        count=lambda seq:bisect.bisect_left(seq,b)-bisect.bisect_left(seq,a)
        longest.append({'ms':(b-a)/1e6,'task_id':task,'cpu_runs':count(cpu_by_tid[tid]),'svcs':count(svc_by_tid[tid])})
summary['longest_task_polls']=sorted(longest,key=lambda x:x['ms'],reverse=True)[:10]
if points[50]:
    summary['aggregation']='task summaries; detailed task intervals retained only at >=250 us'
    summary['exclusive_within_task_ms']={name:sum(v for _,_,_,v in points[kind])/1e6 for kind,name in [(50,'GUEST_CPU_wall'),(51,'HOST_API_wall'),(52,'OTHER')]}
    summary['step_attempts']=sum(v for _,_,_,v in points[54])
    summary['host_polls_ready']=sum(v for _,_,_,v in points[56])
    summary['host_polls_pending']=sum(v for _,_,_,v in points[57])
    summary['all_task_poll_count']=sum(v for _,_,_,v in points[58])
    summary['max_task_poll_ms']=max((v for _,_,_,v in points[59]),default=0)/1e6
    by_task=defaultdict(dict)
    for k in range(40,48):
        for t,tid,i,v in points[k]:by_task[i][k]=v
    summary['longest_task_polls']=sorted([{'ms':(b-a)/1e6,'task_id':task,'cpu_runs':by_task[i].get(43,0),'step_attempts':by_task[i].get(44,0),'svcs':by_task[i].get(45,0),'host_ready':by_task[i].get(46,0),'cpu_exclusive_ms':by_task[i].get(40,0)/1e6,'host_exclusive_ms':by_task[i].get(41,0)/1e6,'other_ms':by_task[i].get(42,0)/1e6} for a,b,tid,k,i,task,result in spans if k==4],key=lambda x:x['ms'],reverse=True)[:10]
    summary['span_wall_times'].pop('cpu_run',None);summary['span_wall_times'].pop('host_active_poll',None)
    summary['span_wall_times']['long_task_polls_only']=summary['span_wall_times'].pop('task_poll')
summary['clock_sync_boottime_minus_monotonic_ns']=[i-v for _,_,i,v in points[99]]
if len(points[99])>=2:
    duration=(points[99][-1][0]-points[99][0][0])/1e9
    summary['capture_seconds']=duration
    summary['step_attempts_per_second']=summary['step_attempts']/duration if duration>0 else None
    summary['guest_paints_per_second']=len(points[15])/duration if duration>0 else None
summary['individual_inputs']=[{'input_id':i,'listener_to_worker_ms':(d[12]-d[25])/1e6 if 12 in d and 25 in d else None,'listener_to_queue_pop_ms':(d[14]-d[25])/1e6 if 14 in d and 25 in d else None} for i,d in inputs.items()]
summary['limitations']=['AVD results are not physical phone results.','Paint and commit are not actual presentation.','No validated first-affected-frame oracle; no causal response latency claimed.','CPU counts are step attempts, not universal architectural instructions.','Wall-time buckets include descheduling and lock waiting.','Task edge spans may be truncated; inspect platform trace for preemption.']
print(json.dumps(summary,indent=2))
