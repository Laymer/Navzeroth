import importlib.util
import json
import os
import pathlib
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1"
from navzeroth_datagen import db2, envelope  # noqa: E402

_spec = importlib.util.spec_from_file_location("gen_world_db2", ROOT / "Navzeroth-data-generators" / "gen_world_db2.py")
gen = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gen)

from test_coords import MAPS, ROWS  # noqa: E402  (Elwynn / Eastern Kingdoms UiMapAssignment rows)


def sparse(**kw):
    r = {"ID": "1077", "Display_lang": "Defias Mage Ring", "OverallQualityID": "2", "InventoryType": "11",
         "ItemLevel": "26", "RequiredLevel": "21", "AllowableClass": "-1", "AllowableRace_0": "-1",
         "AllowableRace_1": "-1", "Bonding": "2", "Stackable": "1", "BuyPrice": "100", "SellPrice": "25"}
    for i in range(10):
        r[f"StatModifier_bonusStat_{i}"], r[f"StatPercentEditor_{i}"] = "-1", "0"
    r.update(StatModifier_bonusStat_0="5", StatPercentEditor_0="8000",
             StatModifier_bonusStat_1="6", StatPercentEditor_1="6000")
    r.update({k: str(v) for k, v in kw.items()})
    return r


# RandPropPoints row 26 (wowf 1.60.1.70245): Good_2 = 5
RPP = {26: {"ID": "26", "Good_0": "11", "Good_1": "8", "Good_2": "5", "Good_3": "4", "Good_4": "3",
            "Superior_2": "7", "Epic_2": "9"}}


class ItemTest(unittest.TestCase):
    def test_item_mapping_and_stats(self):
        r = gen.item(sparse(), {"ClassID": "4", "SubclassID": "0"}, RPP)
        self.assertEqual(r["name"], "Defias Mage Ring")
        self.assertEqual((r["quality"], r["class_id"], r["subclass_id"], r["inventory_type"]), (2, 4, 0, 11))
        self.assertEqual((r["item_level"], r["required_level"], r["bonding"], r["max_stack"]), (26, 21, 2, 1))
        self.assertEqual((r["buy_price"], r["sell_price"], r["allowable_class"]), (100, 25, -1))
        self.assertEqual(r["allowable_race"], -1)
        self.assertEqual(r["stats"], {"intellect": 4, "spirit": 3})  # CMaNGOS item 1077: int 4, spi 3

    def test_quality_picks_budget_and_ratings_are_left_out(self):
        rare = gen.item(sparse(OverallQualityID=3, StatModifier_bonusStat_1=32), None, RPP)
        self.assertEqual(rare["stats"], {"intellect": 6})  # 7 * 0.8 = 5.6 -> 6; crit rating dropped
        self.assertNotIn("class_id", envelope.clean(rare))

    def test_unknown_stat_type_and_no_budget(self):
        r = gen.item(sparse(StatModifier_bonusStat_0=85), None, RPP)
        self.assertEqual(r["stats"], {"stat_85": 4, "spirit": 3})
        none = envelope.clean(gen.item(sparse(InventoryType=0), None, RPP))
        self.assertNotIn("stats", none)  # no budget: stats unknown, not zero

    def test_race_mask_combines_halves(self):
        self.assertEqual(gen.mask64(-1, -1), -1)
        self.assertEqual(gen.mask64(415, 0), 415)
        alliance = gen.mask64(-1321907123, 1427461461)
        self.assertEqual(alliance, 0x55155555B1354C4D)
        horde = gen.mask64(1309324210, -1440044374)
        self.assertEqual(horde & ((1 << 64) - 1), 0xAA2AAAAA4E0AB3B2)
        self.assertLess(horde, 0)
        self.assertIsNone(gen.mask64(None, 0))
        r = gen.item(sparse(AllowableRace_0=1101, AllowableRace_1=2), None, RPP)
        self.assertEqual(r["allowable_race"], (2 << 32) | 1101)

    def test_absent_versus_zero(self):
        r = envelope.clean(gen.item(sparse(RequiredLevel=0, BuyPrice=0, Bonding=0), None, RPP))
        self.assertEqual((r["required_level"], r["buy_price"], r["bonding"]), (0, 0, 0))
        row = sparse()
        del row["SellPrice"]
        row["Stackable"] = ""
        r = envelope.clean(gen.item(row, None, RPP))
        self.assertNotIn("sell_price", r)
        self.assertNotIn("max_stack", r)

    def test_text_cleaning(self):
        self.assertEqual(gen.text("a\r\nb\tc\x07"), "a\nb c")
        self.assertIsNone(gen.text("  "))
        self.assertEqual(len(gen.text("é" * 5000).encode()), 8192)


