#!/usr/bin/env python3
"""Generated world data shards -> one content v2 catalog for the whole world.

Task V2.8d (ADR 0007 §2-§4, §7, §8). Reads the `navzeroth-world-data` shards under
--input, labels every record with the target pack context from --target, applies
the accepted-origin policy and precedence from --policy, joins the authored slice
(--authored) where it states the same fact, and writes into --out:

    catalog.json    navzeroth-reference-content schema 2 (input of build-content-v2)
    sources.json    the pack source registry; `editions` is each source's true origin
    conflicts.json  every disagreement between inputs and how precedence settled it
    summary.json    counts, dropped records with reasons, parameters and digests

The mapping, IDs, speeds, precedence and limits are documented in
docs/data/WORLD_CONTENT.md. Output is byte-identical for identical input: every
collection is sorted, JSON keys are sorted and no clock or host path is read.

    python3 data-generators/world_to_content.py --input data-generators/output \
        --target release/v1/world-data-target.json --policy release/v1/accepted-origins.json \
        --authored content/authored/slice-v1 --out content/world/1.60.1.70450

Standard library only. Imported data is parsed as JSON and never executed.
"""
import argparse
import collections
import hashlib
import json
import math
import pathlib
import re
import sys

NAME = "world_to_content"
VERSION = "2"
DERIVED_SOURCE = "navzeroth-world-content"

# Inferred travel (ADR 0007 §8). Yards per second.
RUN_SPEED = 7.0  # unmounted run speed
FLIGHT_SPEED = 31.0  # taxi gryphon / wind rider cruise speed, a commonly quoted figure
WALK_NEIGHBOURS = 4  # k nearest nodes per node on one uiMap
WALK_MAX_YARDS = 1000.0  # no inferred walk edge is longer than this
OBJECTIVE_SPAWN_CAP = 3  # spawn nodes kept per quest-objective-only entity
OBJECTIVE_NEIGHBOURS = 1  # an objective spawn node joins only its nearest service/quest node
MAX_REFS = 1024  # core `bounded` limit for lists inside one record
MAX_QUEST_LOCATIONS = 64
MAX_LOOT_ROWS = 4096  # content_v2::MAX_LOOT_ROWS

DOMAINS = {
    "world": "geography", "travel": "travel", "quests": "quests", "items": "equipment",
    "vendors": "acquisition", "loot": "acquisition", "fishing": "acquisition",
    "dungeons": "dungeons", "trainers": "class_roles", "spells": "class_roles",
    "talents": "class_roles", "professions": "acquisition", "enhancements": "equipment",
}
# Datasets this mapping reads; others are listed in the summary as not mapped.
MAPPED = {
    "world.npcs", "world.game_objects", "world.spawns", "world.graveyards", "world.areas",
    "world.ui_maps", "world.ui_map_assignments", "world.area_levels", "travel.taxi_nodes",
    "travel.taxi_paths", "quests.quests", "items.items", "vendors.items", "dungeons.dungeons",
} | {f"loot.{t}" for t in ("creature", "game_object", "item", "reference", "fishing",
                             "pickpocketing", "skinning", "disenchant")}

# 1.12 ChrRaces / ChrClasses mask bits.
RACES = {1: "human", 2: "orc", 4: "dwarf", 8: "night_elf", 16: "undead", 32: "tauren",
         64: "gnome", 128: "troll"}
CLASSES = {1: "warrior", 2: "paladin", 4: "hunter", 8: "rogue", 16: "priest", 64: "shaman",
           128: "mage", 256: "warlock", 1024: "druid"}
ALL_RACES = sum(RACES)
ALL_CLASSES = sum(CLASSES)
QUEST_FLAG_SHARABLE = 0x8

# InventoryType -> equipment slots; same mapping as core `inventory_type_slots`.
SLOTS = {
    1: ["head"], 2: ["neck"], 3: ["shoulder"], 4: ["shirt"], 5: ["chest"], 20: ["chest"],
    6: ["waist"], 7: ["legs"], 8: ["feet"], 9: ["wrist"], 10: ["hands"],
    11: ["finger1", "finger2"], 12: ["trinket1", "trinket2"], 13: ["main_hand", "off_hand"],
    14: ["off_hand"], 22: ["off_hand"], 23: ["off_hand"], 15: ["ranged"], 25: ["ranged"],
    26: ["ranged"], 28: ["ranged"], 16: ["back"], 17: ["main_hand"], 21: ["main_hand"],
    18: ["bag"], 27: ["bag"], 19: ["tabard"], 24: ["ammo"],
}
TWO_HANDED = {17}
# Item fields whose disagreement between origins is reported. Stats, prices and masks are
# not: Forever stats are computed from budgets without armor or ratings and differ by design.
ITEM_COMPARE = {"name", "quality", "class_id", "subclass_id", "inventory_type", "item_level", "required_level"}
ENTITY_PREFIX = {"npc": "npc", "game_object": "object", "item": "item"}

ROLE_SERVICE = {
    "vendor": "vendor", "repair": "repair", "trainer": "trainer", "class_trainer": "trainer",
    "profession_trainer": "trainer", "flight_master": "flight",
}
SERVICE_ORDER = ["flight", "dungeon", "trainer", "vendor", "repair", "other"]
# UiMap.Type ranks: zones first when locating a world point.
TYPE_RANK = {3: 0, 4: 1, 5: 2, 6: 3, 2: 4, 1: 5, 0: 6}
# Loot tables whose entry is, by the CMaNGOS classic-db convention, the creature
# (LootId / PickpocketLootId / SkinningLootId = Entry) or the item it belongs to; the
# generator keys game object loot by the chest or fishing hole whose data1 names it.
# Disenchant loot is keyed by DisenchantID, which several items share: no entity.
LOOT_SOURCE = {"creature": "npc", "pickpocketing": "npc", "skinning": "npc", "item": "item",
               "fishing": "area", "game_object": "object"}


class InputError(Exception):
    pass


# ------------------------------------------------------------------ observations

def known(value):
    return {"status": "known", "value": value}


def status(name):
    return {"status": name}


UNKNOWN = status("unknown")
NOT_APPLICABLE = status("not_applicable")
UNSUPPORTED = status("unsupported")
TRUE_PREDICATE = {"op": "all", "predicates": []}


def natural(text):
    """Sort key that orders embedded numbers numerically (quest:2 before quest:10)."""
    return [(0, int(p), "") if p.isdigit() else (1, 0, p) for p in re.split(r"(\d+)", str(text))]


def rnd(value, places=6):
    out = round(float(value), places)
    return 0.0 if out == 0 else out


# ------------------------------------------------------------------ input

class Shard:
    def __init__(self, path, rel, doc):
        self.path = path
        self.rel = rel
        self.dataset = doc["dataset"]
        self.edition = doc["origin"]["edition"]
        build = doc["origin"].get("build") or {}
        self.build = build.get("value") if build.get("status") == "known" else None
        self.confidence = doc["confidence"]
        self.sources = doc.get("sources", [])
        self.records = doc["records"]


def load_input(root):
    """Shards grouped as {dataset: {edition: [Shard, ...]}} and a digest of the input."""
    if not root.is_dir():
        raise InputError(f"input is not a directory: {root}")
    shards = []
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*.json"), key=lambda p: p.relative_to(root).as_posix()):
        rel = path.relative_to(root).as_posix()
        parts = rel.split("/")
        if path.name == "run-manifest.json" or "facts" in parts[:-1] or any(p.startswith(".") for p in parts):
            continue
        if path.is_symlink():
            raise InputError(f"symbolic link in input: {rel}")
        raw = path.read_bytes()
        try:
            doc = json.loads(raw)
        except ValueError as error:
            raise InputError(f"{rel}: not JSON ({error}); is Git LFS checked out?") from None
        if not isinstance(doc, dict) or doc.get("format") != "navzeroth-world-data":
            continue
        digest.update(rel.encode() + b"\0" + hashlib.sha256(raw).hexdigest().encode() + b"\n")
        shards.append(Shard(path, rel, doc))
    grouped = collections.defaultdict(lambda: collections.defaultdict(list))
    for shard in shards:
        grouped[shard.dataset][shard.edition].append(shard)
    return grouped, digest.hexdigest()


def read_json(path, what):
    try:
        return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise InputError(f"cannot read {what} {path}: {error}") from None


# ------------------------------------------------------------------ builder

