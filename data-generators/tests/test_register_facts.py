import hashlib
import json
import pathlib
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import register_facts as rf  # noqa: E402

REPO = HERE.parent.parent
ADR6 = b"synthetic ADR 0006 section 14 evidence\n"
GPL = b"synthetic GPL licence text\n"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def evidence(url, data, scope):
    return {"url": url, "revision": "a" * 40, "sha256": sha(data), "reviewed_scope": scope}


def upstream(id, license, identifier, proof):
    return {"id": id, "revision": "b" * 40, "publisher": id, "origin": {"edition": None, "build": None},
            "rights": {"status": "admitted", "license": license, "license_identifier": identifier,
                       "reviewer": f"reviewer of {id}", "attribution": f"credit {id}",
                       "obligations": f"obligations of {id}.", "evidence": proof},
            "retrieval": {"method": "https_rate_limited", "max_total_bytes": 1024, "minimum_interval_ms": 1000,
                          "max_requests": 4, "access_evidence": proof[0]}, "files": []}


def fact(domain, record, field, value):
    return {"domain": domain, "record": record, "field": field, "datum": {"status": "known", "value": value},
            "confidence": "cross_edition_unverified", "observed_at_utc_ms": None, "clock_trust": "uncertain"}


class Workspace:
    def __init__(self, root):
        self.root = pathlib.Path(root)
        self.facts = self.root / "facts"
        self.objects = self.root / "objects"
        self.objects.mkdir()
        self.registry = self.root / "registry.json"
        self.plan = self.root / "plan.json"
        self.updates = self.root / "updates.json"
        self.notice = self.root / "NOTICE.md"
        self.adr = self.root / "adr.md"
        self.attributes = self.root / "attributes"
        self.world = self.root / "world"
        adr6 = evidence("https://github.com/Laymer/Navzeroth/blob/main/docs/decisions/0006-v1-program.md", ADR6,
                        "Owner acceptance; ADR 0006 §14.")
        gpl = evidence("https://raw.githubusercontent.com/cmangos/classic-db/x/LICENSE.md", GPL, "GPL text.")
        (self.objects / adr6["sha256"]).write_bytes(ADR6)
        (self.objects / gpl["sha256"]).write_bytes(GPL)
        hand = {"id": "hand-source", "revision": "c" * 40, "publisher": "hand", "origin": {"edition": None, "build": None},
                "rights": {"status": "unreviewed", "license": "unknown", "license_identifier": None, "reviewer": None,
                           "attribution": None, "obligations": None, "evidence": []},
                "retrieval": {"method": "manual_local", "max_total_bytes": 1, "minimum_interval_ms": 1000,
                              "max_requests": 1, "access_evidence": None}, "files": []}
        registry = {"schema_version": 1, "sources": [
            hand,
            upstream("cmangos-classic-db", "share_alike", "GPL-3.0-only", [gpl, adr6]),
            upstream("wago-tools-wowf", "owner_accepted", "LicenseRef-wago", [adr6]),
        ]}
        self.registry.write_text(rf.dumps(registry), encoding="utf-8")
        plan = {"schema_version": 1, "target": {"edition": "wow_forever", "build": "1.60.1.70374"},
                "as_of_utc_ms": 1, "inputs": [{"source": "hand-source", "file": "a.lua", "adapter": "atlasloot_areas_v1"}]}
        self.plan.write_text(rf.dumps(plan), encoding="utf-8")
        self.updates.write_text(rf.dumps({"schema_version": 1, "registry_sha256": "0" * 64}), encoding="utf-8")
        self.notice.write_text("# Retained material\n\nHand-written notice.\n", encoding="utf-8")
        self.adr.write_bytes(b"# ADR 0007\r\n\r\nSection 7: register every fact.\r\n")
        self.attributes.write_text("content/source-materials/objects/* -text\n", encoding="utf-8")

    def doc(self, rel, facts):
        path = self.facts / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        data = ('{"facts":[\n' + ",\n".join(json.dumps(f, sort_keys=True) for f in facts)
                + '\n],"schema_version":1}\n').encode()
        path.write_bytes(data)
        return data

    def shard(self, rel, generator, edition, build, ids):
        path = self.world / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"generator": {"name": generator}, "sources": [{"id": i} for i in ids],
                                    "origin": {"edition": edition, "build": {"status": "known", "value": build}}}))

    def run(self, world=True):
        return rf.register(self.facts, self.registry, self.plan, self.objects, self.updates, self.notice,
                           world=self.world if world else None, adr=self.adr, attributes=self.attributes)

    def snapshot(self):
        files = [self.registry, self.plan, self.updates, self.notice, self.attributes]
        objects = sorted(p.name for p in self.objects.iterdir())
        return [p.read_bytes() for p in files], objects

    def load(self, path):
        return json.loads(path.read_text(encoding="utf-8"))


class RegisterFactsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.ws = Workspace(self.tmp.name)
        self.cm0 = self.ws.doc("gen_world_cmangos/vanilla_1_12/1.12.1/part-0000.json",
                               [fact("quests", "quest:1", "title", "A"), fact("geography", "npc:1", "name", "B")])
        self.cm1 = self.ws.doc("gen_world_cmangos/vanilla_1_12/1.12.1/part-0001.json",
                               [fact("quests", "quest:2", "min_level", 0)])
        self.db = self.ws.doc("gen_world_db2/wow_forever/unknown-build/part-0000.json",
                              [fact("equipment", "item:1", "name", "Sword")])

    def tearDown(self):
        self.tmp.cleanup()

    def test_committed_json_files_round_trip_byte_identically(self):
        for rel in ["docs/sources/source-admission-v1.json", "content/source-build-v1.json",
                    "docs/sources/source-updates-v1.json"]:
            data = (REPO / rel).read_bytes()
            self.assertEqual(rf.dumps(json.loads(data)).encode("utf-8"), data, rel)

    def test_upserts_sources_inputs_objects_and_hashes(self):
        summary = self.ws.run(world=False)
        reg = self.ws.load(self.ws.registry)
        ids = [s["id"] for s in reg["sources"]]
        self.assertEqual(ids[:3], ["hand-source", "cmangos-classic-db", "wago-tools-wowf"])
        self.assertEqual(ids[3:], ["gen-world-cmangos-vanilla-1-12-1.12.1", "gen-world-db2-wow-forever-unknown-build"])
        self.assertEqual(summary["documents"], 3)
        cm = reg["sources"][3]
        self.assertEqual(cm["origin"], {"edition": "vanilla_1_12", "build": "1.12.1"})
        self.assertEqual(reg["sources"][4]["origin"], {"edition": "wow_forever", "build": None})
        self.assertEqual([f["path"] for f in cm["files"]], ["gen_world_cmangos/vanilla_1_12/1.12.1/part-0000.json",
                                                            "gen_world_cmangos/vanilla_1_12/1.12.1/part-0001.json"])
        first = cm["files"][0]
        self.assertEqual(first["sha256"], sha(self.cm0))
        self.assertEqual(first["bytes"], len(self.cm0))
        self.assertEqual(first["fields"], ["geography.name", "quests.title"])
        self.assertEqual(cm["files"][1]["fields"], ["quests.min_level"])
        self.assertEqual(first["transformation"], "knowledge_v1")
        self.assertTrue(first["url"].endswith("/content/source-materials/objects/" + sha(self.cm0)))
        self.assertEqual((self.ws.objects / sha(self.cm0)).read_bytes(), self.cm0)
        lines = "".join(f"{f['path']} {f['sha256']}\n" for f in cm["files"])
        self.assertEqual(cm["revision"], sha(lines.encode()))
        # Rights are the upstream's; ADR 0007 (LF-normalized) is the access evidence.
        self.assertEqual(cm["rights"]["license"], "share_alike")
        self.assertEqual(cm["rights"]["license_identifier"], "GPL-3.0-only")
        self.assertEqual(len(cm["rights"]["evidence"]), 2)
        access = cm["retrieval"]["access_evidence"]
        adr = self.ws.adr.read_bytes().replace(b"\r\n", b"\n")
        self.assertEqual(access["sha256"], sha(adr))
        self.assertEqual(access["revision"], hashlib.sha1(b"blob %d\0" % len(adr) + adr).hexdigest())
        self.assertEqual((self.ws.objects / sha(adr)).read_bytes(), adr)
        closure = len(self.cm0) + len(self.cm1) + len(GPL) + len(ADR6) + len(adr)
        self.assertEqual(cm["retrieval"]["method"], "manual_local")
        self.assertEqual(cm["retrieval"]["max_total_bytes"], closure)
        self.assertEqual(cm["retrieval"]["max_requests"], 5)
        plan = self.ws.load(self.ws.plan)
        self.assertEqual(plan["inputs"][0]["source"], "hand-source")
        self.assertEqual([(i["source"], i["adapter"]) for i in plan["inputs"][1:]],
                         [("gen-world-cmangos-vanilla-1-12-1.12.1", "knowledge_v1")] * 2
                         + [("gen-world-db2-wow-forever-unknown-build", "knowledge_v1")])
        self.assertEqual(self.ws.load(self.ws.updates)["registry_sha256"], sha(self.ws.registry.read_bytes()))
        notice = self.ws.notice.read_text(encoding="utf-8")
        self.assertTrue(notice.startswith("# Retained material\n\nHand-written notice.\n\n" + rf.NOTICE_BEGIN))
        self.assertIn(f"[`{sha(self.cm0)}`](objects/{sha(self.cm0)})", notice)
        attributes = self.ws.attributes.read_text(encoding="utf-8")
        self.assertIn(rf.ATTR_LINE.format(sha(self.db)), attributes)
        self.assertNotIn(sha(adr), attributes)
        self.assertNotIn(sha(GPL), attributes)

    def test_rerun_is_byte_identical(self):
        self.ws.run()
        before = self.ws.snapshot()
        self.ws.run()
        self.assertEqual(self.ws.snapshot(), before)

    def test_stale_sources_inputs_and_objects_are_removed(self):
        self.ws.run()
        (self.ws.facts / "gen_world_db2/wow_forever/unknown-build/part-0000.json").unlink()
        (self.ws.facts / "gen_world_cmangos/vanilla_1_12/1.12.1/part-0001.json").unlink()
        summary = self.ws.run()
        self.assertEqual(sorted(summary["removed_objects"]), sorted([sha(self.db), sha(self.cm1)]))
        reg = self.ws.load(self.ws.registry)
        self.assertEqual([s["id"] for s in reg["sources"]][3:], ["gen-world-cmangos-vanilla-1-12-1.12.1"])
        self.assertEqual(len(self.ws.load(self.ws.plan)["inputs"]), 2)
        self.assertFalse((self.ws.objects / sha(self.db)).exists())
        self.assertTrue((self.ws.objects / sha(self.cm0)).exists())
        self.assertTrue((self.ws.objects / sha(GPL)).exists())
        self.assertNotIn(sha(self.db), self.ws.attributes.read_text(encoding="utf-8"))
        # With no documents left every generated entry goes, hand entries stay.
        for p in self.ws.facts.rglob("part-*.json"):
            p.unlink()
        self.ws.run()
        reg = self.ws.load(self.ws.registry)
        self.assertEqual(len(reg["sources"]), 3)
        self.assertIn("No generated fact documents are registered.", self.ws.notice.read_text(encoding="utf-8"))
        self.assertEqual(sorted(p.name for p in self.ws.objects.iterdir()), sorted([sha(GPL), sha(ADR6),
                         sha(self.ws.adr.read_bytes().replace(b"\r\n", b"\n"))]))

    def test_world_citations_add_upstream_rights(self):
        self.ws.shard("world/npcs/vanilla_1_12/1.12.1/part-0000.json", "gen_world_cmangos", "vanilla_1_12", "1.12.1",
                      ["cmangos-classic-db", "wago-tools-wowf", "not-a-registry-id"])
        self.ws.run()
        cm = self.ws.load(self.ws.registry)["sources"][3]
        self.assertEqual(cm["rights"]["license"], "owner_accepted")
        self.assertEqual(cm["rights"]["license_identifier"], "GPL-3.0-only AND LicenseRef-wago")
        self.assertIn("cmangos-classic-db, wago-tools-wowf", cm["rights"]["reviewer"])
        self.assertEqual(len(cm["rights"]["evidence"]), 2)  # the shared ADR 0006 evidence counts once

    def test_existing_object_with_other_bytes_is_never_overwritten(self):
        (self.ws.objects / sha(self.cm0)).write_bytes(b"tampered")
        with self.assertRaises(rf.RegisterError):
            self.ws.run()
        self.assertEqual((self.ws.objects / sha(self.cm0)).read_bytes(), b"tampered")

    def test_refuses_unknown_generators_bad_paths_and_oversized_documents(self):
        self.ws.doc("gen_unknown/vanilla_1_12/1.12.1/part-0000.json", [fact("quests", "q", "f", 1)])
        with self.assertRaises(rf.RegisterError):
            self.ws.run()
        (self.ws.facts / "gen_unknown/vanilla_1_12/1.12.1/part-0000.json").unlink()
        self.ws.doc("gen_world_cmangos/Vanilla/1.12.1/part-0000.json", [fact("quests", "q", "f", 1)])
        with self.assertRaises(rf.RegisterError):
            self.ws.run()
        (self.ws.facts / "gen_world_cmangos/Vanilla/1.12.1/part-0000.json").unlink()
        big = self.ws.facts / "gen_world_cmangos/vanilla_1_12/1.12.1/part-0002.json"
        big.write_bytes(b" " * (rf.MAX_FILE_BYTES + 1))
        with self.assertRaises(rf.RegisterError):
            self.ws.run()
        big.unlink()
        self.assertEqual(rf.main(["--facts", str(self.ws.root / "missing"), "--registry", str(self.ws.registry),
                                  "--plan", str(self.ws.plan), "--objects", str(self.ws.objects),
                                  "--updates", str(self.ws.updates), "--notice", str(self.ws.notice)]), 1)

    def test_main_registers_and_reports(self):
        args = ["--facts", str(self.ws.facts), "--registry", str(self.ws.registry), "--plan", str(self.ws.plan),
                "--objects", str(self.ws.objects), "--updates", str(self.ws.updates), "--notice", str(self.ws.notice),
                "--world", str(self.ws.world), "--adr", str(self.ws.adr), "--attributes", str(self.ws.attributes)]
        self.assertEqual(rf.main(args), 0)
        self.assertEqual(len(self.ws.load(self.ws.registry)["sources"]), 5)

    def test_source_ids_are_sanitized(self):
        self.assertEqual(rf.source_id("gen_world_db2", "wow_classic_era", "unknown-build"),
                         "gen-world-db2-wow-classic-era-unknown-build")
        self.assertEqual(rf.source_id("Odd Name", "x", "1.2"), "gen-odd-name-x-1.2")


if __name__ == "__main__":
    unittest.main()
