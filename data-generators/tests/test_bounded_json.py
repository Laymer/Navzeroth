import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from navzeroth_datagen import bounded


class BoundedJson(unittest.TestCase):
    def test_duplicate_members_and_non_json_numbers_fail(self):
        for raw in (b'{"id":1,"id":2}', b'{"x":{"id":1,"id":2}}', b'[NaN]', b'[Infinity]', b'[1e999]'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                bounded.json_document(raw)

    def test_nesting_is_checked_before_recursive_parse(self):
        self.assertEqual(bounded.json_document(b'[[1]]', max_depth=2), [[1]])
        with self.assertRaises(ValueError):
            bounded.json_document(b'[' * 10000 + b']' * 10000)

    def test_brackets_in_escaped_strings_do_not_count(self):
        self.assertEqual(bounded.json_document(b'{"id":"[[[\\\"{{{"}', max_depth=1), {'id': '[[["{{{'})
