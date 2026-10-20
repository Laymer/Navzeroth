#!/usr/bin/env python3
"""Hydrate only the two immutable role-audit inputs, never the world/media corpus."""
import argparse
import importlib.util
import json
import pathlib
import re

_spec = importlib.util.spec_from_file_location('role_parity', pathlib.Path(__file__).with_name('verify-world-source-parity.py'))
parity = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(parity)
QUEST = 'data-generators/output/quests/quests/vanilla_1_12/1.12.1/part-0000.json'
RUN = 'data-generators/output/run-manifest.json'
REPORT = 'docs/data/role-coverage-v1.json'
INPUTS = (QUEST, RUN)
MAX_INPUT = 8 * 1024 * 1024  # existing shard/metadata bound; actual quest ~2MiB
POINTER = re.compile(rb'version https://git-lfs.github.com/spec/v1\noid sha256:([a-f0-9]{64})\nsize ([1-9][0-9]*)\n')


def pins(root):
    result = {}
    for path in INPUTS:
        raw = parity.git(root, 'show', 'HEAD:' + path)
        match = POINTER.fullmatch(raw)
        if not match:
            raise ValueError('role-audit input must be an exact canonical HEAD LFS pointer')
        size = int(match[2])
        if size > MAX_INPUT:
            raise ValueError('role-audit input exceeds its fixed role bound')
        result[path] = ({'path': path, 'bytes': size, 'sha256': match[1].decode()}, raw)
    return result


def hydrate(root, offline=False):
    root = pathlib.Path(root).resolve()
    head = parity.git(root, 'rev-parse', 'HEAD').strip()
    selected = pins(root)
    report = parity.verified_json(root, REPORT)
    source = report['source']
    expected = selected[QUEST][0]
    if (source.get('path') != QUEST or type(source.get('bytes')) is not int
            or {k: source.get(k) for k in ('path', 'bytes', 'sha256')} != expected):
        raise ValueError('role audit differs from exact allowed quest input pin')
    pending = []
    for path, (pin, pointer) in selected.items():
        actual, raw = parity.read_resource(root / path, MAX_INPUT, capture=True)
        if raw == pointer:
            pending.append(path)
        elif actual != {k: pin[k] for k in ('bytes', 'sha256')}:
            raise ValueError('working input is neither exact HEAD pointer nor verified bytes')
    # Verify the bounded manifest before any quest-object fetch. A pointer is
    # first restored only from its exact immutable HEAD metadata selection.
    if RUN in pending:
        parity.fetch(root, [RUN], offline)
    manifest = parity.read_json(root / RUN, selected[RUN][0])
    if (manifest.get('format') != 'navzeroth-world-data-run'
            or type(manifest.get('schema_version')) is not int or manifest['schema_version'] != 1):
        raise ValueError('unsupported role-audit run manifest')
    rows = [row for generator in manifest['generators'] for row in generator['files']
            if row.get('path') == QUEST.removeprefix('data-generators/output/')]
    if len(rows) != 1 or type(rows[0].get('bytes')) is not int or any(rows[0].get(k) != expected[k] for k in ('bytes', 'sha256')):
        raise ValueError('run manifest does not bind the exact role-audit quest pin once')
    if QUEST in pending:
        parity.fetch(root, [QUEST], offline)
    values = {QUEST: parity.read_json(root / QUEST, expected)}
    if parity.read_json(root / RUN, selected[RUN][0]) != manifest:
        raise ValueError('run manifest changed during hydration')
    shard = values[QUEST]
    if (shard.get('origin') != source.get('origin') or shard.get('confidence') != source.get('confidence')
            or type(source.get('records')) is not int or len(shard['records']) != source['records']):
        raise ValueError('hydrated quest provenance/count differs from role audit')
    if parity.git(root, 'rev-parse', 'HEAD').strip() != head or pins(root) != selected:
        raise ValueError('source identity changed during hydration')
    if parity.verified_json(root, REPORT) != report:
        raise ValueError('role audit changed during hydration')
    return {'status': 'verified', 'source_commit': head.decode(), 'inputs': [selected[p][0] for p in INPUTS],
            'hydrated': pending, 'network': bool(pending and not offline)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, default=pathlib.Path('.'))
    parser.add_argument('--offline', action='store_true', help='Use already-local immutable LFS objects only')
    args = parser.parse_args()
    print(json.dumps(hydrate(args.root, args.offline), sort_keys=True))


if __name__ == '__main__':
    main()
