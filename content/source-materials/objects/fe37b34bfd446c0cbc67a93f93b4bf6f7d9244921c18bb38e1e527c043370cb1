#!/usr/bin/env python3
"""Whole-world server data from the CMaNGOS classic-db dump (navzeroth-world-data v1).

Every race, class, faction, zone and dungeon of the CMaNGOS Classic database
(https://github.com/cmangos/classic-db, Full_DB/ClassicDB_1_12_1_*.sql.gz), the
community reconstruction of the 1.12 server tables. The dump is read as data
(navzeroth_datagen.sqldump); its SQL is never executed.

Datasets (origin vanilla_1_12 / 1.12.1, product cmangos-classic-db, confidence
cross_edition_unverified; never relabelled as Forever data):

  world.npcs            creature_template
  world.game_objects    gameobject_template
  world.spawns          creature + gameobject (+ *_spawn_entry for random entries)
  quests.quests         quest_template + quest relations + item_template.startquest
  vendors.items         npc_vendor + npc_vendor_template (VendorTemplateId)
  trainers.spells       npc_trainer + npc_trainer_template (TrainerTemplateId)
  loot.<table>          the eight *_loot_template tables (loot.game_object keyed by game object)
  items.items           item_template (the fallback for items the Forever client table lacks)
  dungeons.dungeons     instance_template + areatrigger_teleport + bosses
  world.graveyards      world_safe_locs + game_graveyard_zone
  world.area_levels     creature level range per zone, from located spawns

Spawn, graveyard and dungeon `world` points stay CMaNGOS 1.12 world coordinates.
The derived `position` (and a spawn's zone `area_id`) uses the WoW Forever
client's map layout: wago.tools `wowf` UiMapAssignment + UiMap through
navzeroth_datagen.coords.Locator, so it is the Forever client's map position of
1.12 server data. It is `unsupported` where no uiMap covers the point. Where
zone rectangles overlap, the server's quest POI points (of quests whose zone is
that uiMap's area) pick the uiMap (Locator hints). Zone uiMaps whose area the
server data never names (zones the Forever client added, e.g. Riverglades,
Mount Hyjal) are left out of that layout. Names of maps (dungeons) and areas,
NPC reactions (FactionTemplate) and dungeon entrance points (AreaTrigger) also
come from the wowf client tables; an entrance whose teleport trigger the client
table lacks stays `unknown`.

Field rules: an absent field means the source does not give it. Source
quantities are kept as stored, zero included (quest money, races / classes
masks 0 = any, levels, costs). Reference IDs stored as 0 mean "none" and are
omitted (previous / next quest, exclusive group, reward spell, source item,
required skill, loot group and condition). The 1.12 quest table has no XP
column, so reward_xp is never written. A spawn with entry 0 (random entry)
gives one record per candidate of creature_spawn_entry / gameobject_spawn_entry
under the same guid; a negative spawn time (despawned until a script spawns it)
gives no respawn_secs. A loot reference row has min_count 1 and max_count = the
times the reference is rolled (maxcount, at least 1). gameobject_loot_template is
keyed by its lootId, which chests (type 3) and fishing holes (type 25) name in
gameobject_template.data1; its rows are written once per such game object with
`entry` = the game object entry (a loot ID no object names is left out and
reported), so every loot entry except item, reference, fishing and disenchant is
an entity ID. Disenchant loot stays keyed by DisenchantID: every ID is shared by
several items, so it names no single entity. items.items maps item_template:
stats are stat_typeN / stat_valueN under the ITEM_MOD_* names gen_world_db2 uses,
plus `armor` and the `<school>_resistance` columns. Area level ranges count
attackable, non-friendly creatures that are not critters, totems, triggers
(creature type 10), bosses or service NPCs; they are plain min / max, so a
stray high-level spawn widens them.

NPC reaction (`alliance`, `horde`, `both`, `hostile`) compares the creature's
faction template with the Human (1) and Orc (2) player templates by the client's
group masks and enemy / friend lists. Default "at war" reputations are not
applied; a creature neutral to a side (neither friendly nor hostile to it) and
not friendly to the other gets no reaction.

    python3 gen_world_cmangos.py [--out DIR] [--refresh] [--offline] [--wago-product wowf]
"""
import argparse
import collections
import json
import pathlib
import re
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from navzeroth_datagen import coords, db2, envelope, http, sqldump  # noqa: E402

NAME = "gen_world_cmangos"
MAX_TEXT = 8192
CONTROL = re.compile(r"[\x00-\x09\x0b-\x1f\x7f]")

