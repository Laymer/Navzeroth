#!/usr/bin/env python3
"""Retain exact public input bytes from an explicitly selected offline HTTP cache.

This registers existing owner-admitted world sources; it grants no new rights,
performs no network requests and never executes imported data.
"""
import argparse
import ast
import os
import re
import stat
import copy
import hashlib
import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from navzeroth_datagen import bounded
import json

MAX_RAW_BYTES = 64 * 1024 * 1024
UPSTREAMS = {'cmangos-classic-db', 'wago-tools-wowf'}
DERIVED = 'navzeroth-world-content'
MAX_RAW_TOTAL = 128 * 1024 * 1024  # selected public cache is approximately34MiB
MAX_SNAPSHOTS = 256
DERIVED_CODE = ('data-generators/world_to_content.py', 'data-generators/navzeroth_datagen/bounded.py')
CMANGOS_TABLES = ('UiMap', 'UiMapAssignment', 'FactionTemplate', 'AreaTable', 'Map', 'AreaTrigger')



def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def read_json(path, limit=8 * 1024 * 1024):
    return bounded.json_document(bounded.read_file(path, limit))


def file_ref(root, path):
    raw = bounded.read_file(path, MAX_RAW_BYTES)
    return {'path': path.relative_to(root).as_posix(), 'bytes': len(raw), 'sha256': digest(raw)}


def retain(root, raw):
    if not raw or len(raw) > MAX_RAW_BYTES:
        raise ValueError('retained object exceeds bounds')
    parent = root
    for part in ('content', 'source-materials', 'objects'):
        parent = parent / part
        parent.mkdir(exist_ok=True)
        info = parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode):
            raise ValueError('linked retained-object directory refused')
    path = parent / digest(raw)
    try:
        with path.open('xb') as out:
            out.write(raw)
    except FileExistsError:
        if bounded.read_file(path, MAX_RAW_BYTES) != raw:
            raise ValueError('existing immutable retained object differs')
    return file_ref(root, path)


def table_mapping(root):
    path = root / 'data-generators/Navzeroth-data-generators/gen_world_db2.py'
    tree = ast.parse(bounded.read_file(path, bounded.MAX_SHARD_BYTES))
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'TABLES' for t in node.targets):
            value = ast.literal_eval(node.value)
            if not isinstance(value, dict) or any(not isinstance(k, str) or not isinstance(v, tuple)
                    or any(not isinstance(t, str) or not re.fullmatch(r'[A-Za-z0-9_]+', t) for t in v)
                    for k, v in value.items()):
                break
            return value
    raise ValueError('generator TABLES mapping is absent or invalid')


def snapshots(world, tables):
    result = {sid: {'refs': {}, 'origins': set()} for sid in UPSTREAMS}
    budget = bounded.InputBudget()
    for path in bounded.input_files(world, 'part-*.json'):
        if 'facts' in path.relative_to(world).parts:
            continue
        doc = bounded.json_document(budget.read(path, bounded.MAX_SHARD_BYTES))
        if (not isinstance(doc, dict) or not isinstance(doc.get('records'), list)
                or not isinstance(doc.get('sources'), list) or len(doc['sources']) > 128
                or any(not isinstance(source, dict) for source in doc['sources'])):
            raise ValueError('invalid shard source declaration')
        budget.add_records(len(doc['records']))
        for source in doc.get('sources', []):
            sid = source.get('id')
            if sid not in result:
                continue
            expected_url = ('https://wago.tools/db2?product=wowf' if sid == 'wago-tools-wowf'
                            else 'https://github.com/cmangos/classic-db')
            if source.get('url') != expected_url:
                raise ValueError('snapshot source URL differs from its declared upstream')
            sha = source.get('snapshot_sha256')
            if not isinstance(sha, str) or len(sha) != 64 or any(c not in '0123456789abcdef' for c in sha):
                raise ValueError('world source lacks an exact snapshot hash')
            generator = doc['generator']['name']
            expected = None
            if sid == 'wago-tools-wowf':
                if generator == 'gen_world_db2':
                    expected = tables[doc['dataset']]
                elif generator == 'gen_world_cmangos':
                    expected = CMANGOS_TABLES
                elif generator == 'gen_fishing':
                    expected = ('AreaTable',)
                else:
                    raise ValueError('unknown Wago aggregate producer')
            reference = {'source': source, 'tables': tuple(sorted(expected)) if expected else None}
            old = result[sid]['refs'].setdefault(sha, reference)
            if old != reference:
                raise ValueError('snapshot hash has inconsistent declarations')
            if sum(len(info['refs']) for info in result.values()) > MAX_SNAPSHOTS:
                raise ValueError('snapshot count exceeds limit')
            # Auxiliary references need their own build, never the consuming shard's.
            if sid == 'wago-tools-wowf':
                build = source.get('revision')
                if not isinstance(build, str) or not re.fullmatch(r'[0-9]{1,9}(?:\.[0-9]{1,9}){3}', build):
                    raise ValueError('Wago source build is not exact')
                origin = ('wow_forever', build)
            else:
                build = doc['origin']['build']
                origin = (doc['origin']['edition'], build.get('value') if build.get('status') == 'known' else None)
            result[sid]['origins'].add(origin)
    for source in result.values():
        if not source['refs'] or len(source['origins']) != 1:
            raise ValueError('source snapshots are absent or have multiple origins; split them explicitly')
    return result


