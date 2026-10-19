"""world_to_content.py on tiny synthetic shards (offline): every mapping rule,
absent vs zero, precedence and conflicts, bounded reference loot, transform math."""
import contextlib
import io
import json
import math
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
import world_to_content as w  # noqa: E402

CMANGOS = [{"id": "cmangos-classic-db", "url": "https://github.com/cmangos/classic-db", "revision": "r1",
            "snapshot_sha256": "a" * 64, "retrieved_at_utc_ms": 1700000000000}]
WAGO = [{"id": "wago-tools-wowf", "url": "https://wago.tools/db2?product=wowf", "revision": "1.60.1.70245",
         "snapshot_sha256": "b" * 64}]
WAGO_ERA = [{"id": "wago-tools-classic-era", "url": "https://wago.tools/db2?product=wow_classic_era"}]
# wowf 1.60.1.70245 UiMapAssignment of Elwynn Forest (uiMap 1429).
ELWYNN = {"kind": "ui_map_assignment", "map_id": 0, "order_index": 0, "ui_map_id": 1429,
          "region_min": [-10254.166015625, -1935.4166259766, -1000000.0],
          "region_max": [-7939.5830078125, 1535.4166259766, 1000000.0], "ui_min": [0.0, 0.0], "ui_max": [1.0, 1.0]}
HOGGER = (-9947.88, 594.77)  # CMaNGOS creature 448, world X / Y
TARGET = {"format": "navzeroth-world-data-target", "schema_version": 1, "edition": "wow_forever",
          "build": "1.60.1.70450", "interface": "16001", "locale": "enUS"}
POLICY = {"format": "navzeroth-accepted-origins", "schema_version": 1, "target_edition": "wow_forever",
          "domains": {d: ["vanilla_1_12", "wow_classic_era"] for d in
                      ("quests", "equipment", "acquisition", "geography", "travel", "dungeons", "class_roles")},
          "accept_inferred": True, "accept_unknowns": True,
          "precedence": ["owner_observation", "wow_forever", "authored", "vanilla_1_12", "wow_classic_era"]}
AUTHORED = {"dungeons": [{"title": "The Deadmines", "entrance": {"status": "unknown"}}]}


def pos(nx, ny):
    return {"status": "known", "value": {"ui_map_id": 1429, "x": nx, "y": ny}}


def world_of(nx, ny):
    """World point of a normalized Elwynn position (inverse of the generator's locator)."""
    t = w.transform_of(ELWYNN)
    y, x = w.apply_transform(t, nx, ny)
    return {"map_id": 0, "x": x, "y": y, "z": 0.0}


def spawn(guid, entity, eid, nx, ny):
    return {"kind": "spawn", "guid": guid, "entity": entity, "entity_id": eid, "position": pos(nx, ny),
            "world": world_of(nx, ny)}


def loot(table, entry, chance, item=None, reference=None, group=None, mn=1, mx=1, condition=None):
    r = {"kind": "loot", "table": table, "entry": entry, "chance": chance, "min_count": mn, "max_count": mx}
    for k, v in (("item", item), ("reference", reference), ("group", group), ("condition", condition)):
        if v is not None:
            r[k] = v
    return r


def item(iid, inv, name, **extra):
    r = {"kind": "item", "id": iid, "name": name, "inventory_type": inv, "allowable_class": -1,
         "allowable_race": -1, "required_level": 0, "buy_price": 100, "class_id": 4, "subclass_id": 2,
         "quality": 1}
    r.update(extra)
    return r


