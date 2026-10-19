#!/usr/bin/env python3
"""Retain original project-authored writing under the existing repository license.

No upstream admission, inferred ownership or target-build observation is created.
Run only after independently reviewing and committing the authored source files.
"""
import argparse
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'data-generators'))
import retain_world_sources as retain

FIELDS = sorted(['class_roles.models', 'dungeons.preparation_checklist', 'reminders.rules', 'party.procedures'])
SID = 'navzeroth-authored-v1'
ORIGINALS = ('catalog.json', 'evidence/authoring-statement.md', 'evidence/role-model-assumptions.md', 'party-support.json', 'procedures/duo-advancement.md', 'procedures/duo-agreement.md', 'procedures/duo-fairness.md', 'procedures/duo-joint-planning.md', 'procedures/duo-review.md', 'procedures/duo-sharing.md', 'procedures/duo-stale-members.md', 'procedures/duo-unequal-progress.md', 'procedures/solo-advancement.md', 'procedures/solo-joint-planning.md', 'procedures/solo-review.md', 'procedures/solo-stale-members.md')


def register(root, source_commit=None):
    folder = root / 'content/authored/slice-v1'
    registry_path = root / 'docs/sources/source-admission-v1.json'
    sources_path = folder / 'sources.json'
    records = retain.read_json(sources_path)
    if len(records['sources']) != 1:
        raise ValueError('authored writing must have exactly one source')
    source = records['sources'][0]
    if (source['id'] != SID or source['publisher'] != 'Navzeroth project (repository owner)'
            or source['license']['value'] != 'BSD-3-Clause (repository LICENSE)'
            or source['attribution']['value'] != 'Original Navzeroth authoring, (c) 2026 Igor K'
            or any(source[field]['status'] != 'known' for field in ('license','attribution','allowed_fields','transformations'))
            or sorted(source['allowed_fields']['value']) != FIELDS
            or source['transformations']['value'] != ['canonical_typed_serialization']):
        raise ValueError('authored source differs from reviewed original-writing scope')
    paths = sorted(p.relative_to(root).as_posix() for p in retain.bounded.input_files(folder, '*') if p.name != 'sources.json')
    expected = ['content/authored/slice-v1/'+p for p in ORIGINALS]
    if paths != expected:
        raise ValueError('authored originals differ from explicit reviewed file inventory')
    commit, originals = retain.pinned_code(root, ['LICENSE', *paths], source_commit)
    files = []
    for path in paths:
        ref = retain.retain(root, originals[path])
        files.append({**ref, 'url': f'https://raw.githubusercontent.com/Laymer/Navzeroth/{commit}/{path}', 'fields': FIELDS, 'transformation': 'canonical_typed_serialization'})
    # Exact retained original identities determine the source revision; source
    # metadata itself is excluded to avoid a self-referential hash.
    revision = hashlib.sha1(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    license_ref = retain.retain(root, originals['LICENSE'])
    proof = dict(url=f'https://raw.githubusercontent.com/Laymer/Navzeroth/{commit}/LICENSE', revision=commit, sha256=license_ref['sha256'], reviewed_scope='Existing project BSD license for exact original authored catalog, preparation/reminder/model prose and party procedures only. No game facts or third-party ownership grant; authored-only validator remains mandatory.')
    admitted = dict(id=SID, revision=revision, publisher=source['publisher'], origin=dict(edition='wow_forever', build=None),
        rights=dict(status='admitted', license='permissive', license_identifier=source['license']['value'], reviewer='Codex /root authorized original project writing admission; independent exact-scope review required', attribution=source['attribution']['value'], obligations='Retain repository BSD-3-Clause LICENSE and original author attribution; show heuristic assumptions and unresolved game facts.', evidence=[proof]),
        retrieval=dict(method='manual_local', max_total_bytes=sum(f['bytes'] for f in files)+license_ref['bytes'], minimum_interval_ms=1000, max_requests=len(files)+1, access_evidence=proof), files=files)
    registry = retain.read_json(registry_path)
    existing = retain.sources_by_id(registry['sources'])
    existing[SID] = admitted
    registry['sources'] = [s for s in registry['sources'] if s['id'] != SID] + [admitted]
    source['verified_revision'] = {'status': 'known', 'value': revision, 'clock_trust': 'uncertain', 'source': 'unrecorded'}
    registry_path.write_text(json.dumps(registry, indent=2, ensure_ascii=False)+'\n', encoding='utf-8')
    sources_path.write_text(json.dumps(records, indent=2, ensure_ascii=False)+'\n', encoding='utf-8')
    return dict(source_id=SID, revision=revision, source_commit=commit, original_files=len(files), original_bytes=sum(f['bytes'] for f in files), target_validation='unverified')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=pathlib.Path, default=ROOT)
    parser.add_argument('--source-commit')
    args = parser.parse_args()
    try:
        print(json.dumps(register(args.root.resolve(), args.source_commit), sort_keys=True))
    except (ValueError, OSError, KeyError, TypeError) as error:
        parser.exit(1, f'register-authored-release-source: {error}\n')