def allowed_raw_url(sid, url, build):
    if not isinstance(url, str):
        return False
    if sid == 'cmangos-classic-db':
        return url.startswith('https://raw.githubusercontent.com/cmangos/classic-db/') and '?' not in url and '#' not in url
    prefix = 'https://wago.tools/db2/'
    suffix = '/csv?product=wowf&build=' + build
    if not url.startswith(prefix) or not url.endswith(suffix):
        return False
    table = url[len(prefix):-len(suffix)]
    return bool(table) and table.isascii() and all(c.isalnum() or c == '_' for c in table)


def cache_inputs(cache, selected):
    # Index only bounded metadata first; body bytes are admitted only if selected.
    metadata = {}
    for path in bounded.input_files(cache, '*.meta.json'):
        meta = read_json(path, 64 * 1024)
        if not isinstance(meta, dict):
            raise ValueError('cache metadata is not an object')
        url = meta.get('url')
        if not isinstance(url, str):
            raise ValueError('cache metadata lacks URL')
        if url in metadata and metadata[url][0] != meta:
            raise ValueError('cache URL has conflicting metadata')
        metadata[url] = (meta, path)
    remaining = MAX_RAW_TOTAL
    bodies = {}
    def original(url, sid, build):
        nonlocal remaining
        if not allowed_raw_url(sid, url, build) or url not in metadata:
            raise ValueError('selected original URL is missing or incompatible')
        meta, path = metadata[url]
        if type(meta.get('status')) is not int or meta['status'] != 200:
            raise ValueError('selected original has no successful raw body; snapshot unresolved')
        if url not in bodies:
            raw = bounded.read_file(path.with_name(path.name[:-len('.meta.json')]), min(MAX_RAW_BYTES, remaining))
            if type(meta.get('bytes')) is not int or meta['bytes'] != len(raw) or meta.get('sha256') != digest(raw):
                raise ValueError('selected raw cache body differs from exact snapshot')
            remaining -= len(raw)
            bodies[url] = raw
        return meta, bodies[url]
    result = {}
    for sid in sorted(selected):
        info = selected[sid]
        build = next(iter(info['origins']))[1]
        inputs, aggregate_refs = {}, []
        for sha, reference in sorted(info['refs'].items()):
            if sid == 'wago-tools-wowf':
                urls = [f'https://wago.tools/db2/{table}/csv?product=wowf&build={build}' for table in reference['tables']]
                pairs = []
                for url in urls:
                    meta, raw = original(url, sid, build)
                    pairs.append((url, meta['sha256']))
                    inputs[url] = raw
                # Match gen_world_db2.source_for and Wago.source exactly: default JSON separators.
                if digest(json.dumps(sorted(pairs)).encode()) != sha:
                    raise ValueError('Wago constituent inputs do not reproduce aggregate snapshot')
                aggregate_refs.append({'snapshot_sha256': sha, 'inputs': [{'url': u, 'sha256': h} for u, h in sorted(pairs)]})
            else:
                matches = [url for url,(meta,_) in metadata.items() if meta.get('sha256') == sha and allowed_raw_url(sid,url,build)]
                if len(matches) != 1:
                    raise ValueError('raw snapshot URL is absent or ambiguous')
                url = matches[0]
                meta, raw = original(url, sid, build)
                if digest(raw) != sha:
                    raise ValueError('original raw snapshot differs')
                revision = reference['source'].get('revision', '')
                match = re.fullmatch(r'([^/]+\.sql\.gz) \(git blob ([0-9a-f]{40})\)', revision)
                blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
                if not match or not url.endswith('/' + match[1]) or match[2] != blob:
                    raise ValueError('CMaNGOS filename/blob identity differs')
                inputs[url] = raw
                aggregate_refs.append({'snapshot_sha256': sha, 'inputs': [{'url': url, 'sha256': sha}]})
        result[sid] = {'inputs': inputs, 'snapshots': aggregate_refs}
    return result