# Vanilla 1.12 UNIT_NPC_FLAG bits (mangos-classic Unit.h), checked against the
# dump: Innkeeper Farley 295 = 0x87, Thor 523 = 0xb, Auctioneer Fitch 8719 =
# 0x1000, Olivia Burnside 2455 = 0x101, Erma 6749 = 0x2001, Spirit Healer 6491 =
# 0x21, Thelman Slatefist 7410 = 0x801. Gossip, spirit guide, petitioner and
# tabard designer have no role in the contract.
NPC_ROLE_BITS = [(0x2, "quest_giver"), (0x4, "vendor"), (0x4000, "repair"), (0x10, "trainer"),
                 (0x80, "innkeeper"), (0x8, "flight_master"), (0x100, "banker"), (0x1000, "auctioneer"),
                 (0x20, "spirit_healer"), (0x2000, "stable_master"), (0x800, "battlemaster")]
ROLE_ORDER = ["quest_giver", "vendor", "repair", "trainer", "class_trainer", "profession_trainer", "innkeeper",
              "flight_master", "banker", "auctioneer", "spirit_healer", "stable_master", "battlemaster"]
TRAINER_FLAG = 0x10
TRAINER_CLASS, TRAINER_TRADESKILLS = 0, 2
RANK_BOSS = 3
CRITTER, NOT_SPECIFIED, TOTEM = 8, 10, 11
UNIT_FLAG_NON_ATTACKABLE, UNIT_FLAG_NOT_SELECTABLE = 0x2, 0x02000000
QUEST_SPECIAL_EXPLORATION_OR_EVENT = 0x2
TEAMS = {469: "alliance", 67: "horde"}
PLAYER_TEMPLATES = {"alliance": 1, "horde": 2}  # Human, Orc
LOOT_TABLES = [("creature", "creature_loot_template"), ("game_object", "gameobject_loot_template"),
               ("item", "item_loot_template"), ("reference", "reference_loot_template"),
               ("fishing", "fishing_loot_template"), ("pickpocketing", "pickpocketing_loot_template"),
               ("skinning", "skinning_loot_template"), ("disenchant", "disenchant_loot_template")]
DUNGEON_INSTANCE_TYPES = {1, 2}  # Map.InstanceType: party, raid (3 pvp, 4 arena are left out)
ENTITY_ORDER = {"npc": 0, "game_object": 1, "item": 2}
GO_LOOT_TYPES = {3, 25}  # GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_FISHINGHOLE: data1 = lootId
# item_template columns kept as stats beside stat_typeN (names as the ITEM_MOD_* vocabulary).
ITEM_EXTRA_STATS = [("armor", "armor"), ("holy_res", "holy_resistance"), ("fire_res", "fire_resistance"),
                    ("nature_res", "nature_resistance"), ("frost_res", "frost_resistance"),
                    ("shadow_res", "shadow_resistance"), ("arcane_res", "arcane_resistance")]


def text(value):
    """Trimmed text without control characters (newlines kept), at most 8192 bytes; None if empty."""
    if value is None:
        return None
    s = CONTROL.sub("", str(value).replace("\r\n", "\n").replace("\r", "\n")).strip()
    if not s:
        return None
    b = s.encode("utf-8")
    if len(b) > MAX_TEXT:
        s = b[:MAX_TEXT].decode("utf-8", "ignore").rstrip()
    return s


def num(value):
    """Integer from a dump cell; None stays None."""
    return None if value is None or value == "" else int(value)


def ref(value):
    """A reference ID: 0 or NULL means none (None)."""
    v = num(value)
    return v if v else None


def positive(value):
    v = num(value)
    return v if v and v > 0 else None


# ---------------------------------------------------------------- npcs


def npc_roles(npc_flags, trainer_type=None, trainer_class=None):
    """Contract roles from NpcFlags (contract order); trainer kinds from TrainerType / TrainerClass."""
    flags = npc_flags or 0
    roles = {role for bit, role in NPC_ROLE_BITS if flags & bit}
    if flags & TRAINER_FLAG:
        if trainer_type == TRAINER_CLASS and trainer_class:
            roles.add("class_trainer")
        elif trainer_type == TRAINER_TRADESKILLS:
            roles.add("profession_trainer")
    return [r for r in ROLE_ORDER if r in roles]


