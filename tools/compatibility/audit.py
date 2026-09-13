#!/usr/bin/env python3
"""Private device compatibility evidence. Smoke success does not imply playability."""
import argparse
import hashlib
import io
import json
import pathlib
import re
import shlex
import subprocess
import struct
import zipfile

PACKAGE = 'local.wie.nativeapp'
RUNNER = PACKAGE + '.test/' + PACKAGE + '.RescueInstrumentation'
ERROR = re.compile(r'\bERROR\b|Unimplemented|Unknown .*SVC|No such (?:method|field)|vtable index|Exception|panicked|Fatal|FATAL|Error:', re.I)


def parse_keys(value):
    import argparse
    aliases = {'LSOFT': 'L', 'RSOFT': 'R'}
    supported = {'UP', 'DOWN', 'LEFT', 'RIGHT', 'OK', 'L', 'R', 'CLR', 'CALL', '*', '#', *'0123456789'}
    keys = [aliases.get(key.strip().upper(), key.strip().upper()) for key in value.split(',')]
    unknown = [key for key in keys if key not in supported]
    if unknown:
        raise argparse.ArgumentTypeError('Unsupported native input key: ' + repr(unknown))
    return ','.join(keys)


def classify(samples, errors):
    if any(s['status'].startswith('Error:') for s in samples):
        return 'native-error'
    if errors:
        return 'logged-error-review'
    if samples and samples[-1]['status'].lower().startswith(('stopped', 'game stopped')):
        return 'stopped-review'
    if not samples or samples[-1]['paints'] == 0:
        return 'no-paints-review'
    if samples[-1]['paints'] == samples[0]['paints']:
        return 'static-screen-review'
    return 'smoke-only-no-detected-error'


def symbols(data):
    # Independent strings are leads, never asserted owner/method/slot associations.
    strings = {m.group().decode('ascii') for m in re.finditer(rb'[ -~]{3,}', data)}
    classes = sorted(s for s in strings if re.fullmatch(r'(?:java|javax|org/kwis|com)/[\w/$]+', s))
    descriptors = sorted(s for s in strings if re.fullmatch(r'\([\w/;$\[\]]*\)[\w/;$\[\]]+', s))
    return classes, descriptors


def native_members(data, abi):
    """Recognize pointer-backed LgtJavaClassMethod-shaped records, without guessing owner/slot."""
    if data[:6] != b'\x7fELF\x01\x01' or len(data) < 52:
        return []
    section_offset = struct.unpack_from('<I', data, 32)[0]
    section_size, section_count = struct.unpack_from('<HH', data, 46)
    if section_size < 40 or section_offset + section_size * section_count > len(data):
        raise ValueError('Malformed ELF section table')
    sections = []
    for index in range(section_count):
        header = struct.unpack_from('<10I', data, section_offset + section_size * index)
        _, kind, _, address, offset, size, *_ = header
        if kind != 8 and address and size:
            if offset + size > len(data):
                raise ValueError('ELF section outside archive member')
            sections.append((address, data[offset:offset + size]))
    def string_at(pointer):
        for address, blob in sections:
            if address <= pointer < address + len(blob):
                start = pointer - address
                end = blob.find(b'\0', start, start + 1024)
                if end >= 0:
                    try:
                        return blob[start:end].decode('ascii')
                    except UnicodeDecodeError:
                        pass
        return ''
    records = []
    for address, blob in sections:
        for offset in range(0, len(blob) - 27, 4):
            owner, name_ptr, descriptor_ptr, flags, words, _, target, _ = struct.unpack_from('<IIIHHIII', blob, offset)
            name, descriptor = string_at(name_ptr), string_at(descriptor_ptr)
            if not re.fullmatch(r'(?:[a-zA-Z_$][\w$]*|<init>|<clinit>)', name):
                continue
            if not re.fullmatch(r'\([\w/;$\[\]]*\)[\w/;$\[\]]+', descriptor):
                continue
            if flags & ~0x1dff or words > 255:
                continue
            matches = [{'class': c, 'index': m['index']} for c, methods in abi.items()
                       for m in methods if m['name'] == name and m['descriptor'] == descriptor]
            records.append({'address': hex(address + offset), 'name': name, 'descriptor': descriptor,
                            'owner_pointer': hex(owner), 'target': hex(target),
                            'matching_explicit_mappings': matches,
                            'evidence': 'pointer-backed metadata candidate; owner and numeric slot not inferred'})
    return records


