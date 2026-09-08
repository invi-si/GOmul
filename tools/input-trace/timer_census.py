#!/usr/bin/env python3
"""Identify queued WIPI timer instances. Counts are observations, not API semantics."""
import csv,json,sys
from collections import Counter,defaultdict
from pathlib import Path
with Path(sys.argv[1]).open() as f:
    header=next(f)
    if header.split('lost=')[-1].strip()!='0':raise ValueError('Lost records: census incomplete')
    rows=[(int(t),int(tid),int(k),ph,int(i),int(v)) for t,tid,k,ph,i,v in csv.reader(f)]
    rows.sort(key=lambda r:r[0])
regs=defaultdict(dict);event_reg={};event_type={};physical={};definitions={}
fields={113:'ptr',114:'callback',115:'param',116:'timeout',117:'due',118:'guest_now',125:'parent'}
for t,tid,k,ph,i,v in rows:
    if k in fields:regs[i][fields[k]]=v
    if k in (121,122,123):regs[i][{121:'ptr',122:'callback',123:'param'}[k]]=v
    if k==112:definitions[i]=v
    if k==110:event_reg[i]=v
    if k in (60,63):event_type[i]=v
    if k==61 and v:physical[i]=v
queue=[];state={};begins={};backend_callbacks={};callbacks=[];operations=[];keys=[];peak=defaultdict(Counter);capture_partial=False
cancelled=set();started=set();cancelled_entries=[];skipped=[]
reg_events=defaultdict(set)
for eid,rid in event_reg.items():reg_events[rid].add(eid)
def ptr_of(eid):return regs.get(event_reg.get(eid),{}).get('ptr')
def snapshot(ptr=None):
    c=Counter()
    for eid,st in state.items():
        if event_type.get(eid)==5 and st!='done' and (ptr is None or ptr_of(eid)==ptr):c[st]+=1
    return dict(c)
def update_peak():
    pointers={ptr_of(eid) for eid,st in state.items() if st!='done' and event_type.get(eid)==5}
    for ptr in pointers:
        c=snapshot(ptr) if ptr is not None else Counter(st for eid,st in state.items() if st!='done' and event_type.get(eid)==5 and ptr_of(eid) is None)
        for k,v in c.items():peak[str(ptr)][k]=max(peak[str(ptr)][k],v)
        peak[str(ptr)]['total']=max(peak[str(ptr)]['total'],sum(c.values()))
for t,tid,k,ph,i,v in rows:
    if k==60:
        if v in (2,3) and i in physical:
            ahead=[{'event':e,'type':event_type.get(e),'registration':event_reg.get(e),'ptr':ptr_of(e),'callback':regs.get(event_reg.get(e),{}).get('callback')} for e in queue]
            keys.append({'time_ns':t,'input':physical[i],'event':i,'queue_ahead':ahead,'outstanding':snapshot()})
        if i in queue:raise ValueError('Same event enqueued twice without pop')
        queue.append(i);state[i]='queued';update_peak()
    elif k==63:
        if i not in queue:capture_partial=True
        else:
            if queue[0]!=i:raise ValueError('FIFO reconstruction mismatch')
            queue.pop(0)
        if v==5:state[i]='popped'
        else:state.pop(i,None)
    elif k==80 and not v:state[i]='deferred'
    elif k==75 and ph=='B':backend_callbacks[i]=v
    elif k==75 and ph=='E' and i in backend_callbacks:state.pop(backend_callbacks.pop(i),None)
    elif k==120 and ph=='B':
        begins[i]=(t,v)
        started.add(v)
        if v in cancelled:cancelled_entries.append({'registration':v,'time_ns':t})
        for e in reg_events[v]:state[e]='in_flight'
        update_peak()
    elif k==120 and ph=='E' and i in begins:
        a,rid=begins.pop(i);callbacks.append({'registration':rid,'start_ns':a,'end_ns':t,'duration_ms':(t-a)/1e6})
        for e in reg_events[rid]:state.pop(e,None)
    elif k==126:skipped.append({'event':i,'registration':event_reg.get(i),'time_ns':t})
    elif k in (111,113,119):
        if k==119:
            cancelled.update(event_reg[e] for e in state if ptr_of(e)==v and event_reg.get(e) not in started)
        operations.append({'time_ns':t,'operation':{111:'Def',113:'Set',119:'Unset'}[k],'sequence':i,'ptr':v,'existing_instances':snapshot(v),'details':regs.get(i,{}) if k==113 else {'callback':definitions.get(i)} if k==111 else {}})
children_by_parent=defaultdict(list)
for op in operations:
    if op['operation']=='Set':children_by_parent[regs[op['sequence']].get('parent')].append(op)
for cb in callbacks:
    children=[]
    for op in children_by_parent[cb['registration']]:
        children.append({'registration':op['sequence'],'offset_ms':(op['time_ns']-cb['start_ns'])/1e6,'timeout_ms':regs[op['sequence']].get('timeout'),'callback_remaining_ms':(cb['end_ns']-op['time_ns'])/1e6})
    cb['replacement_sets']=children
result={'cancelled_registrations':sorted(cancelled),'callbacks_started_after_cancel':cancelled_entries,'cancelled_registrations_rearming':[op['sequence'] for op in operations if op['operation']=='Set' and regs[op['sequence']].get('parent') in cancelled],'cancelled_events_skipped':skipped,'partial_initial_queue':capture_partial,'logical_timer_objects':len({r['ptr'] for r in regs.values() if 'ptr' in r}),'registrations':dict(regs),'event_registration':event_reg,'peak_instances_per_ptr':{p:dict(c) for p,c in peak.items()},'operations':operations,'keys':keys,'callbacks':callbacks,'limits':['Guest pointers are identities only within this process/guest lifetime.','An in-flight callback plus one successor is not itself duplicate pending registration.','Def or reuse of a timer pointer may begin a new logical lifetime; inspect operation order.','Unknown initial queue entries make population counts incomplete.','Cancellation audit applies Unset to registrations that have not entered their callback; repeated Set/Def replacement is not assumed.']}
print(json.dumps(result,indent=2))
