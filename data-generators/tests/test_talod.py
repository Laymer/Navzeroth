"""Offline tests of the adapted TALOD generators (small inline fixtures, no network)."""
import contextlib
import io
import json
import hashlib
import os
import pathlib
import sys
import tempfile
import unittest

DATAGEN = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(DATAGEN))
sys.path.insert(0, str(DATAGEN / "TALOD-data-generators" / "tools"))
os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1"
from navzeroth_datagen import envelope, http, sqldump  # noqa: E402
import gen_enhancements as ge  # noqa: E402
import gen_fishing as gf  # noqa: E402
import gen_professions as gp  # noqa: E402

ENCHANTING = (
    'var listviewspells = [{id:7414,name:"Enchanting",rank:"Apprentice",learnedat:9999,skill:[333],trainingcost:10},'
    '{id:7418,name:"Enchant Bracer - Minor Health",learnedat:9999,colors:[0,1,30,50],reagents:[[10940,1]],'
    'skill:[333],source:[6],trainingcost:10},'
    '{id:7421,name:"Runed Copper Rod",learnedat:1,colors:[1,5,7,10],reagents:[[6217,1],[10940,1]],'
    'creates:[6218,1,1],skill:[333],source:[6]},'
    '{id:41000,name:"Season of Discovery rune",learnedat:1,colors:[1,5,7,10],reagents:[[10940,1]],skill:[333]}];\n'
    'WH.Gatherer.addData(3, 4, {"10940":{"name_enus":"Strange Dust","quality":1,"icon":"inv_enchant_dust",'
    '"jsonequip":{"sellprice":5,"avgbuyout":120}},"6218":{"name_enus":"Runed Copper Rod","quality":1}});\n'
    'WH.Gatherer.addData(6, 4, {"7418":{"description_enus":"Permanently enchant bracers to increase health by 5."}});\n')
BLACKSMITHING = (
    'var listviewspells = [{id:2660,name:"Rough Sharpening Stone",learnedat:1,colors:[1,15,35,55],'
    'reagents:[[2835,1]],creates:[2862,0,0],skill:[164],source:[6]},'
    '{id:2661,name:"Copper Chain Belt",learnedat:35,colors:[35,75,95,115],reagents:[[2840,6]],'
    'creates:[2851,2,4],skill:[164],source:[]}];\n'
    'WH.Gatherer.addData(3, 4, {"2835":{"name_enus":"Rough Stone","quality":1},'
    '"2862":{"name_enus":"Rough Sharpening Stone","quality":1},"2840":{"name_enus":"Copper Bar","quality":1}});\n')
EMPTY = "var listviewspells = [];\n"
TOOLTIPS = {
    ("spell", 7418): '<table><tr><td><b>Enchant Bracer - Minor Health</b></td></tr></table>'
                     '<div class="q">Permanently enchant bracers to increase health by 5.</div>',
    ("spell", 7421): 'Tools: <a href="/classic/item=6217">Copper Rod</a><br>Reagents: x',
    ("spell", 2660): 'Tools: <a href="/classic/item=5956">Blacksmith Hammer</a><br>Reagents: x',
    ("spell", 2661): 'Tools: <a href="/classic/item=5956">Blacksmith Hammer</a><br>Reagents: x',
    ("item", 2862): '<span>Use: Increase sharp weapon damage by 2 for 30 minutes.</span>Requires Level <!--rlvl-->5',
}


