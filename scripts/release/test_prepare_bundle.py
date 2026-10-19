import copy
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess
import sys

spec = importlib.util.spec_from_file_location('prepare_bundle', Path(__file__).with_name('prepare-bundle.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class Preparation(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        def pin(path, raw):
            file = self.root / path
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(raw)
            return dict(path=path, bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
        self.pin = pin
        self.manifest = pin('content/pack/manifest.json', b'actual synthetic manifest bytes')
        self.review = pin('docs/review.json', b'explicit synthetic review, not actual acceptance')
        self.origins = pin('content/origins.json', b'origin policy')
        self.target = dict(edition='wow_forever',version='1.12.1',build='70374',interface=11200)
        key = dict(scope=dict(context=dict(build=dict(edition='wow_forever',build='1.12.1.70374',interface='11200'),namespace='reference'),fixture=False),revision='actual-revision')
        self.pack = dict(directory='content/pack',manifest=self.manifest,key=key,conflicts=None,normalization_conflicts=None)
        self.preview = dict(schema_version=2,version='1.0.0',production_sha256='a'*64,target=self.target,
                            packs=[dict(manifest=self.manifest,dataset_revisions=['actual-revision'])],
                            bundled_packs=[self.pack],files=[dict(input=self.manifest,kind='content_pack')])
        self.evidence = dict(schema_version=4,production_sha256='a'*64,tested_source_commit='b'*40,
                             target=self.target,origin_policy=self.origins,sources=[],coverage=[],checks=[dict(id=key,method=value,status='unverified',artifacts=[]) for key,value in m.REQUIRED_CHECKS.items()])

    def run_prepare(self):
        return m.prepare(self.root,self.preview,self.evidence,self.review,'b'*40,'a'*64,'1.0.0')

    def test_reproducible_exact_closure_and_rust_payload_order(self):
        first = self.run_prepare()
        self.assertEqual(first,self.run_prepare())
        manifest,payload = map(m.decode,first)
        self.assertEqual([ref['path'] for ref in manifest['files']],sorted([self.manifest['path'],self.review['path'],self.origins['path']]))
        self.assertEqual(payload['bundle_manifest_sha256'],hashlib.sha256(first[0]).hexdigest())
        self.assertEqual(list(payload['review']),['path','bytes','sha256'])
        self.assertEqual(list(payload),['schema_version','app_version','target','bundle_manifest_sha256','production_sha256','tested_source_commit','review'])
        self.assertNotIn(b'\n',first[1])

    def test_wrong_source_target_revision_fixture_and_missing_actual_keys(self):
        for mutate in [lambda: self.preview.update(schema_version=1),lambda: self.preview.update(production_sha256='c'*64),
                       lambda:self.evidence.update(tested_source_commit='c'*40),lambda:self.preview.update(bundled_packs=[]),
                       lambda:self.pack['key'].update(revision='invented'),lambda:self.pack['key']['scope'].update(fixture=True),
                       lambda:self.pack['key']['scope']['context']['build'].update(build='other'),lambda:self.pack.update(directory='other'),
                       lambda:self.preview['bundled_packs'].append(copy.deepcopy(self.pack))]:
            original=copy.deepcopy((self.preview,self.evidence))
            mutate()
            with self.assertRaises(ValueError):self.run_prepare()
            self.preview,self.evidence=original
            self.pack=self.preview['bundled_packs'][0]

    def test_private_registry_and_failed_or_partial_checks_refused(self):
        self.evidence['admission_registry']=self.manifest
        with self.assertRaisesRegex(ValueError,'registry'):self.run_prepare()
        del self.evidence['admission_registry']
        for check in [dict(status='fail'),dict(status='unverified',operator='partial')]:
            self.evidence['checks'][0].update(check)
            with self.assertRaises(ValueError):self.run_prepare()
        self.evidence['checks'][0]=dict(id='HOSTED-INSTALL',method='hosted_windows',status='unverified',artifacts=[],operator=None)
        self.run_prepare()

    def test_conflict_admission_and_tampered_missing_linked_resources(self):
        conflict=self.pin('content/conflicts.json',b'normalization report')
        self.pack['normalization_conflicts']=conflict
        with self.assertRaises(ValueError):self.run_prepare()
        self.evidence['normalization_conflict_files']=[conflict]
        self.preview['files'].append(dict(input=conflict))
        self.run_prepare()
        self.pack['normalization_conflicts']=None
        with self.assertRaisesRegex(ValueError,'attach'):self.run_prepare()
        self.pack['normalization_conflicts']=conflict
        self.pack['conflicts']=conflict
        with self.assertRaisesRegex(ValueError,'Typed'):self.run_prepare()
        self.pack['conflicts']=None
        file=self.root/self.manifest['path']
        file.write_bytes(b'changed')
        with self.assertRaises(ValueError):self.run_prepare()
        file.unlink()
        with self.assertRaises(OSError):self.run_prepare()
        file.symlink_to(self.root/self.origins['path'])
        with self.assertRaisesRegex(ValueError,'Linked'):self.run_prepare()

    def test_cli_preparation_uses_tracked_ancestor_and_never_overwrites(self):
        def git(*args):
            return subprocess.check_output(['git',*args],cwd=self.root)
        git('init','-q')
        git('add','.')
        git('-c','user.name=Synthetic Test','-c','user.email=synthetic@example.invalid','commit','-qm','synthetic tracked resources')
        head=git('rev-parse','HEAD').decode().strip()
        production=m.fingerprint(git('ls-tree','-r','-z','--full-tree',head))
        self.preview.update(candidate_source_commit=head,production_sha256=production)
        self.evidence.update(production_sha256=production,tested_source_commit=head)
        directory=self.root/'release/v1'
        directory.mkdir(parents=True)
        (directory/'acceptance-evidence.json').write_bytes(m.canonical(self.evidence))
        original=m.command
        def command(root,args,*rest,**kwargs):
            if args[0]=='cargo':return m.canonical(self.preview)
            return original(root,args,*rest,**kwargs)
        argv=['prepare-bundle.py','--root',str(self.root),'--version','1.0.0','--tested-source',head,
              '--production',production,'--review-path',self.review['path'],'--review-sha256',self.review['sha256'],
              '--review-bytes',str(self.review['bytes'])]
        with patch.object(m,'command',command),patch.object(sys,'argv',argv):
            m.main()
            first=(directory/'bundled-content-build-identity.json').read_bytes()
            with self.assertRaisesRegex(ValueError,'overwrites'):m.main()
            self.assertEqual(first,(directory/'bundled-content-build-identity.json').read_bytes())

    def test_only_pristine_windows_pending_can_remain_unverified(self):
        self.evidence['coverage']=[dict(status='unverified')]
        with self.assertRaisesRegex(ValueError,'coverage'):self.run_prepare()
        self.evidence['coverage']=[]
        self.evidence['checks'][0]['status']='pass'
        with self.assertRaisesRegex(ValueError,'Partial'):self.run_prepare()
        artifact=self.pin('docs/hosted-artifact.json',b'synthetic test, not actual Windows acceptance')
        self.evidence['candidate_installer_sha256']='c'*64
        self.evidence['checks'][0].update(installer_sha256='c'*64,artifacts=[artifact],operator='synthetic',environment='synthetic',recorded_at='2026-10-19T00:00:00Z')
        self.run_prepare()
        self.evidence['checks'][0]['installer_sha256']='d'*64
        with self.assertRaisesRegex(ValueError,'mismatched'):self.run_prepare()

    def test_strict_data_and_bounded_commands(self):
        for raw in [b'{"x":1,"x":2}',b'{"x":NaN}',b'\xff']:
            with self.assertRaises(ValueError):m.decode(raw)
        for path in ['../private','a//b','C:/private','a\\b']:
            with self.assertRaises(ValueError):m.safe(path)
        self.assertEqual(m.command(self.root,['python3','-c','print("ok",end="")']),b'ok')
        with self.assertRaises(ValueError):m.command(self.root,['python3','-c','print("oversized")'],maximum=1)
        with self.assertRaises(ValueError):m.command(self.root,['python3','-c','raise SystemExit(1)'])
        with self.assertRaises(ValueError):m.command(self.root,['python3','-c','import time;time.sleep(1)'],timeout=.01)


if __name__ == '__main__':
    unittest.main()
