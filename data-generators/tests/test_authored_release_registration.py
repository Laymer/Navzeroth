"""Synthetic local original-writing retention; no network or new upstream grant."""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import tempfile
import unittest

REPO=pathlib.Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('authored_registration',REPO/'scripts/register-authored-release-source.py')
registration=importlib.util.module_from_spec(spec);spec.loader.exec_module(registration)


class AuthoredRegistration(unittest.TestCase):
    def fixture(self, directory):
        root=pathlib.Path(directory)
        shutil.copytree(REPO/'content/authored/slice-v1',root/'content/authored/slice-v1')
        (root/'LICENSE').write_text('Synthetic BSD source fixture, not a real grant.\n')
        (root/'docs/sources').mkdir(parents=True)
        (root/'docs/sources/source-admission-v1.json').write_text(json.dumps(dict(schema_version=2,sources=[])))
        for args in [('init','-q'),('config','user.name','Synthetic test'),('config','user.email','test@example.invalid'),('add','.'),('commit','-qm','Synthetic original writing')]:
            subprocess.run(['git','-C',str(root),*args],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        return root

    def test_exact_scope_reproducible_retention_never_claims_target_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            root=self.fixture(directory)
            result=registration.register(root)
            self.assertEqual(result['target_validation'],'unverified')
            self.assertEqual(result['original_files'],16)
            registry=(root/'docs/sources/source-admission-v1.json').read_bytes()
            metadata=(root/'content/authored/slice-v1/sources.json').read_bytes()
            self.assertEqual(registration.register(root),result)
            self.assertEqual((root/'docs/sources/source-admission-v1.json').read_bytes(),registry)
            self.assertEqual((root/'content/authored/slice-v1/sources.json').read_bytes(),metadata)
            (root/'content/authored/slice-v1/catalog.json').write_bytes(b'changed')
            with self.assertRaises(ValueError): registration.register(root)
            self.assertEqual((root/'docs/sources/source-admission-v1.json').read_bytes(),registry)

    def test_unreviewed_files_or_fields_do_not_gain_admission(self):
        for kind in ('file','fields'):
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as directory:
                root=self.fixture(directory)
                if kind=='file': (root/'content/authored/slice-v1/new-source.txt').write_text('unreviewed')
                else:
                    path=root/'content/authored/slice-v1/sources.json';v=json.loads(path.read_bytes());v['sources'][0]['allowed_fields']['value'].append('quests.quests');path.write_text(json.dumps(v))
                with self.assertRaises(ValueError): registration.register(root)