class Reactions:
    """NPC reaction to each side from client FactionTemplate rows (wago CSV dicts)."""

    def __init__(self, faction_templates):
        self.rows = {}
        for r in faction_templates:
            self.rows[int(r["ID"])] = {
                "faction": int(r["Faction"]), "group": int(r["FactionGroup"]),
                "friend_group": int(r["FriendGroup"]), "enemy_group": int(r["EnemyGroup"]),
                "enemies": {int(r[f"Enemies_{i}"]) for i in range(8) if f"Enemies_{i}" in r and int(r[f"Enemies_{i}"])},
                "friends": {int(r[f"Friend_{i}"]) for i in range(8) if f"Friend_{i}" in r and int(r[f"Friend_{i}"])},
            }

    @staticmethod
    def _relation(npc, player):
        """'friendly', 'hostile' or 'neutral', as the client's IsFriendlyTo / IsHostileTo."""
        if player["faction"] in npc["enemies"]:
            return "hostile"
        if player["faction"] in npc["friends"] or player["faction"] == npc["faction"]:
            return "friendly"
        if npc["enemy_group"] & player["group"] or npc["group"] & player["enemy_group"]:
            return "hostile"
        if npc["friend_group"] & player["group"] or npc["group"] & player["friend_group"]:
            return "friendly"
        return "neutral"

    def reaction(self, template_id):
        npc = self.rows.get(template_id)
        players = {side: self.rows.get(t) for side, t in PLAYER_TEMPLATES.items()}
        if npc is None or None in players.values():
            return None
        a, h = (self._relation(npc, players[s]) for s in ("alliance", "horde"))
        if a == "friendly" and h == "friendly":
            return "both"
        if a == "friendly":
            return "alliance"
        if h == "friendly":
            return "horde"
        if a == "hostile" and h == "hostile":
            return "hostile"
        return None


def npc_record(row, reactions=None):
    name = text(row["Name"])
    if not name:
        return None
    faction = num(row.get("Faction"))
    roles = npc_roles(row.get("NpcFlags"), row.get("TrainerType"), row.get("TrainerClass"))
    return {
        "id": num(row["Entry"]), "name": name, "subname": text(row.get("SubName")),
        "min_level": num(row.get("MinLevel")), "max_level": num(row.get("MaxLevel")),
        "faction_template": faction,
        "reaction": reactions.reaction(faction) if reactions and faction is not None else None,
        "npc_flags": num(row.get("NpcFlags")), "rank": num(row.get("Rank")),
        "creature_type": num(row.get("CreatureType")),
        "roles": roles or None,
        "trainer_class": ref(row.get("TrainerClass")) if "class_trainer" in roles else None,
    }


def game_object_record(row):
    name = text(row["name"])
    if not name:
        return None
    return {"id": num(row["entry"]), "name": name, "object_type": num(row.get("type"))}


# ---------------------------------------------------------------- spawns


def zone_areas(assignments, ui_maps):
    """uiMap ID -> AreaID for zone and dungeon uiMaps (the lowest OrderIndex row with an area)."""
    types = {int(r["ID"]): int(r.get("Type") or 6) for r in ui_maps}
    best = {}
    for r in assignments:
        ui, area = int(r["UiMapID"]), int(r.get("AreaID") or 0)
        if area and types.get(ui) in (3, 4):
            key = (int(r.get("OrderIndex") or 0), int(r.get("ID") or 0))
            if ui not in best or key < best[ui][0]:
                best[ui] = (key, area)
    return {ui: area for ui, (_, area) in best.items()}


def server_zones(dump):
    """Zone IDs the 1.12 server data names: quest zones, graveyard zones and fishing areas."""
    zones = {r["ZoneOrSort"] for r in dump.rows("quest_template") if (r.get("ZoneOrSort") or 0) > 0}
    zones.update(r["ghost_loc"] for r in dump.rows("game_graveyard_zone"))
    zones.update(r["entry"] for r in dump.rows("skill_fishing_base_level"))
    return zones


def server_zone_assignments(assignments, ui_maps, zones):
    """Drops zone uiMaps whose area the server data never names (zones the Forever client added, such
    as Riverglades or Mount Hyjal), so 1.12 spawns are not placed in them; other maps are kept."""
    types = {int(r["ID"]): int(r.get("Type") or 6) for r in ui_maps}
    return [r for r in assignments
            if types.get(int(r["UiMapID"])) != 3 or int(r.get("AreaID") or 0) in zones | {0}]


def poi_hints(pois, points, locator, quest_zones, zone_of_ui):
    """(map_id, ui_map_id, x, y) hints from the server's quest POI points.

    quest_poi labels each POI with a 3.x WorldMapArea ID (mapAreaId) the client
    no longer has. Each mapAreaId is matched to the uiMap that holds the most of
    its points (at least half; ties go to the more specific, smaller map, so a
    city keeps its own map). A point becomes a hint for that uiMap only when its
    quest's zone (quest_zones: quest -> ZoneOrSort) is the uiMap's area
    (zone_of_ui), since a POI drawn on a neighbour's map often crosses the border."""
    area_of = {(p["questId"], p["poiId"]): (p["mapId"], p["mapAreaId"]) for p in pois}
    by_area = collections.defaultdict(list)
    for p in points:
        key = area_of.get((p["questId"], p["poiId"]))
        if key and key[1]:
            by_area[key].append((float(p["x"]), float(p["y"]), p["questId"]))
    hints = set()
    for (map_id, _), pts in sorted(by_area.items()):
        held = [locator.candidates(map_id, x, y) for x, y, _ in pts]
        count, order = collections.Counter(), {}
        for uis in held:
            for i, ui in enumerate(dict.fromkeys(uis)):
                count[ui] += 1
                order[ui] = min(order.get(ui, i), i)
        if not count:
            continue
        ui = max(count, key=lambda u: (count[u], -order[u], -u))
        if count[ui] * 2 < len(pts):
            continue
        zone = zone_of_ui.get(ui)
        hints.update((map_id, ui, x, y) for (x, y, quest), uis in zip(pts, held)
                     if ui in uis and zone is not None and quest_zones.get(quest) == zone)
    return sorted(hints)


