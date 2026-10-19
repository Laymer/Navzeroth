import json
import hashlib
import os
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1"
from navzeroth_datagen import envelope, http, sqldump  # noqa: E402


class SqlDumpTest(unittest.TestCase):
    SQL = ("CREATE TABLE `t` (\n  `a` int,\n  `b` varchar(10),\n  `c` float\n)\n"
           "INSERT INTO `t` VALUES (1,'x\\'y',1.5),(2,NULL,-3),(3,'NULL','it''s\\n');\n")

    def test_rows_keep_null_and_escapes(self):
        rows = sqldump.Dump(self.SQL).rows("t")
        self.assertEqual(rows[0], {"a": 1, "b": "x'y", "c": 1.5})
        self.assertIsNone(rows[1]["b"])
        self.assertEqual(rows[1]["c"], -3)
        self.assertEqual(rows[2]["b"], "NULL")
        self.assertEqual(rows[2]["c"], "it's\n")

    def test_unknown_table(self):
        with self.assertRaises(KeyError):
            sqldump.Dump(self.SQL).rows("missing")


class EnvelopeTest(unittest.TestCase):
    def test_shards_are_deterministic_and_bounded(self):
        records = [{"id": i, "name": f"N{i}", "subname": None, "elite": False} for i in range(50)]
        with tempfile.TemporaryDirectory() as out:
            kw = dict(origin=envelope.origin(envelope.VANILLA, "1.12.1.5875"),
                      sources=[envelope.source("s", "https://example.org")],
                      generator=envelope.generator("test"), out=out, sort_key=lambda r: -r["id"], max_bytes=1200)
            paths = envelope.write("world.npcs", "npc", records, **kw)
            first = [p.read_bytes() for p in paths]
            self.assertGreater(len(paths), 1)
            again = envelope.write("world.npcs", "npc", records, **kw)
            self.assertEqual(first, [p.read_bytes() for p in again])
            doc = json.loads(first[0])
            self.assertEqual(doc["confidence"], "cross_edition_unverified")
            self.assertEqual(doc["shard"], {"index": 0, "count": len(paths)})
            self.assertEqual(doc["records"][0]["id"], 49)
            self.assertNotIn("subname", doc["records"][0])
            self.assertIs(doc["records"][0]["elite"], False)
            self.assertEqual(doc["target"]["build"], {"status": "unknown"})

    def test_empty_is_refused(self):
        with self.assertRaises(ValueError):
            envelope.write("a.b", "npc", [], origin=envelope.origin("x"), sources=[], generator={})


class HttpTest(unittest.TestCase):
    def test_offline_cache_hit_and_failures(self):
        with tempfile.TemporaryDirectory() as cache:
            http.configure(cache=cache, offline_only=True)
            with self.assertRaises(http.FetchError):
                http.fetch("https://example.org/missing")
            body, meta = http._paths("https://example.org/a", None)
            body.parent.mkdir(parents=True, exist_ok=True)
            body.write_bytes(b"ok")
            meta.write_text(json.dumps({"url": "https://example.org/a", "status": 200, "sha256": hashlib.sha256(b"ok").hexdigest(), "bytes": 2}))
            self.assertEqual(http.fetch_text("https://example.org/a"), "ok")
            _, meta404 = http._paths("https://example.org/b", "b")
            meta404.write_text(json.dumps({"url": "https://example.org/b", "status": 403}))
            with self.assertRaises(http.FetchError) as e:
                http.fetch("https://example.org/b", "b")
            self.assertEqual(e.exception.status, 403)
            self.assertEqual(len(http.snapshot_digest("https://example.org/")), 64)


if __name__ == "__main__":
    unittest.main()


class HardeningTest(unittest.TestCase):
    def test_cache_names_are_sanitised(self):
        body, meta = http._paths("https://example.org/x", "../../evil name.gz")
        self.assertEqual(body.parent, http.cache_root() / "example.org")
        self.assertEqual(body.name, "evil_name.gz")
        body, _ = http._paths("https://example.org/x", "..")
        self.assertEqual(len(body.name), 64)  # falls back to the URL hash

    def test_transient_statuses_are_not_cached(self):
        self.assertTrue({202, 403, 429, 503} <= http.NOT_CACHED)
        self.assertNotIn(404, http.NOT_CACHED)
        self.assertNotIn(400, http.NOT_CACHED)  # wago: table absent from this build

    def test_insert_column_list_wins(self):
        sql = ("CREATE TABLE `t` (\n  `a` int,\n  `b` int\n)\n"
               "INSERT INTO `t` (`b`, `a`) VALUES (1,2);\nINSERT INTO `t` VALUES (3,4);\n")
        self.assertEqual(sqldump.Dump(sql).rows("t"), [{"b": 1, "a": 2}, {"a": 3, "b": 4}])
