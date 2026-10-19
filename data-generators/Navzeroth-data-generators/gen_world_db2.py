#!/usr/bin/env python3
"""World, item, spell, talent and travel data from the client's own DB2 tables.

wago.tools serves the WoW Forever client tables as CSV (product `wowf`), so the
shards carry the Forever origin (edition wow_forever, the pinned build) and
`publisher_documented` confidence. If the pinned build lacks a table a dataset
needs, that dataset is read from Classic Era instead and written under the
Classic Era origin (its own shard folder), never mixed into a Forever shard.

Datasets: world.areas, world.ui_maps, world.ui_map_assignments, items.items,
spells.spells, talents.talents, travel.taxi_nodes, travel.taxi_paths.

Item stats: ItemSparse holds no stat amounts in this client, only the stat type
(StatModifier_bonusStat_N) and its share of the item's budget in 1/10000
(StatPercentEditor_N). The amount is the client's formula
round(RandPropPoints[item level].<quality>_<slot column> * share / 10000),
checked against the CMaNGOS 1.12 dump (unchanged vanilla items agree on 98.7% of
shared stat values). Combat ratings also need a per-item-level multiplier the
CSV tables do not carry, so they are left out rather than guessed. Armor is not
in ItemSparse here and is omitted.

    python3 gen_world_db2.py [--out DIR] [--refresh | --offline] [--wago-product wowf] [--wago-build B]
"""
import argparse
import hashlib
import json
import math
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from navzeroth_datagen import coords, db2, envelope, http  # noqa: E402

NAME = "gen_world_db2"
VERSION = "1"
FALLBACK_PRODUCT = "wow_classic_era"
MAX_TEXT = 8192
I64 = 1 << 64

# ITEM_MOD_* stat types (UnitDefines / ItemTemplate); others become stat_<id>.
STAT_NAMES = {
    0: "mana", 1: "health", 3: "agility", 4: "strength", 5: "intellect", 6: "spirit", 7: "stamina",
    12: "defense_skill_rating", 13: "dodge_rating", 14: "parry_rating", 15: "block_rating",
    16: "hit_melee_rating", 17: "hit_ranged_rating", 18: "hit_spell_rating", 19: "crit_melee_rating",
    20: "crit_ranged_rating", 21: "crit_spell_rating", 22: "hit_taken_melee_rating",
    23: "hit_taken_ranged_rating", 24: "hit_taken_spell_rating", 25: "crit_taken_melee_rating",
    26: "crit_taken_ranged_rating", 27: "crit_taken_spell_rating", 28: "haste_melee_rating",
    29: "haste_ranged_rating", 30: "haste_spell_rating", 31: "hit_rating", 32: "crit_rating",
    33: "hit_taken_rating", 34: "crit_taken_rating", 35: "resilience_rating", 36: "haste_rating",
    37: "expertise_rating", 38: "attack_power", 39: "ranged_attack_power", 40: "feral_attack_power",
    41: "spell_healing_done", 42: "spell_damage_done", 43: "mana_regeneration",
    44: "armor_penetration_rating", 45: "spell_power", 46: "health_regen", 47: "spell_penetration",
    48: "block_value", 49: "mastery_rating", 50: "extra_armor", 51: "fire_resistance",
    52: "frost_resistance", 53: "holy_resistance", 54: "shadow_resistance", 55: "nature_resistance",
    56: "arcane_resistance",
}
# Combat ratings scale with a GtCombatRatingsMultByILvl multiplier that is not a DB2 table.
RATINGS = set(range(12, 38)) | {44, 49}
# RandPropPoints column per InventoryType, fitted on the CMaNGOS cross-check
# (0: head/chest/legs/shirt/robe/two-hand; 1: shoulder/waist/feet/hands/trinket;
# 2: neck/wrist/finger/shield/cloak/held; 3: one-hand weapons; 4: ranged/thrown/relic).
SLOT_COLUMN = {inv: col for col, invs in enumerate(
    [(1, 4, 5, 7, 17, 20), (3, 6, 8, 10, 12), (2, 9, 11, 14, 16, 23), (13, 21, 22), (15, 25, 26, 28)])
    for inv in invs}