class Builder:
    def __init__(self, shards, digest, target, policy, authored):
        if target.get("format") != "navzeroth-world-data-target":
            raise InputError("target file is not navzeroth-world-data-target")
        if policy.get("format") != "navzeroth-accepted-origins":
            raise InputError("policy file is not navzeroth-accepted-origins")
        if policy.get("schema_version") not in (1, 2):
            raise InputError("unsupported accepted-origin policy version")
        if policy["schema_version"] == 1 and policy.get("historical_builds"):
            raise InputError("historical build grants require policy schema 2")
        self.target = target
        self.policy = policy
        self.authored = authored
        self.input_digest = digest
        self.context = {"build": {"build": target["build"], "edition": target["edition"],
                                  "interface": target["interface"]}, "namespace": "reference"}
        self.rank = {name: i for i, name in enumerate(policy["precedence"])}
        self.conflicts = []
        self.dropped = collections.Counter()
        self.unmapped = collections.Counter()
        self.skipped_origins = []
        self.evidence = {}  # id -> record
        self.evidence_sources = collections.defaultdict(set)  # source id -> datasets
        self.source_refs = {}  # source id -> first shard source ref
        self.shards = shards
        self.used = {}  # dataset -> chosen editions in precedence order
        for dataset in sorted(shards):
            accepted = []
            domain = DOMAINS.get(dataset.split(".")[0])
            allowed = set(policy.get("domains", {}).get(domain, [])) | {policy.get("target_edition")}
            for edition in sorted(shards[dataset], key=self.edition_rank):
                if edition not in allowed:
                    self.skipped_origins.append({"dataset": dataset, "origin": edition,
                                                 "reason": f"origin not accepted for domain {domain}"})
                    continue
                accepted.append(edition)
            self.used[dataset] = accepted
            if dataset not in MAPPED:
                self.unmapped[dataset] += sum(len(s.records) for e in accepted for s in shards[dataset][e])

    def edition_rank(self, edition):
        return (self.rank.get(edition, len(self.rank)), edition)

    # ---- records with precedence

    def records(self, dataset, key=None, compare=None):
        """[(record, edition)] for one dataset. With `key`, one record per key: the
        highest-precedence origin wins and every differing field of a lower one is a conflict
        (with `compare`, only differences in those fields are reported)."""
        out = []
        for edition in self.used.get(dataset, []):
            for shard in self.shards[dataset][edition]:
                for record in shard.records:
                    out.append((record, edition))
        if key is None:
            return out
        chosen = {}
        for record, edition in out:
            k = key(record)
            if k not in chosen:
                chosen[k] = (record, edition)
                continue
            best, best_edition = chosen[k]
            if best_edition == edition:
                self.dropped[f"{dataset}: duplicate key within one origin"] += 1
                continue
            for field in sorted(set(best) | set(record)):
                if compare is not None and field not in compare:
                    continue
                if field != "kind" and best.get(field) != record.get(field):
                    self.conflict(f"{dataset}:{k}:{field}", best_edition, best.get(field),
                                  edition, record.get(field))
        return [chosen[k] for k in sorted(chosen, key=natural)]

    def conflict(self, fact, chosen_origin, chosen, other_origin, other, kind="value_disagreement"):
        self.conflicts.append({"fact": fact, "kind": kind,
                               "chosen": {"origin": chosen_origin, "value": chosen},
                               "rejected": [{"origin": other_origin, "value": other}],
                               "rule": "precedence " + " > ".join(self.policy["precedence"])})

    def evidence_for(self, dataset, edition):
        """Evidence IDs for records of one dataset and origin: one per upstream source of the
        record's origin edition (origin_sources). A shard's other sources (the Forever client
        tables that place 1.12 spawns or name 1.12 areas) are named in the notes, never cited
        as the source, so a source's editions always read [true origin, pack edition]."""
        ids = []
        for shard in self.shards[dataset][edition]:
            own, aux = origin_sources(shard.sources, edition)
            for ref in own:
                eid = f"ev:{ref['id']}:{edition}:{dataset}"
                if eid not in self.evidence:
                    self.evidence[eid] = {"dataset": dataset, "edition": edition, "source": ref["id"],
                                          "confidence": shard.confidence, "builds": set(),
                                          "paths": [], "refs": [], "retrieved": [], "aux": []}
                ev = self.evidence[eid]
                if shard.rel not in ev["paths"]:
                    ev["paths"].append(shard.rel)
                    ev["builds"].add(shard.build or "unknown")
                    if len(ev["builds"]) > 1:
                        raise InputError(f"{eid}: multiple origin builds require separate evidence")
                    ev["refs"].append(ref)
                    ev["aux"].extend(a for a in aux if a not in ev["aux"])
                    if "retrieved_at_utc_ms" in ref:
                        ev["retrieved"].append(ref["retrieved_at_utc_ms"])
                self.evidence_sources[ref["id"]].add(dataset)
                self.source_refs.setdefault(ref["id"], ref)
                if eid not in ids:
                    ids.append(eid)
        return sorted(ids)

    def inferred(self, rule, notes):
        eid = f"ev:{DERIVED_SOURCE}:inferred:{rule}"
        self.evidence.setdefault(eid, {"inferred": True, "source": DERIVED_SOURCE, "rule": rule,
                                       "notes": notes})
        self.evidence_sources[DERIVED_SOURCE].add(rule)
        return eid

    # ---- keys

    def map_key(self, map_id):
        return {"context": self.context, "floor": "0", "instance": None, "map_id": str(map_id)}

    def item_key(self, item_id):
        return {"context": self.context, "item_id": str(item_id), "variant": "base"}

    def node_ref(self, node_id):
        return {"id": node_id, "map": self.map_key(self.node_map[node_id])}

    # ---- build

    def build(self):
        self.build_maps()
        self.build_entities_and_nodes()
        self.build_entrances()
        self.build_edges()
        self.build_items()
        self.build_quests()
        self.build_acquisitions()
        self.build_loot_tables()
        self.build_dungeons()
        self.compare_areas()
        return self.assemble()

    # map spaces ------------------------------------------------------------

    def build_maps(self):
        self.ui_types = {}
        self.map_spaces = {}
        ui_maps = self.records("world.ui_maps", key=lambda r: r["id"])
        assigns = collections.defaultdict(list)
        self.assignments = []
        assign_ev = []
        for record, edition in self.records("world.ui_map_assignments"):
            assigns[record["ui_map_id"]].append(record)
            self.assignments.append(record)
            assign_ev = self.evidence_for("world.ui_map_assignments", edition)
        self.ui_map_ids = set()
        for record, edition in ui_maps:
            self.ui_types[record["id"]] = record.get("map_type")
            self.ui_map_ids.add(record["id"])
            ev = self.evidence_for("world.ui_maps", edition)
            self.add_map_space(record["id"], assigns.get(record["id"], []), ev, assign_ev)
        self.assign_by_ui = assigns
        self.assign_ev = assign_ev

    def add_map_space(self, ui_map_id, assignments, ev, assign_ev):
        transform = UNKNOWN
        distinct = {json.dumps([a["region_min"], a["region_max"], a["ui_min"], a["ui_max"]])
                    for a in assignments}
        evidence = list(ev)
        if len(distinct) == 1:
            t = transform_of(assignments[0])
            if t is not None:
                transform = known(t)
                evidence = sorted(set(ev) | set(assign_ev))
        elif len(distinct) > 1:
            self.dropped["map transform unknown: uiMap spans several map assignments"] += 1
        self.map_spaces[str(ui_map_id)] = {"evidence": evidence, "key": self.map_key(ui_map_id),
                                           "transform": transform}

    def ensure_ui_map(self, ui_map_id):
        if str(ui_map_id) not in self.map_spaces:
            self.add_map_space(ui_map_id, self.assign_by_ui.get(ui_map_id, []),
                               self.assign_ev, self.assign_ev)
            self.dropped["map space added for a uiMap missing from world.ui_maps"] += 1
        return str(ui_map_id)

    def locate(self, map_id, x, y):
        """A world point on its most specific uiMap: (ui_map_id, nx, ny) or None."""
        hits = []
        for a in self.assignments:
            lo, hi = a["region_min"], a["region_max"]
            if a["map_id"] == map_id and lo[0] <= x <= hi[0] and lo[1] <= y <= hi[1]:
                rank = TYPE_RANK.get(self.ui_types.get(a["ui_map_id"]), 9)
                hits.append((rank, (hi[0] - lo[0]) * (hi[1] - lo[1]), a["ui_map_id"], a))
        if not hits:
            return None
        _, _, ui, a = min(hits, key=lambda h: h[:3])
        (lo, hi), (u0, v0), (u1, v1) = (a["region_min"], a["region_max"]), a["ui_min"], a["ui_max"]
        nx = u0 + (u1 - u0) * (hi[1] - y) / (hi[1] - lo[1])
        ny = v0 + (v1 - v0) * (hi[0] - x) / (hi[0] - lo[0])
        if not (0 <= nx <= 1 and 0 <= ny <= 1):
            return None
        return ui, round(nx, 4), round(ny, 4)

    # entities and nodes ------------------------------------------------------

    def build_entities_and_nodes(self):
        npcs = self.records("world.npcs", key=lambda r: r["id"])
        objects = self.records("world.game_objects", key=lambda r: r["id"])
        self.npcs = {r["id"]: (r, e) for r, e in npcs}
        self.objects = {r["id"]: (r, e) for r, e in objects}
        quests = self.records("quests.quests", key=lambda r: r["id"])
        self.quests = {r["id"]: (r, e) for r, e in quests}

        # Which entities get travel nodes, and how many of their spawns.
        full, capped = set(), set()
        for nid, (npc, _) in self.npcs.items():
            if npc.get("roles"):
                full.add(("npc", nid))
        objective_items = collections.defaultdict(set)  # item -> quests needing it
        for qid, (quest, _) in self.quests.items():
            for ref in quest.get("givers", []) + quest.get("enders", []):
                if ref["entity"] in ("npc", "game_object"):
                    full.add((ref["entity"], ref["id"]))
            for obj in quest.get("objectives", []):
                target = obj.get("target_id")
                if target is None:
                    continue
                if obj["objective"] == "kill":
                    capped.add(("npc", target))
                elif obj["objective"] == "object":
                    capped.add(("game_object", target))
                elif obj["objective"] == "item":
                    objective_items[target].add(qid)
        self.objective_items = objective_items
        for table in ("creature", "game_object"):
            for row, _ in self.records(f"loot.{table}"):
                if table == "creature" and row.get("chance", 0) < 0 and row.get("item") in objective_items:
                    capped.add(("npc", row["entry"]))
        capped -= full

        spawns = collections.defaultdict(list)  # (entity kind, id) -> spawn records
        spawn_ev = {}
        for record, edition in self.records("world.spawns"):
            spawn_ev[edition] = self.evidence_for("world.spawns", edition)
            if record["position"]["status"] != "known":
                self.dropped[f"spawn position {record['position']['status']}"] += 1
                continue
            spawns[(record["entity"], record["entity_id"])].append((record, edition))

        self.nodes = {}
        self.node_map = {}
        self.node_world = {}
        self.node_facts = {}
        self.node_services = collections.defaultdict(set)
        self.entity_nodes = collections.defaultdict(set)
        for key in sorted(full | capped, key=lambda k: (k[0], k[1])):
            rows = sorted(spawns.get(key, []), key=lambda s: s[0]["guid"])
            if not rows:
                continue
            if key in capped:
                rows = spread(rows, OBJECTIVE_SPAWN_CAP)
            elif len(rows) > MAX_REFS:
                self.dropped["entity spawns beyond 1024 nodes"] += len(rows) - MAX_REFS
                rows = rows[:MAX_REFS]
            kind = "npc" if key[0] == "npc" else "object"
            for spawn, edition in rows:
                pos = spawn["position"]["value"]
                node = f"node:{kind}-spawn:{spawn['guid']}"
                self.add_node(node, pos["ui_map_id"], pos["x"], pos["y"], spawn["world"], spawn_ev[edition])
                self.entity_nodes[(key[0], key[1])].add(node)
                self.node_services[node] |= self.entity_services(key, objective=key in capped)

        for record, edition in self.records("travel.taxi_nodes", key=lambda r: r["id"]):
            ev = self.evidence_for("travel.taxi_nodes", edition)
            if record["position"]["status"] != "known":
                self.dropped[f"taxi node position {record['position']['status']}"] += 1
                continue
            pos = record["position"]["value"]
            node = f"node:taxi:{record['id']}"
            self.add_node(node, pos["ui_map_id"], pos["x"], pos["y"], record["world"], ev)
            self.node_services[node].add("flight")
            if record.get("flight_master") is not None:
                self.entity_nodes[("npc", record["flight_master"])].add(node)

        for record, edition in self.records("world.graveyards", key=lambda r: r["id"]):
            ev = self.evidence_for("world.graveyards", edition)
            if record["position"]["status"] != "known":
                self.dropped[f"graveyard position {record['position']['status']}"] += 1
                continue
            pos = record["position"]["value"]
            node = f"node:graveyard:{record['id']}"
            self.add_node(node, pos["ui_map_id"], pos["x"], pos["y"], record["world"], ev)
            self.node_services[node].add("other")

        self.entities = []
        for nid, (npc, edition) in sorted(self.npcs.items()):
            ev = self.evidence_for("world.npcs", edition)
            lo, hi = npc.get("min_level"), npc.get("max_level")
            levels = known([lo, hi]) if lo and hi and 1 <= lo <= hi else UNKNOWN
            nodes = sorted(self.entity_nodes.get(("npc", nid), ()), key=natural)
            self.entities.append({
                "evidence": ev, "id": f"npc:{nid}", "kind": "npc", "levels": levels,
                "name": npc["name"], "nodes": nodes[:MAX_REFS],
                "reaction": known(npc["reaction"]) if npc.get("reaction") else UNKNOWN,
                "services": sorted({ROLE_SERVICE.get(r, "other") for r in npc.get("roles", [])},
                                   key=SERVICE_ORDER.index),
                "subname": known(npc["subname"]) if npc.get("subname") else UNKNOWN,
            })
        for oid, (obj, edition) in sorted(self.objects.items()):
            ev = self.evidence_for("world.game_objects", edition)
            nodes = sorted(self.entity_nodes.get(("game_object", oid), ()), key=natural)
            self.entities.append({
                "evidence": ev, "id": f"object:{oid}", "kind": "game_object",
                "levels": NOT_APPLICABLE, "name": obj["name"], "nodes": nodes[:MAX_REFS],
                "reaction": NOT_APPLICABLE, "services": [], "subname": NOT_APPLICABLE,
            })
        self.entity_ids = {e["id"] for e in self.entities}
        for (kind, eid) in self.entity_nodes:
            prefix = "npc" if kind == "npc" else "object"
            if f"{prefix}:{eid}" not in self.entity_ids:
                self.dropped["spawned entity missing from npcs/game_objects"] += 1

    def entity_services(self, key, objective):
        if objective:
            return set()
        if key[0] == "npc":
            roles = self.npcs.get(key[1], ({}, None))[0].get("roles", [])
            return {ROLE_SERVICE.get(r, "other") for r in roles} or {"other"}
        return {"other"}

    def add_node(self, node, ui_map_id, x, y, world, evidence):
        if node in self.nodes:
            self.node_facts[node] = sorted(set(self.node_facts[node]) | set(evidence))
            return
        ui = self.ensure_ui_map(ui_map_id)
        self.nodes[node] = {"id": node, "position": {"map": self.map_key(ui), "x": x, "y": y}}
        self.node_map[node] = ui
        self.node_world[node] = (world["map_id"], world["x"], world["y"])
        self.node_facts[node] = list(evidence)

    # edges -------------------------------------------------------------------

    def build_edges(self):
        self.edges = []
        walk_ev = self.inferred("travel.walk", (
            f"Inferred walking edges (ADR 0007 §8): each service, quest, flight, graveyard or entrance node joins "
            f"its {WALK_NEIGHBOURS} nearest such nodes on the same uiMap within {WALK_MAX_YARDS:g} yards, and each "
            f"quest-objective spawn node its {OBJECTIVE_NEIGHBOURS} nearest one, in both directions; expected time is "
            f"the straight-line world distance divided by the run speed {RUN_SPEED:g} yards/s. Terrain, "
            "water, walls and hostile creatures are not considered."))
        flight_ev = self.inferred("travel.flight-time", (
            f"Inferred flight time (ADR 0007 §8): TaxiPath point-to-point length in yards (3D segments on "
            f"one map) divided by the flight speed {FLIGHT_SPEED:g} yards/s; take-off, landing and "
            "client speed changes are not modelled."))
        for record, edition in self.records("travel.taxi_paths", key=lambda r: r["id"]):
            ev = self.evidence_for("travel.taxi_paths", edition)
            a, b = f"node:taxi:{record['from_node']}", f"node:taxi:{record['to_node']}"
            if a not in self.nodes or b not in self.nodes:
                self.dropped["taxi path endpoint without a located taxi node"] += 1
                continue
            length = path_length(record.get("points", []))
            expected = known(round(length / FLIGHT_SPEED * 1000)) if length > 0 else UNKNOWN
            self.edges.append(self.edge(f"edge:flight:{record['id']}", a, b, "flight", UNKNOWN,
                                        expected, UNKNOWN, sorted(set(ev) | {flight_ev})))
        by_map = collections.defaultdict(list)
        for node, ui in self.node_map.items():
            by_map[ui].append(node)
        pairs = set()
        for ui, nodes in sorted(by_map.items()):
            nodes.sort(key=natural)
            points = [self.node_world[n] for n in nodes]
            hubs = [not self.objective_node(n) for n in nodes]
            for i, node in enumerate(nodes):
                near = []
                for j, other in enumerate(nodes):
                    if i == j or not hubs[j] or points[i][0] != points[j][0]:
                        continue
                    d = math.hypot(points[i][1] - points[j][1], points[i][2] - points[j][2])
                    if d <= WALK_MAX_YARDS:
                        near.append((d, natural(other), j))
                near.sort()
                for d, _, j in near[:WALK_NEIGHBOURS if hubs[i] else OBJECTIVE_NEIGHBOURS]:
                    pairs.add((i, j, ui))
                    pairs.add((j, i, ui))
        for i, j, ui in sorted(pairs, key=lambda p: (natural(p[2]), p[0], p[1])):
            nodes = by_map[ui]
            a, b = nodes[i], nodes[j]
            pa, pb = self.node_world[a], self.node_world[b]
            d = math.hypot(pa[1] - pb[1], pa[2] - pb[2])
            self.edges.append(self.edge(f"edge:walk:{a}:{b}", a, b, "walking", known(TRUE_PREDICATE),
                                        known(round(d / RUN_SPEED * 1000)), known(False), [walk_ev]))
        self.edges.sort(key=lambda e: natural(e["id"]))

    def edge(self, eid, a, b, mode, requirements, expected, consumes, evidence):
        return {"consumes_resource": consumes, "context": self.context, "cooldown_ms": NOT_APPLICABLE,
                "evidence": evidence, "expected_ms": expected, "from": self.node_ref(a), "id": eid,
                "mode": mode, "requirements": requirements, "status": "unverified",
                "to": self.node_ref(b), "uncertainty": UNKNOWN, "wait_ms": NOT_APPLICABLE}

    # items -------------------------------------------------------------------

    def build_items(self):
        self.items = {}
        self.prices = {}
        self.equipment = []
        self.item_refs = collections.defaultdict(collections.Counter)
        self.item_origin = {}
        for record, edition in self.records("items.items", key=lambda r: r["id"], compare=ITEM_COMPARE):
            ev = self.evidence_for("items.items", edition)
            inv = record.get("inventory_type")
            if inv is None:
                self.dropped["item without inventory type"] += 1
                continue
            if inv != 0 and inv not in SLOTS:
                self.dropped["item with an unmapped inventory type"] += 1
                continue
            slots = SLOTS.get(inv, [])
            two_handed = known(inv in TWO_HANDED) if slots else NOT_APPLICABLE
            stats = record.get("stats") or {}
            attributes = known({f"stat:{k}": v for k, v in sorted(stats.items())})
            self.item_origin[record["id"]] = (edition, record.get("name"))
            self.items[record["id"]] = {
                "attributes": attributes, "effects": UNKNOWN, "evidence": ev,
                "key": self.item_key(record["id"]), "requirements": item_requirements(record),
                "slots": slots, "two_handed": two_handed, "unique_group": UNKNOWN,
            }
            if slots and record.get("class_id") is not None and record.get("subclass_id") is not None:
                category = known(f"item_class:{record['class_id']}:{record['subclass_id']}")
            elif slots:
                category = UNKNOWN
            else:
                category = known("non_equippable")
            facts = {"category": category, "configuration": UNKNOWN, "evidence": ev,
                     "item": self.item_key(record["id"])}
            if record.get("name"):
                facts["name"] = record["name"]
            self.equipment.append(facts)
            if record.get("buy_price") is not None:
                self.prices[record["id"]] = record["buy_price"]

    def item(self, item_id, used_by):
        """Counts the reference; True when the item is defined."""
        self.item_refs[item_id][used_by] += 1
        return item_id in self.items

    # quests ------------------------------------------------------------------

    def build_quests(self):
        self.quest_defs = []
        self.quest_facts = []
        groups = collections.defaultdict(list)
        unlocks = collections.defaultdict(list)  # quest -> quests whose next_quest is it
        for qid, (quest, _) in self.quests.items():
            if quest.get("exclusive_group", 0) > 0:
                groups[quest["exclusive_group"]].append(qid)
            if quest.get("next_quest", 0) > 0:
                unlocks[quest["next_quest"]].append(qid)
        requires = {}
        for qid, (quest, _) in self.quests.items():
            requires[qid] = self.prerequisites(qid, quest, groups, unlocks)
        self.break_cycles(requires)
        for qid, (quest, edition) in sorted(self.quests.items()):
            ev = self.evidence_for("quests.quests", edition)
            objectives = quest.get("objectives", [])
            stages = [f"quest:{qid}:objective:{i + 1}" for i in range(len(objectives))] or [f"quest:{qid}:complete"]
            min_level = quest.get("min_level")
            if min_level is None:
                eligibility = UNKNOWN
            elif min_level >= 1:
                eligibility = known({"level": min_level, "op": "level_at_least"})
            else:
                eligibility = known(TRUE_PREDICATE)
            terms = requires[qid]
            if terms is None:
                prerequisites = UNKNOWN
            elif not terms:
                prerequisites = known(TRUE_PREDICATE)
            elif len(terms) == 1:
                prerequisites = known(terms[0])
            else:
                prerequisites = known({"op": "all", "predicates": terms})
            locations = []
            for ref in quest.get("givers", []) + quest.get("enders", []):
                kind = {"npc": "npc", "game_object": "game_object"}.get(ref["entity"])
                for node in sorted(self.entity_nodes.get((kind, ref["id"]), ()), key=natural):
                    position = self.nodes[node]["position"]
                    if position not in locations:
                        locations.append(position)
            rewards = []
            for reward in quest.get("reward_items", []) + quest.get("reward_choice_items", []):
                if self.item(reward["item"], "quest_reward"):
                    key = self.item_key(reward["item"])
                    if key not in rewards:
                        rewards.append(key)
            self.quest_defs.append({
                "completion": UNKNOWN, "context": self.context, "eligibility": eligibility,
                "evidence": ev, "id": f"quest:{qid}",
                "locations": known(locations[:MAX_QUEST_LOCATIONS]) if locations else UNKNOWN,
                "prerequisites": prerequisites, "rewards": known(rewards), "shared_progress": UNKNOWN,
                "stages": stages,
            })
            group = sorted(groups.get(quest.get("exclusive_group", 0), []))
            exclusive = [{"quests": [f"quest:{q}" for q in group], "taken_when": "turned_in"}] if len(group) > 1 else []
            flags = quest.get("flags")
            self.quest_facts.append({
                "access": eligibility, "access_requirements": access_requirements(quest),
                "evidence": ev, "exclusive_groups": known(exclusive[:MAX_REFS]), "quest": f"quest:{qid}",
                "reset": UNKNOWN,
                "shareable": known(bool(flags & QUEST_FLAG_SHARABLE)) if flags is not None else UNKNOWN,
                "sharing_conditions": UNKNOWN,
            })
            facts = self.quest_facts[-1]
            if quest.get("title"):
                facts["title"] = quest["title"]
            for field, refs in (("givers", quest.get("givers", [])), ("enders", quest.get("enders", []))):
                ids = []
                for ref in refs:
                    rid = f"{ENTITY_PREFIX[ref['entity']]}:{ref['id']}"
                    if rid not in ids:
                        ids.append(rid)
                if ids:
                    facts[field] = ids[:MAX_REFS]

    def prerequisites(self, qid, quest, groups, unlocks):
        """Predicate terms (all must hold), or None when a referenced quest is missing."""
        terms = []
        prev = quest.get("prev_quest")
        if prev:
            if abs(prev) not in self.quests:
                self.dropped["quest prerequisite names a quest outside the data"] += 1
                return None
            state = "turned_in" if prev > 0 else "accepted"
            terms.append({"op": "quest_state", "quest": f"quest:{abs(prev)}", "state": state, "value": True})
        positive = []
        for source in sorted(unlocks.get(qid, [])):
            group = quest_group(self.quests[source][0], groups)
            if len(group) > 1:
                alt = {"op": "any", "predicates": [
                    {"op": "quest_state", "quest": f"quest:{g}", "state": "turned_in", "value": True}
                    for g in group]}
                if alt not in terms:
                    terms.append(alt)
            else:
                positive.append(source)
        for source in positive:
            term = {"op": "quest_state", "quest": f"quest:{source}", "state": "turned_in", "value": True}
            if term not in terms:
                terms.append(term)
        return terms

    def break_cycles(self, requires):
        """Drop unconditional turned-in prerequisites that close a cycle (coverage refuses them)."""
        def deps(q):
            return [int(t["quest"].split(":")[1]) for t in requires.get(q) or [] if t.get("op") == "quest_state"]
        state = {}
        for start in sorted(requires):
            if start in state:
                continue
            stack = [(start, iter(deps(start)))]
            state[start] = 1
            while stack:
                node, children = stack[-1]
                child = next(children, None)
                if child is None:
                    state[node] = 2
                    stack.pop()
                elif state.get(child) == 1:
                    requires[node] = [t for t in requires[node]
                                      if not (t.get("op") == "quest_state" and t["quest"] == f"quest:{child}")]
                    self.conflicts.append({"fact": f"quests.quests:{node}:prerequisites", "kind": "prerequisite_cycle",
                                           "chosen": {"origin": "vanilla_1_12", "value": f"dropped quest:{child}"},
                                           "rejected": [{"origin": "vanilla_1_12", "value": f"quest:{child}"}],
                                           "rule": "a prerequisite that closes a cycle is dropped"})
                elif child not in state:
                    state[child] = 1
                    stack.append((child, iter(deps(child))))

    # acquisitions ------------------------------------------------------------

    def build_acquisitions(self):
        self.acquisitions = []
        self.acquisition_facts = []
        self.drops_by_entity = collections.defaultdict(list)
        for record, edition in self.records("vendors.items", key=lambda r: (r["npc_id"], r["item_id"])):
            ev = self.evidence_for("vendors.items", edition)
            npc, item_id = record["npc_id"], record["item_id"]
            if npc not in self.npcs:
                self.dropped["vendor row for an NPC outside world.npcs"] += 1
                continue
            if not self.item(item_id, "vendor"):
                continue
            item = self.items[item_id]
            item_ev = item["evidence"]
            price = self.prices.get(item_id)
            aid = f"vendor:{npc}:{item_id}"
            evidence = sorted(set(ev) | set(item_ev) | set(self.evidence_for("world.npcs", self.npcs[npc][1])))
            self.acquisitions.append({"conditions": UNKNOWN, "context": self.context,
                                      "cost": known(str(price)) if price is not None else UNKNOWN,
                                      "evidence": evidence, "id": aid, "items": [self.item_key(item_id)],
                                      "kind": "vendor", "probability": known(1.0)})
            nodes = sorted(self.entity_nodes.get(("npc", npc), ()), key=natural)
            origin = known({"entity": f"npc:{npc}", "kind": "vendor", "location": self.node_ref(nodes[0])}) if nodes else UNKNOWN
            if not nodes:
                self.dropped["vendor origin unknown: NPC has no located spawn"] += 1
            self.acquisition_facts.append({
                "acquisition": aid, "availability": UNKNOWN,
                "costs": known([{"amount": str(price), "kind": "copper"}]) if price is not None else UNKNOWN,
                "entitlement": known("individual"), "evidence": evidence, "origin": origin,
                "prerequisites": UNKNOWN, "rewards": UNKNOWN})
        for qid, (quest, edition) in sorted(self.quests.items()):
            fixed = [r for r in quest.get("reward_items", []) if r["item"] in self.items]
            choice = [r for r in quest.get("reward_choice_items", []) if r["item"] in self.items]
            if not fixed and not choice:
                continue
            ev = self.evidence_for("quests.quests", edition)
            rewards, keys = [], []
            for r, group in [(r, None) for r in fixed] + [(r, f"quest:{qid}:choice") for r in choice]:
                key = self.item_key(r["item"])
                if key in keys:
                    continue
                keys.append(key)
                rewards.append({"choice_group": group, "item": key, "quantity": r["count"]})
            aid = f"quest-reward:{qid}"
            evidence = sorted(set(ev) | {e for k in keys for e in self.items[int(k["item_id"])]["evidence"]})
            self.acquisitions.append({"conditions": known(TRUE_PREDICATE), "context": self.context,
                                      "cost": NOT_APPLICABLE, "evidence": evidence, "id": aid, "items": keys,
                                      "kind": "quest",
                                      "probability": known(1.0) if not choice else NOT_APPLICABLE})
            self.acquisition_facts.append({
                "acquisition": aid, "availability": UNKNOWN, "costs": known([]),
                "entitlement": known("individual"), "evidence": evidence,
                "origin": known({"kind": "quest", "quest": f"quest:{qid}"}),
                "prerequisites": UNKNOWN, "rewards": known(rewards)})
        self.build_drops()
        self.acquisitions.sort(key=lambda a: natural(a["id"]))
        self.acquisition_facts.sort(key=lambda a: natural(a["acquisition"]))

    def build_drops(self):
        dungeon_bosses = {}
        for record, _ in self.records("dungeons.dungeons", key=lambda r: r["map_id"]):
            for boss in record.get("bosses", []):
                dungeon_bosses[boss] = self.dungeon_id(record)
        self.dungeon_bosses = dungeon_bosses
        self.expansions = {}
        reference_rows = collections.defaultdict(list)
        for row, edition in self.records("loot.reference"):
            reference_rows[row["entry"]].append((row, edition))
        expansion_ev = self.inferred("loot.reference-expansion", (
            "Boss loot reached through one reference table (depth 1): probability = 1 - (1 - p_roll * "
            "p_item)^rolls, where p_roll is the boss row chance, rolls its max_count and p_item the item's "
            "chance inside the reference table. Only dungeon bosses are expanded, so counts stay bounded."))
        for table in ("creature", "game_object", "item", "fishing", "pickpocketing", "skinning", "disenchant"):
            dataset = f"loot.{table}"
            rows = self.records(dataset)
            shares = group_shares([r for r, _ in rows])
            for row, edition in rows:
                entity = loot_entity(table, row["entry"], self)
                boss = table == "creature" and row["entry"] in dungeon_bosses
                if "reference" in row:
                    if boss:
                        self.expand_boss_reference(row, edition, reference_rows, expansion_ev)
                    continue
                item_id = row["item"]
                if item_id not in self.items:
                    continue
                quest_only = row["chance"] < 0
                if not (self.items[item_id]["slots"] or quest_only or boss):
                    self.dropped["loot row kept only in loot_tables (not gear, quest or boss loot)"] += 1
                    continue
                probability = row_probability(row, shares)
                self.add_drop(f"drop:{table}:{row['entry']}:{item_id}", item_id, entity, row, probability,
                              sorted(set(self.evidence_for(dataset, edition)) | set(self.items[item_id]["evidence"])),
                              dungeon_bosses.get(row["entry"]) if boss else None)
        self.flush_expansions()

    def expand_boss_reference(self, row, edition, reference_rows, expansion_ev):
        refs = reference_rows.get(row["reference"], [])
        shares = group_shares([r for r, _ in refs])
        p_roll = abs(row["chance"]) / 100 if row["chance"] else None
        rolls = max(1, row.get("max_count", 1))
        for ref, ref_edition in refs:
            if "item" not in ref:
                continue
            if ref["item"] not in self.items:
                continue
            p_item = row_probability(ref, shares)
            if p_roll is not None and p_item.get("status") == "known":
                p = 1 - (1 - p_roll * p_item["value"]) ** rolls
                probability = known(rnd(min(1.0, max(0.0, p))))
            else:
                probability = UNKNOWN
            evidence = sorted(set(self.evidence_for("loot.creature", edition))
                              | set(self.evidence_for("loot.reference", ref_edition))
                              | set(self.items[ref["item"]]["evidence"]) | {expansion_ev})
            merged = dict(ref, chance=-1 if (row["chance"] < 0 or ref["chance"] < 0) else 1)
            merged.pop("condition", None)
            if "condition" in row or "condition" in ref:
                merged["condition"] = 1
            aid = f"drop:creature:{row['entry']}:ref:{row['reference']}:{ref['item']}"
            entry = self.expansions.setdefault(aid, {"item": ref["item"], "row": merged, "probabilities": [],
                                                     "evidence": set(), "entry": row["entry"]})
            entry["probabilities"].append(probability)
            entry["evidence"] |= set(evidence)
            if merged["chance"] < 0:
                entry["row"]["chance"] = -1
            if "condition" in merged:
                entry["row"]["condition"] = 1

    def flush_expansions(self):
        """One acquisition per boss, reference and item: independent occurrences combine as 1 - prod(1 - p)."""
        for aid in sorted(self.expansions, key=natural):
            e = self.expansions[aid]
            if all(p.get("status") == "known" for p in e["probabilities"]):
                miss = 1.0
                for p in e["probabilities"]:
                    miss *= 1 - p["value"]
                probability = known(rnd(min(1.0, max(0.0, 1 - miss))))
            else:
                probability = UNKNOWN
            self.add_drop(aid, e["item"], loot_entity("creature", e["entry"], self), e["row"], probability,
                          sorted(e["evidence"]), self.dungeon_bosses[e["entry"]])

    def add_drop(self, aid, item_id, entity, row, probability, evidence, dungeon):
        if row["chance"] < 0:
            quests = sorted(self.objective_items.get(item_id, ()))
            if quests:
                states = [{"op": "quest_state", "quest": f"quest:{q}", "state": "accepted", "value": True}
                          for q in quests[:MAX_REFS]]
                conditions = known(states[0] if len(states) == 1 else {"op": "any", "predicates": states})
            else:
                conditions = UNKNOWN
        elif "condition" in row:
            conditions = UNSUPPORTED
        else:
            conditions = known(TRUE_PREDICATE)
        self.acquisitions.append({"conditions": conditions, "context": self.context, "cost": NOT_APPLICABLE,
                                  "evidence": evidence, "id": aid, "items": [self.item_key(item_id)],
                                  "kind": "dungeon" if dungeon else "other", "probability": probability})
        origin = known({"dungeon": dungeon, "entity": entity, "kind": "drop"}) if entity else UNKNOWN
        self.acquisition_facts.append({"acquisition": aid, "availability": UNKNOWN, "costs": known([]),
                                       "entitlement": UNKNOWN, "evidence": evidence, "origin": origin,
                                       "prerequisites": UNKNOWN, "rewards": UNKNOWN})
        if dungeon:
            self.drops_by_entity[dungeon].append(aid)

    # loot tables -------------------------------------------------------------

    def build_loot_tables(self):
        self.loot_tables = []
        for table in ("creature", "game_object", "item", "reference", "fishing", "pickpocketing",
                      "skinning", "disenchant"):
            dataset = f"loot.{table}"
            by_entry = collections.defaultdict(list)
            rows = self.records(dataset)
            shares = group_shares([r for r, _ in rows])
            for row, edition in rows:
                by_entry[row["entry"]].append((row, edition))
            for entry in sorted(by_entry):
                out, evidence = [], set()
                for row, edition in by_entry[entry]:
                    evidence |= set(self.evidence_for(dataset, edition))
                    cell = {"max_count": row["max_count"], "min_count": row["min_count"]}
                    probability = row_probability(row, shares)
                    if probability["status"] == "known":
                        cell["probability"] = probability["value"]
                    if "reference" in row:
                        cell["table"] = f"loot:reference:{row['reference']}"
                    else:
                        cell["item"] = str(row["item"])
                        self.item(row["item"], "loot")
                    if row.get("group") is not None:
                        cell["group"] = row["group"]
                    if row["chance"] < 0:
                        cell["quest_only"] = True
                    if "condition" in row:
                        cell["conditional"] = True
                    out.append(cell)
                out.sort(key=lambda c: (c.get("group", 0), natural(c.get("item", "")), natural(c.get("table", ""))))
                if len(out) > MAX_LOOT_ROWS:
                    self.dropped["loot rows beyond the per-table limit"] += len(out) - MAX_LOOT_ROWS
                    out = out[:MAX_LOOT_ROWS]
                source = loot_entity(table, entry, self)
                self.loot_tables.append({"evidence": sorted(evidence), "id": f"loot:{table}:{entry}",
                                         "rows": out, "source": known(source) if source else UNKNOWN,
                                         "table": table})
        ids = {t["id"] for t in self.loot_tables}
        for t in self.loot_tables:
            missing = [c for c in t["rows"] if "table" in c and c["table"] not in ids]
            if missing:
                self.dropped["loot reference to a missing reference table"] += len(missing)
                t["rows"] = [c for c in t["rows"] if c not in missing]
        self.loot_tables = [t for t in self.loot_tables if t["rows"]]
        self.loot_tables.sort(key=lambda t: natural(t["id"]))

    # dungeons ----------------------------------------------------------------

    def dungeon_id(self, record):
        title = self.dungeon_title(record)
        return "dungeon:" + re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")

    def authored_dungeons(self):
        if self.authored is None:
            return {}
        out = {}
        for d in self.authored.get("dungeons", []):
            out[normal_title(d["title"])] = d
        return out

    def dungeon_title(self, record):
        authored = self.authored_dungeons().get(normal_title(record["name"]))
        return authored["title"] if authored else record["name"]

    def build_entrances(self):
        self.entrances = {}
        for record, edition in self.records("dungeons.dungeons", key=lambda r: r["map_id"]):
            if record["entrance"]["status"] != "known":
                continue
            w = record["entrance"]["value"]
            located = self.locate(w["map_id"], w["x"], w["y"])
            if not located:
                self.dropped["dungeon entrance outside every uiMap"] += 1
                continue
            node = f"node:dungeon-entrance:{record['map_id']}"
            self.add_node(node, located[0], located[1], located[2], w, self.evidence_for("dungeons.dungeons", edition))
            self.node_services[node].add("dungeon")
            self.entrances[record["map_id"]] = node

    def objective_node(self, node):
        return not self.node_services.get(node)

    def build_dungeons(self):
        self.dungeons = []
        authored = self.authored_dungeons()
        zone_quests = collections.defaultdict(list)
        for qid, (quest, _) in sorted(self.quests.items()):
            if quest.get("zone_or_sort", 0) > 0:
                zone_quests[quest["zone_or_sort"]].append(f"quest:{qid}")
        for record, edition in self.records("dungeons.dungeons", key=lambda r: r["map_id"]):
            ev = self.evidence_for("dungeons.dungeons", edition)
            did = self.dungeon_id(record)
            mine = authored.get(normal_title(record["name"]))
            if mine and mine["title"] != record["name"]:
                self.conflict(f"dungeons.dungeons:{record['map_id']}:name", "authored", mine["title"],
                              edition, record["name"])
            map_id = f"instance:{record['map_id']}"
            if map_id not in self.map_spaces:
                self.map_spaces[map_id] = {"evidence": ev, "key": self.map_key(map_id), "transform": UNKNOWN}
            entrance = UNKNOWN
            if record["map_id"] in self.entrances:
                entrance = known(self.node_ref(self.entrances[record["map_id"]]))
            elif mine and mine.get("entrance", {}).get("status") == "known":
                entrance = mine["entrance"]
            bosses = [f"npc:{b}" for b in record.get("bosses", []) if f"npc:{b}" in self.entity_ids]
            area = record.get("area_id")
            dungeon = {
                "access": UNKNOWN, "checklist": UNKNOWN, "entrance": entrance,
                "evidence": ev, "id": did, "map": self.map_key(map_id),
                "preparation_quests": known(zone_quests.get(area, [])[:MAX_REFS]) if area else UNKNOWN,
                "rewards": known(sorted(self.drops_by_entity.get(did, []), key=natural)[:MAX_REFS]),
                "title": self.dungeon_title(record), "travel_edges": UNKNOWN}
            if bosses:
                dungeon["bosses"] = bosses
            self.dungeons.append(dungeon)
        self.dungeons.sort(key=lambda d: d["id"])

    def compare_areas(self):
        """Area names from the client table outrank the 1.12 level table; report differences."""
        forever = {r["id"]: (r, e) for r, e in self.records("world.areas", key=lambda r: r["id"])}
        for record, edition in self.records("world.area_levels", key=lambda r: r["id"]):
            other = forever.get(record["id"])
            if other and other[0].get("name") != record.get("name"):
                self.conflict(f"area:{record['id']}:name", other[1], other[0].get("name"), edition, record.get("name"))

    # assemble ----------------------------------------------------------------

    def assemble(self):
        for node, services in self.node_services.items():
            pick = next((s for s in SERVICE_ORDER if s in services), None)
            self.nodes[node]["service"] = known(pick) if pick else NOT_APPLICABLE
        top = self.policy.get("target_edition")
        for item_id, (edition, name) in sorted(self.item_origin.items()):
            if edition != top:
                uses = self.item_refs.get(item_id, {})
                self.conflicts.append({
                    "fact": f"item:{item_id}", "kind": "missing_in_higher_precedence",
                    "chosen": {"origin": edition, "value": name}, "rejected": [{"origin": top, "value": None}],
                    "references": dict(sorted(uses.items())),
                    "rule": f"the {top} item table lacks the item; the {edition} item table defines it"})
        for item_id, uses in sorted(self.item_refs.items()):
            if item_id not in self.items and sum(uses.values()):
                self.conflicts.append({
                    "fact": f"item:{item_id}", "kind": "missing_item",
                    "chosen": {"origin": None, "value": None},
                    "rejected": [{"origin": "vanilla_1_12", "value": dict(sorted(uses.items()))}],
                    "rule": "no item table defines the item; references are dropped from acquisitions and "
                            "quest rewards and kept as raw IDs in loot_tables"})
        evidence = [self.evidence_record(eid) for eid in sorted(self.evidence, key=natural)]
        revision = ""  # Filled from the complete canonical mapping below, without circular hashing.
        payload = {
            "acquisitions": self.acquisitions, "edges": self.edges, "evidence": evidence,
            "items": [self.items[i] for i in sorted(self.items)],
            "map_spaces": [self.map_spaces[k] for k in sorted(self.map_spaces, key=natural)],
            "nodes": [self.nodes[n] for n in sorted(self.nodes, key=natural)],
            "objectives": [], "quests": self.quest_defs, "revision": revision, "rules": [],
        }
        catalog = {
            "acquisition_facts": self.acquisition_facts, "dungeons": self.dungeons,
            "entities": self.entities, "equipment": sorted(self.equipment, key=lambda e: natural(e["item"]["item_id"])),
            "format": "navzeroth-reference-content", "loot_tables": self.loot_tables, "models": [],
            "node_facts": [{"evidence": self.node_facts[n], "node": n} for n in sorted(self.nodes, key=natural)],
            "quest_facts": self.quest_facts,
            "reference": {
                "fixture": False, "format": "azeroth-navigator",
                "game": {"build": self.target["build"], "edition": self.target["edition"],
                         "interface": self.target["interface"], "locale": self.target["locale"]},
                "kind": "reference_dataset", "namespace": "reference", "payload": payload,
                "producer": {"component": "cli", "version": f"{NAME}-{VERSION}"},
                "provenance": {"clock_trust": "uncertain", "source": "reference_data"},
                "schema_version": 1,
            },
            "schema_version": 2,
        }
        sources = self.source_registry()
        # Shards alone do not identify a mapping: policy, authored facts, target
        # context and conversion changes can all change its bytes. Hash canonical
        # output with an empty revision together with every declared input.
        identity = {"catalog": catalog, "sources": sources, "conflicts": self.conflicts,
                    "input_sha256": self.input_digest, "target": self.target,
                    "policy": self.policy, "authored": self.authored,
                    "generator": {"name": NAME, "version": VERSION}}
        digest = hashlib.sha256(dumps(identity).encode()).hexdigest()
        revision = f"navzeroth-world-{self.target['build']}-{digest[:24]}"
        payload["revision"] = revision
        return catalog, sources, revision

    def evidence_record(self, eid):
        ev = self.evidence[eid]
        if ev.get("inferred"):
            return {"confidence": "inferred", "conflicts": [], "context": self.context, "id": eid,
                    "license": known(SOURCES[DERIVED_SOURCE]["license"]), "notes": ev["notes"],
                    "observed_at_utc_ms": NOT_APPLICABLE, "source": DERIVED_SOURCE}
        builds = ", ".join(sorted(ev["builds"]))
        refs = "; ".join(sorted({f"{r.get('url', '')} revision {r.get('revision', 'unknown')} snapshot "
                                 f"{r.get('snapshot_sha256', 'unknown')}" for r in ev["refs"]}))
        paths = ", ".join(ev["paths"])
        aux = ""
        if ev.get("aux"):
            used = "; ".join(sorted({f"{a['id']} ({a.get('url', '')} revision {a.get('revision', 'unknown')} "
                                     f"snapshot {a.get('snapshot_sha256', 'unknown')})" for a in ev["aux"]}))
            aux = (f" The generator also read {used}, a {self.target['edition']} client source, for map "
                   f"positions, zone areas, names of maps and areas or NPC reactions; the records stay "
                   f"{ev['edition']} data.")
        notes = (f"Origin {ev['edition']} build {builds}; dataset {ev['dataset']}; shards {paths}; "
                 f"source {refs}.{aux} Mapped by {NAME} v{VERSION} into the {self.target['edition']} "
                 f"{self.target['build']} pack; the origin edition is kept, never relabelled (ADR 0007 §3).")
        if len(notes.encode()) > 4096:
            notes = notes.encode()[:4090].decode(errors="ignore") + " ..."
        info = SOURCES.get(ev["source"], {})
        # A target-context envelope is not evidence that historical source bytes
        # describe that build. Keep the actual origin structured; earlier same-
        # edition builds remain unverified unless release policy accepts them.
        origin_build = next(iter(ev["builds"])) if len(ev["builds"]) == 1 else None
        origin_build = None if origin_build == "unknown" else origin_build
        confidence = ev["confidence"]
        if (ev["edition"] != self.target["edition"] or origin_build != self.target["build"]) \
                and confidence in ("publisher_documented", "verified_on_build"):
            confidence = "cross_edition_unverified"
        return {"confidence": confidence, "conflicts": [], "context": self.context, "id": eid,
                "origin": {"edition": ev["edition"], "build": known(origin_build) if origin_build else UNKNOWN},
                "license": known(info["license"]) if info.get("license") else UNKNOWN, "notes": notes,
                "observed_at_utc_ms": known(min(ev["retrieved"])) if ev["retrieved"] else UNKNOWN,
                "source": ev["source"]}

    def source_registry(self):
        out = []
        for sid in sorted(self.evidence_sources):
            info = SOURCES.get(sid)
            ref = self.source_refs.get(sid, {})
            retrieved = [r for e in self.evidence.values() if e.get("source") == sid for r in e.get("retrieved", [])]
            fields = sorted(self.evidence_sources[sid])
            rec = {
                "allowed_fields": known(fields), "authentication": known("none"),
                "conditional_requests": UNKNOWN, "failure_behavior": NOT_APPLICABLE, "id": sid,
                "last_retrieval_utc_ms": known(max(retrieved)) if retrieved else UNKNOWN,
                "offline_usable": known(True), "provider": None,
                "retention": known("Retained in the repository under data-generators/output (Git LFS)"),
                "transformations": known([f"{NAME} v{VERSION} mapping (docs/data/WORLD_CONTENT.md)",
                                          "canonical_typed_serialization"]),
                "update_schedule": known("Weekly generator run (ADR 0007 §1)"),
                "verified_revision": UNKNOWN,
            }
            if info:
                editions = sorted(set(info["editions"]) | {self.target["edition"]}) if info.get("editions") else None
                rec.update({
                    "access": known(info["access"]) if info.get("access") else NOT_APPLICABLE,
                    "attribution": known(info["attribution"]),
                    "editions": known(editions) if editions else NOT_APPLICABLE,
                    "license": known(info["license"]),
                    "location": known(info["location"]),
                    "publisher": info["publisher"],
                    "record_class": info["record_class"],
                    "redistribution": known(info["redistribution"]),
                })
                if sid == DERIVED_SOURCE:
                    rec["authentication"] = NOT_APPLICABLE
                    rec["retention"] = known("Regenerated from the retained world data by data-generators/world_to_content.py")
                    rec["update_schedule"] = known("With every world content build")
            else:
                editions = sorted({e["edition"] for e in self.evidence.values() if e.get("source") == sid and "edition" in e}
                                  | {self.target["edition"]})
                rec.update({"access": UNKNOWN, "attribution": UNKNOWN, "editions": known(editions),
                            "license": UNKNOWN, "location": known(ref["url"]) if ref.get("url") else UNKNOWN,
                            "publisher": sid, "record_class": "secondary_inference",
                            "redistribution": UNKNOWN})
            out.append(rec)
        return {"schema_version": 1, "sources": out}


