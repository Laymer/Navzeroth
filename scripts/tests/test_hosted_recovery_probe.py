"""Synthetic fixture/process tests only; never installs or downloads anything."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import stat
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[2]
loader = importlib.util.spec_from_file_location('recovery_probe', ROOT/'scripts/windows/hosted-recovery-probe.py')
probe = importlib.util.module_from_spec(loader)
loader.loader.exec_module(probe)


class RecoveryProbe(unittest.TestCase):
    def fixture(self, root, source=None, extra=()):
        archive = root/'fixture.zip'
        payload = b'synthetic installer bytes, never execute'
        source = source if source is not None else 'b'*40
        with zipfile.ZipFile(archive, 'w') as output:
            output.writestr('setup.exe', payload)
            output.writestr('SOURCE_COMMIT.txt', source+'\n')
            for name, data in extra:
                output.writestr(name, data)
        spec = dict(installer_name='setup.exe', installer_source_commit='b'*40,
                    installer_sha256=hashlib.sha256(payload).hexdigest(), installer_bytes=len(payload),
                    artifact_bytes=archive.stat().st_size, artifact_sha256=hashlib.sha256(archive.read_bytes()).hexdigest())
        return archive, spec

    def test_exact_zip_and_source_are_verified_before_extracting_one_installer(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            archive, spec = self.fixture(root)
            output = root/'out';output.mkdir()
            installer = probe.extract_installer(archive, output, spec)
            self.assertEqual(installer.name, 'setup.exe')
            self.assertEqual(probe.digest(installer, 1024), spec['installer_sha256'])
            self.assertEqual([p.name for p in output.iterdir()], ['setup.exe'])
            with self.assertRaises(FileExistsError):
                probe.extract_installer(archive, output, spec)

    def test_wrong_archive_installer_source_and_unsafe_members_refuse(self):
        for mutation in ('zip_hash','zip_size','installer_hash','source','traversal','absolute','backslash','duplicate','link'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                extra=[]
                if mutation in ('traversal','absolute','backslash','duplicate'):
                    extra=[({'traversal':'../outside','absolute':'/outside','backslash':'a\\outside','duplicate':'setup.exe'}[mutation], b'bad')]
                if mutation == 'backslash':
                    info=zipfile.ZipInfo('placeholder');info.filename='a\\outside';info.orig_filename=info.filename;extra=[(info,b'bad')]
                if mutation == 'link':
                    info=zipfile.ZipInfo('link');info.create_system=3;info.external_attr=(stat.S_IFLNK|0o777)<<16;extra=[(info,b'target')]
                archive,spec=self.fixture(root, 'c'*40 if mutation=='source' else None, extra)
                if mutation=='zip_hash':spec['artifact_sha256']='0'*64
                if mutation=='zip_size':spec['artifact_bytes']+=1
                if mutation=='installer_hash':spec['installer_sha256']='0'*64
                output=root/'out';output.mkdir()
                with self.assertRaises(ValueError):probe.extract_installer(archive,output,spec)
                self.assertFalse((root/'outside').exists())

    def test_size_limits_apply_before_installer_decompression(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);archive,spec=self.fixture(root);output=root/'out';output.mkdir()
            with patch.object(probe,'MAX_INSTALLER',2),self.assertRaises(ValueError):probe.extract_installer(archive,output,spec)
            self.assertFalse(list(output.iterdir()))
            with patch.object(probe,'MAX_ZIP',2),self.assertRaises(ValueError):probe.extract_installer(archive,output,spec)

    def test_actual_child_output_and_timeout_are_bounded_and_retained(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            output=probe.run([sys.executable,'-c','print("synthetic")'],root,'success',5)
            self.assertEqual(output.read_text().strip(),'synthetic')
            self.assertEqual(json.loads((root/'success.json').read_text())['exit_code'],0)
            start=time.monotonic()
            with self.assertRaises(ValueError):probe.run([sys.executable,'-c','import time; print("started",flush=True); time.sleep(30)'],root,'timeout',0.2)
            self.assertLess(time.monotonic()-start,10)
            self.assertTrue(json.loads((root/'timeout.json').read_text())['timed_out'])
            self.assertTrue((root/'timeout.stdout').exists())
            with self.assertRaises(ValueError):probe.run([sys.executable,'-c','print("x"*10000)'],root,'overrun',5,maximum=16)
            self.assertLessEqual((root/'overrun.stdout').stat().st_size,16)
            self.assertTrue(json.loads((root/'overrun.json').read_text())['capture_errors'])
            with self.assertRaises(ValueError):probe.run([sys.executable,'-c','raise SystemExit(7)'],root,'failure',5)
            self.assertEqual(json.loads((root/'failure.json').read_text())['exit_code'],7)

    def test_root_exit_does_not_abandon_owned_child_or_pipe_deadline(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);pidfile=root/'owned-child.pid'
            child=f"import os,time,pathlib; pathlib.Path({str(pidfile)!r}).write_text(str(os.getpid())); time.sleep(30)"
            parent=f"import subprocess,sys; subprocess.Popen([sys.executable,'-c',{child!r}]); print('root exits',flush=True)"
            start=time.monotonic()
            try:
                with self.assertRaises(ValueError):probe.run([sys.executable,'-c',parent],root,'orphan',5)
                self.assertLess(time.monotonic()-start,9)
                result=json.loads((root/'orphan.json').read_text())
                self.assertTrue(result['timed_out']);self.assertEqual(result['exit_code'],0)
                self.assertTrue(pidfile.exists(),'synthetic child must actually start')
                pid=int(pidfile.read_text())
                if probe.os.name=='nt':
                    import ctypes as c
                    from ctypes import wintypes as w
                    api=c.WinDLL('kernel32',use_last_error=True)
                    api.OpenProcess.argtypes=[w.DWORD,w.BOOL,w.DWORD];api.OpenProcess.restype=w.HANDLE
                    api.GetExitCodeProcess.argtypes=[w.HANDLE,c.POINTER(w.DWORD)];api.GetExitCodeProcess.restype=w.BOOL
                    api.CloseHandle.argtypes=[w.HANDLE]
                    handle=api.OpenProcess(0x1000,False,pid)
                    if handle:
                        try:
                            code=w.DWORD();self.assertTrue(api.GetExitCodeProcess(handle,c.byref(code)));self.assertNotEqual(code.value,259)
                        finally:api.CloseHandle(handle)
                else:
                    status=Path(f'/proc/{pid}/stat')
                    self.assertTrue(not status.exists() or status.read_text().split(') ',1)[1].startswith('Z '),'owned child survived cleanup')
            finally:
                if pidfile.exists() and probe.os.name!='nt':
                    try:probe.os.kill(int(pidfile.read_text()),9)
                    except ProcessLookupError:pass

    def test_ownership_marker_matches_only_this_guarded_fixture_attempt(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);marker=root/'owner.json';nonce='1'*32;source='a'*40;sha='b'*64;install=root/'install'
            def owned():return probe.owns_fixture(marker,nonce,source,sha,install)
            self.assertFalse(owned())
            record=dict(schema_version=1,ownership_nonce=nonce,source_commit=source,installer_sha256=sha,install_directory=str(install))
            marker.write_text(json.dumps(record));self.assertTrue(owned())
            for key in record:
                bad=dict(record);bad[key]='foreign';marker.write_text(json.dumps(bad));self.assertFalse(owned())
            marker.write_text('x'*4097);self.assertFalse(owned())
            with patch.object(probe,'regular',return_value=1):self.assertFalse(owned())
            marker.write_text('{}');self.assertFalse(owned())

    def test_artifact_metadata_and_strict_json_do_not_relabel_the_pr_merge(self):
        spec=json.loads((ROOT/probe.SPEC).read_text())
        value=dict(id=spec['artifact_id'],name=spec['artifact_name'],size_in_bytes=spec['artifact_bytes'],expired=False,digest='sha256:'+spec['artifact_sha256'],workflow_run=dict(id=spec['artifact_run_id'],head_sha=spec['api_head_commit']))
        probe.artifact_metadata(value,spec)
        self.assertNotEqual(spec['api_head_commit'],spec['installer_source_commit'])
        for key in ('id','name','size_in_bytes','expired','digest'):
            changed=copy.deepcopy(value);changed[key]='wrong'
            with self.assertRaises(ValueError):probe.artifact_metadata(changed,spec)
        for key in ('id','head_sha'):
            changed=copy.deepcopy(value);changed['workflow_run'][key]='wrong'
            with self.assertRaises(ValueError):probe.artifact_metadata(changed,spec)
        with self.assertRaises(ValueError):probe.strict_json(b'{"id":1,"id":2}')

    def test_ordinary_environment_is_refused_before_output_mutation(self):
        with tempfile.TemporaryDirectory() as folder:
            output=Path(folder)/'must-not-exist'
            with patch.dict(probe.os.environ,{'GITHUB_ACTIONS':'false'}),self.assertRaises(ValueError):probe.guarded_source(ROOT,output)
            self.assertFalse(output.exists())


if __name__=='__main__':unittest.main()