SHARDS = {
    ("world.ui_maps", "wow_forever"): (WAGO, "publisher_documented", [
        {"kind": "ui_map", "id": 1429, "name": "Elwynn Forest", "map_type": 3}]),
    ("world.ui_map_assignments", "wow_forever"): (WAGO, "publisher_documented", [ELWYNN]),
    ("world.areas", "wow_forever"): (WAGO, "publisher_documented", [
        {"kind": "area", "id": 12, "name": "Elwynn Forest", "map_id": 0}]),
    ("world.area_levels", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        {"kind": "area", "id": 12, "name": "Elwynn", "map_id": 0, "level_range": [1, 10]}]),
    ("world.npcs", "vanilla_1_12"): (CMANGOS + WAGO, "cross_edition_unverified", [
        {"kind": "npc", "id": 448, "name": "Hogger", "min_level": 11, "max_level": 11, "reaction": "hostile"},
        {"kind": "npc", "id": 295, "name": "Innkeeper Farley", "subname": "Innkeeper", "min_level": 30,
         "max_level": 30, "reaction": "alliance", "roles": ["vendor", "innkeeper"]},
        {"kind": "npc", "id": 240, "name": "Marshal Dughan", "min_level": 0, "max_level": 0,
         "roles": ["quest_giver"]},
        {"kind": "npc", "id": 352, "name": "Dungar Longdrink", "min_level": 55, "max_level": 55,
         "reaction": "alliance", "roles": ["flight_master"]},
        {"kind": "npc", "id": 639, "name": "Edwin VanCleef", "min_level": 21, "max_level": 21,
         "reaction": "hostile"},
        {"kind": "npc", "id": 900, "name": "Quiet Guard"}]),
    ("world.game_objects", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        {"kind": "game_object", "id": 68, "name": "Wanted Board", "object_type": 2},
        {"kind": "game_object", "id": 2843, "name": "Battered Chest", "object_type": 3}]),
    ("world.spawns", "vanilla_1_12"): (CMANGOS + WAGO, "cross_edition_unverified", [
        spawn(1, "npc", 448, 0.271, 0.868), spawn(2, "npc", 448, 0.2736, 0.9271),
        spawn(3, "npc", 448, 0.2502, 0.929), spawn(4, "npc", 448, 0.2600, 0.9000),
        spawn(5, "npc", 295, 0.4400, 0.6600), spawn(6, "npc", 240, 0.4211, 0.6593),
        spawn(7, "npc", 352, 0.4300, 0.6500),
        {"kind": "spawn", "guid": 8, "entity": "npc", "entity_id": 639, "position": {"status": "unsupported"},
         "world": {"map_id": 36, "x": 1, "y": 2, "z": 3}},
        {**spawn(9, "game_object", 68, 0.4220, 0.6600)}]),
    ("world.graveyards", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        {"kind": "graveyard", "id": 1, "position": pos(0.5, 0.5), "world": world_of(0.5, 0.5)},
        {"kind": "graveyard", "id": 2, "position": {"status": "unsupported"}, "world": {"map_id": 1, "x": 0,
                                                                                         "y": 0, "z": 0}}]),
    ("travel.taxi_nodes", "wow_forever"): (WAGO, "publisher_documented", [
        {"kind": "taxi_node", "id": 2, "name": "Stormwind", "position": pos(0.43, 0.65),
         "world": world_of(0.43, 0.65), "flight_master": 352},
        {"kind": "taxi_node", "id": 4, "name": "Sentinel Hill", "position": pos(0.30, 0.95),
         "world": world_of(0.30, 0.95)}]),
    ("travel.taxi_paths", "wow_forever"): (WAGO, "publisher_documented", [
        {"kind": "taxi_path", "id": 6, "from_node": 2, "to_node": 4, "cost": 110,
         "points": [{"map_id": 0, "x": 0, "y": 0, "z": 0}, {"map_id": 0, "x": 300, "y": 400, "z": 0},
                    {"map_id": 1, "x": 0, "y": 0, "z": 0}, {"map_id": 1, "x": 0, "y": 0, "z": 310}]},
        {"kind": "taxi_path", "id": 7, "from_node": 4, "to_node": 99, "points": []}]),
    ("items.items", "wow_forever"): (WAGO, "publisher_documented", [
        item(1931, 0, "Huge Gnoll Claw", buy_price=0),
        item(6085, 5, "Footman Tunic", stats={"agility": 1, "stamina": 2}),
        item(6215, 17, "Balanced Fighting Stick", required_level=5),
        item(2589, 0, "Linen Cloth"), item(2592, 0, "Wool Cloth"),
        item(159, 0, "Refreshing Spring Water", buy_price=25),
        item(5000, 2, "Mage Pendant", allowable_class=128),
        item(5001, 99, "Odd thing")]),
    ("items.items", "wow_classic_era"): (WAGO_ERA, "cross_edition_unverified", [
        item(6085, 5, "Footman's Tunic", stats={"agility": 1, "stamina": 2})]),
    # CMaNGOS item_template: the fallback for items the Forever table lacks (872).
    ("items.items", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        item(872, 17, "Rockslicer", required_level=16, stats={"strength": 7}),
        item(6085, 5, "Footman Tunic", stats={"agility": 1, "stamina": 2, "armor": 30}, buy_price=7),
        item(6215, 17, "Balanced Fighting Stick", required_level=6)]),
    ("quests.quests", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        {"kind": "quest", "id": 176, "title": "Wanted: Hogger", "level": 11, "min_level": 5, "flags": 8,
         "required_races": 77, "required_classes": 0, "zone_or_sort": 12, "reward_money": 0,
         "givers": [{"entity": "game_object", "id": 68}], "enders": [{"entity": "npc", "id": 240}],
         "objectives": [{"objective": "item", "target_id": 1931, "count": 1}],
         "reward_choice_items": [{"item": 6085, "count": 1}, {"item": 6215, "count": 1},
                                 {"item": 77777, "count": 1}]},
        {"kind": "quest", "id": 2, "title": "Orphan", "min_level": 0, "flags": 0, "required_races": 0,
         "required_classes": 1, "prev_quest": 31337, "reward_items": [{"item": 159, "count": 5}]},
        {"kind": "quest", "id": 10, "title": "Loop A", "min_level": 1, "prev_quest": 11},
        {"kind": "quest", "id": 11, "title": "Loop B", "min_level": 1, "prev_quest": 10},
        {"kind": "quest", "id": 20, "title": "Pick A", "min_level": 1, "exclusive_group": 5, "next_quest": 22},
        {"kind": "quest", "id": 21, "title": "Pick B", "min_level": 1, "exclusive_group": 5, "next_quest": 22},
        {"kind": "quest", "id": 22, "title": "After pick", "min_level": 1, "prev_quest": -20},
        {"kind": "quest", "id": 23, "title": "Kill ten", "min_level": 1,
         "objectives": [{"objective": "kill", "target_id": 448, "count": 10}]}]),
    ("vendors.items", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        {"kind": "vendor_item", "npc_id": 295, "item_id": 159},
        {"kind": "vendor_item", "npc_id": 295, "item_id": 77777},
        {"kind": "vendor_item", "npc_id": 4242, "item_id": 159}]),
    ("loot.creature", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        loot("creature", 448, -100.0, item=1931),
        loot("creature", 448, 30.0, item=2592, group=1),
        loot("creature", 448, 0.0, item=2589, group=1, mx=5),
        loot("creature", 448, 0.0, item=6085),
        loot("creature", 448, 5.0, reference=50, mx=2),
        loot("creature", 448, 2.0, item=872),
        loot("creature", 639, 10.0, item=6215, condition=3),
        loot("creature", 639, 20.0, reference=50, mx=2),
        loot("creature", 639, 10.0, reference=50, group=2)]),
    ("loot.game_object", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        loot("game_object", 2843, 5.0, item=6085), loot("game_object", 2843, 50.0, item=2589)]),
    ("loot.disenchant", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        loot("disenchant", 23, 100.0, item=2589)]),
    ("loot.reference", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        loot("reference", 50, 0.0, item=6085, group=1),
        loot("reference", 50, 0.0, item=6215, group=1),
        loot("reference", 50, 50.0, item=2589)]),
    ("dungeons.dungeons", "vanilla_1_12"): (CMANGOS + WAGO, "cross_edition_unverified", [
        {"kind": "dungeon", "map_id": 36, "name": "Deadmines", "min_level": 17, "max_level": 26,
         "entrance": {"status": "known", "value": world_of(0.25, 0.75)}, "bosses": [639, 12345],
         "area_id": 12},
        {"kind": "dungeon", "map_id": 34, "name": "Stormwind Stockade", "entrance": {"status": "unknown"},
         "bosses": []}]),
    ("trainers.spells", "vanilla_1_12"): (CMANGOS, "cross_edition_unverified", [
        {"kind": "trainer_spell", "npc_id": 240, "spell_id": 1}]),
}


