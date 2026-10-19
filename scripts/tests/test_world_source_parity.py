import copy
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'verify-world-source-parity.py'
SPEC = importlib.util.spec_from_file_location('world_parity', SCRIPT)
p = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(p)


class ParityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.base = 'content/world/1.60.1.70450'
        self.raw = 'data-generators/output/quests/part-0000.json'
        self.original = 'content/source-materials/objects/' + 'a' * 64
        self.refs = {}
        self.docs = {}
        def ref(path, size=10, sha='b' * 64):
            self.refs[path] = dict(path=path, bytes=size, sha256=sha)
            return self.refs[path]
        self.ref = ref
        for path in [p.RUN, self.base + '/source-closure.json', p.TARGET, p.POLICY, p.REGISTRY,
                     'release/v1/accepted-origins.json', 'content/authored/slice-v1/catalog.json']:
            ref(path)
        for name in p.OUTPUTS:
            ref(self.base + '/' + name)
        raw = ref(self.raw)
        self.docs[p.RUN] = dict(format='navzeroth-world-data-run', generators=[dict(files=[dict(raw, path='quests/part-0000.json')])])
        self.docs[self.base + '/source-closure.json'] = dict(schema_version=2, source_commit='c' * 40,
            transformation='world_to_content_v2', sources=[dict(id='cmangos-classic-db',
            files=[ref(self.original, 20, 'a' * 64)], permission_evidence=[])], corresponding_source=dict(files=[]))
        self.docs[p.TARGET] = dict(build='1.60.1.70450')

    def selection(self):
        with mock.patch.object(p, 'tracked', return_value=set(self.refs)), mock.patch.object(p, 'pin', side_effect=lambda root, path: self.refs[path]), mock.patch.object(p, 'verified_json', side_effect=lambda root, path: self.docs[path]):
            return p.selection(self.root, self.base)

    def test_selection_exact_raw_original_and_outputs_excludes_media_facts(self):
        self.ref('screenshots/private-image.png')
        self.ref('data-generators/output/facts/huge.json', 3 * p.TOTAL)
        result = self.selection()
        self.assertEqual((result['raw_shards'], result['upstream_originals']), (1, 1))
        paths = {r['path'] for r in result['files']}
        self.assertIn(self.raw, paths)
        self.assertIn(self.original, paths)
        self.assertNotIn('screenshots/private-image.png', paths)
        self.assertNotIn('data-generators/output/facts/huge.json', paths)

    def test_duplicate_unlisted_and_mismatched_shards_refused(self):
        files = self.docs[p.RUN]['generators'][0]['files']
        files.append(copy.deepcopy(files[0]))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.selection()
        files.pop()
        self.ref('data-generators/output/quests/unlisted.json')
        with self.assertRaisesRegex(ValueError, 'tracked raw'):
            self.selection()
        del self.refs['data-generators/output/quests/unlisted.json']
        files[0]['sha256'] = 'f' * 64
        with self.assertRaisesRegex(ValueError, 'declared input'):
            self.selection()

    def test_role_bounds_original64_metadata8_and_shared_proof8(self):
        original = self.refs[self.original]
        original['bytes'] = p.META + 1
        self.assertEqual(self.selection()['upstream_originals'], 1)
        source = self.docs[self.base + '/source-closure.json']['sources'][0]
        source['permission_evidence'] = [original]
        with self.assertRaisesRegex(ValueError, 'role bound'):
            self.selection()
        source['permission_evidence'] = []
        original['bytes'] = p.ORIGINAL + 1
        with self.assertRaisesRegex(ValueError, 'role bound'):
            self.selection()
        original['bytes'] = 20
        self.refs[self.raw]['bytes'] = p.META + 1
        with self.assertRaisesRegex(ValueError, 'declared input|role bound'):
            self.selection()

    def test_original_must_be_content_addressed(self):
        self.docs[self.base + '/source-closure.json']['sources'][0]['files'][0]['path'] = 'media/pretend.csv'
        with self.assertRaisesRegex(ValueError, 'content addressed'):
            self.selection()

    def test_file_count_and_aggregate_bounds(self):
        with mock.patch.object(p, 'MAX_FILES', 2):
            with self.assertRaisesRegex(ValueError, 'closure exceeds'):
                self.selection()
        with mock.patch.object(p, 'TOTAL', 30):
            with self.assertRaisesRegex(ValueError, 'closure exceeds'):
                self.selection()

    def test_absent_and_partial_corpus_are_distinct(self):
        with mock.patch.object(p, 'tracked', return_value=set()):
            self.assertIsNone(p.corpus(self.root))
        with mock.patch.object(p, 'tracked', return_value={p.POLICY}):
            with self.assertRaisesRegex(ValueError, 'partial'):
                p.corpus(self.root)
        with mock.patch.object(p, 'tracked', return_value={p.POLICY, p.TARGET}), mock.patch.object(p, 'verified_json', return_value={'build': '1.60.1.70450'}):
            with self.assertRaisesRegex(ValueError, 'lacks'):
                p.corpus(self.root)

    def test_fetch_is_exact_bounded_and_never_globbed(self):
        with mock.patch.object(p, 'git') as git:
            p.fetch(self.root, ['content/a.json', self.raw, 'content/a.json'])
            self.assertEqual(git.call_args.args[1:], ('lfs', 'pull', '--include=content/a.json,' + self.raw, '--exclude='))
            p.fetch(self.root, [self.raw], offline=True)
            self.assertEqual(git.call_args.args[1:], ('lfs', 'checkout', self.raw))
            for bad in ['../secret', '/absolute', 'media/*', 'a,b', 'a/./b', 'a/.hidden', 'C:/foo', '--all']:
                with self.assertRaises(ValueError):
                    p.fetch(self.root, [bad])
            with mock.patch.object(p, 'MAX_FILES', 1), self.assertRaises(ValueError):
                p.fetch(self.root, ['a', 'b'])

    def test_owned_bounded_json_bytes_and_identity(self):
        path = self.root / 'metadata.json'
        raw = b'{"known":false,"zero":0,"unknown":null}\n'
        path.write_bytes(raw)
        expected = dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
        with mock.patch.object(pathlib.Path, 'open', side_effect=AssertionError('must not reopen captured bytes')):
            self.assertEqual(p.read_json(path, expected), dict(known=False, zero=0, unknown=None))
        expected['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'differs'):
            p.read_json(path, expected)
        with mock.patch.object(p, 'META', len(raw) - 1), self.assertRaisesRegex(ValueError, 'bound'):
            p.read_json(path)
        for invalid in [b'{"x":1,"x":2}', b'{"x":NaN}', b'[' * 66 + b'0' + b']' * 66]:
            path.write_bytes(invalid)
            with self.assertRaises(ValueError):
                p.read_json(path)

    def test_symlink_and_changed_identity_refused(self):
        path = self.root / 'metadata.json'
        path.write_bytes(b'{}')
        with mock.patch.object(p, 'stamp', side_effect=[1, 2]), self.assertRaisesRegex(ValueError, 'changed before'):
            p.digest_file(path, 10)
        linked = self.root / 'linked.json'
        try:
            linked.symlink_to(path)
        except OSError:
            self.skipTest('symlink creation unavailable on this runner')
        with self.assertRaisesRegex(ValueError, 'linked'):
            p.digest_file(linked, 10)

    def test_committed_lfs_pin_and_malformed_pointer(self):
        sha = 'd' * 64
        raw = f'version https://git-lfs.github.com/spec/v1\noid sha256:{sha}\nsize 99\n'.encode()
        with mock.patch.object(p, 'git', side_effect=[str(len(raw)).encode(), raw]):
            self.assertEqual(p.pin(self.root, 'a.json'), dict(path='a.json', bytes=99, sha256=sha))
        with mock.patch.object(p, 'git', side_effect=[b'200', raw.replace(b'\n', b'\r\n')]), self.assertRaisesRegex(ValueError, 'malformed'):
            p.pin(self.root, 'a.json')

    def test_hydration_verifies_all_exact_pins_and_fails_changed_resource(self):
        inputs = self.selection()
        by_path = {self.root / ref['path']: {k: ref[k] for k in ('bytes', 'sha256')} for ref in inputs['files']}
        with mock.patch.object(p, 'corpus', return_value=self.base), mock.patch.object(p, 'selection', return_value=inputs), mock.patch.object(p, 'fetch') as fetch, mock.patch.object(p, 'digest_file', side_effect=lambda path, bound: by_path[path]), mock.patch.object(p, 'git') as git:
            self.assertEqual(p.hydrate(self.root), inputs)
            self.assertEqual(fetch.call_args_list[0].args, (self.root, [p.RUN, self.base + '/source-closure.json'], False))
            self.assertEqual(set(fetch.call_args_list[1].args[1]), {ref['path'] for ref in inputs['files']})
            git.assert_called_once_with(self.root, 'cat-file', '-e', inputs['source_commit'] + '^{commit}')
        with mock.patch.object(p, 'corpus', return_value=self.base), mock.patch.object(p, 'selection', return_value=inputs), mock.patch.object(p, 'fetch'), mock.patch.object(p, 'digest_file', return_value=dict(bytes=1, sha256='0' * 64)):
            with self.assertRaisesRegex(ValueError, 'differs'):
                p.hydrate(self.root, offline=True)

    def test_legacy_verification_is_explicitly_unverified_and_revision_pinned(self):
        output = self.root / 'output'
        with mock.patch.object(p, 'corpus', return_value=None), mock.patch.object(p, 'git', return_value=b'c' * 40):
            receipt = p.verify(self.root, output)
        self.assertEqual(receipt['status'], 'absent')
        self.assertEqual(receipt['reason'], p.ABSENT)
        self.assertNotIn('products', receipt)
        self.assertEqual(p.read_json(output / 'world-parity.json')['status'], 'absent')
        (output / 'world-parity.json').unlink()
        with mock.patch.object(p, 'corpus', return_value=None), mock.patch.object(p, 'git', side_effect=[b'c' * 40, b'd' * 40]):
            with self.assertRaisesRegex(ValueError, 'revision changed'):
                p.verify(self.root, output)
        self.assertFalse((output / 'world-parity.json').exists())

    def test_receipts_require_complete_bound_products_and_matching_inputs(self):
        a = dict(schema_version=1, source_commit='d' * 40, status='verified', inputs=self.selection(),
                 products=[dict(path=n, bytes=10, sha256='e' * 64) for n in p.OUTPUTS], raw_shard_regeneration=False)
        left, right = self.root / 'a.json', self.root / 'b.json'
        def write(b):
            left.write_text(json.dumps(a), encoding='utf-8')
            right.write_text(json.dumps(b), encoding='utf-8')
        b = copy.deepcopy(a)
        b.update(platform='win32', elapsed_seconds=55)
        write(b)
        p.compare(left, right, 'd' * 40)
        with self.assertRaisesRegex(ValueError, 'requested source revision'):
            p.compare(left, right, 'c' * 40)
        b['products'][0]['sha256'] = 'f' * 64
        write(b)
        with self.assertRaisesRegex(ValueError, 'hashes differ'):
            p.compare(left, right)
        for products in [[], a['products'][:-1], a['products'] + [a['products'][0]]]:
            b = copy.deepcopy(a)
            b['products'] = products
            write(b)
            with self.assertRaises(ValueError):
                p.compare(left, right)
        b = dict(schema_version=1, source_commit='d' * 40, status='absent', reason=p.ABSENT)
        write(b)
        with self.assertRaises(ValueError):
            p.compare(left, right)
        p.validate_receipt(b)
        b['reason'] = 'passed'
        with self.assertRaisesRegex(ValueError, 'unverified'):
            p.validate_receipt(b)


if __name__ == '__main__':
    unittest.main()
