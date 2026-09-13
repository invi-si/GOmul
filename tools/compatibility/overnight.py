#!/usr/bin/env python3
"""Bounded, resumable blank-boot triage using the existing isolated audit runner."""
import argparse
import collections
import json
import pathlib
import subprocess
import sys
import time


def priority(case):
    if case.get('classification') == 'harness-or-process-failure':
        return 'high: process/harness failure (inspect evidence)'
    samples = case.get('samples', [])
    tail = [s for s in samples if s.get('phase', '').startswith('settled:')][-3:]
    if len(tail) >= 3:
        if all('frameHash' not in s for s in tail):
            return 'high: no framebuffer after input'
        if all(max(s.get('blackFraction', 0), s.get('whiteFraction', 0)) >= .98 for s in tail):
            return 'high: persistent blank screen after input (suspected)'
    if case.get('errors') or any(s.get('status', '').startswith('Error:') for s in samples):
        return 'low: logged/native failure review'
    if samples and samples[-1].get('status', '').startswith('Game stopped'):
        return 'low: stopped review'
    if tail and len({s.get('frameHash') for s in tail}) == 1:
        return 'review: static nonblank screen; may be a menu'
    return 'no detected boot failure (not gameplay verified)'


def report(root, results, state):
    counts = dict(collections.Counter(r['priority'] for r in results))
    (root / 'status.json').write_text(json.dumps(dict(state=state,completed=len(results),counts=counts,updated=time.time()),indent=2))
    (root / 'results.json').write_text(json.dumps(results,ensure_ascii=False,indent=2))
    rank = lambda r: (0 if r['priority'].startswith('high:') else 1 if r['priority'].startswith('low:') else 2, r['title'])
    lines = ['# Overnight KTF boot triage', '', f'Status: {state}. Completed: {len(results)}.', '',
             'Blank screens are suspects, not proven loops. Static menus are not classified as crashes. Logs may contain caught exceptions. No full-game compatibility claim.', '',
             '| Game | Priority | Evidence |', '| --- | --- | --- |']
    for r in sorted(results,key=rank):
        lines.append(f"| {r['title'].replace('|','/')} | {r['priority']} | [case]({r['evidence']}/result.json) |")
    for r in sorted(results,key=rank):
        if r['priority'].startswith('high:'):
            images=sorted((root/r['evidence']).glob('sample-*.png'))
            if images:lines.extend(['', '## '+r['title'], '', '!['+r['title']+']('+str(images[-1].relative_to(root))+')'])
    (root / 'REPORT.md').write_text('\n'.join(lines)+'\n')


def main():
    p=argparse.ArgumentParser();p.add_argument('--plan',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--serial',required=True);p.add_argument('--restore-apk',required=True);a=p.parse_args()
    root=a.output;root.mkdir(parents=True,exist_ok=True);plan=json.loads(a.plan.read_text());results=[]
    audit=pathlib.Path(__file__).with_name('audit.py');package='local.wie.nativeapp'
    def run_case(item,stage,boot_samples,keys):
        folder=root/stage;case=folder/item['id'];case.mkdir(parents=True,exist_ok=True)
        target=case/'result.json'
        if target.exists(): return json.loads(target.read_text())
        command=[sys.executable,str(audit),'--serial',a.serial,'--output',str(folder),'--phase','boot','--game-id',item['id'],'--boot-samples',str(boot_samples),'--keys',keys,'--restart-after-stop','--capture-rescue']
        try:
            with (case/'driver.log').open('w') as log:
                completed=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,timeout=150)
            if completed.returncode or not target.exists(): raise RuntimeError('audit driver failed; see driver.log')
            return json.loads(target.read_text())
        except (subprocess.TimeoutExpired, RuntimeError) as e:
            subprocess.run(['adb','-s',a.serial,'shell','am','force-stop',package],timeout=20,check=False)
            failure={'game':item['game'],'classification':'harness-or-process-failure','error':str(e)}
            target.write_text(json.dumps(failure,indent=2));return failure
    state='running'
    try:
        for item in plan['games']:
            case=run_case(item,'first',2,'OK,DOWN,OK,RSOFT,1')
            results.append(dict(title=item['title'],id=item['id'],priority=priority(case),evidence='first/'+item['id']))
            report(root,results,'first pass');print(len(results),item['title'],results[-1]['priority'],flush=True)
        for r in results:
            if not r['priority'].startswith('high:'):continue
            item=next(x for x in plan['games'] if x['id']==r['id'])
            case=run_case(item,'retry',6,'RSOFT,OK,DOWN,OK,RSOFT,1,OK')
            r['first_priority']=r['priority'];r['priority']=priority(case);r['evidence']='retry/'+r['id']
            report(root,results,'retrying high-priority suspects')
        state='complete'
    except Exception as e:
        state='interrupted: '+repr(e)
        raise
    finally:
        report(root,results,state)
        # Restore the ordinary APK, preserving library and persistent saves.
        try:
            with (root/'restore.log').open('w') as log:
                subprocess.run(['adb','-s',a.serial,'install','-r',a.restore_apk],stdout=log,stderr=subprocess.STDOUT,timeout=90,check=True)
                subprocess.run(['adb','-s',a.serial,'shell','am','start','-a','android.intent.action.MAIN','-c','android.intent.category.LAUNCHER','-n',package+'/.MainActivity'],stdout=log,stderr=subprocess.STDOUT,timeout=20,check=True)
        except Exception as e:
            (root/'RESTORE-FAILED.txt').write_text(str(e))


if __name__=='__main__':main()
