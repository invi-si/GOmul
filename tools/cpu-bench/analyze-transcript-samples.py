#!/usr/bin/env python3
"""Attribute native replay samples using the exact sampled ELF, excluding restore.
The denominator is samples inside Arm32CpuEngine::run, not all process CPU time.
"""
import argparse, collections, importlib.util, json, pathlib, re, subprocess
p=argparse.ArgumentParser(description=__doc__)
for name in ['samples','dump','elf','nm','readelf','symbolizer']: p.add_argument('--'+name,required=True)
a=p.parse_args()
spec=importlib.util.spec_from_file_location('native_samples',pathlib.Path(__file__).parents[1]/'input-trace/native_callback_samples.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
notes=subprocess.check_output([a.readelf,'-n',a.elf],text=True)
bid=re.search(r'Build ID: ([0-9a-f]+)',notes)[1]
dump=pathlib.Path(a.dump).read_text()
ids={b for b,path in re.findall(r'build_id 0x([0-9a-f]+)\s+filename ([^\n]+)',dump) if path.endswith('/native-profile')}
if ids!={bid}: raise ValueError('Sampled ELF build ID mismatch')
if re.search(r'record lost(?:_samples)?:',dump): raise ValueError('Lost samples')
nm=subprocess.check_output([a.nm,'-S','-C',a.elf],text=True)
ranges=[]
for line in nm.splitlines():
 if 'Arm32CpuEngine as' in line and line.endswith('::run'):
  fields=line.split();start,size=int(fields[0],16),int(fields[1],16);ranges.append((start,start+size))
if len(ranges)!=1: raise ValueError('Expected one engine run function')
samples=module.samples(a.samples)
selected=[s for s in samples if s['file'].endswith('/native-profile') and any(lo<=int(s['vaddr_in_file'],16)<hi for lo,hi in ranges)]
counts=collections.Counter(int(s['vaddr_in_file'],16) for s in selected)
addresses=sorted(counts);symbols={}
for i in range(0,len(addresses),256):
 out=subprocess.check_output([a.symbolizer,'--obj='+a.elf,'--inlines','--demangle','--output-style=JSON','--no-debuginfod']+[hex(n) for n in addresses[i:i+256]],text=True)
 for s in json.loads(out): symbols[int(s['Address'],16)]=s['Symbol']
lines=collections.Counter();functions=collections.Counter()
for addr,count in counts.items():
 frames=symbols[addr];leaf=frames[0] if frames else {};lines[(leaf.get('FunctionName','?'),leaf.get('FileName','?'),leaf.get('Line',0))]+=count
 for name in {f['FunctionName'] for f in frames}: functions[name]+=count
print(json.dumps({'build_id':bid,'exported_samples':len(samples),'engine_run_samples':len(selected),'excluded_samples':len(samples)-len(selected),
 'run_ranges':ranges,'leaf_lines':[{'function':f,'file':p,'line':line,'samples':n} for (f,p,line),n in lines.most_common()],
 'inclusive_functions':functions.most_common(),'addresses':[{'address':hex(addr),'samples':n,'frames':symbols[addr]} for addr,n in counts.most_common()],
 'limits':['Native samples are observations, not exact or exclusive stage costs. Sampling skid applies.', 'Restore/validation and non-inlined helpers outside the run function are excluded. Inclusive function shares overlap.', 'External sampling results never replace uninstrumented control timings.']},indent=2))
