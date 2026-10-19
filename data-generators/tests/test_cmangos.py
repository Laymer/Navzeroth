"""Offline tests of gen_world_cmangos mapping functions on a tiny synthetic dump."""
import os
import pathlib
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "Navzeroth-data-generators"))
os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1"
import gen_world_cmangos as gen  # noqa: E402
from navzeroth_datagen import coords, envelope, sqldump  # noqa: E402


def table(name, columns, rows):
    cols = ",\n".join(f"  `{c}` int" for c in columns)
    body = ",".join("(" + ",".join("NULL" if v is None else repr(v) if isinstance(v, str) else str(v)
                                   for v in r) + ")" for r in rows)
    return f"CREATE TABLE `{name}` (\n{cols}\n)\n" + (f"INSERT INTO `{name}` VALUES {body};\n" if rows else "")


QUEST_COLS = (["entry", "Title", "Objectives", "QuestLevel", "MinLevel", "ZoneOrSort", "RequiredRaces",
               "RequiredClasses", "RequiredSkill", "RequiredSkillValue", "PrevQuestId", "NextQuestId",
               "NextQuestInChain", "ExclusiveGroup", "SrcItemId", "RewOrReqMoney", "RewSpell", "QuestFlags",
               "SpecialFlags"]
              + [f"ReqCreatureOrGOId{i}" for i in range(1, 5)] + [f"ReqCreatureOrGOCount{i}" for i in range(1, 5)]
              + [f"ReqSpellCast{i}" for i in range(1, 5)] + [f"ObjectiveText{i}" for i in range(1, 5)]
              + [f"ReqItemId{i}" for i in range(1, 5)] + [f"ReqItemCount{i}" for i in range(1, 5)]
              + [f"RewChoiceItemId{i}" for i in range(1, 7)] + [f"RewChoiceItemCount{i}" for i in range(1, 7)]
              + [f"RewItemId{i}" for i in range(1, 5)] + [f"RewItemCount{i}" for i in range(1, 5)])


def quest_row(**values):
    row = {c: 0 for c in QUEST_COLS}
    row.update(Title="", Objectives="", ObjectiveText1="", ObjectiveText2="", ObjectiveText3="", ObjectiveText4="")
    row.update(values)
    return [row[c] for c in QUEST_COLS]


SQL = "".join([
    table("quest_template", QUEST_COLS, [
        quest_row(entry=176, Title='Wanted:  "Hogger"', Objectives="Slay Hogger.\r\nReturn.", QuestLevel=11,
                  MinLevel=5, ZoneOrSort=12, RequiredRaces=77, ReqItemId1=1931, ReqItemCount1=1,
                  RewChoiceItemId1=6085, RewChoiceItemCount1=1, RewChoiceItemId2=6084, RewChoiceItemCount2=1,
                  QuestFlags=8),
        quest_row(entry=2, Title="Mixed", QuestLevel=-1, ZoneOrSort=-141, PrevQuestId=-5, ExclusiveGroup=7,
                  RequiredSkill=356, RequiredSkillValue=100, SrcItemId=99, RewOrReqMoney=-500, RewSpell=1234,
                  ReqCreatureOrGOId1=448, ReqCreatureOrGOCount1=3, ObjectiveText1="Kill",
                  ReqCreatureOrGOId2=-1000, ReqCreatureOrGOCount2=2,
                  ReqCreatureOrGOId3=500, ReqCreatureOrGOCount3=1, ReqSpellCast3=777,
                  ReqCreatureOrGOId4=600, ReqCreatureOrGOCount4=0,
                  RewItemId1=10, RewItemCount1=2, RewItemId2=11, RewItemCount2=0),
        quest_row(entry=3, Title="Explore", SpecialFlags=2),
        quest_row(entry=4, Title="Event", SpecialFlags=2),
        quest_row(entry=5, Title="  "),
    ]),
    table("creature_questrelation", ["id", "quest"], [(240, 2), (240, 2)]),
    table("gameobject_questrelation", ["id", "quest"], [(156561, 176), (68, 176)]),
    table("creature_involvedrelation", ["id", "quest"], [(240, 176)]),
    table("gameobject_involvedrelation", ["id", "quest"], []),
    table("item_template", ["entry", "startquest"], [(9000, 2), (9001, 0)]),
    table("areatrigger_involvedrelation", ["id", "quest"], [(88, 3), (87, 3)]),
])


