#!/usr/bin/env python3
"""Enumerate verified shipped content without promoting environment observations."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'data-generators'))
from navzeroth_datagen import bounded

DOMAINS = ('quests','equipment','acquisition','geography','travel','dungeons','class_roles','party','reminders')
MAX_ENUMERATION = 128*1024*1024
MAX_MANIFEST = 64*1024*1024
PACKS = ('content/release-packs/world-v2','content/release-packs/authored-v2')


def encoded(value):
    return (json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':'))+'\n').encode()


def read(path, maximum=8*1024*1024):
    return bounded.json_document(bounded.read_file(path,maximum))


def reference(root, relative):
    path=pathlib.PurePosixPath(relative)
    if path.is_absolute() or '..' in path.parts or '.' in path.parts:
        raise ValueError('invalid release reference')
    raw=bounded.read_file(root/relative,64*1024*1024)
    if not raw: raise ValueError('empty release reference')
    return dict(path=relative,bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest())


def object_ref(root, sha):
    ref=reference(root,'content/source-materials/objects/'+sha)
    if ref['sha256']!=sha: raise ValueError('retained source object changed')
    return ref


def derive(root, enumeration, canonical_sha):
    records=enumeration['records']
    if set(records)-set(DOMAINS) or sum(map(len,records.values()))>2_000_000:
        raise ValueError('unexpected canonical record domain/capacity')
    if any(not sources for rows in records.values() for sources in rows.values()):
        raise ValueError('shipped record lacks provenance')
    if any(p['remaining_gaps'] or p['relabelled_evidence'] for p in enumeration['packs']):
        raise ValueError('pack/source classification is unresolved')
    registry=read(root/'docs/sources/source-admission-v1.json')
    admitted={s['id']:s for s in registry['sources']}
    pack_sources={}
    for pack in enumeration['packs']:
        for source in pack['sources']:
            sid=source['id']
            if sid in pack_sources and source!=pack_sources[sid]: raise ValueError('inconsistent source metadata')
            if sid not in admitted or source['verified_revision'].get('value')!=admitted[sid]['revision']:
                raise ValueError('pack source revision lacks exact reviewed admission')
            pack_sources[sid]=source
    closure=read(root/'content/world/1.60.1.70450/source-closure.json')
    conflicts=read(root/'content/world/1.60.1.70450/conflicts.json')
    summary=dict(schema_version=1,validation='verified v2 pack tree, canonical content, target, provenance and accepted-origin/unknown classification; no environment or functional acceptance claim',target=enumeration['target'],canonical_enumeration_sha256=canonical_sha,record_counts={d:len(records.get(d,{})) for d in DOMAINS},absent_record_domains=enumeration['absent_record_domains'],accepted_unknown_fields=sum(p['accepted_unknown_fields'] for p in enumeration['packs']),remaining_gaps=0,missing_record_provenance=0,packs=[{k:v for k,v in p.items() if k!='sources'} for p in enumeration['packs']],normalization_conflict_records=len(conflicts['conflicts']),unknown_fields_are_not_absent_records=True)
    summary['declared_exclusions']=read(root/'release/v1/deferred-features.json')
    artifact_path='release/v1/shipped-content-validation.json'
    artifact=encoded(summary)
    artifact_ref=dict(path=artifact_path,bytes=len(artifact),sha256=hashlib.sha256(artifact).hexdigest())
    domains=[];coverage=[]
    for domain in DOMAINS:
        rows=records.get(domain,{})
        ids=sorted(rows)
        sources=sorted({sid for row in rows.values() for sid in row})
        unit=dict(id=domain+'-shipped-records-v1',scope='Exact shipped '+domain+' identities and source bindings, including explicitly unavailable fields; not whole-game or runtime workflow completeness.',required_record_ids=ids)
        units=[unit]
        coverage.append(dict(unit_id=unit['id'],status='pass',included_record_ids=ids,source_ids=sources,unresolved_conflicts=0,unsupported_corrections=0,validation_evidence=[artifact_ref]))
        if domain in ('quests','class_roles','reminders','dungeons'):
            # Known concrete identities support future functional scenarios. A
            # genuinely absent record inventory remains None, never fabricated IDs.
            selected=ids if domain in ('class_roles','dungeons','reminders') else ids[:1]
            functional=dict(id=domain+'-functional-coverage-pending-v1',scope='Required '+domain+' workflow/coverage proof remains unverified; shipped identity presence and accepted unknowns do not establish usable full-scope behavior.',required_record_ids=selected if selected else None)
            units.append(functional)
            coverage.append(dict(unit_id=functional['id'],status='unverified',included_record_ids=selected,source_ids=sorted({sid for key in selected for sid in rows[key]}),unresolved_conflicts=0,unsupported_corrections=0,validation_evidence=[]))
        boundaries=[dict(id=domain+'-outside-admitted-records',scope='Records absent from the admitted source corpus and later additions are not covered by this exact enumeration.',reason='Historical source corpus scope does not establish completeness or eligibility for the whole Forever game. Promised missing functionality remains unverified separately.')]
        boundaries.append(dict(id=domain+'-deferred-gameplay',scope='Trainer-spell gameplay, talents, professions and enhancements.',reason='Explicit stable1.0.0 exclusions; no planner, scoring or acquisition gameplay support is promised for these systems.'))
        domains.append(dict(domain=domain,completeness='complete' if ids else 'unknown',enumeration_evidence=[artifact_ref],units=units,out_of_scope=boundaries))
    inventory=dict(schema_version=4,inventory_id='forever-1.0.0-admitted-world-records-v1',target=enumeration['target'],slice=None,domains=domains,sources=[dict(id=sid,revision=admitted[sid]['revision']) for sid in sorted(pack_sources)])
    inventory_raw=encoded(inventory)
    sources=[]
    notice=reference(root,'data-generators/NOTICE.md')
    for sid in sorted(pack_sources):
        src=admitted[sid]
        if src['rights']['status']!='admitted': raise ValueError('source rights not admitted')
        original=[]
        for f in src['files']:
            ref=reference(root,f['path'])
            if ref['sha256']!=f['sha256'] or ref['bytes']!=f['bytes']: raise ValueError('source original differs from admission')
            original.append(ref)
        proofs={p['sha256']:object_ref(root,p['sha256']) for p in src['rights']['evidence']+[src['retrieval']['access_evidence']]}
        license_proofs=[proofs[p['sha256']] for p in src['rights']['evidence'] if pathlib.PurePosixPath(p['url']).name.lower() in ('license','license.md')]
        # Wago's owner-accepted LicenseRef has no permissive grant. Preserve the
        # actual terms/risk notice, never manufacture a license text or grant.
        licenses=license_proofs or [notice]
        attributions=[notice]
        if sid=='cmangos-classic-db':
            attributions += [proofs[p['sha256']] for p in src['rights']['evidence'] if p['url'].endswith('/COPYRIGHT.md')]
        if sid=='navzeroth-authored-v1':
            attributions=[next(reference(root,f['path']) for f in src['files'] if f['url'].endswith('/evidence/authoring-statement.md'))]
        sources.append(dict(id=sid,revision=src['revision'],origin_edition=src['origin']['edition'],origin_build=src['origin']['build'],redistribution='permitted',files=original,license_files=licenses,attribution_files=attributions,permission_evidence=list(proofs.values()),target_validation='unverified',target_evidence=[]))
    checks=[dict(id=name,status='unverified',method='hosted_windows' if name!='OWNER-INSTALLER-SMOKE' else 'interactive_windows',operator=None,recorded_at=None,environment=None,artifacts=[]) for name in ('HOSTED-INSTALL','HOSTED-FIRST-START','HOSTED-UPGRADE-RC1','HOSTED-DOWNGRADE-REFUSAL','HOSTED-PERSISTENCE-RECOVERY','HOSTED-RESOURCES','HOSTED-OFFLINE','HOSTED-PERFORMANCE','OWNER-INSTALLER-SMOKE')]
    old=read(root/'release/history/2026-10-19-pre-stable-v3/acceptance-evidence.json')
    evidence=dict(schema_version=4,target=enumeration['target'],inventory_sha256=hashlib.sha256(inventory_raw).hexdigest(),admission_registry=reference(root,'docs/sources/source-admission-v1.json'),tested_source_commit=None,production_sha256=None,checks=checks,coverage=coverage,sources=sources,packs=[dict(manifest=p['manifest'],dataset_revisions=[p['dataset_revision']]) for p in enumeration['packs']],optional_deferrals=old['optional_deferrals'],matrix=[],origin_policy=enumeration['origin_policy'],corresponding_source_files=[{k:f[k] for k in ('path','bytes','sha256')} for f in closure['corresponding_source']['files']],normalization_conflict_files=[reference(root,'content/world/1.60.1.70450/conflicts.json')])
    evidence_raw=encoded(evidence)
    if max(len(inventory_raw),len(evidence_raw))>MAX_MANIFEST: raise ValueError('schema4 output exceeds measured finite bound')
    return {artifact_path:artifact,'release/v1/coverage-inventory.json':inventory_raw,'release/v1/acceptance-evidence.json':evidence_raw},summary


def generate(root,cli):
    # The canonical map is always regenerated through the verified Rust boundary;
    # caller-provided JSON cannot invent record identities or a passing check.
    with tempfile.TemporaryDirectory(prefix='.inventory-enumeration-',dir=root/'release/v1') as tmp:
        out=pathlib.Path(tmp)/'records.json'
        relative=out.relative_to(root).as_posix()
        command=[str(cli),'enumerate-release-packs','--root',str(root)]
        for pack in PACKS: command+=['--pack',pack]
        command+=['--output',relative]
        with open(pathlib.Path(tmp)/'stdout','wb') as stdout,open(pathlib.Path(tmp)/'stderr','wb') as stderr:
            subprocess.run(command,stdout=stdout,stderr=stderr,check=True)
        raw=bounded.read_file(out,MAX_ENUMERATION)
        enumeration=bounded.json_document(raw)
        outputs,summary=derive(root,enumeration,hashlib.sha256(raw).hexdigest())
        for path,data in outputs.items(): (root/path).write_bytes(data)
        return dict(output_bytes={path:len(raw) for path,raw in outputs.items()},records=summary['record_counts'],accepted_unknown_fields=summary['accepted_unknown_fields'])


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=pathlib.Path,default=ROOT)
    parser.add_argument('--cli',type=pathlib.Path,required=True)
    args=parser.parse_args()
    try: print(json.dumps(generate(args.root.resolve(),args.cli.resolve()),sort_keys=True))
    except (ValueError,OSError,KeyError,TypeError,subprocess.CalledProcessError) as error: parser.exit(1,f'generate-stable-inventory: {error}\n')
