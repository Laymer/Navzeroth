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
                admitted = world.checked_admission(fixture.registry, fixture.output)
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
                    world.checked_admission(fixture.registry, fixture.output)

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
                    world.checked_admission(fixture.registry, None)
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
                        world.checked_admission(fixture.registry, fixture.output)
                fixture.output.write_text(json.dumps(original))
                registry['sources'].append(registry['sources'][0])
                fixture.registry.write_text(json.dumps(registry))
                with self.assertRaises(world.InputError):
                    world.checked_admission(fixture.registry, fixture.output)