def write_world(root, shards=SHARDS):
    for (dataset, edition), (sources, confidence, records) in shards.items():
        folder = root.joinpath(*dataset.split("."), edition, "unknown-build" if edition != "vanilla_1_12" else "1.12.1")
        folder.mkdir(parents=True, exist_ok=True)
        build = {"status": "known", "value": "1.12.1"} if edition == "vanilla_1_12" else {"status": "unknown"}
        doc = {"format": "navzeroth-world-data", "schema_version": 1, "dataset": dataset,
               "record_kind": records[0]["kind"], "origin": {"edition": edition, "build": build, "locale": "enUS"},
               "target": {"edition": "wow_forever", "build": {"status": "unknown"}}, "confidence": confidence,
               "sources": sources, "generator": {"name": "t", "version": "1", "credit": "t"},
               "shard": {"index": 0, "count": 1}, "records": records}
        (folder / "part-0000.json").write_text(json.dumps(doc, sort_keys=True))


class Run:
    def __init__(self, tmp, shards=SHARDS, policy=POLICY):
        root = pathlib.Path(tmp)
        write_world(root / "in", shards)
        (root / "target.json").write_text(json.dumps(TARGET))
        (root / "policy.json").write_text(json.dumps(policy))
        (root / "authored").mkdir()
        (root / "authored" / "catalog.json").write_text(json.dumps(AUTHORED))
        self.summary = w.run(root / "in", root / "target.json", root / "policy.json", root / "authored", root / "out")
        self.out = root / "out"
        self.catalog = json.loads((self.out / "catalog.json").read_text())
        self.sources = json.loads((self.out / "sources.json").read_text())
        self.conflicts = json.loads((self.out / "conflicts.json").read_text())["conflicts"]
        p = self.catalog["reference"]["payload"]
        self.p = p
        self.nodes = {n["id"]: n for n in p["nodes"]}
        self.edges = {e["id"]: e for e in p["edges"]}
        self.items = {i["key"]["item_id"]: i for i in p["items"]}
        self.acq = {a["id"]: a for a in p["acquisitions"]}
        self.facts = {a["acquisition"]: a for a in self.catalog["acquisition_facts"]}
        self.quests = {q["id"]: q for q in p["quests"]}
        self.qfacts = {q["quest"]: q for q in self.catalog["quest_facts"]}
        self.entities = {e["id"]: e for e in self.catalog["entities"]}
        self.loot = {t["id"]: t for t in self.catalog["loot_tables"]}


class TransformTest(unittest.TestCase):
    def test_hogger_on_elwynn(self):
        t = w.transform_of(ELWYNN)
        nx, ny = w.invert_transform(t, *HOGGER)
        self.assertAlmostEqual(nx, 0.271, places=3)
        self.assertAlmostEqual(ny, 0.868, places=3)
        y, x = w.apply_transform(t, nx, ny)
        self.assertAlmostEqual(x, HOGGER[0], places=2)
        self.assertAlmostEqual(y, HOGGER[1], places=2)
        self.assertEqual(t["units"], "yards")
        self.assertLess(t["scale_x"], 0)  # normalized x grows east = world -Y
        self.assertIsNone(w.transform_of({**ELWYNN, "ui_max": [0.0, 1.0]}))

    def test_path_length_and_spread(self):
        pts = [{"map_id": 0, "x": 0, "y": 0, "z": 0}, {"map_id": 0, "x": 3, "y": 4, "z": 0},
               {"map_id": 1, "x": 9, "y": 9, "z": 9}]
        self.assertEqual(w.path_length(pts), 5.0)
        rows = [({"guid": g, "world": {"x": float(g), "y": 0.0}}, "e") for g in (1, 2, 3, 10)]
        self.assertEqual([r[0]["guid"] for r in w.spread(rows, 2)], [1, 10])
        self.assertEqual(len(w.spread(rows, 9)), 4)

    def test_probability_rules(self):
        rows = [loot("creature", 1, 30.0, item=1, group=1), loot("creature", 1, 0.0, item=2, group=1),
                loot("creature", 1, 0.0, item=3, group=1), loot("creature", 1, 0.0, item=4),
                loot("creature", 1, 60.0, item=5, group=2), loot("creature", 1, 50.0, item=6, group=2),
                loot("creature", 1, 0.0, item=7, group=2), loot("creature", 1, -80.0, item=8)]
        shares = w.group_shares(rows)
        got = [w.row_probability(r, shares) for r in rows]
        self.assertEqual(got[0], w.known(0.3))
        self.assertEqual(got[1], w.known(0.35))
        self.assertEqual(got[3], w.UNKNOWN)  # ungrouped zero: unknown, never 0
        self.assertEqual(got[6], w.UNKNOWN)  # group already over 100%
        self.assertEqual(got[7], w.known(0.8))  # quest-only: |chance| / 100


class MappingTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.r = Run(cls.tmp.name)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_context_and_envelope(self):
        c = self.r.catalog
        self.assertEqual((c["format"], c["schema_version"]), ("navzeroth-reference-content", 2))
        ref = c["reference"]
        self.assertFalse(ref["fixture"])
        self.assertEqual(ref["game"], {"build": "1.60.1.70450", "edition": "wow_forever", "interface": "16001",
                                       "locale": "enUS"})
        ctx = {"build": {"build": "1.60.1.70450", "edition": "wow_forever", "interface": "16001"},
               "namespace": "reference"}
        self.assertTrue(all(e["context"] == ctx for e in self.r.p["evidence"]))
        self.assertTrue(ref["payload"]["revision"].startswith("navzeroth-world-1.60.1.70450-"))

    def test_map_space_transform(self):
        spaces = {m["key"]["map_id"]: m for m in self.r.p["map_spaces"]}
        self.assertEqual(spaces["1429"]["transform"]["status"], "known")
        self.assertEqual(spaces["1429"]["key"]["floor"], "0")
        self.assertEqual(spaces["instance:36"]["transform"], w.UNKNOWN)

    def test_nodes_and_entities(self):
        hogger = self.r.entities["npc:448"]
        self.assertEqual(len(hogger["nodes"]), w.OBJECTIVE_SPAWN_CAP)  # objective-only entity: capped
        self.assertEqual(self.r.nodes[hogger["nodes"][0]]["service"], w.NOT_APPLICABLE)
        self.assertEqual(hogger["subname"], w.UNKNOWN)  # absent subname stays unknown
        self.assertEqual(hogger["levels"], w.known([11, 11]))
        farley = self.r.entities["npc:295"]
        self.assertEqual(farley["services"], ["vendor", "other"])
        self.assertEqual(self.r.nodes[farley["nodes"][0]]["service"], w.known("vendor"))
        self.assertEqual(self.r.entities["npc:240"]["levels"], w.UNKNOWN)  # level 0 is not a level
        self.assertEqual(self.r.entities["npc:900"]["reaction"], w.UNKNOWN)
        self.assertEqual(self.r.entities["npc:900"]["nodes"], [])
        self.assertEqual(self.r.entities["object:68"]["levels"], w.NOT_APPLICABLE)
        self.assertIn("node:taxi:2", self.r.entities["npc:352"]["nodes"])
        self.assertEqual(self.r.nodes["node:taxi:2"]["service"], w.known("flight"))
        self.assertIn("node:graveyard:1", self.r.nodes)
        self.assertNotIn("node:graveyard:2", self.r.nodes)
        self.assertEqual(self.r.nodes["node:dungeon-entrance:36"]["service"], w.known("dungeon"))
        facts = {f["node"]: f["evidence"] for f in self.r.catalog["node_facts"]}
        self.assertEqual(set(facts), set(self.r.nodes))
        # A 1.12 spawn cites only its CMaNGOS evidence; the Forever map layout is named in the notes.
        self.assertEqual(facts["node:npc-spawn:5"], ["ev:cmangos-classic-db:vanilla_1_12:world.spawns"])

    def test_flight_and_walk_edges(self):
        flight = self.r.edges["edge:flight:6"]
        self.assertEqual(flight["expected_ms"], w.known(round((500 + 310) / w.FLIGHT_SPEED * 1000)))
        self.assertEqual(flight["status"], "unverified")
        self.assertEqual(flight["requirements"], w.UNKNOWN)
        self.assertIn("ev:navzeroth-world-content:inferred:travel.flight-time", flight["evidence"])
        self.assertNotIn("edge:flight:7", self.r.edges)
        walks = [e for e in self.r.edges.values() if e["mode"] == "walking"]
        self.assertTrue(walks)
        pairs = {(e["from"]["id"], e["to"]["id"]) for e in walks}
        self.assertTrue(all((b, a) in pairs for a, b in pairs))  # both directions
        for e in walks:
            self.assertEqual(e["requirements"], w.known(w.TRUE_PREDICATE))
            self.assertEqual(e["evidence"], ["ev:navzeroth-world-content:inferred:travel.walk"])
            self.assertEqual(e["wait_ms"], w.NOT_APPLICABLE)
        a, b = "node:npc-spawn:5", "node:npc-spawn:6"
        e = self.r.edges[f"edge:walk:{a}:{b}"]
        pa, pb = world_of(0.44, 0.66), world_of(0.4211, 0.6593)
        dist = math.hypot(pa["x"] - pb["x"], pa["y"] - pb["y"])
        self.assertAlmostEqual(e["expected_ms"]["value"], dist / w.RUN_SPEED * 1000, delta=1)
        # Objective spawn nodes join only their nearest hub, never each other.
        hogger = set(self.r.entities["npc:448"]["nodes"])
        self.assertFalse([p for p in pairs if p[0] in hogger and p[1] in hogger])
        for node in hogger:
            self.assertEqual(len([p for p in pairs if p[0] == node]), w.OBJECTIVE_NEIGHBOURS)

    def test_items_and_equipment(self):
        claw = self.r.items["1931"]
        self.assertEqual((claw["slots"], claw["two_handed"]), ([], w.NOT_APPLICABLE))
        tunic = self.r.items["6085"]
        self.assertEqual(tunic["slots"], ["chest"])
        self.assertEqual(tunic["attributes"], w.known({"stat:agility": 1, "stat:stamina": 2}))
        self.assertEqual(self.r.items["2589"]["attributes"], w.known({}))
        stick = self.r.items["6215"]
        self.assertEqual((stick["two_handed"], stick["requirements"]),
                         (w.known(True), w.known({"level": 5, "op": "level_at_least"})))
        self.assertEqual(tunic["requirements"], w.known(w.TRUE_PREDICATE))
        self.assertEqual(self.r.items["5000"]["requirements"], w.UNSUPPORTED)
        self.assertNotIn("5001", self.r.items)
        rock = self.r.items["872"]  # absent from the Forever table: the 1.12 item fills the gap
        self.assertEqual((rock["slots"], rock["two_handed"]), (["main_hand"], w.known(True)))
        self.assertEqual(rock["attributes"], w.known({"stat:strength": 7}))
        self.assertEqual(rock["evidence"], ["ev:cmangos-classic-db:vanilla_1_12:items.items"])
        self.assertEqual(tunic["evidence"], ["ev:wago-tools-wowf:wow_forever:items.items"])  # Forever first
        eq = {e["item"]["item_id"]: e for e in self.r.catalog["equipment"]}
        self.assertEqual(eq["1931"]["category"], w.known("non_equippable"))
        self.assertEqual(eq["6085"]["category"], w.known("item_class:4:2"))
        self.assertEqual(eq["6085"]["name"], "Footman Tunic")  # wow_forever outranks classic era

    def test_precedence_conflicts(self):
        facts = {c["fact"]: c for c in self.r.conflicts}
        c = facts["items.items:6085:name"]
        self.assertEqual((c["chosen"]["origin"], c["chosen"]["value"]), ("wow_forever", "Footman Tunic"))
        self.assertEqual(c["rejected"][0], {"origin": "wow_classic_era", "value": "Footman's Tunic"})
        self.assertEqual(facts["area:12:name"]["chosen"]["value"], "Elwynn Forest")
        title = facts["dungeons.dungeons:36:name"]
        self.assertEqual((title["chosen"]["origin"], title["chosen"]["value"]), ("authored", "The Deadmines"))
        self.assertEqual(facts["item:77777"]["kind"], "missing_item")
        self.assertEqual(facts["item:77777"]["rejected"][0]["value"], {"quest_reward": 1, "vendor": 1})
        filled = facts["item:872"]
        self.assertEqual(filled["kind"], "missing_in_higher_precedence")
        self.assertEqual(filled["chosen"], {"origin": "vanilla_1_12", "value": "Rockslicer"})
        self.assertEqual(filled["rejected"], [{"origin": "wow_forever", "value": None}])
        self.assertEqual(filled["references"], {"loot": 1})
        # Overlapping items: only meaningful fields are compared (not stats or prices).
        level = facts["items.items:6215:required_level"]
        self.assertEqual((level["chosen"], level["rejected"]), ({"origin": "wow_forever", "value": 5},
                                                                [{"origin": "vanilla_1_12", "value": 6}]))
        self.assertFalse([f for f in facts if f.startswith("items.items:6085:") and not f.endswith(":name")])
        self.assertIn("prerequisite_cycle", {x["kind"] for x in self.r.conflicts})

    def test_quests(self):
        q = self.r.quests["quest:176"]
        self.assertEqual(q["stages"], ["quest:176:objective:1"])
        self.assertEqual(q["eligibility"], w.known({"level": 5, "op": "level_at_least"}))
        self.assertEqual(q["prerequisites"], w.known(w.TRUE_PREDICATE))
        self.assertEqual([x["item_id"] for x in q["rewards"]["value"]], ["6085", "6215"])
        locations = q["locations"]["value"]
        self.assertEqual({(p["x"], p["y"]) for p in locations}, {(0.422, 0.66), (0.4211, 0.6593)})
        f = self.r.qfacts["quest:176"]
        self.assertEqual(f["access_requirements"]["value"]["races"],
                         ["race:dwarf", "race:gnome", "race:human", "race:night_elf"])
        self.assertIsNone(f["access_requirements"]["value"]["classes"])
        self.assertEqual(f["shareable"], w.known(True))
        self.assertEqual((f["title"], f["givers"], f["enders"]), ("Wanted: Hogger", ["object:68"], ["npc:240"]))
        orphan = self.r.quests["quest:2"]
        self.assertEqual(orphan["prerequisites"], w.UNKNOWN)  # names a quest outside the data
        self.assertEqual(orphan["eligibility"], w.known(w.TRUE_PREDICATE))  # min_level 0: no minimum
        self.assertEqual(orphan["stages"], ["quest:2:complete"])
        self.assertEqual(self.r.qfacts["quest:2"]["shareable"], w.known(False))
        self.assertEqual(self.r.qfacts["quest:2"]["access_requirements"]["value"]["classes"], ["class:warrior"])
        self.assertEqual(self.r.qfacts["quest:10"]["shareable"], w.UNKNOWN)
        loop = [self.r.quests[f"quest:{i}"]["prerequisites"]["value"] for i in (10, 11)]
        self.assertIn(w.TRUE_PREDICATE, loop)  # one edge of the cycle is dropped
        groups = self.r.qfacts["quest:20"]["exclusive_groups"]["value"]
        self.assertEqual(groups, [{"quests": ["quest:20", "quest:21"], "taken_when": "turned_in"}])
        self.assertEqual(self.r.qfacts["quest:23"]["exclusive_groups"], w.known([]))
        after = self.r.quests["quest:22"]["prerequisites"]["value"]
        self.assertEqual(after["predicates"][0], {"op": "quest_state", "quest": "quest:20", "state": "accepted",
                                                  "value": True})
        self.assertEqual(after["predicates"][1]["op"], "any")

    def test_vendor_and_quest_acquisitions(self):
        a = self.r.acq["vendor:295:159"]
        self.assertEqual((a["kind"], a["cost"], a["probability"]), ("vendor", w.known("25"), w.known(1.0)))
        origin = self.r.facts["vendor:295:159"]["origin"]["value"]
        self.assertEqual((origin["kind"], origin["entity"], origin["location"]["id"]),
                         ("vendor", "npc:295", "node:npc-spawn:5"))
        self.assertNotIn("vendor:295:77777", self.r.acq)
        self.assertNotIn("vendor:4242:159", self.r.acq)
        reward = self.r.facts["quest-reward:176"]
        self.assertEqual(reward["origin"], w.known({"kind": "quest", "quest": "quest:176"}))
        self.assertEqual({r["choice_group"] for r in reward["rewards"]["value"]}, {"quest:176:choice"})
        self.assertEqual(self.r.acq["quest-reward:176"]["probability"], w.NOT_APPLICABLE)
        self.assertEqual(self.r.acq["quest-reward:2"]["probability"], w.known(1.0))
        self.assertEqual(self.r.facts["quest-reward:2"]["rewards"]["value"][0]["quantity"], 5)

    def test_drops_and_bounded_reference_loot(self):
        quest = self.r.acq["drop:creature:448:1931"]
        self.assertEqual(quest["probability"], w.known(1.0))
        self.assertEqual(quest["conditions"], w.known({"op": "quest_state", "quest": "quest:176",
                                                       "state": "accepted", "value": True}))
        self.assertEqual(self.r.acq["drop:creature:448:6085"]["probability"], w.UNKNOWN)
        self.assertNotIn("drop:creature:448:2589", self.r.acq)  # not gear, quest or boss loot
        self.assertEqual(self.r.facts["drop:creature:448:6085"]["origin"],
                         w.known({"dungeon": None, "entity": "npc:448", "kind": "drop"}))
        # Non-boss reference rows stay one row naming the table: no expansion.
        self.assertFalse([k for k in self.r.acq if k.startswith("drop:creature:448:ref:")])
        rows = self.r.loot["loot:creature:448"]["rows"]
        self.assertIn({"max_count": 2, "min_count": 1, "probability": 0.05, "table": "loot:reference:50"}, rows)
        cloth = next(r for r in rows if r.get("item") == "2589")
        self.assertEqual((cloth["probability"], cloth["group"], cloth["max_count"]), (0.7, 1, 5))
        self.assertNotIn("probability", next(r for r in rows if r.get("item") == "6085"))
        self.assertTrue(next(r for r in rows if r.get("item") == "1931")["quest_only"])
        self.assertIn({"item": "872", "max_count": 1, "min_count": 1, "probability": 0.02}, rows)
        self.assertEqual(self.r.loot["loot:creature:448"]["source"], w.known("npc:448"))
        self.assertEqual(self.r.loot["loot:reference:50"]["source"], w.UNKNOWN)
        # Game object loot is keyed by the object; disenchant IDs name no single item.
        self.assertEqual(self.r.loot["loot:game_object:2843"]["source"], w.known("object:2843"))
        self.assertEqual(self.r.facts["drop:game_object:2843:6085"]["origin"],
                         w.known({"dungeon": None, "entity": "object:2843", "kind": "drop"}))
        self.assertEqual(self.r.loot["loot:disenchant:23"]["source"], w.UNKNOWN)
        self.assertIn("drop:creature:448:872", self.r.acq)  # the 1.12 item makes the drop resolvable
        # Boss: one-level expansion, occurrences combined.
        boss = self.r.acq["drop:creature:639:ref:50:6085"]
        p1 = 1 - (1 - 0.2 * 0.5) ** 2
        p2 = 0.1 * 0.5
        self.assertAlmostEqual(boss["probability"]["value"], 1 - (1 - p1) * (1 - p2), places=6)
        self.assertEqual(boss["kind"], "dungeon")
        self.assertIn("ev:navzeroth-world-content:inferred:loot.reference-expansion", boss["evidence"])
        self.assertEqual(self.r.acq["drop:creature:639:6215"]["conditions"], w.UNSUPPORTED)
        self.assertEqual(self.r.facts["drop:creature:639:6215"]["origin"]["value"]["dungeon"],
                         "dungeon:the-deadmines")

    def test_dungeons(self):
        d = {x["id"]: x for x in self.r.catalog["dungeons"]}
        dm = d["dungeon:the-deadmines"]
        self.assertEqual(dm["title"], "The Deadmines")
        self.assertEqual(dm["bosses"], ["npc:639"])  # unknown boss 12345 is not an entity
        self.assertEqual(dm["entrance"]["value"]["id"], "node:dungeon-entrance:36")
        self.assertEqual(dm["map"]["map_id"], "instance:36")
        self.assertIn("drop:creature:639:6215", dm["rewards"]["value"])
        self.assertEqual(dm["preparation_quests"], w.known(["quest:176"]))
        stocks = d["dungeon:stormwind-stockade"]
        self.assertNotIn("bosses", stocks)
        self.assertEqual((stocks["entrance"], stocks["preparation_quests"], stocks["rewards"]),
                         (w.UNKNOWN, w.UNKNOWN, w.known([])))

    def test_evidence_and_sources(self):
        ev = {e["id"]: e for e in self.r.p["evidence"]}
        npc = ev["ev:cmangos-classic-db:vanilla_1_12:world.npcs"]
        self.assertEqual(npc["confidence"], "cross_edition_unverified")
        self.assertEqual(npc["observed_at_utc_ms"], w.known(1700000000000))
        self.assertIn("Origin vanilla_1_12 build 1.12.1", npc["notes"])
        self.assertIn("world/npcs/vanilla_1_12/1.12.1/part-0000.json", npc["notes"])
        self.assertEqual(ev["ev:wago-tools-wowf:wow_forever:items.items"]["observed_at_utc_ms"], w.UNKNOWN)
        self.assertEqual(ev["ev:navzeroth-world-content:inferred:travel.walk"]["confidence"], "inferred")
        src = {s["id"]: s for s in self.r.sources["sources"]}
        self.assertEqual(src["cmangos-classic-db"]["editions"], w.known(["vanilla_1_12", "wow_forever"]))
        self.assertEqual(src["wago-tools-wowf"]["editions"], w.known(["wow_forever"]))
        self.assertNotIn("wago-tools-classic-era", src)  # no kept record cites the outranked origin
        # A source without editions is refused by the release gate (V2.8 review): it lists the target.
        self.assertEqual(src["navzeroth-world-content"]["editions"], w.known(["wow_forever"]))
        # Every evidence record cites a source of its own origin: [origin, pack edition].
        for e in self.r.p["evidence"]:
            if e["source"] == "navzeroth-world-content":
                continue
            origin = e["id"].split(":")[2]
            self.assertEqual(src[e["source"]]["editions"], w.known(sorted({origin, "wow_forever"})), e["id"])
        self.assertNotIn("ev:wago-tools-wowf:vanilla_1_12:world.spawns", ev)
        spawns = ev["ev:cmangos-classic-db:vanilla_1_12:world.spawns"]["notes"]
        self.assertIn("also read wago-tools-wowf (https://wago.tools/db2?product=wowf revision 1.60.1.70245", spawns)
        self.assertEqual(src["wago-tools-wowf"]["allowed_fields"]["value"],
                         sorted(["items.items", "travel.taxi_nodes", "travel.taxi_paths",
                                 "world.ui_map_assignments", "world.ui_maps"]))
        self.assertEqual(src["navzeroth-world-content"]["redistribution"], w.known("permitted"))
        self.assertEqual(src["cmangos-classic-db"]["last_retrieval_utc_ms"], w.known(1700000000000))

    def test_summary(self):
        s = self.r.summary
        self.assertEqual(s["not_mapped"], {"trainers.spells": 1})
        self.assertEqual(s["dropped"]["vendor row for an NPC outside world.npcs"], 1)
        self.assertEqual(s["counts"]["entities"], 8)
        self.assertEqual(s["parameters"]["run_speed_yards_per_s"], w.RUN_SPEED)
        self.assertEqual(s["catalog_bytes"], len((self.r.out / "catalog.json").read_bytes()))