def spawn_entries(rows):
    """guid -> sorted entry list from creature_spawn_entry / gameobject_spawn_entry."""
    out = collections.defaultdict(set)
    for r in rows:
        out[r["guid"]].add(r["entry"])
    return {g: sorted(e) for g, e in out.items()}


def respawn_secs(row):
    """spawntimesecsmin; a negative value (an object that stays despawned until a script spawns it) is
    not a respawn time and is omitted."""
    v = num(row.get("spawntimesecsmin"))
    return v if v is not None and v >= 0 else None


def spawn_records(rows, entity, locator, zone_of_ui=None, random_entries=None, known_ids=None):
    """One record per spawn and entity (a random-entry spawn, id 0, gives one per candidate)."""
    out = []
    for r in rows:
        ids = [r["id"]] if r["id"] else (random_entries or {}).get(r["guid"], [])
        world = {"map_id": r["map"], "x": float(r["position_x"]), "y": float(r["position_y"]),
                 "z": float(r["position_z"])}
        position = locator.observation(world["map_id"], world["x"], world["y"], world["z"])
        area = (zone_of_ui or {}).get(position["value"]["ui_map_id"]) if position["status"] == "known" else None
        for entity_id in ids:
            if known_ids is not None and entity_id not in known_ids:
                continue
            out.append({"guid": r["guid"], "entity": entity, "entity_id": entity_id, "world": world,
                        "position": position, "area_id": area, "respawn_secs": respawn_secs(r)})
    return out


# ---------------------------------------------------------------- quests


def relations(rows, entity):
    """quest -> [EntityRef] from a *_questrelation / *_involvedrelation table."""
    out = collections.defaultdict(list)
    for r in rows:
        out[r["quest"]].append({"entity": entity, "id": r["id"]})
    return out


def quest_objectives(row, explore_triggers=()):
    out = []
    for i in range(1, 5):
        target, count = num(row.get(f"ReqCreatureOrGOId{i}")) or 0, num(row.get(f"ReqCreatureOrGOCount{i}")) or 0
        spell = ref(row.get(f"ReqSpellCast{i}"))
        label = text(row.get(f"ObjectiveText{i}"))
        if target and count > 0:
            if spell:  # cast the spell on the creature / object
                out.append({"objective": "spell", "target_id": spell, "count": count, "text": label})
            elif target < 0:
                out.append({"objective": "object", "target_id": -target, "count": count, "text": label})
            else:
                out.append({"objective": "kill", "target_id": target, "count": count, "text": label})
    for i in range(1, 5):
        item, count = positive(row.get(f"ReqItemId{i}")), num(row.get(f"ReqItemCount{i}")) or 0
        if item and count > 0:
            out.append({"objective": "item", "target_id": item, "count": count})
    for trigger in sorted(explore_triggers):
        out.append({"objective": "explore", "target_id": trigger, "count": 1})
    if not explore_triggers and (num(row.get("SpecialFlags")) or 0) & QUEST_SPECIAL_EXPLORATION_OR_EVENT:
        out.append({"objective": "event", "count": 1})
    return out


def item_list(row, ids, counts):
    out = []
    for id_col, count_col in zip(ids, counts):
        item, count = positive(row.get(id_col)), num(row.get(count_col)) or 0
        if item and count > 0:
            out.append({"item": item, "count": count})
    return out


def _refs(refs):
    unique = {(e["entity"], e["id"]): e for e in refs}
    return [unique[k] for k in sorted(unique, key=lambda k: (ENTITY_ORDER[k[0]], k[1]))] or None


