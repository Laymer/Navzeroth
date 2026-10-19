import json
import os
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
import import_wowhead_pages as ip  # noqa: E402
from navzeroth_datagen import http  # noqa: E402


class ImportPagesTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.src = pathlib.Path(self.tmp.name) / "saved"
        self.src.mkdir()
        http.configure(cache=pathlib.Path(self.tmp.name) / "cache", offline_only=True, refresh_cache=False)

    def test_expected_has_eighteen_pages(self):
        pages = ip.expected()
        self.assertEqual(len(pages), 18)
        self.assertEqual(pages["https://www.wowhead.com/classic/spells/professions/alchemy"][0], "page_alchemy.html")
        self.assertEqual(pages["https://www.wowhead.com/classic/items/recipes/first-aid"][0], "recipeitems_first-aid.html")

    def test_import_by_canonical_and_by_name_then_report_missing(self):
        (self.src / "x.html").write_text('<link rel="canonical" href="https://www.wowhead.com/classic/spells/professions/alchemy"/>'
                                         "var listviewspells = [];", encoding="utf-8")
        (self.src / "recipeitems_alchemy.html").write_text("var listviewitems = [];", encoding="utf-8")
        (self.src / "junk.html").write_text("hello", encoding="utf-8")
        (self.src / "page_mining.html").write_text("<html>Just a moment... challenge</html>", encoding="utf-8")
        done, problems = ip.import_dir(self.src)
        self.assertEqual(len(done), 2)
        self.assertTrue(any("junk.html" in p for p in problems))
        self.assertTrue(any("page_mining.html" in p and "no embedded list" in p for p in problems))
        self.assertEqual(len([p for p in problems if p.startswith("missing")]), 16)
        # The generators read these through the normal cache path, as a 200 answer.
        text = http.fetch_text("https://www.wowhead.com/classic/spells/professions/alchemy", "page_alchemy.html")
        self.assertIn("listviewspells", text)
        _, meta = http._paths("https://www.wowhead.com/classic/spells/professions/alchemy", "page_alchemy.html")
        self.assertTrue(json.loads(meta.read_text())["imported"])

    def test_main_list_and_exit_codes(self):
        self.assertEqual(ip.main(["--list"]), 0)
        self.assertEqual(ip.main([str(self.src)]), 1)


if __name__ == "__main__":
    unittest.main()
