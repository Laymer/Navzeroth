"""Real pointer checkout/local-object hydration; no network or generation inputs."""
import hashlib
import importlib.util
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'hydrate-role-audit-inputs.py'
spec = importlib.util.spec_from_file_location('role_inputs', SCRIPT)
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)


class Inputs(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Synthetic test')
        self.git('config', 'user.email', 'synthetic@example.invalid')
        self.git('lfs', 'install', '--local')
        (self.root / '.gitattributes').write_text('\n'.join(p + ' filter=lfs diff=lfs merge=lfs -text' for p in h.INPUTS) + '\n')
        self.origin = {'edition': 'vanilla_1_12'}
        self.quest = self.write(h.QUEST, {'origin': self.origin, 'confidence': 'cross_edition_unverified', 'records': [1, 2]})
        self.pin = {'path': h.QUEST, 'bytes': len(self.quest), 'sha256': hashlib.sha256(self.quest).hexdigest()}
        self.report = {'source': {**self.pin, 'origin': self.origin, 'confidence': 'cross_edition_unverified', 'records': 2}}
        self.manifest = {'format': 'navzeroth-world-data-run', 'schema_version': 1, 'generators': [{'files': [{**self.pin, 'path': h.QUEST.removeprefix('data-generators/output/')}]}]}
        self.write(h.RUN, self.manifest)
        self.write(h.REPORT, self.report)
        self.commit()
        self.stub()

    def git(self, *args, raw=None):
        return subprocess.check_output(['git', '-C', str(self.root), *args], input=raw, stderr=subprocess.DEVNULL)

    def write(self, path, value):
        p = self.root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        raw = (json.dumps(value, sort_keys=True) + '\n').encode()
        p.write_bytes(raw)
        return raw

    def commit(self):
        self.git('add', '.')
        self.git('commit', '-qm', 'synthetic inputs')

    def stub(self):
        for path in h.INPUTS:
            (self.root / path).write_bytes(self.git('show', 'HEAD:' + path))

    def pointer(self, path, raw):
        oid = self.git('hash-object', '-w', '--stdin', raw=raw).decode().strip()
        self.git('update-index', '--cacheinfo', '100644,' + oid + ',' + path)
        self.git('commit', '-qm', 'synthetic changed pointer')

    def test_offline_cached_hydration_exactly_two_inputs_then_idempotent(self):
        with mock.patch.object(h.parity, 'fetch', wraps=h.parity.fetch) as fetch:
            result = h.hydrate(self.root, offline=True)
            self.assertEqual([call.args[1:] for call in fetch.call_args_list], [([h.RUN], True), ([h.QUEST], True)])
        self.assertEqual((self.root / h.QUEST).read_bytes(), self.quest)
        self.assertEqual(result['hydrated'], list(h.INPUTS))
        self.assertFalse(result['network'])
        with mock.patch.object(h.parity, 'fetch') as fetch:
            self.assertEqual(h.hydrate(self.root)['hydrated'], [])
            fetch.assert_not_called()

    def test_autocrlf_checkout_preserves_exact_report_with_repository_attribute(self):
        # Reproduce the Windows failure first: an ordinary LF JSON blob becomes
        # CRLF under core.autocrlf=true and strict verification correctly refuses.
        canonical = (self.root / h.REPORT).read_bytes()
        self.git('config', 'core.autocrlf', 'true')
        (self.root / h.REPORT).unlink()
        self.git('checkout', 'HEAD', '--', h.REPORT)
        self.assertIn(b'\r\n', (self.root / h.REPORT).read_bytes())
        with self.assertRaises(ValueError): h.hydrate(self.root, offline=True)
        # Apply the real, exact repository rule without normalizing read bytes.
        attributes = (SCRIPT.parent.parent / '.gitattributes').read_text()
        rule = h.REPORT + ' text eol=lf'
        self.assertEqual(attributes.splitlines().count(rule), 1)
        with (self.root / '.gitattributes').open('a') as output:
            output.write(rule + '\n')
        self.commit()
        (self.root / h.REPORT).unlink()
        self.git('checkout', 'HEAD', '--', h.REPORT)
        self.assertEqual((self.root / h.REPORT).read_bytes(), canonical)
        self.assertNotIn(b'\r', (self.root / h.REPORT).read_bytes())
        self.assertEqual(h.hydrate(self.root, offline=True)['status'], 'verified')

    def test_wrong_working_bytes_refused_before_fetch(self):
        (self.root / h.QUEST).write_bytes(b'wrong')
        with mock.patch.object(h.parity, 'fetch') as fetch:
            with self.assertRaises(ValueError): h.hydrate(self.root)
            fetch.assert_not_called()

    def test_wrong_or_nonallowlisted_report_pin_refused_before_fetch(self):
        for field, value in [('sha256', 'f' * 64), ('bytes', True), ('path', 'other.json')]:
            report = json.loads(json.dumps(self.report)); report['source'][field] = value
            self.write(h.REPORT, report); self.commit()
            with mock.patch.object(h.parity, 'fetch') as fetch:
                with self.assertRaises(ValueError): h.hydrate(self.root)
                fetch.assert_not_called()

    def test_oversized_or_noncanonical_head_pointer_refused(self):
        for raw in [b'version https://git-lfs.github.com/spec/v1\noid sha256:' + b'a' * 64 + b'\nsize 8388609\n', b'bad pointer']:
            self.pointer(h.QUEST, raw)
            with mock.patch.object(h.parity, 'fetch') as fetch:
                with self.assertRaises(ValueError): h.hydrate(self.root)
                fetch.assert_not_called()

    def test_post_fetch_wrong_bytes_and_missing_cache_fail_closed(self):
        for action in [lambda *args: None, lambda *args: (self.root / h.QUEST).write_bytes(b'altered')]:
            self.stub()
            with mock.patch.object(h.parity, 'fetch', side_effect=action):
                with self.assertRaises(ValueError): h.hydrate(self.root, offline=True)

    def test_manifest_mismatch_and_duplicate_registration_refused(self):
        for duplicate in [False, True]:
            manifest = json.loads(json.dumps(self.manifest))
            if duplicate: manifest['generators'][0]['files'] *= 2
            else: manifest['generators'][0]['files'][0]['sha256'] = 'f' * 64
            self.write(h.RUN, manifest); self.commit(); self.stub()
            with mock.patch.object(h.parity, 'fetch', wraps=h.parity.fetch) as fetch:
                with self.assertRaises(ValueError): h.hydrate(self.root, offline=True)
                self.assertEqual([call.args[1] for call in fetch.call_args_list], [[h.RUN]])

    def test_origin_or_record_count_mismatch_refused_after_exact_hydration(self):
        for field, value in [('origin', {'edition': 'other'}), ('records', 3)]:
            report = json.loads(json.dumps(self.report)); report['source'][field] = value
            self.write(h.REPORT, report); self.commit(); self.stub()
            with self.assertRaises(ValueError): h.hydrate(self.root, offline=True)

    def test_links_and_oversized_working_files_refused_before_fetch(self):
        path = self.root / h.QUEST
        path.write_bytes(b'x' * (h.MAX_INPUT + 1))
        with self.assertRaises(ValueError): h.hydrate(self.root)
        path.unlink()
        try: os.symlink(self.root / h.REPORT, path)
        except (OSError, NotImplementedError): self.skipTest('symlink creation unavailable')
        with mock.patch.object(h.parity, 'fetch') as fetch:
            with self.assertRaises(ValueError): h.hydrate(self.root)
            fetch.assert_not_called()

    def test_manifest_change_after_quest_restore_is_refused(self):
        restore = h.parity.fetch
        def changed(root, paths, offline):
            restore(root, paths, offline)
            if paths == [h.QUEST]: (root / h.RUN).write_bytes(b'changed')
        with mock.patch.object(h.parity, 'fetch', side_effect=changed):
            with self.assertRaises(ValueError): h.hydrate(self.root, offline=True)


class Wiring(unittest.TestCase):
    def test_workspace_ci_and_stability_hydrate_before_tests(self):
        workflow = (SCRIPT.parent.parent / '.github/workflows/ci.yml').read_text()
        for name in ['rust', 'stability']:
            body = workflow.split('  ' + name + ':\n', 1)[1]
            # End at the next job, not at a nested YAML property.
            import re
            body = re.split(r'^  [a-z_]+:\n', body, maxsplit=1, flags=re.M)[0]
            self.assertIn('timeout-minutes: 5', body)
            self.assertLess(body.index('python scripts/hydrate-role-audit-inputs.py --root .'), body.index('cargo test --locked --workspace'))

    def test_standalone_coverage_collector_verifies_inputs_before_workspace(self):
        collector = (SCRIPT.parent / 'coverage/collect-rust.mjs').read_text()
        self.assertLess(collector.index("run('role-audit-inputs'"), collector.index("run('workspace-tests'"))
        self.assertIn("'scripts/hydrate-role-audit-inputs.py'),'--root',root],{timeout:240000}", collector)


if __name__ == '__main__':
    unittest.main()
