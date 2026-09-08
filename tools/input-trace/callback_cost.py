#!/usr/bin/env python3
"""Callback-owned active CPU/host wall costs; never treat these as hardware CPU cycles."""
import csv
import json
import statistics
import sys
from pathlib import Path

FIELDS = {130: 'cpu_exclusive_ns', 131: 'host_exclusive_ns', 132: 'cpu_runs',
          133: 'step_attempts', 134: 'svc_exits', 135: 'host_ready_polls', 136: 'host_pending_polls'}

def analyze(path):
    with Path(path).open() as f:
        if f.readline().split('lost=')[-1].strip() != '0':
            raise ValueError('Incomplete recording: lost records')
        rows = [(int(t), int(k), p, int(i), int(v)) for t, tid, k, p, i, v in csv.reader(f)]
    stages = {}; mapping = {}; registrations = {}
    for t, k, p, i, v in rows:
        if k == 110: mapping[i] = v
        if k == 113: registrations.setdefault(i, {}).update(set_ns=t, timer_pointer=v)
        if k in (116, 117, 125): registrations.setdefault(i, {})[{116:'timeout_ms',117:'due_guest_ms',125:'parent'}[k]] = v
        if k == 75 and p == 'B': stages[i] = {'event': v, 'start_ns': t}
        if k == 75 and p == 'E' and i in stages: stages[i]['end_ns'] = t
        if k in FIELDS and i in stages: stages[i][FIELDS[k]] = v
        if k == 92 and i in stages: stages[i]['active_poll_ns'] = v
        if 140 <= k <= 147 and i in stages: stages[i].setdefault('run_length_histogram', [0]*8)[k-140] = v
        if k == 126:
            for s in stages.values():
                if s['event'] == i: s['cancelled'] = True
    complete = []
    for s in stages.values():
        if s.get('cancelled') or 'end_ns' not in s or not all(f in s for f in FIELDS.values()): continue
        s['registration'] = mapping.get(s['event'])
        s['timer'] = registrations.get(s['registration'], {})
        s['lifetime_ns'] = s['end_ns'] - s['start_ns']
        complete.append(s)
    def dist(name, scale=1):
        values = sorted(s[name]/scale for s in complete if name in s)
        return {'n':len(values), 'median':statistics.median(values), 'mean':statistics.mean(values), 'max':max(values)} if values else {'n':0}
    return {'callbacks':complete, 'summary':{**{f:dist(f, 1e6 if f.endswith('_ns') else 1) for f in [*FIELDS.values(), 'active_poll_ns', 'lifetime_ns']},
            'time_output_unit':'ms', 'run_length_bins':['0','1–16','17–64','65–256','257–1024','1025–4096','4097–9999','10000+']},
            'limits':['Active wall time includes host preemption; it is not hardware CPU time.',
                      'SVC exits are distinct from Ready/Pending host future polls.',
                      'Timer stage boundaries include adapter work around guest callback entry.',
                      'No state equivalence, SVC sequence identity or output equivalence is established by aggregate counts.']}

if __name__ == '__main__': print(json.dumps(analyze(sys.argv[1]), indent=2))
