"""Standalone converter refuses input before large or ambiguous data can be used."""
import json
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import world_to_content as world
from navzeroth_datagen import bounded


class WorldInputBounds(unittest.TestCase):
    def test_file_and_aggregate_limits_precede_parsing(self):
        with tempfile.TemporaryDirectory() as root:
            root = pathlib.Path(root)
            (root / 'a.json').write_bytes(b'{"records": []}')
            with mock.patch.object(bounded, 'MAX_SHARD_BYTES', 4):
                with self.assertRaises(world.InputError):
                    world.load_input(root)
            with mock.patch.object(bounded, 'MAX_INPUT_BYTES', 4):
                with self.assertRaises(world.InputError):
                    world.load_input(root)

    def test_duplicate_nested_deep_and_alternate_encoding_inputs_are_refused(self):
        with tempfile.TemporaryDirectory() as root:
            root = pathlib.Path(root)
            for raw in (b'{"format":"navzeroth-world-data","format":"other"}',
                        b'[' * 65 + b']' * 65,
                        json.dumps({'x': '"', 'a': [[[1]]]}).encode('utf-16')):
                with self.subTest(raw=raw):
                    (root / 'a.json').write_bytes(raw)
                    with self.assertRaises(world.InputError):
                        world.load_input(root)
                    with self.assertRaises(world.InputError):
                        world.read_json(root / 'a.json', 'policy')

    def test_non_array_records_and_excess_count_are_refused(self):
        with tempfile.TemporaryDirectory() as root:
            root = pathlib.Path(root)
            path = root / 'a.json'
            path.write_text('{"format":"navzeroth-world-data","records":{}}')
            with self.assertRaises(world.InputError):
                world.load_input(root)
            path.write_text('{"format":"navzeroth-world-data","records":[{}]}')
            with mock.patch.object(bounded, 'MAX_INPUT_RECORDS', 0):
                with self.assertRaises(world.InputError):
                    world.load_input(root)

    def test_directory_enumeration_count_is_finite(self):
        with tempfile.TemporaryDirectory() as root:
            root = pathlib.Path(root)
            (root / 'a.json').write_text('{}')
            with mock.patch.object(bounded, 'MAX_INPUT_ENTRIES', 0):
                with self.assertRaises(world.InputError):
                    world.load_input(root)
