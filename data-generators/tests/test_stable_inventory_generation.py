"""Synthetic metadata cases only; the production entrypoint reruns Rust verification."""
import copy
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
import test_authored_release_registration as authored_fixture
registration=authored_fixture.registration
REPO=authored_fixture.REPO

spec=importlib.util.spec_from_file_location('stable_inventory',REPO/'scripts/generate-stable-inventory.py')
generator=importlib.util.module_from_spec(spec);spec.loader.exec_module(generator)


class StableInventoryGeneration(unittest.TestCase):
    def fixture(self,directory):
        root=authored_fixture.AuthoredRegistration().fixture(directory);registration.register(root)
        (root/'release/v1').mkdir(parents=True)
        old=root/'release/history/2026-10-19-pre-stable-v3';old.mkdir(parents=True)
        (old/'acceptance-evidence.json').write_text(json.dumps(dict(optional_deferrals=[])))
        (root/'release/v1/deferred-features.json').write_text(json.dumps(dict(schema_version=1,features=[])))
        policy=dict(precedence=['authored']);(root/'release/v1/accepted-origins.json').write_text(json.dumps(policy))
        world=root/'content/world/1.60.1.70450';world.mkdir(parents=True)
        (world/'source-closure.json').write_text(json.dumps(dict(source_commit='a'*40,transformation='synthetic_v1',normalization_policy_sha256='b'*64,corresponding_source=dict(files=[]))))
        (world/'conflicts.json').write_text(json.dumps(dict(format='navzeroth-world-content-conflicts',schema_version=1,revision='synthetic-r1',precedence=['authored'],conflicts=[])))
        (root/'data-generators').mkdir();(root/'data-generators/NOTICE.md').write_text('Synthetic public fixture notice, not a source grant.')
        registration.retain.retain(root,(root/'data-generators/NOTICE.md').read_bytes())
        source=json.loads((root/'content/authored/slice-v1/sources.json').read_bytes())['sources'][0]
        enumeration=dict(schema_version=1,target=dict(edition='wow_forever',version='1.60.1',build='70450',interface=16001),origin_policy=generator.reference(root,'release/v1/accepted-origins.json'),absent_record_domains=[d for d in generator.DOMAINS if d!='reminders'],records={'reminders':{'["synthetic-r1","rules","r1"]':[source['id']]}},packs=[dict(manifest=generator.object_ref(root,hashlib.sha256((root/'data-generators/NOTICE.md').read_bytes()).hexdigest()),dataset_revision='synthetic-r1',catalog_sha256='a'*64,sources=[source],accepted_origins=0,accepted_unknown_fields=1,remaining_gaps=0,relabelled_evidence=0)])
        raw=b'synthetic corresponding code';sha=hashlib.sha256(raw).hexdigest();registration.retain.retain(root,raw)
        closure_path=world/'source-closure.json';closure=json.loads(closure_path.read_bytes());closure['corresponding_source']['files']=[dict(generator.object_ref(root,sha),source_path='code.py',url='https://example.invalid/'+'a'*40+'/code.py')];closure['corresponding_source']['permission_path']='private/not-shipped';closure_path.write_text(json.dumps(closure))
        return root,enumeration

    def test_deterministic_exact_identity_preserves_absence_and_pristine_acceptance(self):
        with tempfile.TemporaryDirectory() as directory:
            root,e=self.fixture(directory)
            first,summary=generator.derive(root,e,'a'*64);second,_=generator.derive(root,e,'a'*64)
            self.assertEqual(first,second)
            inventory=json.loads(first['release/v1/coverage-inventory.json']);evidence=json.loads(first['release/v1/acceptance-evidence.json'])
            self.assertIsNone(inventory['slice'])
            self.assertEqual(inventory['domains'][-1]['units'][0]['required_record_ids'],list(e['records']['reminders']))
            self.assertEqual(inventory['domains'][0]['units'][0]['required_record_ids'],[])
            self.assertIsNone(inventory['domains'][0]['units'][1]['required_record_ids'])
            self.assertEqual(len(evidence['checks']),9);self.assertTrue(all(c['status']=='unverified' for c in evidence['checks']))
            self.assertTrue(all(s['target_validation']=='unverified' for s in evidence['sources']))
            self.assertTrue(all(c['status']=='unverified' for c in evidence['coverage'] if 'functional' in c['unit_id']))
            self.assertEqual(evidence['inventory_sha256'],hashlib.sha256(first['release/v1/coverage-inventory.json']).hexdigest())
            self.assertEqual(summary['accepted_unknown_fields'],1)
            index=json.loads(first['content/source-materials/corresponding-source-index.json']);self.assertEqual(index['files'][0]['source_path'],'code.py');self.assertNotIn('permission_path',index)
            self.assertIn('content/source-materials/corresponding-source-index.json',[f['path'] for f in evidence['corresponding_source_files']])

    def test_missing_provenance_or_unadmitted_scope_cannot_become_a_pass(self):
        for kind in ('provenance','gap','revision','fields','normalization','mapping'):
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as directory:
                root,e=self.fixture(directory);e=copy.deepcopy(e)
                if kind=='provenance':e['records']['reminders']['["synthetic-r1","rules","r1"]']=[]
                elif kind=='gap':e['packs'][0]['remaining_gaps']=1
                elif kind=='revision':e['packs'][0]['sources'][0]['verified_revision']['value']='0'*40
                elif kind=='fields':e['packs'][0]['sources'][0]['allowed_fields']['value'].append('quests.quests')
                elif kind=='mapping':
                    p=root/'content/world/1.60.1.70450/source-closure.json';v=json.loads(p.read_bytes());v['corresponding_source']['files'][0]['url']='https://example.invalid/unpinned/code.py';p.write_text(json.dumps(v))
                else:
                    p=root/'content/world/1.60.1.70450/conflicts.json';v=json.loads(p.read_bytes());v['precedence']=[];p.write_text(json.dumps(v))
                with self.assertRaises(ValueError):generator.derive(root,e,'a'*64)