def quest_record(row, givers=(), enders=(), explore_triggers=()):
    title = text(row["Title"])
    if not title:
        return None
    skill = ref(row.get("RequiredSkill"))
    return {
        "id": num(row["entry"]), "title": title, "level": num(row.get("QuestLevel")),
        "min_level": num(row.get("MinLevel")), "zone_or_sort": num(row.get("ZoneOrSort")),
        "required_races": num(row.get("RequiredRaces")), "required_classes": num(row.get("RequiredClasses")),
        "required_skill": [skill, num(row.get("RequiredSkillValue")) or 0] if skill else None,
        "prev_quest": ref(row.get("PrevQuestId")), "next_quest": ref(row.get("NextQuestId")),
        "next_in_chain": ref(row.get("NextQuestInChain")), "exclusive_group": ref(row.get("ExclusiveGroup")),
        "givers": _refs(givers), "enders": _refs(enders), "objectives_text": text(row.get("Objectives")),
        "objectives": quest_objectives(row, explore_triggers) or None,
        "reward_choice_items": item_list(row, [f"RewChoiceItemId{i}" for i in range(1, 7)],
                                         [f"RewChoiceItemCount{i}" for i in range(1, 7)]) or None,
        "reward_items": item_list(row, [f"RewItemId{i}" for i in range(1, 5)],
                                  [f"RewItemCount{i}" for i in range(1, 5)]) or None,
        "reward_money": num(row.get("RewOrReqMoney")),
        "reward_xp": num(row["RewXP"]) if "RewXP" in row else None,
        "reward_spell": ref(row.get("RewSpell")), "source_item": ref(row.get("SrcItemId")),
        "flags": num(row.get("QuestFlags")),
    }


def quest_records(dump):
    givers, enders = collections.defaultdict(list), collections.defaultdict(list)
    for table, entity, target in [("creature_questrelation", "npc", givers),
                                  ("gameobject_questrelation", "game_object", givers),
                                  ("creature_involvedrelation", "npc", enders),
                                  ("gameobject_involvedrelation", "game_object", enders)]:
        for quest, refs in relations(dump.rows(table), entity).items():
            target[quest].extend(refs)
    for item in dump.rows("item_template"):
        if item.get("startquest"):
            givers[item["startquest"]].append({"entity": "item", "id": item["entry"]})
    explore = collections.defaultdict(set)
    for r in dump.rows("areatrigger_involvedrelation"):
        explore[r["quest"]].add(r["id"])
    out = []
    for row in dump.rows("quest_template"):
        q = row["entry"]
        rec = quest_record(row, givers.get(q, ()), enders.get(q, ()), explore.get(q, ()))
        if rec:
            out.append(rec)
    return out


# ---------------------------------------------------------------- vendors, trainers, loot


def expand_templates(direct, templates, template_of_npc, key):
    """Rows per NPC: its own rows plus the rows of its template (direct rows win on a duplicate key)."""
    by_template = collections.defaultdict(list)
    for r in templates:
        by_template[r["entry"]].append(r)
    out = {}
    for r in direct:
        out.setdefault((r["entry"], r[key]), r)
    for npc, template in sorted(template_of_npc.items()):
        for r in by_template.get(template, ()):
            out.setdefault((npc, r[key]), dict(r, entry=npc))
    return [out[k] for k in sorted(out)]


def vendor_record(row):
    limited = positive(row.get("maxcount"))
    return {"npc_id": row["entry"], "item_id": row["item"], "max_count": limited,
            "restock_secs": positive(row.get("incrtime")) if limited else None}


def trainer_record(row):
    skill = ref(row.get("reqskill"))
    return {"npc_id": row["entry"], "spell_id": row["spell"], "cost": num(row.get("spellcost")),
            "required_skill_line": skill, "required_skill": num(row.get("reqskillvalue")) if skill else None,
            "required_level": num(row.get("reqlevel"))}


def loot_record(table, row):
    """mincountOrRef < 0 is a reference to reference_loot_template -mincountOrRef (rolled maxcount times)."""
    lead, maxcount = num(row["mincountOrRef"]) or 0, num(row.get("maxcount")) or 0
    rec = {"table": table, "entry": row["entry"], "chance": float(row["ChanceOrQuestChance"] or 0),
           "group": ref(row.get("groupid")), "condition": ref(row.get("condition_id"))}
    if lead < 0:
        rec.update(reference=-lead, min_count=1, max_count=max(1, maxcount))
    else:
        rec.update(item=row["item"], min_count=lead, max_count=max(lead, maxcount))
    return rec


def game_object_loot_owners(go_templates):
    """loot ID -> sorted game object entries whose template names it: data1 is the lootId of
    chests (type 3) and fishing holes (type 25); the other types use data1 for something else."""
    out = collections.defaultdict(set)
    for r in go_templates:
        loot = positive(r.get("data1"))
        if r.get("type") in GO_LOOT_TYPES and loot:
            out[loot].add(r["entry"])
    return {k: sorted(v) for k, v in out.items()}


