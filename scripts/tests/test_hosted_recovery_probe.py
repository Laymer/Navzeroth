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

    def cargo_protocol(self, root):
        manifest=root/'crates/app/Cargo.toml';manifest.parent.mkdir(parents=True)
        manifest.write_text('[package]\nname="synthetic"\n')
        harness=root/'crates/app/tests/hosted_recovery_probe.rs';harness.parent.mkdir()
        harness.write_text('// inert source, never compiled\n')
        target=root/'build';binary=target/'release/deps/hosted_recovery_probe-0123456789abcdef.exe'
        binary.parent.mkdir(parents=True);binary.write_bytes(b'inert binary, never execute')
        artifact=dict(reason='compiler-artifact',manifest_path=str(manifest),
                      target=dict(name='hosted_recovery_probe',kind=['test'],src_path=str(harness)),
                      executable=str(binary))
        return target,binary,[artifact,dict(reason='build-finished',success=True)]

    def assert_owned_child_stopped(self, pid):
        # Termination is asynchronous; poll only the PID issued by this fixture.
        deadline=time.monotonic()+2
        while True:
            if probe.os.name=='nt':
                import ctypes as c
                from ctypes import wintypes as w
                api=c.WinDLL('kernel32',use_last_error=True)
                api.OpenProcess.argtypes=[w.DWORD,w.BOOL,w.DWORD];api.OpenProcess.restype=w.HANDLE
                api.GetExitCodeProcess.argtypes=[w.HANDLE,c.POINTER(w.DWORD)];api.GetExitCodeProcess.restype=w.BOOL
                api.CloseHandle.argtypes=[w.HANDLE]
                handle=api.OpenProcess(0x1000,False,pid)
                alive=False
                if handle:
                    try:
                        code=w.DWORD();self.assertTrue(api.GetExitCodeProcess(handle,c.byref(code)));alive=code.value==259
                    finally:api.CloseHandle(handle)
            else:
                status=Path(f'/proc/{pid}/stat')
                alive=status.exists() and not status.read_text().split(') ',1)[1].startswith('Z ')
            if not alive:return
            self.assertLess(time.monotonic(),deadline,'owned child remained executable after cleanup')
            time.sleep(.01)

    def test_cargo_root_completion_kills_owned_children_with_or_without_inherited_pipes(self):
        for inherited in (True,False):
            with self.subTest(inherited=inherited),tempfile.TemporaryDirectory() as folder:
                root=Path(folder).resolve();target,binary,events=self.cargo_protocol(root)
                pidfile=root/'child.pid'
                child=f"import os,time,pathlib;pathlib.Path({str(pidfile)!r}).write_text(str(os.getpid()));time.sleep(30)"
                redirect='' if inherited else ',stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL'
                parent=(f"import subprocess,sys,time,pathlib;subprocess.Popen([sys.executable,'-c',{child!r}]{redirect});"
                        f"p=pathlib.Path({str(pidfile)!r});deadline=time.monotonic()+3;\n"
                        "while not p.exists() and time.monotonic()<deadline:time.sleep(.01)\n"
                        f"assert p.exists();print({chr(10).join(json.dumps(v) for v in events)!r},flush=True)")
                start=time.monotonic()
                stdout=probe.run([sys.executable,'-c',parent],root,'cargo',5,compiler_build=True)
                self.assertLess(time.monotonic()-start,4)
                self.assert_owned_child_stopped(int(pidfile.read_text()))
                actual,metadata=probe.validate_cargo_build(stdout,root,target)
                self.assertEqual(actual,binary);self.assertEqual(metadata['binary_sha256'],hashlib.sha256(binary.read_bytes()).hexdigest())
                result=json.loads((root/'cargo.json').read_text())
                self.assertFalse(result['timed_out']);self.assertEqual(result['root_exit']['exit_code'],0)
                self.assertTrue(result['root_exit']['owned_tree_active'])
                self.assertTrue(result['owned_cleanup']['attempted']);self.assertTrue(result['owned_cleanup']['completed'])
                self.assertTrue(result['stdout_eof']);self.assertTrue(result['stderr_eof'])

    def test_cargo_completion_does_not_accept_root_failure_timeout_or_output_overflow(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            for name,body,limit in [('failed','raise SystemExit(7)',1024),('overflow','print("x"*10000)',16),('hung','import time;time.sleep(30)',1024)]:
                with self.subTest(name=name),self.assertRaises(ValueError):
                    probe.run([sys.executable,'-c',body],root,name,.3 if name=='hung' else 5,maximum=limit,compiler_build=True)
            self.assertEqual(json.loads((root/'failed.json').read_text())['exit_code'],7)
            self.assertTrue(json.loads((root/'hung.json').read_text())['timed_out'])
            self.assertTrue(json.loads((root/'overflow.json').read_text())['capture_errors'])

    def test_cargo_cleanup_and_capture_failures_are_recorded_and_refused(self):
        original=probe.OwnedTree
        class KillFailure(original):
            def kill(self):
                super().kill()
                raise OSError('synthetic owned kill failure after actual own cleanup')
        class CloseFailure(original):
            def close(self):
                super().close()
                raise OSError('synthetic own close failure')
        class ReadFailure(original):
            def __init__(self,*args,**kwargs):
                super().__init__(*args,**kwargs)
                inner=self.process.stdout
                class Pipe:
                    def read(self,_count):raise OSError('synthetic capture failure')
                    def close(self):inner.close()
                self.process.stdout=Pipe()
        for name,owner,body in [('kill',KillFailure,'import time;time.sleep(30)'),
                                ('close',CloseFailure,'print("done")'),
                                ('read',ReadFailure,'print("done")')]:
            with self.subTest(name=name),tempfile.TemporaryDirectory() as folder:
                root=Path(folder)
                with patch.object(probe,'OwnedTree',owner),self.assertRaises(ValueError):
                    probe.run([sys.executable,'-c',body],root,name,.2 if name=='kill' else 5,compiler_build=True)
                result=json.loads((root/(name+'.json')).read_text())
                self.assertTrue(result['capture_errors'])
                self.assertLess((root/(name+'.json')).stat().st_size,16384)

    def test_cargo_protocol_and_exact_source_owned_artifact_are_required(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder).resolve();target,binary,events=self.cargo_protocol(root);stdout=root/'build.stdout'
            for mutation in ('missing','duplicate','failed','not-last','not-bool','malformed','array','duplicate-json-field',
                             'ambiguous','absent-artifact','wrong-source','wrong-manifest','wrong-kind','outside','missing-binary','empty-binary','compiler-error'):
                with self.subTest(mutation=mutation):
                    current=copy.deepcopy(events);binary.write_bytes(b'inert binary, never execute')
                    if mutation=='missing':current.pop()
                    elif mutation=='duplicate':current.append(copy.deepcopy(current[-1]))
                    elif mutation=='failed':current[-1]['success']=False
                    elif mutation=='not-last':current.reverse()
                    elif mutation=='not-bool':current[-1]['success']=1
                    elif mutation=='ambiguous':current.insert(0,copy.deepcopy(current[0]))
                    elif mutation=='absent-artifact':current.pop(0)
                    elif mutation=='wrong-source':current[0]['target']['src_path']=str(root/'other.rs')
                    elif mutation=='wrong-manifest':current[0]['manifest_path']=str(root/'Cargo.toml')
                    elif mutation=='wrong-kind':current[0]['target']['kind']=['bin']
                    elif mutation=='outside':current[0]['executable']=str(root/'outside.exe')
                    elif mutation=='missing-binary':binary.unlink()
                    elif mutation=='empty-binary':binary.write_bytes(b'')
                    elif mutation=='compiler-error':current.insert(0,dict(reason='compiler-message',message=dict(level='error')))
                    text='\n'.join(json.dumps(v) for v in current)
                    if mutation=='malformed':text+='\nnot-json'
                    elif mutation=='array':text='[]\n'+text
                    elif mutation=='duplicate-json-field':text=text.replace('"success": true','"success": false,"success": true')
                    stdout.write_text(text+'\n')
                    with self.assertRaises((ValueError,OSError)):probe.validate_cargo_build(stdout,root,target)

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