# Pack source records for the generator sources (docs/sources/source-admission-v1.md).
SOURCES = {
    "cmangos-classic-db": {
        "publisher": "cmangos/classic-db", "location": "https://github.com/cmangos/classic-db",
        "editions": ["vanilla_1_12"], "access": "file_download", "record_class": "secondary_inference",
        "license": "GPL-3.0-only (share_alike; COPYRIGHT.md reserves Blizzard material, owner accepted, ADR 0006 §14)",
        "attribution": "CMaNGOS Classic-DB contributors; https://github.com/cmangos/classic-db",
        "redistribution": "conditional"},
    "wago-tools-wowf": {
        "publisher": "wago.tools", "location": "https://wago.tools/db2?product=wowf",
        "editions": ["wow_forever"], "access": "file_download", "record_class": "publisher_documentation",
        "license": "LicenseRef-Blizzard-client-data-via-wago-tools (owner_accepted, ADR 0006 §14)",
        "attribution": "World of Warcraft Forever client data (Blizzard Entertainment) via wago.tools DB2 exports; https://wago.tools",
        "redistribution": "conditional"},
    "wago-tools-classic-era": {
        "publisher": "wago.tools", "location": "https://wago.tools/db2?product=wow_classic_era",
        "editions": ["wow_classic_era"], "access": "file_download", "record_class": "publisher_documentation",
        "license": "LicenseRef-Blizzard-client-data-via-wago-tools (owner_accepted, ADR 0006 §14)",
        "attribution": "World of Warcraft Classic Era client data (Blizzard Entertainment) via wago.tools DB2 exports; https://wago.tools",
        "redistribution": "conditional"},
    "wowhead-classic": {
        "publisher": "Wowhead", "location": "https://www.wowhead.com/classic", "editions": ["wow_classic_era"],
        "access": "file_download", "record_class": "secondary_inference",
        "license": "owner_accepted (Wowhead terms, ADR 0006 §14)", "attribution": "Wowhead (https://www.wowhead.com)",
        "redistribution": "conditional"},
    "wowhead-forever": {
        "publisher": "Wowhead", "location": "https://www.wowhead.com/forever", "editions": ["wow_forever"],
        "access": "file_download", "record_class": "secondary_inference",
        "license": "owner_accepted (Wowhead terms, ADR 0006 §14)", "attribution": "Wowhead (https://www.wowhead.com)",
        "redistribution": "conditional"},
    DERIVED_SOURCE: {
        "publisher": "Navzeroth project (repository owner)",
        "location": "data-generators/world_to_content.py", "editions": ["wow_forever"], "access": None,
        "record_class": "secondary_inference", "license": "BSD-3-Clause (repository LICENSE)",
        "attribution": "Navzeroth world content mapping (inferred travel times and loot expansion)",
        "redistribution": "permitted"},
}