def game_object_loot(rows, owners, known_ids):
    """gameobject_loot_template rows keyed by the game object that drops them (one copy per
    object whose data1 names the loot ID), so `entry` is an entity ID like creature loot.
    Returns (records, loot IDs no known object names)."""
    out, orphans = [], set()
    for r in rows:
        objects = [o for o in owners.get(r["entry"], ()) if o in known_ids]
        if not objects:
            orphans.add(r["entry"])
        for obj in objects:
            out.append(loot_record("game_object", dict(r, entry=obj)))
    return out, orphans


# ---------------------------------------------------------------- items


def item_stats(row):
    """{stat name: amount} from stat_typeN / stat_valueN (ITEM_MOD_* names as gen_world_db2 uses),
    plus armor and the six resistances; zero amounts are left out."""
    stats = {}
    for i in range(1, 11):
        stat_type, value = num(row.get(f"stat_type{i}")), num(row.get(f"stat_value{i}")) or 0
        if stat_type is None or not value:
            continue
        key = db2.stat_name(stat_type)
        stats[key] = stats.get(key, 0) + value
    for column, key in ITEM_EXTRA_STATS:
        value = num(row.get(column)) or 0
        if value:
            stats[key] = stats.get(key, 0) + value
    return {k: v for k, v in sorted(stats.items()) if v}


def unsigned(value):
    """A u32 column: None when absent or negative (never invented)."""
    v = num(value)
    return v if v is not None and v >= 0 else None


def item_record(row):
    """One items.items record of item_template (the 1.12 server's item table)."""
    name = text(row.get("name"))
    if not name:
        return None
    return {"id": num(row["entry"]), "name": name, "quality": unsigned(row.get("Quality")),
            "class_id": unsigned(row.get("class")), "subclass_id": unsigned(row.get("subclass")),
            "inventory_type": unsigned(row.get("InventoryType")), "item_level": unsigned(row.get("ItemLevel")),
            "required_level": unsigned(row.get("RequiredLevel")),
            "allowable_class": num(row.get("AllowableClass")), "allowable_race": num(row.get("AllowableRace")),
            "bonding": unsigned(row.get("bonding")), "max_stack": unsigned(row.get("stackable")),
            "buy_price": unsigned(row.get("BuyPrice")), "sell_price": unsigned(row.get("SellPrice")),
            "stats": item_stats(row) or None}


# ---------------------------------------------------------------- dungeons, graveyards, areas


def trigger_entrances(teleports, client_triggers):
    """target map -> WorldPoint of its lowest-ID teleport trigger that the client table places outside it."""
    rows = {int(r["ID"]): r for r in client_triggers}
    out = {}
    for t in sorted(teleports, key=lambda t: t["id"]):
        c = rows.get(t["id"])
        if c is None or int(c["ContinentID"]) == t["target_map"] or t["target_map"] in out:
            continue
        out[t["target_map"]] = {"map_id": int(c["ContinentID"]), "x": float(c["Pos_0"]), "y": float(c["Pos_1"]),
                                "z": float(c["Pos_2"])}
    return out


def dungeon_records(instances, maps, entrances, bosses, teleports=()):
    """instances: instance_template rows; maps: client Map rows by ID; bosses: map -> creature IDs."""
    fallback = {}
    for t in sorted(teleports, key=lambda t: t["id"]):
        label = text(re.sub(r"\s*\(.*\)$", "", (t.get("name") or "").split(" - ")[0]))
        if label:
            fallback.setdefault(t["target_map"], label)
    out = []
    for inst in instances:
        m = maps.get(inst["map"])
        if m is not None and int(m.get("InstanceType") or 0) not in DUNGEON_INSTANCE_TYPES:
            continue
        name = text(m.get("MapName_lang")) if m is not None else fallback.get(inst["map"])
        if not name:
            continue
        point = entrances.get(inst["map"])
        out.append({"map_id": inst["map"], "name": name, "min_level": positive(inst.get("levelMin")),
                    "max_level": positive(inst.get("levelMax")),
                    "entrance": envelope.known(point) if point else envelope.unknown(),
                    "bosses": sorted(bosses.get(inst["map"], ())) or None,
                    "area_id": positive(m.get("AreaTableID")) if m is not None else None})
    return out


def dungeon_bosses(templates, spawns, encounters, dungeon_encounters):
    """map -> boss creature IDs: rank 3 creatures spawned there plus kill-credit encounter creatures."""
    boss_ids = {r["Entry"] for r in templates if r.get("Rank") == RANK_BOSS}
    out = collections.defaultdict(set)
    for s in spawns:
        if s["id"] in boss_ids:
            out[s["map"]].add(s["id"])
    encounter_map = {r["Id"]: r["MapId"] for r in dungeon_encounters}
    for e in encounters:
        if e["creditType"] == 0 and e["creditEntry"] and e["entry"] in encounter_map:
            out[encounter_map[e["entry"]]].add(e["creditEntry"])
    return out


