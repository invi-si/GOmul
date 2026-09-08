#!/usr/bin/env python3
"""Join simpleperf monotonic native-PC samples to retained active timer polls.

Uses exact-build ELF source attribution. Samples in unretained short polls stay
unclassified; callback wall overlap alone is not proof of active ownership.
"""
import argparse
import bisect
from collections import Counter
import csv
import json
from pathlib import Path
import re
import subprocess


def samples(path):
    result = []; row = None
    for line in Path(path).read_text().splitlines():
        if line == 'sample:':
            if row: result.append(row)
            row = {}
        elif row is not None and line.startswith('  ') and not line.startswith('   ') and ': ' in line:
            key, value = line.strip().split(': ', 1)
            row[key] = value
    if row: result.append(row)
    return result


def active_polls(path):
    with Path(path).open() as f:
        if f.readline().split('lost=')[-1].strip() != '0': raise ValueError('Lost diagnostic records')
        rows = list(csv.reader(f))
    timers = {int(i) for t,tid,k,ph,i,v in rows if int(k)==75 and ph=='B'}
    pending = {}; result = {}
    for t,tid,k,ph,i,v in rows:
        t,tid,k,i,v = map(int,(t,tid,k,i,v))
        if k != 84: continue
        if ph=='B' and v in timers: pending[(tid,i)] = t
        if ph=='E' and (tid,i) in pending:
            result.setdefault(tid,[]).append((pending.pop((tid,i)),t))
    return {tid:sorted(intervals) for tid,intervals in result.items()}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--samples',required=True);p.add_argument('--trace',required=True)
    p.add_argument('--elf',required=True);p.add_argument('--build-id',required=True)
    p.add_argument('--perf-dump',required=True)
    p.add_argument('--readelf',required=True);p.add_argument('--symbolizer',required=True)
    a=p.parse_args()
    notes=subprocess.check_output([a.readelf,'-n',a.elf],text=True)
    actual=re.search(r'Build ID: ([0-9a-f]+)',notes)
    if not actual or actual[1].lower()!=a.build_id.lower():raise ValueError('ELF build ID mismatch')
    dump=Path(a.perf_dump).read_text()
    if re.search(r'record lost(?:_samples)?:',dump):raise ValueError('Lost native samples')
    ids={bid.lower() for bid,path in re.findall(r'build_id 0x([0-9a-f]+)\s+filename ([^\n]+)',dump) if path.endswith('/libwie_android.so')}
    if ids!={a.build_id.lower()}:raise ValueError('Recorded library build ID differs from supplied ELF')
    if not re.search(r'\bclockid 1\b',dump):raise ValueError('Expected CLOCK_MONOTONIC native sample timestamps')
    all_samples=samples(a.samples);polls=active_polls(a.trace);starts={tid:[x for x,y in intervals] for tid,intervals in polls.items()}
    selected=[];outcomes=Counter()
    for s in all_samples:
        tid=int(s['thread_id']);t=int(s['time']);index=bisect.bisect_right(starts.get(tid,[]),t)-1
        if index<0 or t>polls[tid][index][1]:outcomes['outside_retained_active_timer_polls']+=1;continue
        if not s['file'].endswith('/libwie_android.so'):outcomes['active_timer_other_modules']+=1;continue
        outcomes['active_timer_emulator_library']+=1;selected.append(s)
    addresses=sorted({int(s['vaddr_in_file'],16) for s in selected});symbols={}
    for start in range(0,len(addresses),256):
        output=subprocess.check_output([a.symbolizer,'--obj='+a.elf,'--inlines','--demangle','--output-style=JSON','--no-debuginfod']+[hex(n) for n in addresses[start:start+256]],text=True)
        for s in json.loads(output):symbols[int(s['Address'],16)]=s['Symbol']
    counts=Counter();details=[]
    for s in selected:
        frames=symbols[int(s['vaddr_in_file'],16)];leaf=frames[0] if frames else {}
        key=(leaf.get('FunctionName','unknown'),leaf.get('FileName','unknown'),leaf.get('Line',0));counts[key]+=1
        details.append({**s,'inline_frames_innermost_first':frames})
    print(json.dumps({'elf_build_id':actual[1],'total_native_samples':len(all_samples),'classification':dict(outcomes),
        'innermost_lines':[{'function':f,'file':file,'line':line,'samples':n} for (f,file,line),n in counts.most_common()],
        'samples':details,'limits':['Only retained active polls of at least 250 us are classified; shorter polls remain outside the selected set.',
        'Native recording log must also report zero lost samples; perf dump, build ID and clock checks are enforced.',
        'Inlining and sampling skid limit exact machine-stage attribution; source-line shares are not independent timing buckets.',
        'This identifies host execution sites, not guest-PC or opcode frequency.']},indent=2))

if __name__=='__main__':main()