# ------------------------------------------------------------------ pure helpers

def origin_sources(refs, edition):
    """(sources of the origin edition, other sources) of one shard. A known source belongs to
    the origin when SOURCES lists that edition; an unlisted source is taken as the origin's.
    A shard none of whose sources matches keeps all of them (nothing is left uncited)."""
    own = [r for r in refs if edition in (SOURCES.get(r.get("id"), {}).get("editions") or [edition])]
    if not own:
        return list(refs), []
    return own, [r for r in refs if r not in own]

def transform_of(assignment):
    """CoordinateTransform from normalized uiMap x/y to world yards.

    The client map's x grows east, which is world -Y; its y grows south, world -X:
        nx = u0 + (u1 - u0) * (maxY - Y) / (maxY - minY)
        ny = v0 + (v1 - v0) * (maxX - X) / (maxX - minX)
    so target_x (from nx) is world Y and target_y (from ny) is world X, both in yards."""
    lo, hi = assignment["region_min"], assignment["region_max"]
    (u0, v0), (u1, v1) = assignment["ui_min"], assignment["ui_max"]
    if u1 == u0 or v1 == v0 or hi[0] == lo[0] or hi[1] == lo[1]:
        return None
    sy = (hi[1] - lo[1]) / (u1 - u0)
    sx = (hi[0] - lo[0]) / (v1 - v0)
    return {"offset_x": rnd(hi[1] + u0 * sy, 4), "offset_y": rnd(hi[0] + v0 * sx, 4),
            "scale_x": rnd(-sy, 4), "scale_y": rnd(-sx, 4), "units": "yards"}