def cache(url, name, body, status=200):
    """Put one download into the (temporary) cache, as http.py stores it."""
    body_path, meta_path = http._paths(url, name)
    body_path.parent.mkdir(parents=True, exist_ok=True)
    data = body.encode()
    body_path.write_bytes(data)
    meta = {"url": url, "status": status, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    meta_path.write_text(json.dumps(meta), encoding="utf-8")


def wowhead_cache():
    pages = {"professions/enchanting": ENCHANTING, "professions/blacksmithing": BLACKSMITHING}
    for slug in gp.PROFESSIONS:
        name = slug.split("/")[-1]
        cache(gp.BASE + slug, f"page_{name}.html", pages.get(slug, EMPTY))
        items = "<html></html>"
        if name == "enchanting":
            items = ('var listviewitems = [{id:6342,name:"Formula: Runed Copper Rod",source:[5]}];\n'
                     'WH.Gatherer.addData(3, 4, {"6342":{"jsonequip":{"buyprice":100}}});\n')
        cache(gp.ITEMS_BASE + name, f"recipeitems_{name}.html", items)
    for (kind, i), tip in TOOLTIPS.items():
        cache(ge.TOOLTIP.format(kind=kind, id=i), f"{kind}_{i}.json", json.dumps({"tooltip": tip}))


def records(out, dataset):
    folder = envelope.dataset_dir(dataset, envelope.origin(envelope.CLASSIC_ERA), out)
    docs = [json.loads(p.read_text()) for p in sorted(folder.glob("part-*.json"))]
    return docs[0], [r for d in docs for r in d["records"]]


class HelperTest(unittest.TestCase):
    def test_parse_page(self):
        spells, items, descs = ge.parse_page(ENCHANTING)
        self.assertEqual([s["id"] for s in spells], [7414, 7418, 7421, 41000])
        self.assertEqual(spells[1]["reagents"], [[10940, 1]])
        self.assertEqual(items[10940]["name_enus"], "Strange Dust")
        self.assertIn("health by 5", descs[7418])
        with self.assertRaises(ValueError):
            ge.parse_page("<html>challenge</html>")

    def test_makes_of(self):
        for mod in (ge, gp):
            self.assertEqual(mod.makes_of([1, 0, 0]), 1)
            self.assertEqual(mod.makes_of([1]), 1)
            self.assertEqual(mod.makes_of([1, 2, 4]), 3)
            self.assertEqual(mod.makes_of([1, 1, 2]), 1.5)

    def test_slots(self):
        self.assertEqual(ge.slots_for_enchant("Enchant Bracer - Minor Health"), ([9], None))
        self.assertEqual(ge.slots_for_enchant("Enchant 2H Weapon - Agility"), ([16], "twohand"))
        self.assertEqual(ge.slots_for_enchant("Enchant Shield - Stamina"), ([17], "shield"))
        self.assertEqual(ge.slots_for_enchant(None), (None, None))
        self.assertEqual(ge.slots_for_item("stone", "Increase sharp weapon damage"), ([16, 17], "bladed"))
        self.assertEqual(ge.slots_for_item("stone", "Increase blunt weapon damage"), ([16, 17], "blunt"))
        self.assertEqual(ge.slots_for_item("scope", ""), ([18], "bowgun"))
        self.assertEqual(ge.slots_for_item("kit", ""), ([5, 7, 10, 8], None))


class MappingTest(unittest.TestCase):
    def test_recipe_record(self):
        r = {"name": "Runed Copper Rod", "prof": "Enchanting", "skill": 1, "colors": [1, 5, 7, 10],
             "reagents": [[6217, 1], [10940, 1]], "src": ["pattern", "vendor"], "creates": 6218, "makes": 1,
             "pattern": 6342, "patternPrice": 100, "tools": [], "guess": True}
        rec = envelope.clean(ge.recipe_record(7421, r))
        self.assertEqual(rec["spell_id"], 7421)
        self.assertEqual(rec["skill_line"], 333)
        self.assertEqual(rec["required_skill"], 1)
        self.assertEqual(rec["reagents"], [{"item": 6217, "count": 1}, {"item": 10940, "count": 1}])
        self.assertEqual(rec["makes"], 1.0)
        self.assertIsInstance(rec["makes"], float)
        self.assertEqual(rec["sources"], ["pattern", "vendor"])
        self.assertEqual((rec["pattern_item"], rec["pattern_price"]), (6342, 100))
        self.assertIs(rec["source_inferred"], True)
        self.assertIs(rec["cooldown"], False)
        for absent in ("tools", "skill_up", "trainer_cost", "specialization"):
            self.assertNotIn(absent, rec)

    def test_item_record(self):
        vendor = envelope.clean(ge.item_record(2320, {"name": "Coarse Thread", "q": 1, "vendor": None}))
        self.assertIs(vendor["vendor_sold"], True)
        self.assertNotIn("buy_price", vendor)  # sold, price unknown: not zero
        made = envelope.clean(ge.item_record(2862, {"name": "Rough Sharpening Stone", "q": None, "made": [2660],
                                                    "ah": 40, "sell": 2, "icon": "x"}))
        self.assertEqual(made["made_by"], [2660])
        self.assertEqual((made["auction_average"], made["sell_price"]), (40, 2))
        self.assertNotIn("vendor_sold", made)  # never false without a source saying so
        self.assertNotIn("quality", made)
        hint = envelope.clean(ge.item_record(4470, {"name": "Simple Wood", "q": 1, "get": "Vendor"}))
        self.assertIs(hint["vendor_sold"], True)
        self.assertEqual(hint["obtain_hint"], "Vendor")

    def test_enhancement_record(self):
        e = {"type": "item", "item": 2862, "spell": 2660, "slots": [16, 17], "kind": "bladed", "effect": "x",
             "name": "Rough Sharpening Stone", "temporary": True, "level": 5, "minItemLevel": 10}
        rec = envelope.clean(ge.enhancement_record(e))
        self.assertEqual(rec, {"enhancement_type": "item", "spell_id": 2660, "item_id": 2862, "slots": [16, 17],
                               "weapon_kind": "bladed", "effect": "x", "name": "Rough Sharpening Stone",
                               "temporary": True, "required_level": 5, "min_item_level": 10})

    def test_rank_record(self):
        known = gp.rank_record("Cooking", gp.rank_entry("Cooking", "Expert", 125, 10000))
        self.assertEqual(known["cost"], {"status": "known", "value": 10000})
        self.assertIsNone(known["character_level"])  # secondary skill
        self.assertIn("Expert Cookbook", known["note"])
        guessed = gp.rank_record("Alchemy", gp.rank_entry("Alchemy", "Artisan", 200, None))
        self.assertEqual(guessed["cost"], {"status": "unknown"})
        self.assertEqual((guessed["max_skill"], guessed["learn_at_skill"], guessed["character_level"]), (300, 200, 35))
        zero = gp.rank_record("Alchemy", gp.rank_entry("Alchemy", "Apprentice", 1, 0))
        self.assertEqual(zero["cost"], {"status": "known", "value": 0})


DUMP = """CREATE TABLE `skill_fishing_base_level` (
  `entry` int,
  `skill` int
)
INSERT INTO `skill_fishing_base_level` VALUES (12,-70),(40,55),(9999,100);
CREATE TABLE `fishing_loot_template` (
  `entry` int,
  `item` int,
  `mincountOrRef` int
)
INSERT INTO `fishing_loot_template` VALUES (40,6522,1),(12,0,-500);
CREATE TABLE `reference_loot_template` (
  `entry` int,
  `item` int,
  `mincountOrRef` int
)
INSERT INTO `reference_loot_template` VALUES (500,6522,1);
CREATE TABLE `item_template` (
  `entry` int,
  `name` varchar(64),
  `Quality` int
)
INSERT INTO `item_template` VALUES (6522,'Deviate Fish',2),(6307,'Message in a Bottle',1),(1,'Reward',3);
CREATE TABLE `quest_template` (
  `entry` int,
  `Title` text,
  `ReqItemId1` int,
  `ReqItemId2` int,
  `ReqItemId3` int,
  `ReqItemId4` int,
  `ReqItemCount1` int,
  `ReqItemCount2` int,
  `ReqItemCount3` int,
  `ReqItemCount4` int,
  `RewChoiceItemId1` int,
  `RewItemId1` int
)
INSERT INTO `quest_template` VALUES (100,'Deviate Fish Hunt',6522,0,0,0,5,0,0,0,1,NULL);
"""
AREAS = {12: {"ID": "12", "AreaName_lang": "Elwynn Forest", "ParentAreaID": "0"},
         40: {"ID": "40", "AreaName_lang": "Westfall", "ParentAreaID": "0"},
         41: {"ID": "41", "AreaName_lang": "Longshore", "ParentAreaID": "40"}}


class FishingTest(unittest.TestCase):
    def test_area_rows(self):
        dump = sqldump.Dump(DUMP)
        base = {int(r["entry"]): int(r["skill"]) for r in dump.rows("skill_fishing_base_level")}
        with contextlib.redirect_stdout(io.StringIO()) as printed:
            rows = gf.area_rows(base, AREAS)
        self.assertIn("skip area 9999", printed.getvalue())
        self.assertEqual(rows, [{"area_id": 12, "name": "Elwynn Forest", "parent_name": None, "base_skill": -70},
                                {"area_id": 40, "name": "Westfall", "parent_name": None, "base_skill": 55}])
        sub = gf.area_rows({41: 55}, AREAS)
        self.assertEqual(sub[0]["parent_name"], "Westfall")

    def test_goal_rows(self):
        dump = sqldump.Dump(DUMP)
        goals = [(6522, "rare", "Deviate Fish"), (6307, "rare", "Message in a Bottle")]
        old_goals, old_timed = gf.GOALS, gf.TIMED
        gf.GOALS, gf.TIMED = goals, [{"id": 6522}]
        try:
            rows = gf.goal_rows(dump, AREAS)
            gf.GOALS = [(6522, "rare", "Wrong Name")]
            with self.assertRaises(SystemExit):
                gf.goal_rows(dump, AREAS)
        finally:
            gf.GOALS, gf.TIMED = old_goals, old_timed
        deviate = rows[0]
        self.assertEqual(deviate["item_id"], 6522)
        self.assertEqual(deviate["quality"], 2)
        self.assertEqual(deviate["places"], [["Elwynn Forest"], ["Westfall"]])  # direct and through a reference
        self.assertEqual(deviate["quests"], [{"id": 100, "title": "Deviate Fish Hunt", "count": 5,
                                              "rewards": ["Reward"]}])
        self.assertNotIn("places", rows[1])

    def test_hand_lists(self):
        timed = [envelope.clean(t) for t in gf.timed_rows()]
        self.assertEqual(timed[0]["best_hours"], [0, 6])
        self.assertEqual(timed[2]["season"], [923, 319])
        self.assertNotIn("hours", timed[2])
        event = gf.event_rows()[0]
        self.assertEqual((event["name"], event["weekday"], event["start_minute"], event["length_minutes"]),
                         ("Stranglethorn Fishing Extravaganza", 1, 840, 120))


class EndToEndTest(unittest.TestCase):
    """build() from a synthetic offline cache, as run_all.py would run it."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        http.configure(cache=self.root / "cache", offline_only=True)
        wowhead_cache()

    def tearDown(self):
        self.tmp.cleanup()

    def test_enhancements(self):
        out = self.root / "out"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(ge.main(["--offline", "--out", str(out)]), 0)
        head, enh = records(out, "enhancements.enhancements")
        self.assertEqual(head["origin"], {"edition": "wow_classic_era", "build": {"status": "unknown"},
                                          "locale": "enUS"})
        self.assertEqual(head["confidence"], "cross_edition_unverified")
        self.assertEqual([s["id"] for s in head["sources"]], ["wowhead-classic", "wowhead-classic-tooltips"])
        self.assertEqual(head["generator"]["credit"], envelope.CREDIT)
        by_spell = {e["spell_id"]: e for e in enh}
        self.assertEqual(by_spell[7418]["slots"], [9])
        self.assertIn("health by 5", by_spell[7418]["effect"])
        self.assertEqual((by_spell[2660]["weapon_kind"], by_spell[2660]["required_level"]), ("bladed", 5))
        self.assertIs(by_spell[2660]["temporary"], True)
        _, recipes = records(out, "enhancements.recipes")
        self.assertEqual({r["spell_id"] for r in recipes}, {7418, 2660})
        _, items = records(out, "enhancements.items")
        self.assertIn(5956, {i["id"] for i in items})  # a tool from the tooltip
        written = json.loads((out / ".written" / "gen_enhancements.json").read_text())
        self.assertEqual(len(written["files"]), 3)
        self.assertTrue(all(f.startswith("enhancements/") for f in written["files"]))

    def test_professions(self):
        out = self.root / "out"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(gp.main(["--offline", "--out", str(out)]), 0)
        first = sorted(p.read_bytes() for p in out.rglob("part-*.json"))
        _, ranks = records(out, "professions.ranks")
        self.assertEqual(len(ranks), 4 * len(gp.PROFESSIONS))
        apprentice = next(r for r in ranks if r["profession"] == "Enchanting" and r["rank"] == "Apprentice")
        self.assertEqual(apprentice["cost"], {"status": "known", "value": 10})
        artisan = next(r for r in ranks if r["profession"] == "Enchanting" and r["rank"] == "Artisan")
        self.assertEqual(artisan["cost"], {"status": "unknown"})
        _, notes = records(out, "professions.notes")
        self.assertEqual({n["profession"] for n in notes}, set(gp.GATHERING))
        _, recipes = records(out, "professions.recipes")
        by_id = {r["spell_id"]: r for r in recipes}
        self.assertNotIn(41000, by_id)  # above MAX_CLASSIC_SPELL
        self.assertEqual(by_id[7418]["sources"], ["trainer"])
        self.assertEqual(by_id[7418]["trainer_cost"], 10)
        self.assertEqual(by_id[7418]["required_skill"], 1)  # FROM_START
        self.assertEqual(by_id[7421]["sources"], ["trainer"])  # trainer wins over the formula
        self.assertEqual(by_id[2661]["sources"], ["trainer"])
        self.assertIs(by_id[2661]["source_inferred"], True)
        self.assertEqual(by_id[2661]["makes"], 3.0)
        self.assertEqual(by_id[2661]["tools"], [5956])
        _, items = records(out, "professions.items")
        dust = next(i for i in items if i["id"] == 10940)
        self.assertEqual((dust["auction_average"], dust["sell_price"], dust["icon"]), (120, 5, "inv_enchant_dust"))
        rod = next(i for i in items if i["id"] == 6217)
        self.assertIs(rod["vendor_sold"], True)
        self.assertEqual(rod["name"], "Copper Rod")
        with contextlib.redirect_stdout(io.StringIO()):
            gp.main(["--offline", "--out", str(out)])
        self.assertEqual(first, sorted(p.read_bytes() for p in out.rglob("part-*.json")))

    def test_refused_page_fails(self):
        cache(gp.BASE + "professions/alchemy", "page_alchemy.html", "", status=202)
        out = self.root / "out"
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as err:
            self.assertEqual(gp.main(["--offline", "--out", str(out)]), 2)
        self.assertIn("202", err.getvalue())
        written = json.loads((out / ".written" / "gen_professions.json").read_text())
        self.assertEqual(written["files"], [])
        self.assertIn(202, [s["status"] for s in written["sources"]])


if __name__ == "__main__":
    unittest.main()
