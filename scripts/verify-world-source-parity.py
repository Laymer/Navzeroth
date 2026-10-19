#!/usr/bin/env python3
"""Bounded actual-world mapping parity. Never normalize compared output bytes."""
import argparse
import hashlib
import json
import os
import pathlib
import re
import stat
import subprocess
import sys
import tempfile
import time

META = 8 * 1024 * 1024
ORIGINAL = 64 * 1024 * 1024
CATALOG = 576 * 1024 * 1024
TOTAL = 2 * 1024 * 1024 * 1024
MAX_FILES = 256
OUTPUTS = ('catalog.json', 'sources.json', 'conflicts.json', 'summary.json')
POLICY = 'release/v1/source-normalization-policy.json'
TARGET = 'release/v1/world-data-target.json'
REGISTRY = 'docs/sources/source-admission-v1.json'
RUN = 'data-generators/output/run-manifest.json'
ABSENT = 'Actual admitted world closure/policy are absent; only legacy source knowledge is checked. Actual world parity remains unverified.'


def safe_path(value):
    if (not isinstance(value, str) or not value or len(value) > 1024
            or not re.fullmatch(r'[A-Za-z0-9_./-]+', value)
            or value.startswith(('/', '-')) or any(p in ('', '.', '..') or p.startswith('.') for p in value.split('/'))):
        raise ValueError('unsafe resource path')
    return value


def regular(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
        raise ValueError('linked or special input refused')
    for parent in path.parents:
        info_parent = parent.lstat()
        if stat.S_ISLNK(info_parent.st_mode) or getattr(info_parent, 'st_file_attributes', 0) & 0x400:
            raise ValueError('linked input ancestor refused')
    return info


def stamp(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)


def read_resource(path, maximum, capture=False):
    before = regular(path)
    if before.st_size > maximum:
        raise ValueError('resource exceeds its role bound')
    fd = os.open(path, os.O_RDONLY | getattr(os, 'O_BINARY', 0) | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0))
    result, size, parts = hashlib.sha256(), 0, []
    with os.fdopen(fd, 'rb') as stream:
        if stamp(os.fstat(stream.fileno())) != stamp(before):
            raise ValueError('resource changed before reading')
        while chunk := stream.read(min(1024 * 1024, maximum + 1 - size)):
            size += len(chunk)
            if size > maximum:
                raise ValueError('resource grew beyond its role bound')
            result.update(chunk)
            if capture:
                parts.append(chunk)
        if stamp(os.fstat(stream.fileno())) != stamp(before) or stamp(regular(path)) != stamp(before) or size != before.st_size:
            raise ValueError('resource changed during reading')
    return {'bytes': size, 'sha256': result.hexdigest()}, b''.join(parts) if capture else None


def digest_file(path, maximum):
    return read_resource(path, maximum)[0]


def read_json(path, expected=None):
    actual, raw = read_resource(path, META, capture=True)
    if expected is not None and actual != {k: expected[k] for k in ('bytes', 'sha256')}:
        raise ValueError('metadata differs from committed resource; hydrate exact inputs first')
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError('duplicate JSON key')
            result[key] = value
        return result
    def constant(_):
        raise ValueError('nonfinite JSON number')
    value = json.loads(raw.decode('utf-8'), object_pairs_hook=pairs, parse_constant=constant)
    pending = [(value, 0)]
    while pending:
        node, depth = pending.pop()
        if depth > 64:
            raise ValueError('metadata nesting exceeds bound')
        if isinstance(node, dict):
            pending.extend((v, depth + 1) for v in node.values())
        elif isinstance(node, list):
            pending.extend((v, depth + 1) for v in node)
    return value


def git(root, *args):
    # File-backed capture prevents Git metadata from allocating an unbounded
    # Python buffer. Each successful response is admitted only below META.
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        result = subprocess.run(['git', '-C', str(root), *args], stdout=output, stderr=errors, timeout=120)
        if output.tell() > META or errors.tell() > META:
            raise ValueError('Git metadata exceeds bound')
        if result.returncode:
            raise subprocess.CalledProcessError(result.returncode, result.args)
        output.seek(0)
        return output.read(META + 1)