def ktf_members(data, supported):
    # KtfJvmSupport::JavaFullName stores tag + descriptor + '+' + name + NUL.
    records = []
    pattern = rb'\([A-Za-z0-9_/$;\[\]]*\)[A-Za-z0-9_/$;\[\]]+\+[A-Za-z0-9_$<>]+\x00'
    for match in re.finditer(pattern, data):
        descriptor, name = match.group()[:-1].decode('ascii').split('+', 1)
        records.append({'offset': match.start(), 'tag': data[match.start()-1] if match.start() else None,
                        'name': name, 'descriptor': descriptor,
                        'matching_source_prototypes': supported.get((name, descriptor), []),
                        'evidence': 'KTF encoded member name; owner and numeric slot not inferred'})
    return records


def source_prototypes(repo):
    result = {}
    # Source declaration matches are leads, not proof of runtime registration or behavior.
    for folder in ['wie-wipi-java/src', 'wie-midp/src', 'wie-ktf/src']:
        for path in (repo / folder).rglob('*.rs'):
            source = path.read_text()
            for name, descriptor in re.findall(r'JavaMethodProto::new(?:_abstract)?\(\s*"([^"\n]+)"\s*,\s*"([^"\n]+)"', source):
                result.setdefault((name, descriptor), []).append(str(path.relative_to(repo)))
    return result