def corresponding_paths(root):
    paths = []
    for path in bounded.input_files(root / 'data-generators', '*.py'):
        relative = path.relative_to(root).as_posix()
        if path.name == 'retain_world_sources.py' or any(part in ('.cache', 'output', 'tests', '__pycache__') for part in path.relative_to(root / 'data-generators').parts):
            continue
        paths.append(relative)
    return sorted(set(paths) | {'LICENSE', 'data-generators/NOTICE.md', 'data-generators/TALOD-data-generators/PERMISSION.md'})


def pinned_code(root, paths, commit=None):
    if commit is None:
        commit = subprocess.check_output(['git', '-C', str(root), 'log', '-1', '--format=%H', '--', *paths], text=True).strip()
    if not re.fullmatch(r'[0-9a-f]{40}', commit or ''):
        raise ValueError('source commit must be a full immutable commit')
    if subprocess.check_output(['git', '-C', str(root), 'cat-file', '-t', commit], stderr=subprocess.PIPE).strip() != b'commit':
        raise ValueError('source pin must identify a commit object')
    raw_by_path = {}
    total = 0
    for path in paths:
        raw = bounded.read_file(root / path, bounded.MAX_SHARD_BYTES)
        total += len(raw)
        if total > MAX_RAW_TOTAL:
            raise ValueError('corresponding source exceeds aggregate bound')
        # A selected code file is at most8MiB; git show output is equally bounded
        # by comparing against the repository blob size before asking for its bytes.
        size = subprocess.check_output(['git','-C',str(root),'cat-file','-s',f'{commit}:{path}'],text=True,stderr=subprocess.PIPE).strip()
        if not size.isdecimal() or int(size) != len(raw):
            raise ValueError('working source differs from pinned commit')
        pinned = subprocess.check_output(['git','-C',str(root),'show',f'{commit}:{path}'])
        if pinned != raw:
            raise ValueError('working source differs from pinned commit')
        raw_by_path[path] = raw
    return commit, raw_by_path


def source_file(reference, url, fields, transformation):
    return {**reference, 'url': url, 'fields': fields, 'transformation': transformation}


def transformation_version(raw):
    for node in ast.parse(raw).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'VERSION' for t in node.targets):
            version = ast.literal_eval(node.value)
            if isinstance(version, str) and re.fullmatch(r'[1-9][0-9]{0,8}', version):
                return 'world_to_content_v' + version
    raise ValueError('mapper VERSION is absent or unsupported')


def sources_by_id(sources):
    result = {}
    for source in sources:
        sid = source['id']
        if sid in result:
            raise ValueError('duplicate source identity')
        result[sid] = source
    return result


