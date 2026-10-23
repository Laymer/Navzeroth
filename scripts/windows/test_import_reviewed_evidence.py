import importlib.util
import json
import os
import stat
import types
from pathlib import Path
import subprocess
import sys
import time
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
spec = importlib.util.spec_from_file_location('transport', Path(__file__).with_name('import-reviewed-evidence.py'))
transport = importlib.util.module_from_spec(spec)
spec.loader.exec_module(transport)


class TransportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'synthetic@example.invalid')
        self.git('config', 'user.name', 'Synthetic test')
        (self.root / 'source.txt').write_text('tagged source must never execute archive code')
        self.git('add', '.')
        self.git('commit', '-qm', 'tagged source')
        self.source = self.git('rev-parse', 'HEAD').strip()

    def tearDown(self):
        self.temp.cleanup()

    def git(self, *args, data=None):
        return subprocess.check_output(['git', '-C', str(self.root), *args], input=data).decode()

    def archive(self, extra=None):
        for name, value in {'acceptance-evidence/acceptance-evidence.json': b'{"untrusted":"gate must check"}',
                            'draft-evidence/raw.json': b'{"passed":false}', **(extra or {})}.items():
            path = self.root / transport.PREFIX / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(value)
        # A different commit may contain hostile code; it is never checked out/executed.
        (self.root / 'source.txt').write_text('hostile different code never imported')
        self.git('add', '.')
        self.git('commit', '-qm', 'inert archive')
        commit = self.git('rev-parse', 'HEAD').strip()
        self.git('checkout', '-q', self.source)
        return commit

    def import_archive(self, commit):
        return transport.import_blobs(self.root, commit, self.source)

    def test_positive_inert_source_preservation_and_failed_claim_not_promoted(self):
        commit = self.archive()
        self.assertEqual(self.import_archive(commit), 2)
        self.assertEqual(self.git('rev-parse', 'HEAD').strip(), self.source)
        self.assertEqual((self.root / 'source.txt').read_text(), 'tagged source must never execute archive code')
        self.assertEqual(json.loads((self.root / 'dist/draft-evidence/raw.json').read_text()), {'passed': False})
        self.assertEqual(self.git('status', '--porcelain', '--untracked-files=no'), '')

    def test_existing_destination_refused_without_merge(self):
        commit = self.archive()
        self.import_archive(commit)
        with self.assertRaises(ValueError):
            self.import_archive(commit)

    def test_lfs_pointer_refused_and_no_partial_destinations(self):
        commit = self.archive({'draft-evidence/pointer': b'version https://git-lfs.github.com/spec/v1\noid sha256:' + b'a' * 64 + b'\nsize 2\n'})
        with self.assertRaises(ValueError):
            self.import_archive(commit)
        self.assertFalse((self.root / 'dist/draft-evidence').exists())
        self.assertFalse((self.root / 'dist/acceptance-evidence').exists())

    def test_case_collision(self):
        commit = self.archive()
        self.git('checkout', '-q', commit)
        oid = self.git('hash-object', '-w', '--stdin', data=b'{}').strip()
        # Build the ambiguous tree in Git, never on a case-insensitive filesystem.
        self.git('-c', 'core.ignorecase=false', 'update-index', '--add', '--cacheinfo',
                 f'100644,{oid},{transport.PREFIX}draft-evidence/RAW.json')
        tree = self.git('write-tree').strip()
        ambiguous = self.git('commit-tree', tree, '-p', commit, '-m', 'ambiguous paths').strip()
        self.git('reset', '-q', commit)
        self.git('checkout', '-q', self.source)
        with self.assertRaisesRegex(ValueError, 'Case-colliding'):
            self.import_archive(ambiguous)

    def test_limits(self):
        commit = self.archive()
        for attribute, value in [('MAX_FILE', 2), ('MAX_TOTAL', 2), ('MAX_FILES', 1), ('MAX_TREE', 8)]:
            with self.subTest(attribute=attribute), patch.object(transport, attribute, value):
                # command default is bound at definition: tree output bound tested directly.
                with self.assertRaises(ValueError):
                    if attribute == 'MAX_TREE':
                        transport.command(self.root, ['ls-tree', '-r', commit], limit=8)
                    else:
                        self.import_archive(commit)

    def test_dirty_and_wrong_checkout_refused(self):
        commit = self.archive()
        (self.root / 'source.txt').write_text('dirty')
        with self.assertRaises(ValueError):
            self.import_archive(commit)
        self.git('checkout', '--', 'source.txt')
        with self.assertRaises(ValueError):
            transport.import_blobs(self.root, commit, commit)

    def test_unsafe_portable_paths(self):
        for name in [b'/absolute', b'../draft-evidence/file', b'draft-evidence/../file',
                     b'draft-evidence/a\\b', b'draft-evidence/.git/config', b'draft-evidence/NUL.json',
                     b'draft-evidence/a:', b'draft-evidence/a.', b'draft-evidence/a\xff']:
            with self.subTest(name=name), self.assertRaises(ValueError):
                transport.safe_relative(name)

    def test_scope_inputs(self):
        transport.validate_inputs('a' * 40, '1.0.0', 'public', '')
        for commit, version, phase, run in [('A'*40, '1.0.0', 'public', ''), ('a'*40, '1.0.0-rc.1', 'public', ''),
                                           ('a'*40, '1.0.0', 'draft', ''), ('a'*40, '1.0.0', 'public', '123')]:
            with self.assertRaises(ValueError):
                transport.validate_inputs(commit, version, phase, run)

    def test_non_regular_git_modes(self):
        commit = self.archive()
        self.git('checkout', '-q', commit)
        for mode in ['120000', '160000', '100755']:
            oid = self.git('hash-object', '-w', '--stdin', data=b'source.txt').strip() if mode != '160000' else self.source
            self.git('update-index', '--add', '--cacheinfo', f'{mode},{oid},{transport.PREFIX}draft-evidence/unsafe')
            tree = self.git('write-tree').strip()
            special = self.git('commit-tree', tree, '-p', commit, '-m', 'unsafe metadata').strip()
            self.git('reset', '-q', commit)
            self.git('checkout', '-q', self.source)
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                transport.inventory(self.root, special)
            self.git('checkout', '-q', commit)

    def test_missing_overlay(self):
        commit = self.archive()
        self.git('checkout', '-q', commit)
        self.git('rm', '-q', transport.PREFIX + 'acceptance-evidence/acceptance-evidence.json')
        self.git('commit', '-qm', 'missing overlay')
        missing = self.git('rev-parse', 'HEAD').strip()
        self.git('checkout', '-q', self.source)
        with self.assertRaises(ValueError):
            self.import_archive(missing)

    def test_cli_same_repo_fetch_is_data_only(self):
        commit = self.archive()
        self.git('remote', 'add', 'origin', 'https://github.com/example/project.git')
        original = transport.command
        calls = []
        def fake_fetch(root, args, *positional, **keywords):
            if 'fetch' in args:
                calls.append(args)
                return b''  # Commit is already in this synthetic repository: no network test.
            return original(root, args, *positional, **keywords)
        argv = ['transport', '--root', str(self.root), '--commit', commit,
                '--expected-source', self.source, '--repository', 'example/project',
                '--version', '1.0.0', '--phase', 'public']
        with patch.object(sys, 'argv', argv), patch.object(transport, 'command', fake_fetch):
            transport.main()
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][-2:], ['origin', commit])
        self.assertEqual(self.git('rev-parse', 'HEAD').strip(), self.source)

    def test_cli_rejects_other_origin_before_fetch(self):
        commit = self.archive()
        self.git('remote', 'add', 'origin', 'https://github.com/other/repository.git')
        argv = ['transport', '--root', str(self.root), '--commit', commit,
                '--expected-source', self.source, '--repository', 'example/project',
                '--version', '1.0.0', '--phase', 'public']
        with patch.object(sys, 'argv', argv), self.assertRaisesRegex(ValueError, 'same GitHub'):
            transport.main()
        self.assertFalse((self.root / 'dist').exists())

    def test_overall_deadline_and_no_archive_command_execution(self):
        with patch.object(transport, 'DEADLINE', time.monotonic() - 1):
            with self.assertRaisesRegex(ValueError, 'overall deadline'):
                transport.command(self.root, ['rev-parse', 'HEAD'])

    def test_parent_exit_does_not_abandon_owned_descendant_or_pipes(self):
        pidfile = self.root / 'owned-child.pid'
        child = f"import os,time,pathlib;pathlib.Path({str(pidfile)!r}).write_text(str(os.getpid()));time.sleep(30)"
        parent = f"import subprocess,sys;subprocess.Popen([sys.executable,'-c',{child!r}]);print('parent exit',flush=True)"
        start = time.monotonic()
        with self.assertRaisesRegex(ValueError, 'deadline'):
            transport.owned_output([sys.executable, '-c', parent], 4096, 3)
        self.assertLess(time.monotonic() - start, 8)
        self.assertTrue(pidfile.exists(), 'Owned child actually started')
        pid = int(pidfile.read_text())
        if os.name == 'nt':
            import ctypes as c
            from ctypes import wintypes as w
            api = c.WinDLL('kernel32', use_last_error=True)
            api.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]; api.OpenProcess.restype = w.HANDLE
            api.GetExitCodeProcess.argtypes = [w.HANDLE, c.POINTER(w.DWORD)]; api.GetExitCodeProcess.restype = w.BOOL
            api.CloseHandle.argtypes = [w.HANDLE]
            handle = api.OpenProcess(0x1000, False, pid)
            if handle:
                try:
                    code = w.DWORD(); self.assertTrue(api.GetExitCodeProcess(handle, c.byref(code)))
                    self.assertNotEqual(code.value, 259)
                finally:
                    api.CloseHandle(handle)
        else:
            status = Path(f'/proc/{pid}/stat')
            self.assertTrue(not status.exists() or status.read_text().split(') ', 1)[1].startswith('Z '))

    def test_output_bound_and_owned_timeout(self):
        with self.assertRaisesRegex(ValueError, 'bounds'):
            transport.owned_output([sys.executable, '-c', "print('x'*10000)"], 16, 5)
        with self.assertRaisesRegex(ValueError, 'deadline'):
            transport.owned_output([sys.executable, '-c', 'import time;time.sleep(30)'], 4096, 0.2)

    def test_windows_reparse_attribute_refused(self):
        path = self.root / 'mock-dist'
        metadata = types.SimpleNamespace(st_mode=stat.S_IFDIR, st_file_attributes=stat.FILE_ATTRIBUTE_REPARSE_POINT)
        with patch.object(Path, 'lstat', return_value=metadata), self.assertRaisesRegex(ValueError, 'Reparse'):
            transport.refuse_reparse(path)

    @unittest.skipUnless(os.name == 'nt', 'Actual Windows junction requires Windows')
    def test_native_junction_destination_refused(self):
        commit = self.archive()
        junction = self.root / 'dist'
        subprocess.run(['cmd.exe', '/c', 'mklink', '/J', str(junction), str(self.root)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=10)
        try:
            with self.assertRaisesRegex(ValueError, 'Reparse'):
                self.import_archive(commit)
        finally:
            os.rmdir(junction)

    def test_symlink_destination_refused(self):
        commit = self.archive()
        try:
            (self.root / 'dist').symlink_to(self.root, target_is_directory=True)
        except OSError:
            self.skipTest('Host cannot create synthetic symlink')
        with self.assertRaises(ValueError):
            self.import_archive(commit)


if __name__ == '__main__':
    unittest.main()