class NpcTest(unittest.TestCase):
    def test_role_bits(self):
        self.assertEqual(gen.npc_roles(0x87), ["quest_giver", "vendor", "innkeeper"])  # Innkeeper Farley
        self.assertEqual(gen.npc_roles(0xb), ["quest_giver", "flight_master"])  # Thor
        self.assertEqual(gen.npc_roles(0x1000 | 0x100 | 0x2000 | 0x20 | 0x800 | 0x4004),
                         ["vendor", "repair", "banker", "auctioneer", "spirit_healer", "stable_master",
                          "battlemaster"])
        self.assertEqual(gen.npc_roles(0x13, 0, 8), ["quest_giver", "trainer", "class_trainer"])
        self.assertEqual(gen.npc_roles(0x10, 2, 0), ["trainer", "profession_trainer"])
        self.assertEqual(gen.npc_roles(0x10, 3, 0), ["trainer"])
        self.assertEqual(gen.npc_roles(0x1 | 0x40 | 0x200 | 0x400), [])
        self.assertEqual(gen.npc_roles(None), [])

    def test_npc_record_and_reaction(self):
        fts = [{"ID": "1", "Faction": "1", "FactionGroup": "3", "FriendGroup": "2", "EnemyGroup": "12"},
               {"ID": "2", "Faction": "2", "FactionGroup": "5", "FriendGroup": "4", "EnemyGroup": "10"},
               {"ID": "12", "Faction": "72", "FactionGroup": "2", "FriendGroup": "2", "EnemyGroup": "4"},
               {"ID": "29", "Faction": "76", "FactionGroup": "4", "FriendGroup": "4", "EnemyGroup": "2"},
               {"ID": "35", "Faction": "31", "FactionGroup": "0", "FriendGroup": "1", "EnemyGroup": "0"},
               {"ID": "14", "Faction": "14", "FactionGroup": "8", "FriendGroup": "0", "EnemyGroup": "1"},
               {"ID": "7", "Faction": "7", "FactionGroup": "0", "FriendGroup": "0", "EnemyGroup": "0"},
               {"ID": "50", "Faction": "50", "FactionGroup": "0", "FriendGroup": "0", "EnemyGroup": "0",
                "Enemies_0": "1", "Friend_0": "2"}]
        r = gen.Reactions(fts)
        self.assertEqual([r.reaction(i) for i in (12, 29, 35, 14, 7, 50, 999)],
                         ["alliance", "horde", "both", "hostile", None, "horde", None])
        rec = gen.npc_record({"Entry": 5497, "Name": "Jennea Cannon", "SubName": "", "MinLevel": 40, "MaxLevel": 40,
                              "Faction": 12, "NpcFlags": 0x13, "Rank": 0, "CreatureType": 7, "TrainerType": 0,
                              "TrainerClass": 8}, r)
        self.assertEqual(rec["reaction"], "alliance")
        self.assertEqual(rec["trainer_class"], 8)
        self.assertIsNone(rec["subname"])
        self.assertIsNone(gen.npc_record({"Entry": 1, "Name": " "}))