def apply_transform(t, nx, ny):
    """(world Y, world X) yards of a normalized position."""
    return t["scale_x"] * nx + t["offset_x"], t["scale_y"] * ny + t["offset_y"]


def invert_transform(t, world_x, world_y):
    """Normalized (nx, ny) of a world point (X north, Y west)."""
    return (world_y - t["offset_x"]) / t["scale_x"], (world_x - t["offset_y"]) / t["scale_y"]


def path_length(points):
    total = 0.0
    for a, b in zip(points, points[1:]):
        if a["map_id"] == b["map_id"]:
            total += math.dist((a["x"], a["y"], a["z"]), (b["x"], b["y"], b["z"]))
    return total


def spread(rows, cap):
    """Up to `cap` spawns far apart: the lowest guid, then repeatedly the farthest one."""
    if len(rows) <= cap:
        return rows
    chosen = [rows[0]]
    rest = rows[1:]
    while len(chosen) < cap and rest:
        def gap(s):
            return min(math.hypot(s[0]["world"]["x"] - c[0]["world"]["x"], s[0]["world"]["y"] - c[0]["world"]["y"])
                       for c in chosen)
        best = max(rest, key=lambda s: (gap(s), -s[0]["guid"]))
        chosen.append(best)
        rest.remove(best)
    return sorted(chosen, key=lambda s: s[0]["guid"])