TABLES = {
    "world.areas": ("AreaTable",),
    "world.ui_maps": ("UiMap",),
    "world.ui_map_assignments": ("UiMapAssignment",),
    "items.items": ("ItemSparse", "Item", "RandPropPoints"),
    "spells.spells": ("SkillLineAbility", "SpellName", "Spell", "SpellLevels"),
    "talents.talents": ("Talent", "TalentTab"),
    "travel.taxi_nodes": ("TaxiNodes", "UiMapAssignment", "UiMap"),
    "travel.taxi_paths": ("TaxiPath", "TaxiPathNode"),
}
KINDS = {
    "world.areas": "area", "world.ui_maps": "ui_map", "world.ui_map_assignments": "ui_map_assignment",
    "items.items": "item", "spells.spells": "spell", "talents.talents": "talent",
    "travel.taxi_nodes": "taxi_node", "travel.taxi_paths": "taxi_path",
}

_CONTROL = re.compile(r"[\x00-\x09\x0b-\x1f\x7f-\x9f]")


# ---------------------------------------------------------------- field helpers
def n(row, key):
    """Integer column or None when absent or empty."""
    return db2.num(row.get(key))


def u(row, key):
    """Unsigned column: None when absent, empty or negative (never invented)."""
    v = n(row, key)
    return v if v is not None and v >= 0 else None


def nz(row, key):
    """Unsigned id column where 0 means none: None for 0."""
    v = u(row, key)
    return v or None


def f(row, key):
    return float(row[key])


def text(value):
    """Client text: CRLF -> LF, tab -> space, other control characters dropped,
    at most 8192 UTF-8 bytes; empty -> None."""
    if value is None:
        return None
    s = value.replace("\r\n", "\n").replace("\r", "\n").replace("\t", " ")
    s = _CONTROL.sub("", s)
    b = s.encode("utf-8")
    if len(b) > MAX_TEXT:
        s = b[:MAX_TEXT].decode("utf-8", "ignore")
    return s if s.strip() else None


def signed64(value):
    value &= I64 - 1
    return value - I64 if value >= 1 << 63 else value


def mask64(low, high):
    """Combine two 32-bit halves (as the CSV gives them, possibly negative) into a
    signed 64-bit mask; both halves -1 (all bits) is -1. None if either is missing."""
    if low is None or high is None:
        return None
    return signed64(((high & 0xFFFFFFFF) << 32) | (low & 0xFFFFFFFF))


def by_id(rows):
    return {int(r["ID"]): r for r in rows}


# ---------------------------------------------------------------- record mapping
def area(r):
    return {"id": int(r["ID"]), "name": text(r.get("AreaName_lang")), "map_id": u(r, "ContinentID"),
            "parent_id": nz(r, "ParentAreaID"),
            # 0 is the unset value (Elwynn Forest, Westfall, ... carry 0), not a level.
            "exploration_level": nz(r, "ExplorationLevel"),
            "faction_group_mask": u(r, "FactionGroupMask")}


def ui_map(r):
    return {"id": int(r["ID"]), "name": text(r.get("Name_lang")), "parent_ui_map_id": nz(r, "ParentUiMapID"),
            "map_type": u(r, "Type")}


def ui_map_assignment(r):
    return {"ui_map_id": int(r["UiMapID"]), "area_id": nz(r, "AreaID"), "map_id": int(r["MapID"]),
            "order_index": u(r, "OrderIndex") or 0,
            "ui_min": [f(r, "UiMin_0"), f(r, "UiMin_1")], "ui_max": [f(r, "UiMax_0"), f(r, "UiMax_1")],
            "region_min": [f(r, f"Region_{i}") for i in range(3)],
            "region_max": [f(r, f"Region_{i}") for i in range(3, 6)]}