class QuestTest(unittest.TestCase):
    def setUp(self):
        self.quests = {q["id"]: q for q in gen.quest_records(sqldump.Dump(SQL))}

    def test_givers_enders_and_text(self):
        q = self.quests[176]
        self.assertEqual(q["givers"], [{"entity": "game_object", "id": 68}, {"entity": "game_object", "id": 156561}])
        self.assertEqual(q["enders"], [{"entity": "npc", "id": 240}])
        self.assertEqual(q["objectives_text"], "Slay Hogger.\nReturn.")
        self.assertEqual(q["objectives"], [{"objective": "item", "target_id": 1931, "count": 1}])
        self.assertEqual(q["reward_choice_items"], [{"item": 6085, "count": 1}, {"item": 6084, "count": 1}])
        self.assertEqual(self.quests[2]["givers"], [{"entity": "npc", "id": 240}, {"entity": "item", "id": 9000}])
        self.assertNotIn(5, self.quests)

    def test_objectives(self):
        obj = self.quests[2]["objectives"]
        self.assertEqual([(o["objective"], o["target_id"], o["count"]) for o in obj],
                         [("kill", 448, 3), ("object", 1000, 2), ("spell", 777, 1)])
        self.assertEqual(obj[0]["text"], "Kill")
        self.assertIsNone(obj[1]["text"])
        self.assertEqual([(o["objective"], o["target_id"]) for o in self.quests[3]["objectives"]],
                         [("explore", 87), ("explore", 88)])
        self.assertEqual(self.quests[4]["objectives"], [{"objective": "event", "count": 1}])

    def test_absent_versus_zero(self):
        hogger, mixed = envelope.clean(self.quests[176]), envelope.clean(self.quests[2])
        self.assertEqual(hogger["reward_money"], 0)
        self.assertEqual(hogger["required_classes"], 0)
        for absent in ("prev_quest", "next_quest", "next_in_chain", "exclusive_group", "reward_spell",
                       "source_item", "required_skill", "reward_xp", "reward_items"):
            self.assertNotIn(absent, hogger)
        self.assertEqual(mixed["reward_money"], -500)
        self.assertEqual(mixed["prev_quest"], -5)
        self.assertEqual(mixed["level"], -1)
        self.assertEqual(mixed["zone_or_sort"], -141)
        self.assertEqual(mixed["required_skill"], [356, 100])
        self.assertEqual(mixed["source_item"], 99)
        self.assertEqual(mixed["reward_items"], [{"item": 10, "count": 2}])

    def test_xp_only_when_the_column_exists(self):
        row = dict(zip(QUEST_COLS, quest_row(entry=9, Title="T")))
        self.assertNotIn("reward_xp", envelope.clean(gen.quest_record(row)))
        self.assertEqual(gen.quest_record(dict(row, RewXP=0))["reward_xp"], 0)