def group_shares(rows):
    """{(table, entry, group): equal share of the group's remaining chance (0..1) or None}."""
    explicit = collections.defaultdict(float)
    zero = collections.Counter()
    for r in rows:
        if r.get("group") is None or "reference" in r:
            continue
        key = (r["table"], r["entry"], r["group"])
        if r["chance"] == 0:
            zero[key] += 1
        else:
            explicit[key] += abs(r["chance"])
    out = {}
    for key, count in zero.items():
        rest = 100.0 - explicit.get(key, 0.0)
        out[key] = rnd(rest / count / 100) if rest > 0 else None
    return out


def row_probability(row, shares):
    """|chance| / 100; a grouped zero-chance row gets its equal share of the group; otherwise unknown."""
    if row["chance"] != 0:
        return known(rnd(min(100.0, abs(row["chance"])) / 100))
    if row.get("group") is not None:
        share = shares.get((row["table"], row["entry"], row["group"]))
        if share is not None:
            return known(share)
    return UNKNOWN


def loot_entity(table, entry, builder):
    kind = LOOT_SOURCE.get(table)
    if kind == "npc":
        return f"npc:{entry}" if entry in builder.npcs else None
    if kind == "object":
        return f"object:{entry}" if entry in builder.objects else None
    if kind == "item":
        return f"item:{entry}"
    if kind == "area":
        return f"area:{entry}"
    return None


