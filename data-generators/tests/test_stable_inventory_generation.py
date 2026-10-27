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
            self.assertEqual(inventory['schema_version'],5)
            self.assertEqual(evidence['schema_version'],5)
            expected={'HOSTED-'+name:'hosted_windows' for name in ('INSTALL','FIRST-START','UPGRADE-RC1','DOWNGRADE-REFUSAL','PERSISTENCE-RECOVERY','RESOURCES','OFFLINE','PERFORMANCE')}
            expected['DRAFT-INSTALLER-SMOKE']='automated_draft_windows'
            self.assertEqual({row['id']:row['method'] for row in evidence['checks']},expected)
            self.assertNotIn('OWNER-INSTALLER-SMOKE',expected)
            self.assertTrue(all(row['operator'] is None and row['recorded_at'] is None and row['environment'] is None and row['artifacts']==[] and 'installer_sha256' not in row for row in evidence['checks']))
            self.assertNotIn('draft_installer_smoke',evidence)
            self.assertNotIn('candidate_installer_sha256',evidence)
            self.assertIsNone(evidence['tested_source_commit'])
            self.assertIsNone(evidence['production_sha256'])
            self.assertEqual(json.loads((root/'release/history/2026-10-19-pre-stable-v3/acceptance-evidence.json').read_bytes()),{'optional_deferrals':[]})
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

    def proof_fixture(self,directory):
        import subprocess
        root,e=self.fixture(directory)
        source=root/generator.FUNCTIONAL_SOURCE;source.parent.mkdir(parents=True);source.write_text('// Synthetic binding fixture only; no actual functional execution claim.\n')
        subprocess.run(['git','-C',str(root),'add',generator.FUNCTIONAL_SOURCE],check=True,stdout=subprocess.DEVNULL)
        subprocess.run(['git','-C',str(root),'commit','-qm','Synthetic proof source'],check=True,stdout=subprocess.DEVNULL)
        commit=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD']).decode().strip()
        def key(collection,*ids):return json.dumps(['synthetic-r1',collection,*ids],separators=(',',':'))
        ids={
            'quests':[key('quests','q1'),key('quests','q2'),key('quest_stages','q1','s1'),key('quest_stages','q2','s2')],
            'class_roles':[key('models',str(i)) for i in range(54)],
            'reminders':[key('rules',str(i)) for i in range(18)],
            'dungeons':[key('dungeons',str(i)) for i in range(27)],
            'equipment':[key('items','i',{'status':'known','value':'base'})]}
        records={domain:{identity:['synthetic-source'] for identity in keys} for domain,keys in ids.items()}
        inputs=[]
        for name in ('data-generators/output/dungeons/dungeons/vanilla_1_12/1.12.1/part-0000.json','docs/data/role-coverage-v1.json','data-generators/output/quests/quests/vanilla_1_12/1.12.1/part-0000.json','docs/data/dungeon-associations-v1.json','scripts/validate-dungeon-associations.py'):
            path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text('{"synthetic":true}')
            inputs.append(generator.reference(root,name))
        subprocess.run(['git','-C',str(root),'add','docs/data/dungeon-associations-v1.json','scripts/validate-dungeon-associations.py'],check=True,stdout=subprocess.DEVNULL)
        subprocess.run(['git','-C',str(root),'commit','-qm','Synthetic oracle binding'],check=True,stdout=subprocess.DEVNULL)
        commit=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD']).decode().strip()
        proof=dict(schema_version=1,kind='manual_engineering_full_pack_functional_scenarios',status='pass',game_observation=False,platform='synthetic fixture only',disclosure='Synthetic contract test, no actual passing application workflow claimed.',test_source=dict(path=generator.FUNCTIONAL_SOURCE,commit=commit,sha256=hashlib.sha256(source.read_bytes()).hexdigest()),packs=[p['manifest'] for p in e['packs']],source_inputs=inputs,additional_source_records=[dict(domain='equipment',id=ids['equipment'][0])],units=[dict(domain=domain,scenario=scenario,scope='Synthetic bounded binding fixture.',source_record_ids=sorted(ids[domain])) for domain,scenario in generator.FUNCTIONAL_SCENARIOS.items()],counts=dict(models=54,classes=9,score_boundary_cases=108,reminder_boundary_cases=24,dungeon_packages=26,raw_dungeons=27,excluded_test_maps=[169],dungeon_quest_references=438,dungeon_reward_references=16390,restored_affected_reward_references=8107))
        (root/generator.FUNCTIONAL_PROOF).write_bytes(generator.encoded(proof))
        return root,e,records,proof

    def test_functional_proof_requires_exact_pack_committed_test_and_canonical_source_bindings(self):
        with tempfile.TemporaryDirectory() as directory:
            root,e,records,proof=self.proof_fixture(directory)
            result=generator.functional_proof(root,e,records)
            self.assertEqual(len(result['units']),4)
            self.assertEqual(result['reference'],generator.reference(root,generator.FUNCTIONAL_PROOF))
            for kind in ('pack','runtime_id','source','missing_unit','counts','input','game_observation','oracle'):
                changed=copy.deepcopy(proof)
                if kind=='pack':changed['packs'][0]['sha256']='0'*64
                elif kind=='runtime_id':changed['units'][0]['source_record_ids'][0]='quest-stage-runtime-only'
                elif kind=='source':changed['test_source']['sha256']='0'*64
                elif kind=='missing_unit':changed['units'].pop()
                elif kind=='counts':changed['counts']['classes']=10
                elif kind=='input':changed['source_inputs'][0]['bytes']+=1
                elif kind=='oracle':
                    (root/'scripts/validate-dungeon-associations.py').write_text('Altered synthetic oracle')
                    changed['source_inputs'][-1]=generator.reference(root,'scripts/validate-dungeon-associations.py')
                else:changed['game_observation']=True
                (root/generator.FUNCTIONAL_PROOF).write_bytes(generator.encoded(changed))
                with self.subTest(kind=kind),self.assertRaises(ValueError):generator.functional_proof(root,e,records)
    def test_bounded_associations_remain_explicitly_unresolved_without_losing_records(self):
        with tempfile.TemporaryDirectory() as directory:
            root,e=self.fixture(directory)
            source=next(iter(e['records']['reminders'].values()))
            e['records']['dungeons']={'["synthetic-r1","dungeons","dungeon:synthetic"]':source}
            path=root/'content/world/1.60.1.70450/conflicts.json'
            report=json.loads(path.read_bytes())
            row=dict(kind='bounded_association_unsupported',fact='dungeon:synthetic:rewards',
                     dungeon='dungeon:synthetic',field='rewards',actual_count=1025,
                     contract_limit=1024,exposed_count=0,unresolved_count=1025,rule='Synthetic v2 bound, not game evidence')
            report['conflicts']=[row];path.write_text(json.dumps(report))
            outputs,summary=generator.derive(root,e,'a'*64)
            self.assertEqual(summary['unresolved_association_references'],1025)
            self.assertEqual(summary['bounded_associations'][0]['exposed_count'],0)
            evidence=json.loads(outputs['release/v1/acceptance-evidence.json'])
            self.assertTrue(all(r['status']=='unverified' for r in evidence['coverage'] if 'functional' in r['unit_id']))
            for field,value in [('actual_count',1024),('actual_count',True),('exposed_count',1),
                                ('unresolved_count',1),('contract_limit',4096),('field','title'),('dungeon','absent')]:
                with self.subTest(field=field,value=value):
                    bad=copy.deepcopy(row);bad[field]=value;report['conflicts']=[bad];path.write_text(json.dumps(report))
                    with self.assertRaises(ValueError):generator.derive(root,e,'a'*64)
            report['conflicts']=[row,row];path.write_text(json.dumps(report))
            with self.assertRaises(ValueError):generator.derive(root,e,'a'*64)