def scan_archive(data, abi, origin='', depth=0, supported=None):
    result = []
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for item in archive.infolist():
            if item.file_size > 128 * 1024 * 1024:
                raise ValueError('Archive member exceeds audit limit')
            name = origin + item.filename
            if item.filename.lower().endswith(('.jar', '.zip')) and depth < 2:
                try:
                    result.extend(scan_archive(archive.read(item), abi, name + '!', depth + 1, supported))
                except zipfile.BadZipFile:
                    result.append({'member': name, 'error': 'Nested archive is not a valid ZIP/JAR'})
            elif item.filename.lower().endswith(('.mod', '.class')) or re.search(r'(?:^|/)client\.bin[0-9]*$', item.filename, re.I):
                blob = archive.read(item)
                classes, descriptors = symbols(blob)
                result.append({'member': name, 'sha256': hashlib.sha256(blob).hexdigest(),
                               'class_symbols': classes, 'descriptor_symbols': descriptors,
                               'native_member_candidates': ktf_members(blob, supported or {}) if re.search(r'client\.bin[0-9]*$', item.filename, re.I) else native_members(blob, abi),
                               'known_lgt_abi_classes': {} if 'client.bin' in item.filename.lower() else {c: abi[c] for c in classes if c in abi},
                               'classes_without_explicit_slot_table': [c for c in classes if c not in abi],
                               'limitation': 'Independent symbols; absent table is not proof of unsupported API. Numeric call sites require ABI evidence.'})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--serial', required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--boot-samples', type=int, default=1)
    parser.add_argument('--capture-rescue', action='store_true')
    parser.add_argument('--game-id', help='Restrict to one installed archive ID for focused regression reruns')
    parser.add_argument('--restart-after-stop', action='store_true', help='Probe a second launch using the same disposable save after an intentional stop')
    parser.add_argument('--keys', type=parse_keys, default='OK,DOWN,OK,1', help='Comma-separated boot-probe input sequence')
    parser.add_argument('--phase', choices=['boot', 'rescue', 'rescue-capture', 'rescue-verify', 'scan'], required=True)
    parser.add_argument('--rescue-fixtures', type=pathlib.Path, help='Boot-audit directory containing per-game rescue.tar captures')
    parser.add_argument('--reference', type=pathlib.Path, help='Baseline output directory for rescue-verify')
    args = parser.parse_args()
    if args.phase == 'rescue-verify' and not args.reference:
        parser.error('rescue-verify requires --reference from a baseline rescue-capture run')
    args.output.mkdir(parents=True, exist_ok=True)
    def adb(*words, timeout=60, binary=False):
        return subprocess.check_output(['adb', '-s', args.serial, *words], timeout=timeout, text=not binary)
    def shell(*words, **kwargs):
        return adb('shell', shlex.join(words), **kwargs)
    def private(*words, **kwargs):
        return shell('run-as', PACKAGE, *words, **kwargs)
    games = private('find', 'files/games', '-type', 'f').splitlines()
    games = sorted(g for g in games if g.endswith(('.jar', '.zip')))
    if args.game_id:
        games = [g for g in games if g.split('/')[2] == args.game_id]
        if not games:
            parser.error('Requested game ID is not installed')
    device_root = '/data/user/0/' + PACKAGE + '/'
    if args.phase == 'scan':
        # Python 3.11+; only standard-library dependencies.
        import tomllib
        repo = pathlib.Path(__file__).resolve().parents[2]
        table = tomllib.loads((repo / 'wie-lgt/data/lgt_java_abi.toml').read_text())
        abi = {c['name']: c.get('vtable', []) for c in table['class']}
        supported = source_prototypes(repo)
        output = []
        for game in games:
            # exec-out avoids shell terminal newline conversion for binary archives.
            data = adb('exec-out', shlex.join(['run-as', PACKAGE, 'cat', game]), binary=True)
            output.append({'game': game, 'archive_sha256': hashlib.sha256(data).hexdigest(), 'references': scan_archive(data, abi, supported=supported)})
        (args.output / 'scan.json').write_text(json.dumps(output, ensure_ascii=False, indent=2))
        print('Scanned', len(output), 'archives', flush=True)
        return
    shell('am', 'force-stop', PACKAGE)
    results = []
    if args.phase == 'boot':
        for game in games:
            case = args.output / game.split('/')[2]
            case.mkdir(exist_ok=True)
            previous = case / 'result.json'
            if previous.exists():
                saved = json.loads(previous.read_text())
                if saved.get('log_capture_verified'):
                    results.append(saved)
                    continue
            # Separate target process and isolated cache saves on every run.
            shell('am', 'force-stop', PACKAGE)
            private('rm', '-rf', 'cache/compatibility-audit')
            output = ''
            try:
                output = shell('am', 'instrument', '-w', '-r', '-e', 'mode', 'compatibility', '-e', 'game', device_root + game, '-e', 'keys', args.keys, '-e', 'bootSamples', str(args.boot_samples), '-e', 'captureRescue', str(args.capture_rescue).lower(), '-e', 'restartAfterStop', str(args.restart_after_stop).lower(), RUNNER, timeout=max(60, 35 + 5 * args.boot_samples + len(args.keys.split(','))))
                (case / 'instrumentation.txt').write_text(output)
                if private('sh', '-c', 'test -f cache/compatibility-audit/boot.png && echo yes || true').strip() == 'yes':
                    (case / 'boot.png').write_bytes(adb('exec-out', shlex.join(['run-as', PACKAGE, 'cat', 'cache/compatibility-audit/boot.png']), binary=True))
                report = json.loads(private('cat', 'cache/compatibility-audit/result.json'))
                for sample in report['samples']:
                    if 'image' in sample:
                        name = pathlib.Path(sample['image']).name
                        (case / name).write_bytes(adb('exec-out', shlex.join(['run-as', PACKAGE, 'cat', 'cache/compatibility-audit/' + name]), binary=True))
                if 'INSTRUMENTATION_CODE: 0' not in output:
                    raise RuntimeError('Instrumentation did not complete successfully')
                logs = adb('logcat', '-d', '--pid=' + str(report['pid']), '-v', 'epoch')
                (case / 'logcat.txt').write_text(logs)
                native = private('cat', 'cache/compatibility-audit/saves/case.frames.log')
                (case / 'native.log').write_text(native)
                if args.restart_after_stop and private('sh', '-c', 'test -f cache/compatibility-audit/first-start.frames.log && echo yes || true').strip() == 'yes':
                    (case / 'first-start.frames.log').write_text(private('cat', 'cache/compatibility-audit/first-start.frames.log'))
                captured = 'cache/compatibility-audit/rescues/case/latest'
                if private('sh', '-c', 'test -d ' + captured + ' && echo yes || true').strip() == 'yes':
                    rescue = adb('exec-out', shlex.join(['run-as', PACKAGE, 'tar', '-cf', '-', '-C', captured, '.']), binary=True)
                    (case / 'rescue.tar').write_bytes(rescue)
                manual = 'cache/compatibility-audit/rescues/case/manual/latest'
                if args.capture_rescue and private('sh', '-c', 'test -d ' + manual + ' && echo yes || true').strip() == 'yes':
                    (case / 'manual-rescue.tar').write_bytes(adb('exec-out', shlex.join(['run-as', PACKAGE, 'tar', '-cf', '-', '-C', manual, '.']), binary=True))
                errors = [line for line in (logs + '\n' + native).splitlines() if ERROR.search(line)]
                report.update(game=game, keys=args.keys, errors=errors, log_capture_verified=True, classification=classify(report['samples'], errors))
            except (subprocess.SubprocessError, ValueError, RuntimeError) as error:
                report = {'game': game, 'classification': 'harness-or-process-failure', 'error': str(error)}
                try:
                    pid = private('cat', 'cache/compatibility-audit/process-id').strip()
                    if pid.isdigit():
                        (case / 'logcat.txt').write_text(adb('logcat', '-d', '--pid=' + pid, '-v', 'epoch'))
                except subprocess.SubprocessError:
                    report['log_limitation'] = 'No process marker available; instrumentation may not have started'
                (case / 'instrumentation.txt').write_text(output)
            (case / 'result.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
            results.append(report)
            (args.output / 'boot.json').write_text(json.dumps(results, ensure_ascii=False, indent=2))
            print(len(results), pathlib.Path(game).name, report['classification'], flush=True)
        # Resumed runs may end with cached cases; persist the complete inventory too.
        (args.output / 'boot.json').write_text(json.dumps(results, ensure_ascii=False, indent=2))
    else:
        # Normal replay retains originating build checks. Never call diagnostic modes implicitly.
        if args.rescue_fixtures:
            reports = []
            for archive in sorted(args.rescue_fixtures.glob('*/rescue.tar')):
                game_id = archive.parent.name
                if not re.fullmatch('[0-9a-f]{64}', game_id):
                    raise ValueError('Invalid rescue fixture archive ID')
                if args.game_id and game_id != args.game_id:
                    continue
                target = 'cache/audit-fixtures/' + game_id + '/latest'
                private('mkdir', '-p', target)
                adb('push', str(archive), '/data/local/tmp/gomul-audit-fixture.tar')
                private('tar', '-xf', '/data/local/tmp/gomul-audit-fixture.tar', '-C', target)
                reports.append(target + '/error.txt')
        else:
            reports = private('find', 'files/rescues', '-name', 'error.txt').splitlines()
        for error_file in reports:
            if args.game_id and error_file.split('/')[2] != args.game_id:
                continue
            report_path = error_file.rsplit('/', 1)[0]
            game = next((g for g in games if g.split('/')[2] == report_path.split('/')[2]), None)
            result = {'report': report_path, 'game': game}
            if game is None:
                result['classification'] = 'matching-game-not-installed'
            else:
                shell('am', 'force-stop', PACKAGE)
                try:
                    extra = []
                    key = report_path.replace('/', '_')
                    if args.phase == 'rescue-capture':
                        extra = ['-e', 'mode', 'rescue-capture']
                    elif args.phase == 'rescue-verify':
                        reference = args.reference / (key + '.tar')
                        if not reference.is_file():
                            raise ValueError('No captured baseline oracle for this report')
                        adb('push', str(reference), '/data/local/tmp/gomul-rescue-reference.tar')
                        private('rm', '-rf', 'cache/audit-reference')
                        private('mkdir', '-p', 'cache/audit-reference')
                        private('tar', '-xf', '/data/local/tmp/gomul-rescue-reference.tar', '-C', 'cache/audit-reference')
                        extra = ['-e', 'mode', 'rescue-verify', '-e', 'reference', device_root + 'cache/audit-reference', '-e', 'runMs', '3000']
                    output = shell('am', 'instrument', '-w', '-r', '-e', 'rescue', device_root + report_path,
                                   '-e', 'game', device_root + game, *extra, RUNNER, timeout=60)
                    result['output'] = output
                    native = private('sh', '-c', 'if [ -f cache/rescue-reproduction/saves/case.frames.log ]; then cat cache/rescue-reproduction/saves/case.frames.log; fi')
                    (args.output / (key + '.native.log')).write_text(native)
                    result['logged_errors'] = [line for line in native.splitlines() if ERROR.search(line)]
                    if args.phase == 'rescue':
                        result['classification'] = 'failure-reproduced' if 'Exact recorded failure reproduced: PASS' in output else 'replay-blocked-or-diverged'
                    elif 'Rescue diagnostic replay: PASS' in output:
                        result['classification'] = 'prefix-oracle-captured' if args.phase == 'rescue-capture' else 'prefix-oracle-matched'
                        if args.phase == 'rescue-capture':
                            reference_path = 'cache/rescue-reproduction/checkpoints/case/rescue-reference'
                            archive = adb('exec-out', shlex.join(['run-as', PACKAGE, 'tar', '-cf', '-', '-C', reference_path, '.']), binary=True)
                            (args.output / (key + '.tar')).write_bytes(archive)
                    else:
                        result['classification'] = 'continuation-native-error' if 'Continuation: Error:' in output else 'replay-blocked-or-diverged'
                    if result['classification'] == 'prefix-oracle-matched' and result['logged_errors']:
                        result['classification'] = 'prefix-matched-with-logged-error'
                except (subprocess.SubprocessError, ValueError) as error:
                    result.update(classification='baseline-unavailable' if str(error) == 'No captured baseline oracle for this report' else 'harness-or-process-failure', error=str(error))
            results.append(result)
            (args.output / (args.phase + '.json')).write_text(json.dumps(results, ensure_ascii=False, indent=2))
            print(report_path, result['classification'], flush=True)
    shell('am', 'force-stop', PACKAGE)
    shell('am', 'start', '-n', PACKAGE + '/.MainActivity')


if __name__ == '__main__':
    main()