def item_requirements(record):
    """Required level as a predicate; class or race restrictions are not expressible: unsupported."""
    classes, races = record.get("allowable_class"), record.get("allowable_race")
    if classes is None or races is None or record.get("required_level") is None:
        return UNKNOWN
    if not (classes == -1 or classes & ALL_CLASSES == ALL_CLASSES) or not (races == -1 or races & ALL_RACES == ALL_RACES):
        return UNSUPPORTED
    level = record["required_level"]
    return known({"level": level, "op": "level_at_least"}) if level >= 1 else known(TRUE_PREDICATE)


def mask_ids(mask, names, prefix):
    out = []
    for bit in range(63):
        if mask >> bit & 1:
            out.append(f"{prefix}:{names.get(1 << bit, f'bit{bit}')}")
    return sorted(out)


def access_requirements(quest):
    races, classes = quest.get("required_races"), quest.get("required_classes")
    if races is None or classes is None:
        return UNKNOWN
    value = {"classes": None, "factions": None, "races": None}
    if classes and classes & ALL_CLASSES != ALL_CLASSES:
        value["classes"] = mask_ids(classes, CLASSES, "class")
    if races and races & ALL_RACES != ALL_RACES:
        value["races"] = mask_ids(races, RACES, "race")
    return known(value)