class LootVendorTrainerTest(unittest.TestCase):
    def test_loot_reference_item_and_chance_sign(self):
        ref = gen.loot_record("creature", {"entry": 1, "item": 24000, "ChanceOrQuestChance": 10, "groupid": 0,
                                           "mincountOrRef": -24000, "maxcount": 2, "condition_id": 0})
        self.assertEqual(envelope.clean(ref), {"table": "creature", "entry": 1, "reference": 24000, "chance": 10.0,
                                               "min_count": 1, "max_count": 2})
        item = gen.loot_record("creature", {"entry": 1, "item": 1931, "ChanceOrQuestChance": -100, "groupid": 3,
                                            "mincountOrRef": 1, "maxcount": 1, "condition_id": 5})
        self.assertEqual(envelope.clean(item), {"table": "creature", "entry": 1, "item": 1931, "chance": -100.0,
                                                "group": 3, "min_count": 1, "max_count": 1, "condition": 5})
        zero = gen.loot_record("reference", {"entry": 2, "item": 0, "ChanceOrQuestChance": 0, "groupid": 1,
                                             "mincountOrRef": -7, "maxcount": 0, "condition_id": None})
        self.assertEqual((zero["reference"], zero["min_count"], zero["max_count"], zero["chance"]), (7, 1, 1, 0.0))

    def test_game_object_loot_is_keyed_by_the_object(self):
        templates = [{"entry": 2843, "type": 3, "data1": 2264}, {"entry": 2844, "type": 3, "data1": 2264},
                     {"entry": 180582, "type": 25, "data1": 5000}, {"entry": 68, "type": 2, "data1": 2264},
                     {"entry": 1731, "type": 3, "data1": 0}]
        owners = gen.game_object_loot_owners(templates)
        self.assertEqual(owners, {2264: [2843, 2844], 5000: [180582]})  # type 2 data1 is no loot ID
        rows = [{"entry": 2264, "item": 872, "ChanceOrQuestChance": 5, "groupid": 0, "mincountOrRef": 1,
                 "maxcount": 1, "condition_id": 0},
                {"entry": 5000, "item": 6303, "ChanceOrQuestChance": 50, "groupid": 0, "mincountOrRef": 1,
                 "maxcount": 1, "condition_id": 0},
                {"entry": 9999, "item": 1, "ChanceOrQuestChance": 50, "groupid": 0, "mincountOrRef": 1,
                 "maxcount": 1, "condition_id": 0}]
        records, orphans = gen.game_object_loot(rows, owners, {2843, 2844})
        self.assertEqual([(r["entry"], r["item"]) for r in records], [(2843, 872), (2844, 872)])
        self.assertEqual(orphans, {5000, 9999})  # 180582 is not a known game object
        self.assertTrue(all(r["table"] == "game_object" for r in records))

    def test_item_record_from_item_template(self):
        row = {"entry": 872, "name": "Rockslicer", "Quality": 2, "class": 2, "subclass": 1, "InventoryType": 17,
               "ItemLevel": 21, "RequiredLevel": 16, "AllowableClass": -1, "AllowableRace": -1, "bonding": 1,
               "stackable": 1, "BuyPrice": 10058, "SellPrice": 2011, "stat_type1": 4, "stat_value1": 7,
               "stat_type2": 7, "stat_value2": 0, "stat_type3": 4, "stat_value3": 2, "stat_type4": 99,
               "stat_value4": 1, "armor": 0, "fire_res": 5}
        rec = envelope.clean(gen.item_record(row))
        self.assertEqual(rec, {"id": 872, "name": "Rockslicer", "quality": 2, "class_id": 2, "subclass_id": 1,
                               "inventory_type": 17, "item_level": 21, "required_level": 16,
                               "allowable_class": -1, "allowable_race": -1, "bonding": 1, "max_stack": 1,
                               "buy_price": 10058, "sell_price": 2011,
                               "stats": {"fire_resistance": 5, "stat_99": 1, "strength": 9}})
        self.assertEqual(gen.item_stats({"armor": 120}), {"armor": 120})
        self.assertIsNone(gen.item_record({"entry": 1, "name": "  "}))
        # Absent or negative unsigned columns stay absent; zero stays zero.
        rec = envelope.clean(gen.item_record({"entry": 2, "name": "X", "Quality": 0, "stackable": -1}))
        self.assertEqual(rec, {"id": 2, "name": "X", "quality": 0})

    def test_vendor_template_expansion(self):
        direct = [{"entry": 10, "item": 1, "maxcount": 0, "incrtime": 0},
                  {"entry": 10, "item": 2, "maxcount": 5, "incrtime": 3600}]
        templates = [{"entry": 900, "item": 2, "maxcount": 0, "incrtime": 0},
                     {"entry": 900, "item": 3, "maxcount": 1, "incrtime": 0}]
        rows = gen.expand_templates(direct, templates, {10: 900, 11: 900, 12: 901}, "item")
        recs = [envelope.clean(gen.vendor_record(r)) for r in rows]
        self.assertEqual(recs, [{"npc_id": 10, "item_id": 1}, {"npc_id": 10, "item_id": 2, "max_count": 5,
                                                                  "restock_secs": 3600},
                                {"npc_id": 10, "item_id": 3, "max_count": 1}, {"npc_id": 11, "item_id": 2},
                                {"npc_id": 11, "item_id": 3, "max_count": 1}])

    def test_trainer_record(self):
        rec = envelope.clean(gen.trainer_record({"entry": 5, "spell": 2550, "spellcost": 0, "reqskill": 0,
                                                 "reqskillvalue": 0, "reqlevel": 0}))
        self.assertEqual(rec, {"npc_id": 5, "spell_id": 2550, "cost": 0, "required_level": 0})
        rec = gen.trainer_record({"entry": 5, "spell": 3, "spellcost": 100, "reqskill": 185, "reqskillvalue": 50,
                                  "reqlevel": 10})
        self.assertEqual((rec["required_skill_line"], rec["required_skill"]), (185, 50))