class WorldTest(unittest.TestCase):
    def test_area_and_ui_maps(self):
        a = envelope.clean(gen.area({"ID": "12", "AreaName_lang": "Elwynn Forest", "ContinentID": "0",
                                     "ParentAreaID": "0", "ExplorationLevel": "0", "FactionGroupMask": "2"}))
        self.assertEqual(a, {"id": 12, "name": "Elwynn Forest", "map_id": 0, "faction_group_mask": 2})
        m = envelope.clean(gen.ui_map({"ID": "1415", "Name_lang": "Eastern Kingdoms", "ParentUiMapID": "947",
                                       "Type": "2"}))
        self.assertEqual(m, {"id": 1415, "name": "Eastern Kingdoms", "parent_ui_map_id": 947, "map_type": 2})
        u = envelope.clean(gen.ui_map_assignment(dict(ROWS[1], AreaID="0")))
        self.assertNotIn("area_id", u)
        self.assertEqual(u["region_min"][0], -10254.166015625)
        self.assertEqual(u["ui_max"], [1.0, 1.0])


class SpellTalentTest(unittest.TestCase):
    def test_spell_masks_and_levels(self):
        sla = [{"ID": "3255", "SkillLine": "237", "Spell": "1459", "ClassMask": "128", "RaceMasks_0": "0",
                "RaceMasks_1": "0"}]
        s = envelope.clean(gen.spell(1459, sla, {1459: {"Name_lang": "Arcane Intellect"}},
                                     {1459: {"NameSubtext_lang": "Rank 1", "Description_lang": "Int\r\nup"}},
                                     {1459: {"BaseLevel": "1", "SpellLevel": "1", "MaxLevel": "10"}}))
        self.assertEqual(s["class_mask"], 128)
        self.assertNotIn("race_mask", s)  # 0 = unrestricted, not a mask of no races
        self.assertEqual((s["skill_line"], s["base_level"], s["spell_level"], s["max_level"]), (237, 1, 1, 10))
        self.assertEqual(s["description"], "Int\nup")
        two = sla + [{"ID": "9", "SkillLine": "6", "ClassMask": "0", "RaceMasks_0": "16", "RaceMasks_1": "0"}]
        self.assertEqual(gen.spell_masks(two), (None, None))

    def test_talent_ranks_in_order(self):
        r = {"ID": "32", "TabID": "41", "TierID": "4", "ColumnIndex": "2", "PrereqTalent_0": "29"}
        for i in range(9):
            r[f"SpellRank_{i}"] = "0"
        r.update(SpellRank_0="11113", SpellRank_1="12360", SpellRank_2="12848")
        t = envelope.clean(gen.talent(r, {41: {"Name_lang": "Fire", "ClassMask": "128"}}))
        self.assertEqual(t["ranks"], [11113, 12360, 12848])
        self.assertEqual((t["tier"], t["column"], t["tab_name"], t["class_mask"], t["prerequisite"]),
                         (4, 2, "Fire", 128, 29))
        r["PrereqTalent_0"] = "0"
        self.assertNotIn("prerequisite", envelope.clean(gen.talent(r, {})))