def quest_group(quest, groups):
    g = quest.get("exclusive_group", 0)
    return sorted(groups.get(g, [])) if g > 0 else []


def normal_title(title):
    t = title.strip().lower()
    return t[4:] if t.startswith("the ") else t


def dumps(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"


def summary(catalog, sources, conflicts, builder, revision, catalog_bytes):
    payload = catalog["reference"]["payload"]
    counts = {f"reference.{k}": len(v) for k, v in sorted(payload.items()) if isinstance(v, list)}
    counts.update({k: len(v) for k, v in sorted(catalog.items()) if isinstance(v, list)})
    counts["quest_stages"] = sum(len(q["stages"]) for q in payload["quests"])
    counts["loot_rows"] = sum(len(t["rows"]) for t in catalog["loot_tables"])
    return {
        "format": "navzeroth-world-content-summary", "schema_version": 1,
        "generator": {"name": NAME, "version": VERSION}, "revision": revision,
        "context": catalog["reference"]["game"], "input_sha256": builder.input_digest,
        "catalog_sha256": hashlib.sha256(catalog_bytes).hexdigest(), "catalog_bytes": len(catalog_bytes),
        "counts": counts, "sources": [s["id"] for s in sources["sources"]],
        "conflicts": dict(sorted(collections.Counter(c["kind"] for c in conflicts).items())),
        "dropped": dict(sorted(builder.dropped.items())),
        "not_mapped": dict(sorted(builder.unmapped.items())),
        "skipped_origins": builder.skipped_origins,
        "parameters": {"run_speed_yards_per_s": RUN_SPEED, "flight_speed_yards_per_s": FLIGHT_SPEED,
                       "walk_neighbours": WALK_NEIGHBOURS, "walk_max_yards": WALK_MAX_YARDS,
                       "objective_spawn_cap": OBJECTIVE_SPAWN_CAP},
        "precedence": builder.policy["precedence"],
    }


def run(input_dir, target_file, policy_file, authored_dir, out_dir):
    shards, digest = load_input(pathlib.Path(input_dir))
    target = read_json(target_file, "target")
    policy = read_json(policy_file, "policy")
    authored = None
    if authored_dir:
        path = pathlib.Path(authored_dir) / "catalog.json"
        authored = read_json(path, "authored catalog")
    builder = Builder(shards, digest, target, policy, authored)
    catalog, sources, revision = builder.build()
    conflicts = sorted(builder.conflicts, key=lambda c: (natural(c["fact"]), c["kind"], dumps(c)))
    catalog_bytes = dumps(catalog).encode()
    out = pathlib.Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "catalog.json").write_bytes(catalog_bytes)
    (out / "sources.json").write_text(dumps(sources), encoding="utf-8")
    (out / "conflicts.json").write_text(dumps({"format": "navzeroth-world-content-conflicts", "schema_version": 1,
                                                "revision": revision, "precedence": policy["precedence"],
                                                "conflicts": conflicts}), encoding="utf-8")
    result = summary(catalog, sources, conflicts, builder, revision, catalog_bytes)
    (out / "summary.json").write_text(json.dumps(result, indent=1, sort_keys=True, ensure_ascii=False) + "\n",
                                      encoding="utf-8")
    return result


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--input", required=True, help="world data output root (shards)")
    p.add_argument("--target", required=True, help="release/v1/world-data-target.json")
    p.add_argument("--policy", required=True, help="release/v1/accepted-origins.json")
    p.add_argument("--authored", help="authored slice directory (catalog.json)")
    p.add_argument("--out", required=True, help="output directory")
    a = p.parse_args(argv)
    try:
        result = run(a.input, a.target, a.policy, a.authored, a.out)
    except InputError as error:
        print(f"world_to_content: {error}", file=sys.stderr)
        return 1
    print(json.dumps({"revision": result["revision"], "catalog_bytes": result["catalog_bytes"],
                      "counts": result["counts"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