class RunTest(unittest.TestCase):
    def test_deterministic_bytes(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            first, second = Run(a), Run(b)
            for name in ("catalog.json", "sources.json", "conflicts.json", "summary.json"):
                self.assertEqual((first.out / name).read_bytes(), (second.out / name).read_bytes(), name)

    def test_windows_text_newline_translation_cannot_change_mapping_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            first = Run(directory)
            root = pathlib.Path(directory)
            original = pathlib.Path.write_text
            def windows_text(path, value, **options):
                options["newline"] = "\r\n"
                return original(path, value, **options)
            with mock.patch.object(pathlib.Path, "write_text", windows_text):
                w.run(root / "in", root / "target.json", root / "policy.json", root / "authored", root / "windows")
            for name in ("catalog.json", "sources.json", "conflicts.json", "summary.json"):
                expected = (first.out / name).read_bytes()
                self.assertNotIn(b"\r\n", expected)
                self.assertEqual(expected, (root / "windows" / name).read_bytes(), name)

    def test_policy_skips_unaccepted_origins(self):
        policy = json.loads(json.dumps(POLICY))
        policy["domains"]["equipment"] = ["vanilla_1_12"]
        with tempfile.TemporaryDirectory() as tmp:
            r = Run(tmp, policy=policy)
            self.assertEqual(r.summary["skipped_origins"][0]["origin"], "wow_classic_era")
            self.assertNotIn("items.items:6085:name", {c["fact"] for c in r.conflicts})

    def test_lfs_pointer_and_bad_files_are_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / "x").mkdir()
            (root / "x" / "part-0000.json").write_text("version https://git-lfs.github.com/spec/v1\n")
            with self.assertRaises(w.InputError):
                w.load_input(root)
            with self.assertRaises(w.InputError):
                w.load_input(root / "missing")
            with self.assertRaises(w.InputError):
                w.read_json(root / "nope.json", "target")
            (root / "x" / "part-0000.json").write_text("{}")
            (root / "t.json").write_text(json.dumps({"format": "other"}))
            self.assertEqual(w.main(["--input", str(root), "--target", str(root / "t.json"), "--policy",
                                     str(root / "t.json"), "--out", str(root / "o")]), 1)

    def test_main_prints_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            write_world(root / "in")
            (root / "target.json").write_text(json.dumps(TARGET))
            (root / "policy.json").write_text(json.dumps(POLICY))
            printed = io.StringIO()
            with contextlib.redirect_stdout(printed):
                code = w.main(["--input", str(root / "in"), "--target", str(root / "target.json"),
                               "--policy", str(root / "policy.json"), "--out", str(root / "o")])
            self.assertEqual(code, 0)
            self.assertIn('"revision"', printed.getvalue())
            d = {x["id"]: x for x in json.loads((root / "o" / "catalog.json").read_text())["dungeons"]}
            self.assertIn("dungeon:deadmines", d)  # without authored input the source title stands


