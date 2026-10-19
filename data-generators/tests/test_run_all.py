"""run_all.py over fake generator scripts in a temporary folder (no network)."""
import contextlib
import hashlib
import io
import json
import os
import pathlib
import sys
import tempfile
import unittest

DATAGEN = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(DATAGEN))
os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1"
import run_all  # noqa: E402
from navzeroth_datagen import http  # noqa: E402

# A generator that writes one shard and, unless told otherwise, its .written report.
FAKE = """import json, pathlib, sys
sys.path.insert(0, {datagen!r})
from navzeroth_datagen import envelope
args = sys.argv[1:]
assert "--offline" in args, args
out = pathlib.Path(args[args.index("--out") + 1])
paths = envelope.write({dataset!r}, "profession_note", [{{"profession": "Fishing", "text": "fishing"}}],
                       origin=envelope.origin(envelope.CLASSIC_ERA),
                       sources=[envelope.source("s", "https://example.org")],
                       generator=envelope.generator({name!r}), out=out)
if {report!r}:
    (out / ".written").mkdir(exist_ok=True)
    (out / ".written" / "{name}.json").write_text(json.dumps({{
        "files": [p.relative_to(out).as_posix() for p in paths],
        "sources": [{{"url": "https://example.org/b", "status": 200, "sha256": "ab", "bytes": 2,
                     "retrieved_at_utc_ms": 1}},
                    {{"url": "https://example.org/a", "status": 403, "retrieved_at_utc_ms": 2}}]}}))
sys.exit({code})
"""


class RunAllTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        http.configure(cache=self.root / "cache")
        self.saved = run_all.GENERATORS
        run_all.GENERATORS = {
            "gen_a": (self.script("gen_a", "test.alpha", True, 0), ()),
            "gen_b": (self.script("gen_b", "test.beta", False, 0), ()),
            "gen_c": (self.script("gen_c", "test.gamma", True, 3), ()),
            "gen_missing": (self.root / "nope.py", ()),
        }

    def tearDown(self):
        run_all.GENERATORS = self.saved
        self.tmp.cleanup()

    def script(self, name, dataset, report, code):
        path = self.root / f"{name}.py"
        path.write_text(FAKE.format(datagen=str(DATAGEN), dataset=dataset, name=name, report=report, code=code))
        return path

    def run_all(self, out, *extra):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return run_all.main(["--offline", "--jobs", "3", "--out", str(out), *extra])

    def test_manifest(self):
        out = self.root / "out"
        self.assertEqual(self.run_all(out), 1)  # gen_c exits 3
        text = (out / "run-manifest.json").read_text()
        manifest = json.loads(text)
        self.assertEqual(manifest["format"], "navzeroth-world-data-run")
        self.assertEqual(manifest["schema_version"], 1)
        self.assertEqual([(g["name"], g["status"]) for g in manifest["generators"]],
                         [("gen_a", "ok"), ("gen_b", "ok"), ("gen_c", "failed"), ("gen_missing", "skipped")])
        a, b, c, _ = manifest["generators"]
        self.assertEqual([f["path"] for f in a["files"]], ["test/alpha/wow_classic_era/unknown-build/part-0000.json"])
        shard = (out / a["files"][0]["path"]).read_bytes()
        self.assertEqual(a["files"][0]["sha256"], hashlib.sha256(shard).hexdigest())
        self.assertEqual((a["files"][0]["bytes"], a["files"][0]["records"]), (len(shard), 1))
        # No report: found by the generator name in the shard header.
        self.assertEqual([f["path"] for f in b["files"]], ["test/beta/wow_classic_era/unknown-build/part-0000.json"])
        self.assertEqual(c["files"], [])  # a failed run's shards are not listed
        self.assertEqual(manifest["sources"], [{"url": "https://example.org/a", "status": 403},
                                               {"url": "https://example.org/b", "status": 200, "sha256": "ab",
                                                "bytes": 2}])
        self.assertNotIn("retrieved_at", text)
        self.assertFalse((out / ".written").exists())
        self.assertTrue((http.cache_root() / "logs" / "gen_a.log").exists())

        self.run_all(out)
        self.assertEqual((out / "run-manifest.json").read_text(), text)  # deterministic

    def test_only(self):
        out = self.root / "out"
        self.assertEqual(self.run_all(out, "--only", "gen_a"), 0)
        manifest = json.loads((out / "run-manifest.json").read_text())
        status = {g["name"]: g["status"] for g in manifest["generators"]}
        self.assertEqual(status, {"gen_a": "ok", "gen_b": "skipped", "gen_c": "skipped", "gen_missing": "skipped"})
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            run_all.main(["--only", "gen_unknown", "--out", str(out)])


if __name__ == "__main__":
    unittest.main()