def ui_row(ui, map_id, lo, hi, area=0, order=0):
    r = {"ID": str(ui), "UiMapID": str(ui), "MapID": str(map_id), "AreaID": str(area), "OrderIndex": str(order),
         "WMOGroupID": "0", "WMODoodadPlacementID": "0", "UiMin_0": "0", "UiMin_1": "0", "UiMax_0": "1",
         "UiMax_1": "1"}
    for i, v in enumerate(lo + hi):
        r[f"Region_{i}"] = str(v)
    return r


# wowf 1.60.1.70245 rows: Eastern Kingdoms, Elwynn Forest, Duskwood, Westfall, Riverglades (Forever only)
ASSIGN = [ui_row(1415, 0, (-16000, -19199.9, -1e6), (7466.6, 16000, 1e6)),
          ui_row(1429, 0, (-10254.166, -1935.4166, -1e6), (-7939.583, 1535.4166, 1e6), area=12),
          ui_row(1431, 0, (-11516.7, -1866.67, -1e6), (-9716.67, 833.33, 1e6), area=10),
          ui_row(1436, 0, (-11733.3, -483.33, -1e6), (-9400, 3016.67, 1e6), area=40),
          ui_row(2548, 0, (-9700, -6742, -1e6), (-6467, -1892, 1e6), area=16591)]
UIMAPS = [{"ID": "1415", "Type": "2"}, {"ID": "1429", "Type": "3"}, {"ID": "1431", "Type": "3"},
          {"ID": "1436", "Type": "3"},
          {"ID": "2548", "Type": "3"}]


class PlacementTest(unittest.TestCase):
    def test_hints_settle_overlapping_zones(self):
        plain = coords.Locator(ASSIGN, UIMAPS)
        hogger = (-9946.0, 604.266)
        self.assertEqual(plain.candidates(0, *hogger), [1431, 1429, 1436])  # smallest first: Duskwood
        pois = [{"questId": 176, "poiId": 0, "mapId": 0, "mapAreaId": 30},
                {"questId": 38, "poiId": 0, "mapId": 0, "mapAreaId": 39},
                {"questId": 106, "poiId": 0, "mapId": 0, "mapAreaId": 39}]
        points = [{"questId": 176, "poiId": 0, "x": -9983, "y": 549}, {"questId": 176, "poiId": 0, "x": -9000, "y": 0},
                  {"questId": 38, "poiId": 0, "x": -10500, "y": 1000},
                  {"questId": 106, "poiId": 0, "x": -9930, "y": 600}]  # Elwynn quest drawn on Westfall's map
        zones = gen.zone_areas(ASSIGN, UIMAPS)
        hints = gen.poi_hints(pois, points, plain, {176: 12, 38: 40, 106: 12}, zones)
        self.assertEqual(hints, [(0, 1429, -9983.0, 549.0), (0, 1429, -9000.0, 0.0), (0, 1436, -10500.0, 1000.0)])
        p = coords.Locator(ASSIGN, UIMAPS, hints).locate(0, *hogger)
        self.assertEqual(p["ui_map_id"], 1429)
        self.assertAlmostEqual(p["x"], 0.268, places=3)
        self.assertAlmostEqual(p["y"], 0.867, places=3)

    def test_spawns_and_forever_only_zones(self):
        kept = gen.server_zone_assignments(ASSIGN, UIMAPS, {12, 40})
        self.assertEqual([r["UiMapID"] for r in kept], ["1415", "1429", "1436"])  # Duskwood: area 10 unnamed
        loc = coords.Locator(kept, UIMAPS)
        rows = [{"guid": 1, "id": 448, "map": 0, "position_x": -9000, "position_y": 0, "position_z": 1,
                 "spawntimesecsmin": 300},
                {"guid": 2, "id": 0, "map": 0, "position_x": -8000, "position_y": -3000, "position_z": 1,
                 "spawntimesecsmin": -5},
                {"guid": 3, "id": 639, "map": 36, "position_x": 1, "position_y": 2, "position_z": 3,
                 "spawntimesecsmin": None}]
        recs = gen.spawn_records(rows, "npc", loc, gen.zone_areas(kept, UIMAPS), {2: [5, 6]}, {448, 5, 639})
        self.assertEqual([(r["guid"], r["entity_id"]) for r in recs], [(1, 448), (2, 5), (3, 639)])
        self.assertEqual((recs[0]["position"]["value"]["ui_map_id"], recs[0]["area_id"], recs[0]["respawn_secs"]),
                         (1429, 12, 300))
        self.assertEqual(recs[1]["position"]["value"]["ui_map_id"], 1415)  # Riverglades dropped: continent
        self.assertIsNone(recs[1]["respawn_secs"])
        self.assertEqual(recs[2]["position"], {"status": "unsupported"})
        self.assertIsNone(recs[2]["area_id"])