def graveyard_records(safe_locs, links, locator):
    zones, teams = collections.defaultdict(set), collections.defaultdict(set)
    for link in links:
        zones[link["id"]].add(link["ghost_loc"])
        teams[link["id"]].add(link.get("faction") or 0)
    out = []
    for r in safe_locs:
        world = {"map_id": r["map"], "x": float(r["x"]), "y": float(r["y"]), "z": float(r["z"])}
        sides = teams.get(r["id"])
        if not sides:
            faction = None
        elif 0 in sides or len(sides) > 1:
            faction = "both"
        else:
            faction = TEAMS.get(next(iter(sides)))
        out.append({"id": r["id"], "world": world,
                    "position": locator.observation(world["map_id"], world["x"], world["y"], world["z"]),
                    "faction": faction, "zones": sorted(zones.get(r["id"], ())) or None})
    return out


def counts_for_levels(npc):
    """Whether a creature counts toward an area's level range: attackable, not friendly, not a critter,
    totem, trigger, boss or service NPC."""
    flags = npc.get("UnitFlags") or 0
    return (npc.get("CreatureType") not in (CRITTER, NOT_SPECIFIED, TOTEM) and not npc.get("NpcFlags")
            and npc.get("Rank") != RANK_BOSS and not flags & (UNIT_FLAG_NON_ATTACKABLE | UNIT_FLAG_NOT_SELECTABLE)
            and (npc.get("MinLevel") or 0) > 0)


def area_level_records(spawns, templates, area_names, reactions=None):
    """Areas by spawn area_id with the level range of eligible creatures spawned there."""
    eligible = {}
    for r in templates:
        if counts_for_levels(r) and (reactions is None or reactions.reaction(r.get("Faction")) not in
                                     ("alliance", "horde", "both")):
            eligible[r["Entry"]] = (r["MinLevel"], max(r["MinLevel"], r.get("MaxLevel") or 0))
    ranges, maps = {}, collections.defaultdict(set)
    for s in spawns:
        area = s.get("area_id")
        if s["entity"] != "npc" or area is None or s["entity_id"] not in eligible or area not in area_names:
            continue
        lo, hi = eligible[s["entity_id"]]
        cur = ranges.get(area)
        ranges[area] = (min(lo, cur[0]), max(hi, cur[1])) if cur else (lo, hi)
        maps[area].add(s["world"]["map_id"])
    return [{"id": a, "name": area_names[a], "map_id": next(iter(maps[a])) if len(maps[a]) == 1 else None,
             "level_range": list(ranges[a])} for a in sorted(ranges)]


# ---------------------------------------------------------------- main


def parse_args(argv):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--out", default=str(envelope.OUTPUT), help="output folder (default data-generators/output)")
    p.add_argument("--refresh", action="store_true", help="download again instead of using the cache")
    p.add_argument("--offline", action="store_true", help="use the cache only")
    p.add_argument("--wago-product", default="wowf", help="wago.tools product for the client tables")
    return p.parse_args(argv)