class ProvenanceRevisionTests(unittest.TestCase):
    def test_versioned_policy_compatibility(self):
        policy = json.loads(json.dumps(POLICY))
        policy["schema_version"] = 2
        policy["historical_builds"] = {"wow_forever": ["1.60.1.70245"]}
        with tempfile.TemporaryDirectory() as tmp:
            Run(tmp, policy=policy)
        for version in (1, 3):
            policy["schema_version"] = version
            with tempfile.TemporaryDirectory() as tmp, self.assertRaises(w.InputError):
                Run(tmp, policy=policy)

    def test_revision_changes_for_policy_authored_target_and_conversion(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            initial = Run(tmp)
            baseline = initial.summary["revision"]
            def run_again():
                return w.run(root / "in", root / "target.json", root / "policy.json",
                             root / "authored", root / "out")["revision"]
            self.assertEqual(run_again(), baseline)
            for filename, value in [
                ("policy.json", dict(POLICY, accept_unknowns=False)),
                ("target.json", dict(TARGET, locale="deDE")),
                ("authored/catalog.json", {"dungeons": []}),
            ]:
                path = root / filename
                previous = path.read_bytes()
                path.write_text(json.dumps(value))
                self.assertNotEqual(run_again(), baseline, filename)
                path.write_bytes(previous)
            old = w.RUN_SPEED
            try:
                w.RUN_SPEED = old + 1
                self.assertNotEqual(run_again(), baseline)
            finally:
                w.RUN_SPEED = old
            changed = json.loads(json.dumps(POLICY))
            changed["domains"]["equipment"] = ["wow_classic_era"]
            (root / "policy.json").write_text(json.dumps(changed))
            self.assertNotEqual(run_again(), baseline)

    def test_multiple_builds_cannot_collapse_into_one_evidence_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            Run(tmp)
            root = pathlib.Path(tmp)
            path = next((root / "in/items/items/wow_forever").rglob("*.json"))
            shard = json.loads(path.read_text())
            shard["origin"]["build"] = w.known("1.60.1.70245")
            path.with_name("part-0001.json").write_text(json.dumps(shard))
            with self.assertRaisesRegex(w.InputError, "multiple origin builds"):
                w.run(root / "in", root / "target.json", root / "policy.json", root / "authored", root / "out")

    def test_true_origin_build_survives_target_mapping(self):
        with tempfile.TemporaryDirectory() as tmp:
            Run(tmp)
            root = pathlib.Path(tmp)
            path = next((root / "in/items/items/wow_forever").rglob("*.json"))
            shard = json.loads(path.read_text())
            shard["origin"]["build"] = w.known("1.60.1.70245")
            path.write_text(json.dumps(shard))
            w.run(root / "in", root / "target.json", root / "policy.json", root / "authored", root / "out")
            catalog = json.loads((root / "out/catalog.json").read_text())
            evidence = catalog["reference"]["payload"]["evidence"]
            old = next(e for e in evidence if e["id"] == "ev:wago-tools-wowf:wow_forever:items.items")
            self.assertEqual(old["origin"], {"edition":"wow_forever","build":w.known("1.60.1.70245")})
            self.assertEqual(old["context"]["build"]["build"], TARGET["build"])
            self.assertEqual(old["confidence"], "cross_edition_unverified")
            vanilla = next(e for e in evidence if e["source"] == "cmangos-classic-db")
            self.assertEqual(vanilla["origin"], {"edition":"vanilla_1_12","build":w.known("1.12.1")})
            shard["origin"]["build"] = w.known(TARGET["build"])
            path.write_text(json.dumps(shard))
            w.run(root / "in", root / "target.json", root / "policy.json", root / "authored", root / "out")
            evidence = json.loads((root / "out/catalog.json").read_text())["reference"]["payload"]["evidence"]
            exact = next(e for e in evidence if e["id"] == old["id"])
            self.assertEqual(exact["confidence"], "publisher_documented")


if __name__ == "__main__":
    unittest.main()

