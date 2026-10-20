"""Synthetic temporary Git/LFS repositories; no network or release signatures."""
import hashlib
import importlib.util
import json
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'hydrate-bundle-inputs.py'
spec = importlib.util.spec_from_file_location('bundle_inputs', SCRIPT)
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)
REVIEW = 'docs/releases/1.0.0-independent-public-review.json'


class Inputs(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Synthetic test')
        self.git('config', 'user.email', 'synthetic@example.invalid')
        self.git('lfs', 'install', '--local')
        (self.root / '.gitattributes').write_text('content/pack.json filter=lfs diff=lfs merge=lfs -text\n')
        self.raw = b'{"synthetic":"pack"}\n'
        self.write('content/pack.json', self.raw)
        self.write(REVIEW, b'{\n  "synthetic": true\n}\n')
        self.manifest = {'format': 'navzeroth-installed-content', 'schema_version': 1,
                         'app_version': '1.0.0', 'files': [self.ref('content/pack.json'), self.ref(REVIEW)]}
        self.save()
        self.stub()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], stderr=subprocess.DEVNULL)

    def write(self, path, raw):
        p = self.root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(raw)

    def ref(self, path):
        raw = (self.root / path).read_bytes()
        return {'path': path, 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}

    def save(self):
        self.write(h.MANIFEST, json.dumps(self.manifest).encode())
        self.write(h.IDENTITY, json.dumps({'app_version': '1.0.0', 'bundle_manifest_sha256': self.ref(h.MANIFEST)['sha256']}).encode())
        self.git('add', '.')
        self.git('commit', '--allow-empty', '-qm', 'synthetic bundle')

    def stub(self):
        self.write('content/pack.json', self.git('show', 'HEAD:content/pack.json'))

    def test_pointer_fails_resource_verification_then_exact_offline_hydration(self):
        self.assertNotEqual(h.parity.digest_file(self.root / 'content/pack.json', h.MAX_FILE),
                            {k: self.manifest['files'][0][k] for k in ('bytes', 'sha256')})
        with mock.patch.object(h.parity, 'fetch', wraps=h.parity.fetch) as fetch:
            result = h.hydrate(self.root, offline=True)
            self.assertEqual(fetch.call_args.args[1:], (['content/pack.json'], True))
        self.assertEqual(result['hydrated'], ['content/pack.json'])
        self.assertFalse(result['publication_allowed'])
        self.assertEqual((self.root / 'content/pack.json').read_bytes(), self.raw)
        self.assertEqual(h.hydrate(self.root, offline=True)['hydrated'], [])

    def test_wrong_working_bytes_refused_before_fetch(self):
        self.write('content/pack.json', b'wrong')
        with mock.patch.object(h.parity, 'fetch') as fetch:
            with self.assertRaises(ValueError): h.hydrate(self.root, True)
            fetch.assert_not_called()

    def test_post_fetch_wrong_or_missing_bytes_refused(self):
        for replacement in (b'bad', None):
            self.stub()
            def corrupt(*args):
                p = self.root / 'content/pack.json'
                p.unlink() if replacement is None else p.write_bytes(replacement)
            with mock.patch.object(h.parity, 'fetch', side_effect=corrupt):
                with self.assertRaises((ValueError, FileNotFoundError)): h.hydrate(self.root, True)

    def test_wrong_head_ref_duplicate_undeclared_and_bounds(self):
        original = json.loads(json.dumps(self.manifest))
        changes = [lambda m: m['files'][0].update(sha256='0'*64),
                   lambda m: m['files'].append(m['files'][0]),
                   lambda m: m['files'][0].update(path='content/absent.json'),
                   lambda m: m['files'][0].update(bytes=h.MAX_FILE+1),
                   lambda m: m.update(files=[]),
                   lambda m: m['files'][0].update(path='../outside'),
                   lambda m: m['files'][0].update(bytes=True)]
        for change in changes:
            self.manifest = json.loads(json.dumps(original)); change(self.manifest); self.save(); self.stub()
            with self.subTest(change=change), mock.patch.object(h.parity, 'fetch') as fetch:
                with self.assertRaises((ValueError, subprocess.CalledProcessError)): h.hydrate(self.root, True)
                fetch.assert_not_called()

    def test_manifest_count_aggregate_and_metadata_bounds(self):
        with mock.patch.object(h, 'MAX_FILES', 1):
            with self.assertRaises(ValueError): h.hydrate(self.root, True)
        with mock.patch.object(h, 'MAX_TOTAL', 1):
            with self.assertRaises(ValueError): h.hydrate(self.root, True)
        with mock.patch.object(h, 'MAX_MANIFEST', 1):
            with self.assertRaises(ValueError): h.hydrate(self.root, True)

    def test_head_metadata_role_bound_refuses_before_body_read_or_fetch(self):
        # Real committed ordinary blobs exceed each role's test limit; the
        # cat-file size query must refuse before show/tempfile streaming.
        for path in (h.MANIFEST, h.IDENTITY):
            with self.subTest(path=path), mock.patch.object(h.parity, 'git', wraps=h.parity.git) as git, \
                    mock.patch.object(h.subprocess, 'run', wraps=h.subprocess.run) as run, \
                    mock.patch.object(h.parity, 'fetch') as fetch:
                with self.assertRaises(ValueError): h.metadata(self.root, path, 1)
                self.assertEqual([call.args[1:] for call in git.call_args_list],
                                 [('cat-file', '-s', 'HEAD:' + path)])
                self.assertEqual(len(run.call_args_list), 1)
                self.assertNotIn('show', run.call_args.args[0])
                fetch.assert_not_called()

    def test_modified_metadata_and_identity_binding(self):
        self.write(h.MANIFEST, b'{}')
        with self.assertRaises(ValueError): h.hydrate(self.root, True)
        self.save(); self.stub()
        self.write(h.IDENTITY, b'{"app_version":"1.0.0","bundle_manifest_sha256":"wrong"}')
        self.git('add', h.IDENTITY); self.git('commit', '-qm', 'wrong identity')
        with self.assertRaises(ValueError): h.hydrate(self.root, True)

    def test_large_ordinary_git_blob_and_noncanonical_pointer(self):
        self.write(REVIEW, b' ' * 2048)
        self.manifest['files'][1] = self.ref(REVIEW)
        self.save(); self.stub()
        self.assertEqual(h.hydrate(self.root, True)['files'], 2)
        self.write(REVIEW, b'version https://git-lfs.github.com/spec/v1\ninvalid\n')
        self.manifest['files'][1] = self.ref(REVIEW)
        self.save(); self.stub()
        with self.assertRaises(ValueError): h.hydrate(self.root, True)

    def test_source_change_and_wrong_schema_refused(self):
        def changed(*args):
            h.parity.git(self.root, 'commit', '--allow-empty', '-qm', 'concurrent source')
            self.write('content/pack.json', self.raw)
        with mock.patch.object(h.parity, 'fetch', side_effect=changed):
            with self.assertRaises(ValueError): h.hydrate(self.root, True)
        self.manifest['schema_version'] = 2
        self.save(); self.stub()
        with self.assertRaises(ValueError): h.hydrate(self.root, True)

    def test_links_refused(self):
        p = self.root / 'content/pack.json'; p.unlink()
        try: p.symlink_to(self.root / REVIEW)
        except OSError: self.skipTest('symlinks unavailable')
        with self.assertRaises(ValueError): h.hydrate(self.root, True)

    def test_authored_markdown_autocrlf_refuses_then_lf_contract_hydrates_exactly(self):
        path = 'content/release-packs/authored-v2/evidence/authoring-statement.md'
        raw = b'# Synthetic engineering evidence\n\nExact authored bytes.\n'
        self.write(path, raw)
        self.manifest['files'].append(self.ref(path))
        self.save(); self.stub()
        self.git('config', 'core.autocrlf', 'true')
        p = self.root / path
        p.unlink(); self.git('checkout', 'HEAD', '--', path)
        self.assertIn(b'\r\n', p.read_bytes())
        with mock.patch.object(h.parity, 'fetch') as fetch:
            with self.assertRaisesRegex(ValueError, 'non-LFS resource differs'): h.hydrate(self.root, True)
            fetch.assert_not_called()
        rule = 'content/release-packs/authored-v2/**/*.md text eol=lf'
        self.assertEqual((SCRIPT.parent.parent / '.gitattributes').read_text().splitlines().count(rule), 1)
        with (self.root / '.gitattributes').open('a') as out: out.write(rule+'\n')
        self.git('add', '.gitattributes'); self.git('commit', '-qm', 'authored Markdown LF contract')
        p.unlink(); self.git('checkout', 'HEAD', '--', path)
        self.assertEqual(p.read_bytes(), raw)
        self.assertEqual(h.hydrate(self.root, True)['files'], 3)
        p.write_bytes(raw + b'altered')
        with self.assertRaisesRegex(ValueError, 'non-LFS resource differs'): h.hydrate(self.root, True)

    def test_replacement_review_v2_exact_lf_rule(self):
        path = REVIEW.replace('.json', '-v2.json')
        rule = path + ' text eol=lf'
        self.assertEqual((SCRIPT.parent.parent / '.gitattributes').read_text().splitlines().count(rule), 1)
        self.write(path, b'{\n  "replacement_synthetic_review": true\n}\n')
        self.manifest['files'].append(self.ref(path))
        with (self.root / '.gitattributes').open('a') as out: out.write(rule+'\n')
        self.save(); self.stub()
        self.git('config', 'core.autocrlf', 'true')
        (self.root / path).unlink(); self.git('checkout', 'HEAD', '--', path)
        self.assertNotIn(b'\r\n', (self.root / path).read_bytes())
        self.assertEqual(h.hydrate(self.root, True)['files'], 3)

    def test_autocrlf_review_refused_then_exact_repository_rule_preserves_bytes(self):
        self.git('config', 'core.autocrlf', 'true')
        p = self.root / REVIEW; p.unlink(); self.git('checkout', 'HEAD', '--', REVIEW)
        self.assertIn(b'\r\n', p.read_bytes())
        with self.assertRaises(ValueError): h.hydrate(self.root, True)
        rule = REVIEW + ' text eol=lf'
        self.assertEqual((SCRIPT.parent.parent / '.gitattributes').read_text().splitlines().count(rule), 1)
        with (self.root / '.gitattributes').open('a') as out: out.write(rule+'\n')
        self.git('add', '.gitattributes'); self.git('commit', '-qm', 'exact review LF')
        p.unlink(); self.git('checkout', 'HEAD', '--', REVIEW)
        self.assertNotIn(b'\r\n', p.read_bytes())
        self.assertEqual(h.hydrate(self.root, True)['files'], 2)


if __name__ == '__main__': unittest.main()