class DungeonGraveyardAreaTest(unittest.TestCase):
    def test_entrance_unknown_without_client_trigger(self):
        teleports = [{"id": 78, "name": "Deadmines - Entering", "target_map": 36},
                     {"id": 2848, "name": "Onyxia's Lair - Entering", "target_map": 249},
                     {"id": 4156, "name": "Naxxramas (Entrance)", "target_map": 533},
                     {"id": 4055, "name": "Naxxramas (Exit)", "target_map": 533}]
        triggers = [{"ID": "2848", "ContinentID": "1", "Pos_0": "-4768.6", "Pos_1": "-3752.1", "Pos_2": "50.4"},
                    {"ID": "4055", "ContinentID": "0", "Pos_0": "3133.1", "Pos_1": "-3730.3", "Pos_2": "137"},
                    {"ID": "4156", "ContinentID": "533", "Pos_0": "3006", "Pos_1": "-3434", "Pos_2": "293"}]
        entrances = gen.trigger_entrances(teleports, triggers)
        self.assertEqual(sorted(entrances), [249, 533])
        self.assertEqual(entrances[533]["map_id"], 0)
        maps = {36: {"MapName_lang": "Deadmines", "InstanceType": "1", "AreaTableID": "1581"},
                249: {"MapName_lang": "Onyxia's Lair", "InstanceType": "2", "AreaTableID": "2159"},
                30: {"MapName_lang": "Alterac Valley", "InstanceType": "3", "AreaTableID": "2597"}}
        instances = [{"map": 36, "levelMin": 17, "levelMax": 26}, {"map": 249, "levelMin": 0, "levelMax": 0},
                     {"map": 30, "levelMin": 51, "levelMax": 60}, {"map": 533, "levelMin": 60, "levelMax": 0}]
        bosses = gen.dungeon_bosses(
            [{"Entry": 639, "Rank": 1}, {"Entry": 10184, "Rank": 3}, {"Entry": 1, "Rank": 3}],
            [{"id": 10184, "map": 249}, {"id": 639, "map": 36}, {"id": 1, "map": 0}],
            [{"entry": 161, "creditType": 0, "creditEntry": 639}, {"entry": 162, "creditType": 1, "creditEntry": 5}],
            [{"Id": 161, "MapId": 36}, {"Id": 162, "MapId": 36}])
        recs = {r["map_id"]: envelope.clean(r) for r in gen.dungeon_records(instances, maps, entrances, bosses,
                                                                            teleports)}
        self.assertEqual(sorted(recs), [36, 249, 533])  # Alterac Valley (pvp) left out
        self.assertEqual(recs[36], {"map_id": 36, "name": "Deadmines", "min_level": 17, "max_level": 26,
                                    "entrance": {"status": "unknown"}, "bosses": [639], "area_id": 1581})
        self.assertEqual(recs[249]["entrance"]["status"], "known")
        self.assertEqual(recs[249]["bosses"], [10184])
        self.assertNotIn("min_level", recs[249])
        self.assertEqual(recs[533]["name"], "Naxxramas")  # no client Map row: teleport name

    def test_graveyards(self):
        loc = coords.Locator(ASSIGN, UIMAPS)
        recs = gen.graveyard_records(
            [{"id": 1, "map": 0, "x": -9000, "y": 0, "z": 1}, {"id": 2, "map": 0, "x": 0, "y": 0, "z": 0},
             {"id": 3, "map": 999, "x": 0, "y": 0, "z": 0}, {"id": 4, "map": 0, "x": 0, "y": 0, "z": 0}],
            [{"id": 1, "ghost_loc": 12, "faction": 469}, {"id": 2, "ghost_loc": 40, "faction": 469},
             {"id": 2, "ghost_loc": 10, "faction": 67}, {"id": 3, "ghost_loc": 1, "faction": 0}], loc)
        self.assertEqual([(r["faction"], r["zones"]) for r in recs],
                         [("alliance", [12]), ("both", [10, 40]), ("both", [1]), (None, None)])
        self.assertEqual(recs[2]["position"], {"status": "unsupported"})

    def test_area_levels(self):
        templates = [{"Entry": 1, "MinLevel": 5, "MaxLevel": 7, "CreatureType": 7, "NpcFlags": 0, "Rank": 0},
                     {"Entry": 2, "MinLevel": 30, "MaxLevel": 30, "CreatureType": 8, "NpcFlags": 0, "Rank": 0},
                     {"Entry": 3, "MinLevel": 2, "MaxLevel": 3, "CreatureType": 7, "NpcFlags": 0, "Rank": 0},
                     {"Entry": 4, "MinLevel": 60, "MaxLevel": 60, "CreatureType": 7, "NpcFlags": 2, "Rank": 0},
                     {"Entry": 5, "MinLevel": 63, "MaxLevel": 63, "CreatureType": 2, "NpcFlags": 0, "Rank": 3}]
        world = {"map_id": 0, "x": 0, "y": 0, "z": 0}
        spawns = [{"entity": "npc", "entity_id": e, "area_id": 12, "world": world} for e in (1, 2, 3, 4, 5)]
        spawns.append({"entity": "npc", "entity_id": 1, "area_id": 99, "world": world})
        self.assertEqual(gen.area_level_records(spawns, templates, {12: "Elwynn Forest"}),
                         [{"id": 12, "name": "Elwynn Forest", "map_id": 0, "level_range": [2, 7]}])


class MainTest(unittest.TestCase):
    def test_offline_failure_exits_non_zero(self):
        with tempfile.TemporaryDirectory() as cache, tempfile.TemporaryDirectory() as out:
            old = os.environ.get("NAVZEROTH_DATAGEN_CACHE")
            os.environ["NAVZEROTH_DATAGEN_CACHE"] = cache
            try:
                self.assertEqual(gen.main(["--offline", "--out", out]), 1)
            finally:
                if old is None:
                    del os.environ["NAVZEROTH_DATAGEN_CACHE"]
                else:
                    os.environ["NAVZEROTH_DATAGEN_CACHE"] = old
            self.assertFalse(pathlib.Path(out, ".written").exists())


if __name__ == "__main__":
    unittest.main()
