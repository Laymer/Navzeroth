"""Source metadata normalization requires retained exact originals and rights."""
import copy
import json
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import world_to_content as world
from test_retain_world_sources import Fixture


class WorldAdmissionBinding(unittest.TestCase):
    def test_retained_closure_is_bound_before_metadata_becomes_usable(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            registry = json.loads(fixture.registry.read_bytes())
            for source in registry["sources"]:
                source["publisher"] = "Synthetic publisher"
                source["rights"].update(license_identifier="LicenseRef-Synthetic-Test", attribution="Synthetic original author")
            fixture.registry.write_text(json.dumps(registry))
            fixture.run()
            with mock.patch.object(world, '__file__', str(fixture.root/'data-generators/world_to_content.py')), mock.patch.object(world, 'VERSION', '7'):
                admitted = world.checked_admission(fixture.registry, fixture.output, fixture.world, fixture.policy)
                self.assertEqual(set(admitted), {'cmangos-classic-db', 'wago-tools-wowf', world.DERIVED_SOURCE})
                for sid, source in admitted.items():
                    builder = object.__new__(world.Builder)
                    builder.admitted = admitted
                    fields = source['files'][0]['fields']
                    builder.evidence_sources = {sid: fields}
                    info = builder.source_info(sid)
                    self.assertEqual(info['license'], source['rights']['license_identifier'])
                    self.assertEqual(info['verified_revision'], source['revision'])
                    self.assertEqual(info['redistribution'], 'permitted')
                    builder.evidence_sources[sid] = ['unreviewed.field']
                    with self.assertRaises(world.InputError):
                        builder.source_info(sid)
                manifest = json.loads(fixture.output.read_bytes())
                source = manifest['sources'][0]['files'][0]
                (fixture.root/source['path']).write_bytes(b'changed')
                with self.assertRaises(world.InputError):
                    world.checked_admission(fixture.registry, fixture.output, fixture.world, fixture.policy)

    def test_missing_wrong_duplicate_or_unreviewed_bindings_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            registry = json.loads(fixture.registry.read_bytes())
            for source in registry["sources"]:
                source["publisher"] = "Synthetic publisher"
                source["rights"].update(license_identifier="LicenseRef-Synthetic-Test", attribution="Synthetic original author")
            fixture.registry.write_text(json.dumps(registry))
            fixture.run()
            original = json.loads(fixture.output.read_bytes())
            registry = json.loads(fixture.registry.read_bytes())
            with mock.patch.object(world, '__file__', str(fixture.root/'data-generators/world_to_content.py')), mock.patch.object(world, 'VERSION', '7'):
                with self.assertRaises(world.InputError):
                    world.checked_admission(fixture.registry, None, fixture.world, fixture.policy)
                for kind in ('version', 'revision', 'origin', 'proof', 'files', 'path'):
                    changed = copy.deepcopy(original)
                    if kind == 'version': changed['transformation'] = 'world_to_content_v8'
                    if kind == 'revision': changed['sources'][0]['revision'] = '0'*40
                    if kind == 'origin': changed['sources'][0]['origin']['build'] = 'wrong'
                    if kind == 'proof': changed['sources'][0]['permission_evidence'] = []
                    if kind == 'files': changed['sources'][0]['files'] = []
                    if kind == 'path': changed['corresponding_source']['files'][0]['path'] = '../escape'
                    fixture.output.write_text(json.dumps(changed))
                    with self.subTest(kind=kind), self.assertRaises(world.InputError):
                        world.checked_admission(fixture.registry, fixture.output, fixture.world, fixture.policy)
                fixture.output.write_text(json.dumps(original))
                registry['sources'].append(registry['sources'][0])
                fixture.registry.write_text(json.dumps(registry))
                with self.assertRaises(world.InputError):
                    world.checked_admission(fixture.registry, fixture.output, fixture.world, fixture.policy)

    def test_exact_source_and_correspondence_closure_cannot_be_deleted_or_substituted(self):
        for mutation in ('no-correspondence', 'wrong-commit', 'missing-terms', 'snapshot', 'no-snapshots', 'origin', 'unbound-grant', 'derived-license', 'derived-code', 'derived-obligations', 'terms-classification', 'policy'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                f = Fixture(directory); f.run()
                closure = json.loads(f.output.read_bytes()); registry = json.loads(f.registry.read_bytes())
                if mutation == 'no-correspondence': closure['corresponding_source']['files'] = []
                elif mutation == 'wrong-commit': closure['source_commit'] = '0'*40
                elif mutation == 'missing-terms': closure['corresponding_source']['files'] = [r for r in closure['corresponding_source']['files'] if r['source_path'] != 'data-generators/NOTICE.md']
                elif mutation == 'snapshot': closure['sources'][0]['snapshots'][0]['snapshot_sha256'] = '0'*64
                elif mutation == 'no-snapshots': closure['sources'][0]['snapshots'] = []
                elif mutation == 'origin':
                    shard = f.world/'db2/part-0000.json';doc=json.loads(shard.read_bytes());doc['sources'][0]['revision']='1.60.1.99999';shard.write_text(json.dumps(doc))
                elif mutation == 'unbound-grant': registry['sources'][0]['rights']['license_identifier'] = 'Fabricated-permission'
                elif mutation == 'derived-license': registry['sources'][-1]['rights']['evidence'][0]['sha256'] = '0'*64
                elif mutation == 'derived-obligations': registry['sources'][-1]['rights']['obligations'] = ''
                elif mutation == 'terms-classification': closure['corresponding_source']['classification'] = 'BSD-only'
                elif mutation == 'derived-code': registry['sources'][-1]['files'][0]['url'] += '-wrong'
                else: closure['normalization_policy_sha256'] = '0'*64
                f.output.write_text(json.dumps(closure));f.registry.write_text(json.dumps(registry))
                with mock.patch.object(world, '__file__', str(f.root/'data-generators/world_to_content.py')), mock.patch.object(world,'VERSION','7'):
                    with self.assertRaises(world.InputError): world.checked_admission(f.registry, f.output, f.world, f.policy)