class TravelTest(unittest.TestCase):
    def node(self, horde, alliance):
        return {"ID": "2", "Name_lang": "Stormwind, Elwynn", "ContinentID": "0", "Pos_0": "-9465.52",
                "Pos_1": "74.0069", "Pos_2": "56.7785", "MountCreatureID_0": str(horde),
                "MountCreatureID_1": str(alliance)}

    def test_taxi_node_faction_and_position(self):
        loc = gen.coords.Locator(ROWS, MAPS)
        n = gen.taxi_node(self.node(0, 541), loc)
        self.assertEqual(n["faction"], "alliance")
        self.assertEqual(n["position"]["status"], "known")
        self.assertEqual(n["position"]["value"]["ui_map_id"], 1429)
        self.assertAlmostEqual(n["position"]["value"]["x"], 0.421, places=3)
        self.assertEqual(n["world"], {"map_id": 0, "x": -9465.52, "y": 74.0069, "z": 56.7785})
        self.assertEqual(gen.taxi_node(self.node(2224, 0), loc)["faction"], "horde")
        self.assertEqual(gen.taxi_node(self.node(2224, 541), loc)["faction"], "both")
        self.assertIsNone(gen.taxi_node(self.node(0, 0), loc)["faction"])
        far = dict(self.node(0, 541), ContinentID="36")
        self.assertEqual(gen.taxi_node(far, loc)["position"], {"status": "unsupported"})

    def test_taxi_path_point_order(self):
        pts = [{"ID": str(10 + i), "PathID": "13", "NodeIndex": str(idx), "ContinentID": "0", "Loc_0": str(idx),
                "Loc_1": "0", "Loc_2": "0"} for i, idx in enumerate((2, 0, 1))]
        p = gen.taxi_path({"ID": "13", "FromTaxiNode": "2", "ToTaxiNode": "6", "Cost": "50"}, pts)
        self.assertEqual([q["x"] for q in p["points"]], [0.0, 1.0, 2.0])
        self.assertEqual((p["from_node"], p["to_node"], p["cost"]), (2, 6, 50))
        self.assertEqual(gen.build_taxi_paths({"TaxiPath": [{"ID": "14", "FromTaxiNode": "1", "ToTaxiNode": "2",
                                                             "Cost": ""}], "TaxiPathNode": pts})[0]["points"], [])


class FallbackTest(unittest.TestCase):
    """A table missing from the Forever build sends the dataset to Classic Era's origin."""

    def test_missing_table_uses_classic_era_origin(self):
        class FakeWago:
            def __init__(self, product, build=None):
                self.product, self.build = product, build or ("1.60.1.70245" if product == "wowf" else "1.15.7.1")

            def url(self, table):
                return f"https://wago.tools/db2/{table}/csv?product={self.product}"

            def table(self, table):
                if self.product == "wowf" and table == "TalentTab":
                    raise gen.http.FetchError(self.url(table), 400)
                return TABLES.get(table, [])

            def origin(self):
                return envelope.origin(db2.EDITIONS[self.product], self.build, product=self.product)

        area_row = {"ID": "12", "AreaName_lang": "Elwynn Forest", "ContinentID": "0"}
        talent_row = {"ID": "1", "TabID": "41", "TierID": "0", "ColumnIndex": "0", "SpellRank_0": "5"}
        TABLES = {"AreaTable": [area_row], "Talent": [talent_row], "TalentTab": [{"ID": "41", "Name_lang": "Fire"}]}
        saved, real = dict(gen.BUILDERS), gen.db2.Wago
        try:
            gen.BUILDERS.clear()
            gen.BUILDERS.update({k: saved[k] for k in ("world.areas", "talents.talents")})
            gen.db2.Wago = FakeWago
            with tempfile.TemporaryDirectory() as out:
                counts = gen.run(pathlib.Path(out))
                self.assertEqual(counts["world.areas"][1], "wow_forever")
                self.assertEqual(counts["talents.talents"][1], "wow_classic_era")
                marker = json.loads((pathlib.Path(out) / ".written" / "gen_world_db2.json").read_text())
                self.assertIn("talents/talents/wow_classic_era/1.15.7.1/part-0000.json", marker["files"])
                doc = json.loads((pathlib.Path(out) / marker["files"][0]).read_text())
                self.assertEqual((doc["origin"]["edition"], doc["confidence"]), ("wow_classic_era", "cross_edition_unverified"))
        finally:
            gen.db2.Wago = real
            gen.BUILDERS.clear()
            gen.BUILDERS.update(saved)


if __name__ == "__main__":
    unittest.main()