def stat_name(stat_type):
    return STAT_NAMES.get(stat_type, f"stat_{stat_type}")


def budget(sparse, rand_prop_points):
    """The item's stat budget from RandPropPoints, or None when it has none."""
    col = SLOT_COLUMN.get(n(sparse, "InventoryType"))
    level = n(sparse, "ItemLevel")
    row = rand_prop_points.get(level)
    if col is None or row is None:
        return None
    quality = n(sparse, "OverallQualityID")
    tier = "Epic" if quality is not None and quality >= 4 else "Superior" if quality == 3 else "Good"
    value = row.get(f"{tier}_{col}")
    return float(value) if value not in (None, "") else None


def item_stats(sparse, rand_prop_points):
    """{stat_name: amount}; combat ratings and stats without a budget are left out."""
    points = budget(sparse, rand_prop_points)
    stats = {}
    for i in range(10):
        stat_type = n(sparse, f"StatModifier_bonusStat_{i}")
        share = n(sparse, f"StatPercentEditor_{i}")
        if stat_type is None or stat_type < 0 or not share or points is None or stat_type in RATINGS:
            continue
        amount = int(math.floor(points * share / 10000 + 0.5))
        if amount:
            key = stat_name(stat_type)
            stats[key] = stats.get(key, 0) + amount
    return {k: v for k, v in stats.items() if v}


def item(sparse, base, rand_prop_points):
    base = base or {}
    return {"id": int(sparse["ID"]), "name": text(sparse.get("Display_lang")),
            "quality": u(sparse, "OverallQualityID"),
            "class_id": u(base, "ClassID"), "subclass_id": u(base, "SubclassID"),
            "inventory_type": u(sparse, "InventoryType"), "item_level": u(sparse, "ItemLevel"),
            "required_level": u(sparse, "RequiredLevel"),
            "allowable_class": n(sparse, "AllowableClass"),
            "allowable_race": mask64(n(sparse, "AllowableRace_0"), n(sparse, "AllowableRace_1")),
            "bonding": u(sparse, "Bonding"), "max_stack": u(sparse, "Stackable"),
            "buy_price": u(sparse, "BuyPrice"), "sell_price": u(sparse, "SellPrice"),
            "stats": item_stats(sparse, rand_prop_points) or None}


def spell_masks(abilities):
    """(class_mask, race_mask) over a spell's SkillLineAbility rows: the union of
    their masks, or None when any row is unrestricted (0)."""
    def union(values):
        if not values or any(v in (None, 0) for v in values):
            return None
        out = 0
        for v in values:
            out |= v & (I64 - 1)
        return signed64(out)
    classes = [n(a, "ClassMask") for a in abilities]
    races = [mask64(n(a, "RaceMasks_0"), n(a, "RaceMasks_1")) for a in abilities]
    return union(classes), union(races)


def spell(spell_id, abilities, names, spells, levels):
    abilities = sorted(abilities, key=lambda a: int(a["ID"]))
    s = spells.get(spell_id, {})
    lv = levels.get(spell_id, {})
    class_mask, race_mask = spell_masks(abilities)
    return {"id": spell_id, "name": text(names.get(spell_id, {}).get("Name_lang")),
            "rank": text(s.get("NameSubtext_lang")), "description": text(s.get("Description_lang")),
            "skill_line": u(abilities[0], "SkillLine"), "class_mask": class_mask, "race_mask": race_mask,
            "base_level": u(lv, "BaseLevel"), "spell_level": u(lv, "SpellLevel"), "max_level": u(lv, "MaxLevel")}


def talent(r, tabs):
    tab = tabs.get(n(r, "TabID"), {})
    ranks = [v for v in (n(r, f"SpellRank_{i}") for i in range(9)) if v and v > 0]
    return {"id": int(r["ID"]), "tab_id": int(r["TabID"]), "tab_name": text(tab.get("Name_lang")),
            "class_mask": n(tab, "ClassMask") or None, "tier": u(r, "TierID"), "column": u(r, "ColumnIndex"),
            "ranks": ranks, "prerequisite": nz(r, "PrereqTalent_0")}