def prepare(root, world, cache, pack_sources, registry_path, output, commit=None):
    commit, code_bytes = pinned_code(root, corresponding_paths(root), commit)
    transformation = transformation_version(code_bytes[DERIVED_CODE[0]])
    if not output.parent.is_dir():
        raise ValueError('closure output directory does not exist')
    selected = snapshots(world, table_mapping(root))
    raw_inputs = cache_inputs(cache, selected)
    registry = read_json(registry_path)
    if registry.get('schema_version') not in (1, 2):
        raise ValueError('unsupported admission registry')
    registry = copy.deepcopy(registry)
    admitted = sources_by_id(registry['sources'])
    pack = read_json(pack_sources)
    records = sources_by_id(pack['sources'])
    if set(records) != UPSTREAMS | {DERIVED}:
        raise ValueError('world source set differs; new sources require independent admission')
    closure = []
    for sid in sorted(UPSTREAMS):
        if sid not in admitted or admitted[sid]['rights']['status'] != 'admitted':
            raise ValueError('upstream source is not admitted')
        source = admitted[sid]
        fields = records[sid]['allowed_fields']['value']
        original_files = [source_file(retain(root, raw), url, fields, transformation)
                          for url, raw in sorted(raw_inputs[sid]['inputs'].items())]
        # Verify every existing rights/access grant before using it. No permission
        # text or status is manufactured from counts or new generated records.
        proofs = {}
        for proof in source['rights']['evidence'] + [source['retrieval']['access_evidence']]:
            path = root / 'content/source-materials/objects' / proof['sha256']
            reference = file_ref(root, path)
            if reference['sha256'] != proof['sha256']:
                raise ValueError('retained permission proof differs from admission')
            proofs[reference['path']] = reference
        origin_edition, origin_build = next(iter(selected[sid]['origins']))
        raw_by_hash = {f['sha256']: f['bytes'] for f in original_files}
        raw_by_hash.update({p['sha256']: p['bytes'] for p in proofs.values()})
        if len(original_files) > 1024 or len(raw_by_hash) > source['retrieval']['max_requests']:
            raise ValueError('original material closure exceeds admitted request count')
        source['origin'] = {'edition': origin_edition, 'build': origin_build}
        source['files'] = original_files
        identity = json.dumps({'files': original_files, 'origin': source['origin']}, sort_keys=True, separators=(',', ':')).encode()
        source['revision'] = hashlib.sha1(b'world-source-v1\0' + identity).hexdigest()
        source['retrieval']['max_total_bytes'] = max(source['retrieval']['max_total_bytes'], sum(raw_by_hash.values()))
        closure.append({'id': sid, 'revision': source['revision'], 'origin': source['origin'],
                        'files': [{k: f[k] for k in ('path', 'bytes', 'sha256')} for f in original_files],
                        'permission_evidence': list(proofs.values()), 'snapshots': raw_inputs[sid]['snapshots']})
    # Only original mapper/helper code is admitted under the repository BSD terms.
    license_ref = retain(root, code_bytes['LICENSE'])
    fields = records[DERIVED]['allowed_fields']['value']
    code = []
    for path in DERIVED_CODE:
        reference = retain(root, code_bytes[path])
        url = f'https://raw.githubusercontent.com/Laymer/Navzeroth/{commit}/{path}'
        code.append(source_file(reference, url, fields, transformation))
    corresponding = []
    for path, raw in sorted(code_bytes.items()):
        corresponding.append({**retain(root, raw), 'source_path': path,
                              'url': f'https://raw.githubusercontent.com/Laymer/Navzeroth/{commit}/{path}'})
    code.sort(key=lambda f: f['url'])
    # Identical helpers may have identical bytes; one admitted path suffices.
    code = list({f['path']: f for f in code}.values())
    revision = hashlib.sha1(json.dumps(code, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    proof = {'url': f'https://raw.githubusercontent.com/Laymer/Navzeroth/{commit}/LICENSE', 'revision': commit,
             'sha256': license_ref['sha256'], 'reviewed_scope': 'Original project-authored transformation under repository BSD-3-Clause; retain exact source and license. Does not grant upstream rights.'}
    derived = {'id': DERIVED, 'revision': revision, 'publisher': records[DERIVED]['publisher'],
               'origin': {'edition': 'wow_forever', 'build': None},
               'rights': {'status': 'admitted', 'license': 'permissive', 'license_identifier': 'BSD-3-Clause',
                          'reviewer': 'Codex /root; original project-authored transformation, independent final review required',
                          'attribution': records[DERIVED]['attribution']['value'],
                          'obligations': 'Retain copyright, BSD license and corresponding generator source; upstream data rights remain separate.',
                          'evidence': [proof]},
               'retrieval': {'method': 'manual_local', 'max_total_bytes': sum(f['bytes'] for f in code) + license_ref['bytes'],
                             'minimum_interval_ms': 1000, 'max_requests': len(code) + 1, 'access_evidence': proof},
               'files': code}
    admitted[DERIVED] = derived
    closure.append({'id': DERIVED, 'revision': revision, 'origin': derived['origin'],
                    'files': [{k: f[k] for k in ('path', 'bytes', 'sha256')} for f in code],
                    'permission_evidence': [license_ref]})
    registry['sources'] = [admitted[s['id']] for s in registry['sources'] if s['id'] != DERIVED] + [derived]
    registry['schema_version'] = 2
    registry_path.write_text(json.dumps(registry, indent=2, ensure_ascii=False) + '\n')
    output.write_text(json.dumps({'schema_version': 2, 'sources': closure, 'transformation': transformation,
                                  'source_commit': commit, 'corresponding_source': {
                                      'classification': 'mixed_project_and_TALOD_permission; not relicensed as BSD',
                                      'scope': 'Includes TALOD-authorized scripts and derived parser; retain original permission and NOTICE. Third-party data rights remain separate.',
                                      'files': corresponding,
                                      'permission_path': 'data-generators/TALOD-data-generators/PERMISSION.md',
                                      'notice_path': 'data-generators/NOTICE.md'}}, indent=2) + '\n')
    return closure


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, default=pathlib.Path(__file__).resolve().parent.parent)
    parser.add_argument('--world', type=pathlib.Path, required=True)
    parser.add_argument('--cache', type=pathlib.Path, required=True)
    parser.add_argument('--pack-sources', type=pathlib.Path, required=True)
    parser.add_argument('--registry', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--source-commit', help='Full reviewed code commit; default latest commit touching selected code/terms')
    args = parser.parse_args()
    try:
        closure = prepare(args.root.resolve(), args.world, args.cache, args.pack_sources, args.registry, args.output, args.source_commit)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'retain_world_sources: {error}\n')
    print(json.dumps({'sources': len(closure), 'files': sum(len(s['files']) for s in closure)}))


if __name__ == '__main__':
    main()
