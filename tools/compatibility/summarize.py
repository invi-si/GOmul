#!/usr/bin/env python3
"""Group boot evidence without turning smoke-test results into compatibility claims."""
import argparse
import collections
import json
import pathlib
import re


def findings(case):
    text = '\n'.join(case.get('errors', []) + [s.get('status', '') for s in case.get('samples', [])])
    result = set()
    for name in re.findall(r'No such class:\s*([^\s]+)', text):
        result.add('Missing class: ' + name)
    for slot, label in re.findall(r'Unimplemented:\s*(\d+):\s*([^\r\n]+)', text):
        result.add('Unimplemented slot ' + slot + ' (reported label: ' + label.strip() + ')')
    for method, owner in re.findall(r'Method ([^\r\n]+?) not found from ([^\s]+)', text):
        result.add('Method lookup: ' + owner + '.' + method)
    if 'Invalid memory access' in text or 'InvalidMemoryAccess(' in text:
        result.add('Invalid guest memory access; call-site investigation required')
    if 'Allocation failure' in text:
        result.add('Guest allocation failure; size/ABI investigation required')
    if 'NumberFormatException' in text:
        result.add('NumberFormatException; inspect caller and input source')
    if 'called `Option::unwrap()`' in text:
        result.add('Host Option unwrap panic; native location required')
    if not result and case.get('classification') != 'smoke-only-no-detected-error':
        result.add(case.get('classification', 'unclassified'))
    return sorted(result)


def summarize(cases):
    groups = collections.defaultdict(list)
    for case in cases:
        for finding in findings(case):
            groups[finding].append(case['game'])
    return {
        'packages_checked': len(cases),
        'classifications': dict(collections.Counter(c['classification'] for c in cases)),
        'findings': dict(sorted(groups.items())),
        'limitation': 'Findings can overlap. Caught exceptions and static screens require review. Smoke success is not playability. Stub labels are not verified ABI identities.',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('boot_json', type=pathlib.Path)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    cases = json.loads(args.boot_json.read_text())
    summary = summarize(cases)
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2))
    lines = ['# KTF/Android boot audit', '', f"Packages checked: **{summary['packages_checked']}**", '', summary['limitation'], '', '| Result | Packages |', '| --- | ---: |']
    lines.extend(f'| {key} | {value} |' for key, value in summary['classifications'].items())
    lines.extend(['', '## Grouped findings', ''])
    for finding, games in summary['findings'].items():
        lines += [f'### {finding}', '', ', '.join(pathlib.Path(g).name for g in games), '']
    lines += ['## Per-package results', '', '| Archive | Result |', '| --- | --- |']
    lines.extend(f"| {pathlib.Path(c['game']).name.replace('|', '/')} | {c['classification']} |" for c in cases)
    (args.output / 'summary.md').write_text('\n'.join(lines) + '\n')
    print(json.dumps(summary['classifications']))


if __name__ == '__main__':
    main()
