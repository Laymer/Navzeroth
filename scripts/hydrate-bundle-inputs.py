#!/usr/bin/env python3
"""Hydrate only HEAD-pinned declared bundle files; production CLI verifies trust."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import tempfile

_spec = importlib.util.spec_from_file_location('bundle_parity', pathlib.Path(__file__).with_name('verify-world-source-parity.py'))
parity = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(parity)
MANIFEST = 'release/v1/bundled-content-manifest.json'
IDENTITY = 'release/v1/bundled-content-build-identity.json'
MAX_MANIFEST = 4 * 1024 * 1024
MAX_FILE = 576 * 1024 * 1024
MAX_TOTAL = 2 * 1024 * 1024 * 1024
MAX_FILES = 8192
POINTER = re.compile(rb'version https://git-lfs.github.com/spec/v1\noid sha256:([a-f0-9]{64})\nsize ([1-9][0-9]*)\n')


def pin(root, path):
    parity.safe_path(path)
    size = int(parity.git(root, 'cat-file', '-s', 'HEAD:' + path))
    if size > MAX_FILE:
        raise ValueError('HEAD blob exceeds bundle bound')
    if size <= 1024:
        raw = parity.git(root, 'show', 'HEAD:' + path)
        match = POINTER.fullmatch(raw)
        if match:
            return {'bytes': int(match[2]), 'sha256': match[1].decode()}, raw
        if raw.startswith(b'version https://git-lfs.github.com/spec/v1'):
            raise ValueError('noncanonical bundle pointer')
        return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}, None
    # Large ordinary Git blobs are hashed without buffering the resource.
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        subprocess.run(['git', '-C', str(root), 'show', 'HEAD:' + path], stdout=output,
                       stderr=errors, check=True, timeout=120)
        if output.tell() != size:
            raise ValueError('HEAD blob size changed')
        output.seek(0)
        digest = hashlib.file_digest(output, 'sha256').hexdigest()
    return {'bytes': size, 'sha256': digest}, None


def metadata(root, path, maximum):
    expected, pointer = pin(root, path)
    if pointer is not None:
        raise ValueError('bundle metadata must be ordinary committed JSON')
    actual, raw = parity.read_resource(root / path, maximum, capture=True)
    if actual != expected:
        raise ValueError('bundle metadata differs from HEAD')
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError('duplicate bundle JSON key')
            result[key] = value
        return result
    def constant(_):
        raise ValueError('nonfinite bundle JSON number')
    value = json.loads(raw.decode('utf-8'), object_pairs_hook=pairs, parse_constant=constant)
    pending = [(value, 0)]
    while pending:
        node, depth = pending.pop()
        if depth > 64:
            raise ValueError('bundle JSON depth exceeds bound')
        if isinstance(node, dict):
            pending.extend((v, depth + 1) for v in node.values())
        elif isinstance(node, list):
            pending.extend((v, depth + 1) for v in node)
    if not isinstance(value, dict):
        raise ValueError('bundle metadata must be an object')
    return value


def hydrate(root, offline=False):
    root = pathlib.Path(root).resolve()
    head = parity.git(root, 'rev-parse', 'HEAD').strip()
    manifest = metadata(root, MANIFEST, MAX_MANIFEST)
    identity = metadata(root, IDENTITY, 65536)
    if identity.get('app_version') != '1.0.0' or manifest.get('app_version') != '1.0.0':
        raise ValueError('bundle hydration requires explicit stable 1.0.0')
    if identity.get('bundle_manifest_sha256') != parity.digest_file(root / MANIFEST, MAX_MANIFEST)['sha256']:
        raise ValueError('identity does not bind bundle manifest')
    if manifest.get('format') != 'navzeroth-installed-content' or type(manifest.get('schema_version')) is not int or manifest.get('schema_version') != 1:
        raise ValueError('unsupported bundle manifest')
    files = manifest.get('files')
    if not isinstance(files, list) or not 1 <= len(files) <= MAX_FILES:
        raise ValueError('bundle file count exceeds bound')
    selected, pending, total = {}, [], 0
    for ref in files:
        if not isinstance(ref, dict) or set(ref) != {'path', 'bytes', 'sha256'}:
            raise ValueError('invalid bundle FileRef')
        path = parity.safe_path(ref['path'])
        if (path in selected or type(ref['bytes']) is not int or not 0 < ref['bytes'] <= MAX_FILE
                or not isinstance(ref['sha256'], str) or not re.fullmatch('[a-f0-9]{64}', ref['sha256'])):
            raise ValueError('duplicate or invalid bounded bundle FileRef')
        total += ref['bytes']
        if total > MAX_TOTAL:
            raise ValueError('bundle aggregate exceeds bound')
        expected = {k: ref[k] for k in ('bytes', 'sha256')}
        committed, pointer = pin(root, path)
        if committed != expected:
            raise ValueError('bundle FileRef differs from HEAD bytes/pointer')
        actual, raw = parity.read_resource(root / path, MAX_FILE, capture=False)
        if pointer is not None and actual != expected:
            _, raw = parity.read_resource(root / path, 1024, capture=True)
            if raw != pointer:
                raise ValueError('working resource is not its exact HEAD pointer')
            pending.append(path)
        elif actual != expected:
            raise ValueError('working non-LFS resource differs from HEAD')
        selected[path] = expected
    # Fetch helper admits at most256 paths/24KiB per invocation. Every selected
    # item above is validated before fetching; no wildcard or directory pull.
    batch = []
    for path in pending:
        if len(batch) == 256 or sum(map(len, batch)) + len(batch) + len(path) + 1 > 24000:
            parity.fetch(root, batch, offline)
            batch = []
        batch.append(path)
    if batch:
        parity.fetch(root, batch, offline)
    for path, expected in selected.items():
        if parity.digest_file(root / path, MAX_FILE) != expected:
            raise ValueError('hydrated resource differs from declared HEAD pin')
    if (parity.git(root, 'rev-parse', 'HEAD').strip() != head
            or metadata(root, MANIFEST, MAX_MANIFEST) != manifest
            or metadata(root, IDENTITY, 65536) != identity):
        raise ValueError('bundle source metadata changed during hydration')
    return {'status': 'head-pins-verified', 'source_commit': head.decode(), 'files': len(files),
            'bytes': total, 'hydrated': pending, 'publication_allowed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, default=pathlib.Path('.'))
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    print(json.dumps(hydrate(args.root, args.offline), sort_keys=True))


if __name__ == '__main__':
    main()
