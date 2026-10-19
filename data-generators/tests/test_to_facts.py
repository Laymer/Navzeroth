import json
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from navzeroth_datagen import envelope, to_facts  # noqa: E402


class ToFactsTest(unittest.TestCase):
    def test_records_become_facts(self):
        quest = {"kind": "quest", "id": 176, "title": "Wanted", "reward_money": 0, "givers": [{"entity": "npc", "id": 1}]}
        f = to_facts.facts_for(quest, "cross_edition_unverified")
        self.assertEqual([x["field"] for x in f], ["givers", "reward_money", "title"])
        self.assertEqual(f[1]["datum"], {"status": "known", "value": 0})  # zero stays zero
        self.assertEqual(f[0]["record"], "quest:176")
        spawn = {"kind": "spawn", "guid": 9, "entity": "npc", "entity_id": 448, "position": {"status": "unsupported"}}
        pos = [x for x in to_facts.facts_for(spawn, "c") if x["field"] == "position"][0]
        self.assertEqual(pos["datum"], {"status": "unsupported"})
        self.assertEqual(to_facts.facts_for({"kind": "fishing_area", "area_id": 1}, "c"), [])
        loot = {"kind": "loot", "table": "creature", "entry": 1, "reference": 5, "chance": 1.0}
        self.assertEqual(to_facts.facts_for(loot, "c")[0]["record"], "loot:creature:1:ref5:group:0")

    def test_convert_groups_by_generator_and_origin(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "out"
            kw = dict(origin=envelope.origin(envelope.VANILLA, "1.12.1"), sources=[envelope.source("s", "https://x.org")],
                      generator=envelope.generator("gen_x"), out=out)
            envelope.write("world.npcs", "npc", [{"id": 1, "name": "A"}, {"id": 2, "name": "B"}], **kw)
            envelope.write("world.more_npcs", "npc", [{"id": 1, "name": "Other"}], **kw)
            summary = to_facts.convert(out, out / "facts")
            self.assertEqual(summary["documents"], ["gen_x/vanilla_1_12/1.12.1/part-0000.json"])
            self.assertEqual(summary["conflicts"], ["gen_x:geography:npc:1:name"])
            doc = json.loads((out / "facts" / summary["documents"][0]).read_text())
            self.assertEqual(doc["schema_version"], 1)
            self.assertEqual(len(doc["facts"]), 2)
            self.assertIsNone(doc["facts"][0]["observed_at_utc_ms"])


if __name__ == "__main__":
    unittest.main()