def build(dump, wago):
    """{dataset: (record_kind, records, sort_key, uses_client_tables)}."""
    ui_maps = wago.table("UiMap")
    assignments = server_zone_assignments(wago.table("UiMapAssignment"), ui_maps, server_zones(dump))
    zone_of_ui = zone_areas(assignments, ui_maps)
    quest_zones = {r["entry"]: r["ZoneOrSort"] for r in dump.rows("quest_template")}
    hints = poi_hints(dump.rows("quest_poi"), dump.rows("quest_poi_points"), coords.Locator(assignments, ui_maps),
                      quest_zones, zone_of_ui)
    locator = coords.Locator(assignments, ui_maps, hints)
    reactions = Reactions(wago.table("FactionTemplate"))
    area_names = {int(r["ID"]): text(r["AreaName_lang"]) for r in wago.table("AreaTable") if text(r["AreaName_lang"])}
    maps = {int(r["ID"]): r for r in wago.table("Map")}
    client_triggers = wago.table("AreaTrigger") if wago.has("AreaTrigger") else []

    templates = dump.rows("creature_template")
    go_templates = dump.rows("gameobject_template")
    npcs = [r for r in (npc_record(t, reactions) for t in templates) if r]
    objects = [r for r in (game_object_record(t) for t in go_templates) if r]
    creature_rows, object_rows = dump.rows("creature"), dump.rows("gameobject")
    spawns = (spawn_records(creature_rows, "npc", locator, zone_of_ui,
                            spawn_entries(dump.rows("creature_spawn_entry")), {r["id"] for r in npcs})
              + spawn_records(object_rows, "game_object", locator, zone_of_ui,
                              spawn_entries(dump.rows("gameobject_spawn_entry")), {r["id"] for r in objects}))
    by_id = lambda r: r["id"]  # noqa: E731
    out = {
        "world.npcs": ("npc", npcs, by_id, True),
        "world.game_objects": ("game_object", objects, by_id, False),
        "world.spawns": ("spawn", spawns, lambda r: (ENTITY_ORDER[r["entity"]], r["guid"], r["entity_id"]), True),
        "quests.quests": ("quest", quest_records(dump), by_id, False),
    }
    vendor_templates = {r["Entry"]: r["VendorTemplateId"] for r in templates if r.get("VendorTemplateId")}
    trainer_templates = {r["Entry"]: r["TrainerTemplateId"] for r in templates if r.get("TrainerTemplateId")}
    vendors = expand_templates(dump.rows("npc_vendor"), dump.rows("npc_vendor_template"), vendor_templates, "item")
    trainers = expand_templates(dump.rows("npc_trainer"), dump.rows("npc_trainer_template"), trainer_templates,
                                "spell")
    out["vendors.items"] = ("vendor_item", [vendor_record(r) for r in vendors],
                            lambda r: (r["npc_id"], r["item_id"]), False)
    out["trainers.spells"] = ("trainer_spell", [trainer_record(r) for r in trainers],
                              lambda r: (r["npc_id"], r["spell_id"]), False)
    loot_key = lambda r: (r["entry"], r.get("group") or 0, r.get("item") or 0, r.get("reference") or 0,  # noqa: E731
                          r["chance"], r["min_count"], r["max_count"], r.get("condition") or 0)
    owners = game_object_loot_owners(go_templates)
    for table, sql_table in LOOT_TABLES:
        if table == "game_object":
            records, orphans = game_object_loot(dump.rows(sql_table), owners, {r["id"] for r in objects})
            if orphans:
                print(f"loot.game_object: {len(orphans)} loot IDs no chest or fishing hole names are left out "
                      f"({', '.join(map(str, sorted(orphans)[:10]))})", file=sys.stderr)
        else:
            records = [loot_record(table, r) for r in dump.rows(sql_table)]
        out[f"loot.{table}"] = ("loot", records, loot_key, False)
    items = [r for r in (item_record(t) for t in dump.rows("item_template")) if r]
    out["items.items"] = ("item", items, by_id, False)
    bosses = dungeon_bosses(templates, creature_rows, dump.rows("instance_encounters"),
                            dump.rows("instance_dungeon_encounters"))
    teleports = dump.rows("areatrigger_teleport")
    out["dungeons.dungeons"] = ("dungeon", dungeon_records(dump.rows("instance_template"), maps,
                                                           trigger_entrances(teleports, client_triggers), bosses,
                                                           teleports), lambda r: r["map_id"], True)
    out["world.graveyards"] = ("graveyard", graveyard_records(dump.rows("world_safe_locs"),
                                                              dump.rows("game_graveyard_zone"), locator), by_id, True)
    out["world.area_levels"] = ("area", area_level_records(spawns, templates, area_names, reactions), by_id, True)
    return out


def main(argv=None):
    args = parse_args(sys.argv[1:] if argv is None else argv)
    if args.refresh:
        http.configure(refresh_cache=True)
    if args.offline:
        http.configure(offline_only=True)
    out = pathlib.Path(args.out)
    started = time.time()
    try:
        dump, dump_name, cmangos = sqldump.load_classic_db()
        wago = db2.Wago(args.wago_product)
        datasets = build(dump, wago)
        client = wago.source()  # after every client table is read
        origin = envelope.origin(envelope.VANILLA, "1.12.1", product="cmangos-classic-db")
        gen = envelope.generator(NAME, credit=envelope.NAVZEROTH_CREDIT + " " + envelope.CREDIT)
        written = []
        for dataset, (kind, records, key, uses_client) in datasets.items():
            paths = envelope.write(dataset, kind, records, origin=origin,
                                   sources=[cmangos, client] if uses_client else [cmangos], generator=gen,
                                   confidence=envelope.confidence_for(origin["edition"]), out=out, sort_key=key)
            written.extend(p.relative_to(out).as_posix() for p in paths)
            print(f"{dataset}: {len(records)} {kind} records in {len(paths)} shard(s)")
        marker = out / ".written" / f"{NAME}.json"
        marker.parent.mkdir(parents=True, exist_ok=True)
        marker.write_text(json.dumps({"files": written, "used": http.used()}, sort_keys=True, indent=1) + "\n",
                          encoding="utf-8")
        print(f"{NAME}: {dump_name}, wago {wago.product} {wago.build}, {time.time() - started:.1f}s")
        return 0
    except (http.FetchError, KeyError, ValueError, OSError) as e:
        print(f"{NAME}: failed: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