def tracked(root):
    files = git(root, 'ls-files', '-z').decode('utf-8').split('\0')
    if len(files) > 32768:
        raise ValueError('tracked resource count exceeds bound')
    return {p for p in files if p}


def pin(root, path):
    safe_path(path)
    size = git(root, 'cat-file', '-s', f'HEAD:{path}').decode().strip()
    if not size.isdecimal() or int(size) > META:
        raise ValueError('Git resource must be a bounded blob or LFS pointer')
    raw = git(root, 'show', f'HEAD:{path}')
    match = re.fullmatch(rb'version https://git-lfs.github.com/spec/v1\noid sha256:([a-f0-9]{64})\nsize ([1-9][0-9]*)\n', raw)
    if raw.startswith(b'version https://git-lfs.github.com/spec/v1') and not match:
        raise ValueError('malformed LFS pointer')
    if match:
        return {'path': path, 'bytes': int(match[2]), 'sha256': match[1].decode()}
    return {'path': path, 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def verified_json(root, path):
    expected = pin(root, path)
    if expected['bytes'] > META:
        raise ValueError('metadata exceeds bound')
    return read_json(root / path, expected)


def corpus(root):
    files = tracked(root)
    closures = {p for p in files if re.fullmatch(r'content/world/[^/]+/source-closure.json', p)}
    if POLICY not in files and not closures:
        return None
    if POLICY not in files or TARGET not in files:
        raise ValueError('partial admitted-world corpus cannot use the legacy fallback')
    target = verified_json(root, TARGET)
    build = target.get('build', '')
    if not re.fullmatch(r'[0-9]{1,9}(?:\.[0-9]{1,9}){3}', build):
        raise ValueError('world target build must be explicit')
    base = f'content/world/{build}'
    if f'{base}/source-closure.json' not in files:
        raise ValueError('target lacks its admitted closure; world parity cannot be skipped')
    return base


def fetch(root, paths, offline=False):
    ordered = sorted({safe_path(p) for p in paths})
    if not ordered or len(ordered) > MAX_FILES or sum(map(len, ordered)) + len(ordered) > 24000:
        raise ValueError('resource fetch selection exceeds bounds')
    # No wildcard, media, broad object-directory fetch or quota change.
    if offline:
        git(root, 'lfs', 'checkout', *ordered)
    else:
        git(root, 'lfs', 'pull', '--include=' + ','.join(ordered), '--exclude=')


def selection(root, base):
    files = tracked(root)
    manifest = verified_json(root, RUN)
    if manifest.get('format') != 'navzeroth-world-data-run':
        raise ValueError('unsupported world run manifest')
    declarations = {}
    for generator in manifest['generators']:
        for reference in generator['files']:
            path = safe_path(reference['path'])
            if 'facts' not in path.split('/') and not any(p.startswith('.') for p in path.split('/')):
                full = 'data-generators/output/' + path
                if full in declarations:
                    raise ValueError('duplicate raw-shard declaration')
                declarations[full] = reference
    actual = {p for p in files if p.startswith('data-generators/output/') and p.endswith('.json')
              and p != RUN and 'facts' not in p.split('/') and not any(s.startswith('.') for s in p.split('/'))}
    if not actual or actual != set(declarations):
        raise ValueError('tracked raw shards differ from declared run closure')
    closure = verified_json(root, base + '/source-closure.json')
    if closure.get('schema_version') != 2 or not re.fullmatch(r'[a-f0-9]{40}', closure.get('source_commit', '')):
        raise ValueError('unsupported retained source closure')
    selected, limits = {}, {}
    def add(path, limit, reference=None):
        expected = pin(root, path)
        if reference is not None and any(reference[k] != expected[k] for k in ('bytes', 'sha256')):
            raise ValueError('declared input differs from committed resource')
        if type(expected['bytes']) is not int or expected['bytes'] <= 0 or expected['bytes'] > limit:
            raise ValueError('resource exceeds its role bound')
        selected[path] = expected
        limits[path] = min(limits.get(path, limit), limit)
        if expected['bytes'] > limits[path] or len(selected) > MAX_FILES or sum(f['bytes'] for f in selected.values()) > TOTAL:
            raise ValueError('world resource closure exceeds bounds')
    for path in [RUN, base + '/source-closure.json', TARGET, POLICY, REGISTRY,
                 'release/v1/accepted-origins.json', 'content/authored/slice-v1/catalog.json']:
        add(path, META)
    for name in OUTPUTS:
        add(base + '/' + name, CATALOG if name == 'catalog.json' else META)
    for path, ref in sorted(declarations.items()):
        add(path, META, ref)
    originals = set()
    def retained(ref, maximum):
        path = safe_path(ref['path'])
        if path != 'content/source-materials/objects/' + ref['sha256'] or not re.fullmatch(r'[a-f0-9]{64}', ref['sha256']):
            raise ValueError('retained input is not content addressed')
        add(path, maximum, ref)
    for source in closure['sources']:
        for ref in source['files']:
            retained(ref, ORIGINAL)
            if source['id'] in ('cmangos-classic-db', 'wago-tools-wowf'):
                originals.add(ref['path'])
        for ref in source['permission_evidence']:
            retained(ref, META)
    for ref in closure['corresponding_source']['files']:
        retained(ref, META)
    return {'schema_version': 1, 'base': base, 'source_commit': closure['source_commit'],
            'transformation': closure['transformation'], 'raw_shards': len(actual),
            'upstream_originals': len(originals), 'files': list(sorted(selected.values(), key=lambda f: f['path']))}


def hydrate(root, offline=False):
    base = corpus(root)
    if base is None:
        return None
    fetch(root, [RUN, base + '/source-closure.json'], offline)
    result = selection(root, base)
    fetch(root, [f['path'] for f in result['files']], offline)
    for reference in result['files']:
        if digest_file(root / reference['path'], reference['bytes']) != {k: reference[k] for k in ('bytes', 'sha256')}:
            raise ValueError('hydrated input differs from committed/declared resource')
    # Shallow checkout need only this explicitly pinned code commit, not history/media.
    if not offline:
        try:
            git(root, 'cat-file', '-e', result['source_commit'] + '^{commit}')
        except subprocess.CalledProcessError:
            git(root, 'fetch', '--no-tags', '--depth=1', 'origin', result['source_commit'])
    return result


def verify(root, output):
    started = time.monotonic()
    output.mkdir(parents=True, exist_ok=True)
    base = corpus(root)
    receipt = {'schema_version': 1, 'source_commit': git(root, 'rev-parse', 'HEAD').decode().strip(),
               'platform': sys.platform, 'python': sys.version, 'status': 'absent' if base is None else 'verified'}
    if base is None:
        receipt['reason'] = ABSENT
        print(ABSENT)
    else:
        inputs = selection(root, base)
        for reference in inputs['files']:
            if digest_file(root / reference['path'], reference['bytes']) != {k: reference[k] for k in ('bytes', 'sha256')}:
                raise ValueError('selected input changed after hydration')
        products = []
        with tempfile.TemporaryDirectory(prefix='navzeroth-world-parity-') as temporary:
            for attempt in (1, 2):
                destination = pathlib.Path(temporary) / str(attempt)
                subprocess.run([sys.executable, str(root / 'data-generators/world_to_content.py'),
                    '--input', str(root / 'data-generators/output'), '--target', str(root / TARGET),
                    '--policy', str(root / 'release/v1/accepted-origins.json'),
                    '--authored', str(root / 'content/authored/slice-v1'), '--out', str(destination),
                    '--admission', str(root / REGISTRY), '--source-closure', str(root / base / 'source-closure.json'),
                    '--normalization-policy', str(root / POLICY)], cwd=root, check=True, timeout=1800)
                current = []
                for name in OUTPUTS:
                    maximum = CATALOG if name == 'catalog.json' else META
                    actual = digest_file(destination / name, maximum)
                    expected = digest_file(root / base / name, maximum)
                    if actual != expected:
                        raise ValueError('actual mapped bytes differ from committed output: ' + name)
                    current.append({'path': name, **actual})
                products.append(current)
        if products[0] != products[1]:
            raise ValueError('actual world mapping is not reproducible')
        receipt.update(inputs=inputs, products=products[0], raw_shard_regeneration=False,
                       scope='Bound content mapper and original/code/shard closure; raw extractor regeneration and desktop performance are separate checks.')
    if git(root, 'rev-parse', 'HEAD').decode().strip() != receipt['source_commit']:
        raise ValueError('checked source revision changed during parity verification')
    receipt['elapsed_seconds'] = time.monotonic() - started
    receipt['memory_limitations'] = 'Parity checks do not establish installer/desktop or ordinary-PC resource use. Hosted Windows application memory remains a separate gate.'
    (output / 'world-parity.json').write_bytes((json.dumps(receipt, sort_keys=True, indent=2) + '\n').encode('utf-8'))
    return receipt


def validate_receipt(value):
    if value.get('schema_version') != 1 or not re.fullmatch(r'[a-f0-9]{40}', value.get('source_commit', '')):
        raise ValueError('invalid platform receipt identity')
    if value.get('status') == 'absent':
        if value.get('reason') != ABSENT:
            raise ValueError('absent world result must disclose unverified scope')
        return
    if value.get('status') != 'verified' or value.get('raw_shard_regeneration') is not False:
        raise ValueError('unsupported world parity result')
    inputs = value['inputs']
    if (inputs.get('schema_version') != 1 or not re.fullmatch(r'[a-f0-9]{40}', inputs.get('source_commit', ''))
            or type(inputs.get('raw_shards')) is not int or inputs['raw_shards'] <= 0
            or type(inputs.get('upstream_originals')) is not int or inputs['upstream_originals'] <= 0):
        raise ValueError('incomplete actual world input identity')
    for references in (inputs['files'], value['products']):
        seen, total = set(), 0
        if not references or len(references) > MAX_FILES:
            raise ValueError('receipt file count exceeds bounds')
        for ref in references:
            path = safe_path(ref['path'])
            if (path in seen or type(ref['bytes']) is not int or not 0 < ref['bytes'] <= CATALOG
                    or not re.fullmatch(r'[a-f0-9]{64}', ref['sha256'])):
                raise ValueError('invalid receipt file identity')
            seen.add(path)
            total += ref['bytes']
        if total > TOTAL:
            raise ValueError('receipt aggregate exceeds bound')
    if {ref['path'] for ref in value['products']} != set(OUTPUTS):
        raise ValueError('world receipt must include every mapped output')


def compare(left, right, expected_commit=None):
    a, b = read_json(left), read_json(right)
    validate_receipt(a)
    validate_receipt(b)
    if expected_commit is not None and (not re.fullmatch(r'[a-f0-9]{40}', expected_commit) or a['source_commit'] != expected_commit):
        raise ValueError('platform receipt is not bound to the requested source revision')
    for key in ('schema_version', 'status', 'source_commit'):
        if a[key] != b[key]:
            raise ValueError('source-platform identity/status differs')
    if a['status'] == 'verified':
        if a['inputs'] != b['inputs'] or a['products'] != b['products']:
            raise ValueError('actual world source platform hashes differ')
        print('Actual admitted world outputs and complete selected input hashes match across platforms.')
    elif a['status'] == 'absent':
        print(ABSENT)
    else:
        raise ValueError('unsupported world parity result')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('mode', 'hydrate', 'verify', 'compare'))
    parser.add_argument('--root', type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[1])
    parser.add_argument('--out', type=pathlib.Path)
    parser.add_argument('--offline', action='store_true', help='Use only already-local LFS objects; no fetch')
    parser.add_argument('--left', type=pathlib.Path)
    parser.add_argument('--right', type=pathlib.Path)
    parser.add_argument('--expected-commit', help='Bind comparison to the workflow source revision')
    args = parser.parse_args()
    try:
        root = args.root.resolve()
        if args.command == 'mode':
            print('actual' if corpus(root) else 'legacy')
        elif args.command == 'hydrate':
            result = hydrate(root, args.offline)
            print(json.dumps(result if result else {'status': 'absent', 'reason': ABSENT}, sort_keys=True))
        elif args.command == 'verify' and args.out:
            verify(root, args.out)
        elif args.command == 'compare' and args.left and args.right:
            compare(args.left, args.right, args.expected_commit)
        else:
            parser.error('output or comparison paths required')
    except (ValueError, OSError, KeyError, TypeError, RecursionError, subprocess.SubprocessError) as error:
        parser.exit(1, f'world source parity: {error}\n')


if __name__ == '__main__':
    main()
