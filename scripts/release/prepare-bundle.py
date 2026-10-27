#!/usr/bin/env python3
"""Prepare unsigned public bundle identities from the trusted material preview."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import tempfile
import time

MAX_JSON = 64 * 1024 * 1024
MAX_TOTAL = 2 * 1024**3
REQUIRED_CHECKS = {'HOSTED-' + name: 'hosted_windows' for name in (
    'INSTALL','FIRST-START','UPGRADE-RC1','DOWNGRADE-REFUSAL','PERSISTENCE-RECOVERY',
    'RESOURCES','OFFLINE','PERFORMANCE')}
REQUIRED_CHECKS['OWNER-INSTALLER-SMOKE'] = 'interactive_windows' 


def canonical(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode()


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate JSON key')
        result[key] = value
    return result


def decode(raw):
    if len(raw) > MAX_JSON:
        raise ValueError('JSON exceeds bound')
    return json.loads(raw.decode('utf-8'), object_pairs_hook=unique,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError('Nonfinite JSON')))


def command(root, args, maximum=MAX_JSON, timeout=1200):
    # File-backed bounded capture avoids an unbounded PIPE allocation. No network.
    with tempfile.TemporaryFile() as output:
        process = subprocess.Popen(args, cwd=root, stdout=output, start_new_session=os.name != 'nt',
                                   creationflags=getattr(subprocess, 'CREATE_NEW_PROCESS_GROUP', 0) if os.name == 'nt' else 0)
        deadline = time.monotonic() + timeout
        try:
            while process.poll() is None:
                if output.tell() > maximum or time.monotonic() > deadline:
                    raise ValueError('Preparation command exceeds bound')
                time.sleep(.05)
            if process.returncode:
                raise ValueError('Trusted preparation command failed')
            output.seek(0)
            raw = output.read(maximum + 1)
            if len(raw) > maximum:
                raise ValueError('Preparation output exceeds bound')
            return raw
        finally:
            if process.poll() is None:
                if os.name == 'nt':
                    subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10, check=False)
                else:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
            process.wait(timeout=10)


def safe(value):
    if not isinstance(value, str) or not re.fullmatch(r'[a-z0-9_./-]+', value) or any(
            part in ('', '.', '..') for part in value.split('/')):
        raise ValueError('Unsafe source path')
    return value


def validate_ref(ref, maximum=576 * 1024**2):
    if (set(ref) != {'path', 'bytes', 'sha256'} or type(ref['bytes']) is not int
            or not 0 < ref['bytes'] <= maximum
            or not re.fullmatch('[a-f0-9]{64}', ref['sha256'])):
        raise ValueError('Invalid source pin')
    return safe(ref['path'])


def resource(root, ref, maximum=576 * 1024**2, capture=False):
    relative = validate_ref(ref, maximum)
    file = root
    for part in relative.split('/'):
        file /= part
        if file.is_symlink():
            raise ValueError('Linked source refused')
    descriptor = os.open(file, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0))
    chunks = []
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or before.st_size != ref['bytes']:
            raise ValueError('Source size/kind differs')
        digest = hashlib.sha256()
        count = 0
        while True:
            chunk = os.read(descriptor, min(65536, ref['bytes'] - count + 1))
            if not chunk:
                break
            count += len(chunk)
            if count > ref['bytes']:
                raise ValueError('Source grew beyond pin')
            digest.update(chunk)
            if capture:
                chunks.append(chunk)
        after, linked = os.fstat(descriptor), os.stat(file, follow_symlinks=False)
        stamp = lambda value: (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns)
        if count != ref['bytes'] or digest.hexdigest() != ref['sha256'] or stamp(before) != stamp(after) or stamp(before) != stamp(linked):
            raise ValueError('Source content/identity differs')
        return b''.join(chunks) if capture else None
    finally:
        os.close(descriptor)


def fingerprint(raw):
    return hashlib.sha256(b''.join(entry + b'\0' for entry in raw.split(b'\0') if entry
                                  and not entry.split(b'\t', 1)[1].startswith((b'docs/', b'release/')))).hexdigest()


def prepare(root, preview, evidence, review, tested, production, version, evidence_sha256):
    if preview['schema_version'] != 2 or evidence['schema_version'] not in (4, 5) or preview['version'] != version:
        raise ValueError('Preparation requires schema4/5 and schema2 matching version')
    if (preview['production_sha256'] != production or evidence['production_sha256'] != production
            or evidence['tested_source_commit'] != tested or preview['target'] != evidence['target']
            or not preview['target'] or not re.fullmatch('[a-f0-9]{40}', tested)):
        raise ValueError('Explicit source/production/target pins differ')
    preparation = preview.get('unsigned_preparation')
    if (not isinstance(preparation, dict) or preparation.get('ready') is not True
            or preparation.get('evidence_sha256') != evidence_sha256):
        raise ValueError('Exact trusted functional/content preparation assessment must pass')
    if any(row['status'] != 'pass' or row.get('unresolved_conflicts',0) != 0 or row.get('unsupported_corrections',0) != 0 for row in evidence['coverage']):
        raise ValueError('Functional/content coverage must pass before preparation')
    required = dict(REQUIRED_CHECKS)
    if evidence['schema_version'] == 5:
        del required['OWNER-INSTALLER-SMOKE']
        required['DRAFT-INSTALLER-SMOKE'] = 'automated_draft_windows'
    if len(evidence['checks']) != len(required) or {row['id']:row['method'] for row in evidence['checks']} != required:
        raise ValueError('Schema4 acceptance rows/methods are incomplete')
    for check in evidence['checks']:
        if check['status'] == 'unverified':
            if (check.get('artifacts') or any(check.get(field) is not None for field in ('operator','environment','recorded_at','installer_sha256'))):
                raise ValueError('Partial pending acceptance is not pristine')
        elif check['status'] != 'pass':
            raise ValueError('Failed acceptance cannot be prepared')
        else:
            installer = evidence.get('candidate_installer_sha256')
            if (not installer or not re.fullmatch('[a-f0-9]{64}', installer)
                    or check.get('installer_sha256') != installer or not check.get('artifacts')
                    or any(not isinstance(check.get(field), str) or not check[field].strip() for field in ('operator','environment','recorded_at'))):
                raise ValueError('Partial or mismatched acceptance pass')
            for ref in check['artifacts']:
                resource(root, ref, 16*1024**2)
    packs = preview.get('bundled_packs', [])
    if not 0 < len(packs) <= 8 or len(packs) != len(preview['packs']):
        raise ValueError('Actual verified pack keys are missing')
    files = {}
    def add(ref):
        previous = files.setdefault(ref['path'], ref)
        if previous != ref:
            raise ValueError('Conflicting source references')
    for material in preview['files']:
        add(material['input'])
    public = list(files.values())
    add(evidence['origin_policy'])
    add(review)
    private = [ref for source in evidence['sources'] for field in ('permission_evidence', 'target_evidence') for ref in source[field]]
    private += [ref for row in evidence['checks'] for ref in row['artifacts']]
    private += [ref for row in evidence['coverage'] for ref in row['validation_evidence']]
    registry = evidence.get('admission_registry')
    same = lambda a, b: a['path'] == b['path'] or (a['sha256'], a['bytes']) == (b['sha256'], b['bytes'])
    for ref in files.values():
        validate_ref(ref)
    if len(files) > 8190 or sum(ref['bytes'] for ref in files.values()) > MAX_TOTAL:
        raise ValueError('Bundle closure exceeds bound')
    for ref in files.values():
        if registry and same(ref, registry):
            raise ValueError('Private admission registry cannot ship')
        if any(same(ref, pin) for pin in private) and ref not in public + [review, evidence['origin_policy']]:
            raise ValueError('Private evidence cannot ship')
        resource(root, ref)
    target = preview['target']
    scopes = []
    for pack in packs:
        key = pack['key']
        scope = key['scope']
        build = scope['context']['build']
        expected = next((p for p in preview['packs'] if p['manifest'] == pack['manifest']), None)
        if (not expected or expected['dataset_revisions'] != [key['revision']] or scope['fixture']
                or scope['context']['namespace'] != 'reference' or build['edition'] != target['edition']
                or build['build'] != target['version'] + '.' + target['build']
                or str(build['interface']) != str(target['interface'])
                or pack['manifest']['path'] != safe(pack['directory']) + '/manifest.json'
                or files.get(pack['manifest']['path']) != pack['manifest']
                or (scopes and scope != scopes[0])):
            raise ValueError('Actual pack identity differs from target/revision')
        scopes.append(scope)
        if pack.get('conflicts') is not None:
            raise ValueError('Typed conflicts require an explicit reviewed preview contract')
        conflict = pack.get('normalization_conflicts')
        if conflict is not None and (conflict not in evidence.get('normalization_conflict_files', []) or files.get(conflict['path']) != conflict):
            raise ValueError('Normalization conflict is not admitted material')
    for ref in evidence.get('normalization_conflict_files', []):
        if sum(pack.get('normalization_conflicts') == ref for pack in packs) != 1:
            raise ValueError('Normalization report must attach to exactly one pack')
    if len({p['manifest']['path'] for p in packs}) != len(packs):
        raise ValueError('Duplicate bundle pack')
    ref_order = lambda ref: dict(path=ref['path'], bytes=ref['bytes'], sha256=ref['sha256'])
    review = ref_order(review)
    target = {field: target[field] for field in ('edition','version','build','interface')}
    manifest = dict(format='navzeroth-installed-content', schema_version=1, app_version=version,
                    target=target, accepted_origins=evidence['origin_policy'], review=review,
                    files=[ref_order(files[key]) for key in sorted(files)], packs=sorted(packs, key=lambda p: p['manifest']['path']))
    raw = canonical(manifest)
    if len(raw) > 4 * 1024**2:
        raise ValueError('Manifest exceeds bound')
    # Field order is the Rust Payload serialization contract, not sorted JSON keys.
    payload = dict(schema_version=1, app_version=version, target=target,
                   bundle_manifest_sha256=hashlib.sha256(raw).hexdigest(),
                   production_sha256=production, tested_source_commit=tested, review=review)
    return raw, canonical(payload)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('root', 'version', 'tested-source', 'production', 'review-path', 'review-sha256'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--review-bytes', type=int, required=True)
    args = parser.parse_args()
    root = Path(args.root).resolve()
    review = dict(path=args.review_path, bytes=args.review_bytes, sha256=args.review_sha256)
    outputs = [root / 'release/v1' / name for name in ('bundled-content-manifest.json', 'bundled-content-build-identity.json')]
    if any(path.exists() or path.is_symlink() for path in outputs):
        raise ValueError('Preparation never overwrites existing outputs')
    git = lambda *values: command(root, ['git', *values])
    head = git('rev-parse', 'HEAD').decode().strip()
    git('merge-base', '--is-ancestor', args.tested_source, head)
    tree = git('ls-tree', '-r', '-z', '--full-tree', head)
    tracked = {entry.split(b'\t', 1)[1].decode() for entry in tree.split(b'\0') if entry}
    if fingerprint(tree) != args.production or fingerprint(git('ls-tree', '-r', '-z', '--full-tree', args.tested_source)) != args.production:
        raise ValueError('Tested ancestor has different production identity')
    preview = decode(command(root, ['cargo', 'run', '--locked', '-p', 'azeroth-cli', '--', 'release-materials', '--root', str(root), '--version', args.version]))
    # Source evidence is itself bounded data; the Rust preview validates its contracts.
    evidence_path = root / 'release/v1/acceptance-evidence.json'
    with evidence_path.open('rb') as source:
        evidence_raw = source.read(MAX_JSON + 1)
        evidence = decode(evidence_raw)
    if preview['candidate_source_commit'] != head:
        raise ValueError('Preview source commit changed')
    reviewed = git('show', head + ':' + safe(review['path']))
    if len(reviewed) != review['bytes'] or hashlib.sha256(reviewed).hexdigest() != review['sha256'] or review['bytes'] > 16*1024**2:
        raise ValueError('Public review must match committed explicit reviewed bytes')
    raw, identity = prepare(root, preview, evidence, review, args.tested_source, args.production, args.version, hashlib.sha256(evidence_raw).hexdigest())
    manifest = decode(raw)
    if any(ref['path'] not in tracked for ref in manifest['files']):
        raise ValueError('Bundle resources must be tracked reviewed source')
    if git('status','--porcelain=v1','-z','--untracked-files=all','--','.',':(exclude)docs/**',':(exclude)release/**') or git('rev-parse', 'HEAD').decode().strip() != head:
        raise ValueError('Source changed during preparation')
    created = []
    try:
        for path, value in zip(outputs, (raw, identity)):
            with path.open('xb') as output:
                created.append(path)
                output.write(value)
    except BaseException:
        for path in created:
            path.unlink()
        raise
    print(json.dumps(dict(prepared=True, publication_allowed=False, manifest_sha256=hashlib.sha256(raw).hexdigest(), file_count=len(manifest['files']))))


if __name__ == '__main__':
    main()