def taxi_faction(r):
    """MountCreatureID_0 is the Horde mount (2224 wind rider, 3574 bat) and _1 the
    Alliance one (541 gryphon, 3837 hippogryph) in this client."""
    horde, alliance = nz(r, "MountCreatureID_0"), nz(r, "MountCreatureID_1")
    if horde and alliance:
        return "both"
    return "horde" if horde else "alliance" if alliance else None


def world_point(map_id, x, y, z):
    return {"map_id": map_id, "x": x, "y": y, "z": z}


def taxi_node(r, locator):
    map_id = int(r["ContinentID"])
    x, y, z = f(r, "Pos_0"), f(r, "Pos_1"), f(r, "Pos_2")
    return {"id": int(r["ID"]), "name": text(r.get("Name_lang")), "world": world_point(map_id, x, y, z),
            "position": locator.observation(map_id, x, y, z), "faction": taxi_faction(r)}


def taxi_path(r, nodes):
    """nodes: the TaxiPathNode rows of this path, any order."""
    ordered = sorted(nodes, key=lambda p: (int(p["NodeIndex"]), int(p["ID"])))
    return {"id": int(r["ID"]), "from_node": int(r["FromTaxiNode"]), "to_node": int(r["ToTaxiNode"]),
            "cost": u(r, "Cost"),
            "points": [world_point(int(p["ContinentID"]), f(p, "Loc_0"), f(p, "Loc_1"), f(p, "Loc_2"))
                       for p in ordered]}


# ---------------------------------------------------------------- dataset builders
def build_areas(t):
    return [a for a in map(area, t["AreaTable"]) if a["name"]]


def build_ui_maps(t):
    return [m for m in map(ui_map, t["UiMap"]) if m["name"]]


def build_ui_map_assignments(t):
    return [ui_map_assignment(r) for r in t["UiMapAssignment"]]


def build_items(t):
    base = by_id(t["Item"])
    rpp = by_id(t["RandPropPoints"])
    return [i for i in (item(r, base.get(int(r["ID"])), rpp) for r in t["ItemSparse"]) if i["name"]]


def build_spells(t):
    abilities = {}
    for a in t["SkillLineAbility"]:
        sid = n(a, "Spell")
        if sid and sid > 0:
            abilities.setdefault(sid, []).append(a)
    names, spells = by_id(t["SpellName"]), by_id(t["Spell"])
    levels = {}
    for lv in t["SpellLevels"]:
        if n(lv, "DifficultyID") == 0:
            levels.setdefault(int(lv["SpellID"]), lv)
    out = [spell(sid, rows, names, spells, levels) for sid, rows in abilities.items()]
    return [s for s in out if s["name"]]


def build_talents(t):
    tabs = by_id(t["TalentTab"])
    out = [talent(r, tabs) for r in t["Talent"]]
    return [x for x in out if x["ranks"] and x["tier"] is not None and x["column"] is not None]


def build_taxi_nodes(t):
    locator = coords.Locator(t["UiMapAssignment"], t["UiMap"])
    return [x for x in (taxi_node(r, locator) for r in t["TaxiNodes"]) if x["name"]]


def build_taxi_paths(t):
    points = {}
    for p in t["TaxiPathNode"]:
        points.setdefault(int(p["PathID"]), []).append(p)
    return [taxi_path(r, points.get(int(r["ID"]), [])) for r in t["TaxiPath"]]


BUILDERS = {
    "world.areas": (build_areas, lambda r: r["id"]),
    "world.ui_maps": (build_ui_maps, lambda r: r["id"]),
    "world.ui_map_assignments": (build_ui_map_assignments, lambda r: (
        r["ui_map_id"], r["map_id"], r["order_index"], r.get("area_id") or 0, r["region_min"], r["region_max"])),
    "items.items": (build_items, lambda r: r["id"]),
    "spells.spells": (build_spells, lambda r: r["id"]),
    "talents.talents": (build_talents, lambda r: r["id"]),
    "travel.taxi_nodes": (build_taxi_nodes, lambda r: r["id"]),
    "travel.taxi_paths": (build_taxi_paths, lambda r: r["id"]),
}


# ---------------------------------------------------------------- run
def source_for(wago, tables):
    """Source ref whose digest covers exactly the CSVs this dataset read."""
    urls = {wago.url(t) for t in tables}
    pairs = sorted((url, m.get("sha256") or str(m.get("status"))) for url, m in http.used().items() if url in urls)
    digest = hashlib.sha256(json.dumps(pairs).encode()).hexdigest()
    return envelope.source(f"wago-tools-{wago.product}", f"https://wago.tools/db2?product={wago.product}",
                           revision=wago.build, snapshot_sha256=digest)


class Tables:
    """One Wago build's tables, each read once; None for a table the build lacks."""
    def __init__(self, wago):
        self.wago, self._rows = wago, {}

    def get(self, table):
        if table not in self._rows:
            try:
                self._rows[table] = self.wago.table(table)
            except http.FetchError as e:
                if e.status not in (400, 404):  # wago answers 400 for a table the build lacks
                    raise
                self._rows[table] = None
        return self._rows[table]

    def all(self, tables):
        rows = {t: self.get(t) for t in tables}
        return rows if all(v is not None for v in rows.values()) else None


def run(out, product="wowf", build=None):
    primary = Tables(db2.Wago(product, build))
    fallback = []

    files, counts = [], {}
    for dataset, (builder, sort_key) in BUILDERS.items():
        tables = TABLES[dataset]
        source, rows = primary, primary.all(tables)
        if rows is None and product != FALLBACK_PRODUCT:
            if not fallback:
                fallback.append(Tables(db2.Wago(FALLBACK_PRODUCT)))
            source, rows = fallback[0], fallback[0].all(tables)
            print(f"{dataset}: {product} {primary.wago.build} lacks a table; read from {FALLBACK_PRODUCT} "
                  f"{source.wago.build} under that edition's origin", file=sys.stderr)
        if rows is None:
            raise RuntimeError(f"{dataset}: no wago build has {', '.join(tables)}")
        wago = source.wago
        records = builder(rows)
        origin = wago.origin()
        paths = envelope.write(dataset, KINDS[dataset], records, origin=origin, sources=[source_for(wago, tables)],
                               generator=envelope.generator(NAME, VERSION, credit=envelope.NAVZEROTH_CREDIT),
                               out=out, sort_key=sort_key)
        files.extend(p.relative_to(out).as_posix() for p in paths)
        counts[dataset] = (len(records), origin["edition"], wago.build)
        print(f"{dataset}: {len(records)} {KINDS[dataset]} records ({origin['edition']} {wago.build}, "
              f"{len(paths)} shard{'s' if len(paths) != 1 else ''})")
    marker = out / ".written" / f"{NAME}.json"
    marker.parent.mkdir(parents=True, exist_ok=True)
    marker.write_text(json.dumps({"files": sorted(files), "used": http.used()}, sort_keys=True, indent=1) + "\n",
                      encoding="utf-8")
    return counts


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--out", type=pathlib.Path, default=envelope.OUTPUT)
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--refresh", action="store_true", help="re-download cached tables")
    mode.add_argument("--offline", action="store_true", help="use the cache only")
    p.add_argument("--wago-product", default="wowf", choices=sorted(db2.EDITIONS))
    p.add_argument("--wago-build", help="pin a build instead of the newest")
    a = p.parse_args(argv)
    http.configure(refresh_cache=a.refresh or None, offline_only=a.offline or None)
    try:
        run(a.out.resolve(), a.wago_product, a.wago_build)
    except Exception as e:  # noqa: BLE001 - report and fail the run
        print(f"{NAME}: failed: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
