"""Builds FishingData.lua: the fishing reference data of FishingGoals.lua.

  - zone fishing levels: per area (zone, subzone or dungeon) the base skill of
    vanilla's fishing check, with the area's name, parent zone and Classic
    UiMap ID,
  - where each goal / timed fish is fished (area names),
  - the goal list (Nat Pagle's fish, the Stranglethorn Fishing Extravaganza
    fish, notable catches) with their quests and rewards,
  - fish caught only at some server hours or seasons, and the Extravaganza
    schedule (hand lists below, each with its source).

Sources:
  - CMaNGOS Classic database (https://github.com/cmangos/classic-db,
    Full_DB/ClassicDB_1_12_1_*.sql.gz): tables skill_fishing_base_level,
    fishing_loot_template / reference_loot_template, conditions, game_event,
    item_template, quest_template. It is the community reconstruction of the
    1.12 server data (Blizzard never published the server tables). Its fishing
    rule (src/game/Entities/GameObject.cpp in cmangos/mangos-classic): a fish is
    hooked when skill >= base and a 1-100 roll <= skill - base + 5, so below the
    base every fish gets away and from base + 95 none does.
  - wago.tools DB2 exports of the Classic Era client
    (https://wago.tools/db2/<table>/csv?product=wow_classic_era): AreaTable
    (area names, parents), UiMapAssignment (UiMap ID -> AreaID).
  - The hand lists (TIMED, EXTRAVAGANZA, WHERE_NOTES) name their own source.

Everything downloaded is cached in tools/cache/ (git-ignored); --refresh
fetches it again.

    python tools/gen_fishing.py            # fetch (cache) and write FishingData.lua
    python tools/gen_fishing.py --refresh  # ignore the cache

See docs/DATA.md.

Navzeroth: adapted with the author's permission (../PERMISSION.md). Instead of
FishingData.lua it writes navzeroth-world-data v1 JSON:

  - fishing.areas and fishing.goals, origin vanilla_1_12 build 1.12.1 (the skill
    rule and the loot are CMaNGOS server data); area names come from the client
    AreaTable of the wago.tools product --wago-product (default wowf, WoW
    Forever; or wow_classic_era), pinned to one build (--wago-build, default the
    newest), listed as a second source;
  - fishing.timed and fishing.events, TALOD's hand lists, origin
    wow_classic_era with an unknown build, source talod-hand-rules.

TALOD's uiMapArea table is not written: gen_world_db2.py emits the client's
UiMapAssignment rows, of which it was a subset. The dump is read with
navzeroth_datagen.sqldump (TALOD's parser, improved), downloads go through
navzeroth_datagen.http (shared cache in data-generators/.cache).

    python3 tools/gen_fishing.py [--refresh | --offline] [--wago-product wowf] [--wago-build B] [--out DIR]
"""
import argparse
import collections
import json
import pathlib
import sys

# Navzeroth: the shared library lives in data-generators/ (python3 -I does not add the script's folder).
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from navzeroth_datagen import db2, envelope, http, sqldump  # noqa: E402

# Navzeroth: the CMaNGOS dump and the wago.tools tables come from navzeroth_datagen
# (sqldump.load_classic_db, db2.Wago), which replace TALOD's CMANGOS_* / WAGO URLs.
CMANGOS_BUILD = "1.12.1"

# Goal list: item ID -> group. Names, quests and rewards are read from the
# database (a wrong ID stops the build). Groups: "pagle" = Nat Pagle's
# "Nat Pagle, Angler Extreme" fish, "stv" = Stranglethorn Fishing
# Extravaganza, "rare" = notable catches (rare fish, chests, seasonal and
# time-of-day fish).
GOALS = [
    (16967, "pagle", "Feralas Ahi"), (16970, "pagle", "Misty Reed Mahi Mahi"),
    (16968, "pagle", "Sar'theris Striker"), (16969, "pagle", "Savage Coast Blue Sailfin"),
    (19807, "stv", "Speckled Tastyfish"), (19805, "stv", "Keefer's Angelfish"),
    (19806, "stv", "Dezian Queenfish"), (19803, "stv", "Brownell's Blue Striped Racer"),
    (13759, "rare", "Raw Nightfin Snapper"), (13760, "rare", "Raw Sunscale Salmon"),
    (13755, "rare", "Winter Squid"), (13756, "rare", "Raw Summer Bass"),
    (13889, "rare", "Raw Whitescale Salmon"), (13893, "rare", "Large Raw Mightfish"),
    (13888, "rare", "Darkclaw Lobster"), (13422, "rare", "Stonescale Eel"),
    (6522, "rare", "Deviate Fish"), (6307, "rare", "Message in a Bottle"),
    (13875, "rare", "Ironbound Locked Chest"), (13918, "rare", "Reinforced Locked Chest"),
]

# Where to look when the database has no fishing loot entry for it (the
# Extravaganza fish come from pools spawned during the event).
WHERE_NOTES = {
    19807: "Stranglethorn Vale coast during the Extravaganza (Sundays 14:00-16:00 server time)",
    19805: "Stranglethorn Vale coast during the Extravaganza",
    19806: "Stranglethorn Vale coast during the Extravaganza",
    19803: "Stranglethorn Vale coast during the Extravaganza",
}
WHERE_SOURCE = "https://warcraft.wiki.gg/wiki/Stranglethorn_Fishing_Extravaganza"

# Fish bound to server hours or seasons. Hours are [from, to) on the server's
# clock (wrapping past midnight); "best" is the window with the best rate.
# Seasons are [from, to] as month * 100 + day (wrapping past New Year).
# Only what a source states; sources disagree in places (noted).
TIMED = [
    {"id": 13759, "hours": [18, 6], "best": [0, 6], "label": "night only",
     "open": "night", "closed": "day",
     "note": "Also caught any time inside the Sunken Temple.",
     "source": "https://warcraft.wiki.gg/wiki/Raw_Nightfin_Snapper (\"only between 6 PM and 6 AM\", best 12-6 AM)"},
    {"id": 13760, "hours": [6, 24], "best": [12, 18], "label": "day and evening",
     "open": "yes", "closed": "night",
     "source": "https://blizzardwatch.com/2019/10/02/make-gold-fishing-wow-classic/ (none 12-6 AM, best 12-6 PM)"},
    {"id": 13755, "season": [923, 319], "best": [12, 18], "label": "winter (Sep 23 - Mar 19)",
     "open": "in season", "closed": "out of season",
     "source": "https://warcraft.wiki.gg/wiki/Winter_Squid (from Sep 23, best 12:00-18:00 server time); "
               "end date: CMaNGOS game_event 35 'Winter Season Fishing' (Sep 23 01:00 + 178 days)"},
    {"id": 13756, "season": [321, 923], "label": "summer (Mar 21 - Sep 23)",
     "open": "in season", "closed": "out of season",
     "source": "CMaNGOS game_event 36 'Summer Season Fishing' (Mar 21 01:00 + 187 days); "
               "https://warcraft.wiki.gg/wiki/Raw_Summer_Bass (\"during the summer\")"},
]

# Weekday 1 = Sunday (as the game's calendar and Lua's os.date count).
EXTRAVAGANZA = {"weekday": 1, "start": 14 * 60, "length": 120,
                "source": "https://warcraft.wiki.gg/wiki/Stranglethorn_Fishing_Extravaganza "
                          "(patch 1.7.0: Sundays 2:00 - 4:00 PM server time; 40 Speckled Tastyfish to Riggle Bassbait, Booty Bay)"}
# Navzeroth: the hand lists' sources, as envelope sources (cited, not fetched).
HAND_SOURCES = {
    "fishing.timed": [("talod-hand-rules", "https://warcraft.wiki.gg/wiki/Raw_Nightfin_Snapper"),
                      ("talod-hand-rules-blizzardwatch",
                       "https://blizzardwatch.com/2019/10/02/make-gold-fishing-wow-classic/"),
                      ("talod-hand-rules-winter-squid", "https://warcraft.wiki.gg/wiki/Winter_Squid"),
                      ("talod-hand-rules-summer-bass", "https://warcraft.wiki.gg/wiki/Raw_Summer_Bass")],
    "fishing.events": [("talod-hand-rules", WHERE_SOURCE)],
}
EVENT_NAME = "Stranglethorn Fishing Extravaganza"


def area_rows(base, areas):
    """Navzeroth: zone fishing levels as FishingArea records. base: {area: skill}
    from skill_fishing_base_level; areas: AreaTable rows by ID. An area the
    client table does not name is skipped (and printed)."""
    def name(a):
        r = areas.get(a)
        return r["AreaName_lang"] if r else None

    out = []
    for a, skill in sorted(base.items()):
        if not name(a):
            print("skip area", a, "(not in the client AreaTable)")
            continue
        parent = int(areas[a]["ParentAreaID"] or 0)
        out.append({"area_id": a, "name": name(a), "parent_name": name(parent) if parent else None,
                    "base_skill": skill})
    return out


def fished_where(dump):
    """Where each item is fished: {item: {area}}, walking the fishing loot
    (references included)."""
    refs = collections.defaultdict(list)
    for r in dump.rows("reference_loot_template"):
        refs[int(r["entry"])].append(r)
    where = collections.defaultdict(set)

    def walk(rows, a, depth=0):
        for r in rows:
            n = int(r["mincountOrRef"])
            if n < 0 and depth < 5:
                walk(refs[-n], a, depth + 1)
            elif n >= 0:
                where[int(r["item"])].add(a)
    by_area = collections.defaultdict(list)
    for r in dump.rows("fishing_loot_template"):
        by_area[int(r["entry"])].append(r)
    for a, rows in by_area.items():
        walk(rows, a)
    return where


def goal_rows(dump, areas):
    """Navzeroth: TALOD's goal list as FishingGoal records (where -> places,
    whereNote -> place_note). Exits when a goal ID names another item."""
    def area_name(a):
        r = areas.get(a)
        return r["AreaName_lang"] if r else None

    def parent_name(a):
        r = areas.get(a)
        p = int(r["ParentAreaID"] or 0) if r else 0
        return area_name(p) if p else None

    where = fished_where(dump)
    items = {int(r["entry"]): r for r in dump.rows("item_template")}
    quests = dump.rows("quest_template")
    goals = []
    for item_id, group, expected in GOALS:
        it = items.get(item_id)
        if not it or it["name"] != expected:
            sys.exit("goal item %d is %r in the database, expected %r" % (item_id, it and it["name"], expected))
        g = {"item_id": item_id, "group": group, "name": it["name"], "quality": int(it["Quality"])}
        places = []
        for a in sorted(where.get(item_id, ()), key=lambda a: (parent_name(a) or area_name(a) or "", area_name(a) or "")):
            if area_name(a):
                places.append([area_name(a), parent_name(a)] if parent_name(a) else [area_name(a)])
        if places:
            g["places"] = places
        if item_id in WHERE_NOTES:
            g["place_note"] = WHERE_NOTES[item_id]
        qs = []
        for q in quests:
            for k in range(1, 5):
                if int(q["ReqItemId%d" % k] or 0) == item_id:
                    rewards = [items[int(q[c])]["name"] for c in
                               ["RewChoiceItemId%d" % i for i in range(1, 7)] + ["RewItemId%d" % i for i in range(1, 5)]
                               if int(q.get(c) or 0) and int(q[c]) in items]
                    quest = {"id": int(q["entry"]), "title": q["Title"], "count": int(q["ReqItemCount%d" % k])}
                    if rewards:
                        quest["rewards"] = rewards
                    qs.append(quest)
        if qs:
            g["quests"] = sorted(qs, key=lambda x: (x["count"], x["id"]))
        goals.append(g)
    for t in TIMED:
        if t["id"] not in items:
            sys.exit("timed fish %d not in the database" % t["id"])
    return goals


def timed_rows():
    """Navzeroth: TIMED as TimedFish records (best -> best_hours)."""
    return [{"item_id": t["id"], "hours": t.get("hours"), "best_hours": t.get("best"), "season": t.get("season"),
             "label": t["label"], "open": t["open"], "closed": t["closed"], "note": t.get("note"),
             "source": t["source"]} for t in TIMED]


def event_rows():
    """Navzeroth: EXTRAVAGANZA as a FishingEvent record."""
    e = EXTRAVAGANZA
    return [{"name": EVENT_NAME, "weekday": e["weekday"], "start_minute": e["start"], "length_minutes": e["length"],
             "source": e["source"]}]


def record_written(out, generator, paths):
    """Navzeroth: <out>/.written/<generator>.json lists the shards written and the
    cache entries used, for run_all.py's run manifest."""
    out = pathlib.Path(out).resolve()
    folder = out / ".written"
    folder.mkdir(parents=True, exist_ok=True)
    doc = {"files": sorted(pathlib.Path(p).resolve().relative_to(out).as_posix() for p in paths),
           "sources": [m for _, m in sorted(http.used().items())]}
    (folder / f"{generator}.json").write_text(json.dumps(doc, sort_keys=True, indent=1) + "\n", encoding="utf-8")


def build(out, product, build_id):
    dump, dump_name, cmangos = sqldump.load_classic_db()
    wago = db2.Wago(product, build_id)
    areas = {int(r["ID"]): r for r in wago.table("AreaTable")}
    base = {int(r["entry"]): int(r["skill"]) for r in dump.rows("skill_fishing_base_level")}
    zone_rows = area_rows(base, areas)
    goals = goal_rows(dump, areas)

    gen = envelope.generator("gen_fishing")
    server = dict(origin=envelope.origin(envelope.VANILLA, CMANGOS_BUILD), sources=[cmangos, wago.source()],
                  generator=gen, out=out)
    paths = envelope.write("fishing.areas", "fishing_area", zone_rows, **server)
    paths += envelope.write("fishing.goals", "fishing_goal", goals, **server)
    hand = envelope.origin(envelope.CLASSIC_ERA)
    for dataset, kind, rows in (("fishing.timed", "timed_fish", timed_rows()),
                                ("fishing.events", "fishing_event", event_rows())):
        sources = [envelope.source(i, u) for i, u in HAND_SOURCES[dataset]]
        paths += envelope.write(dataset, kind, rows, origin=hand, sources=sources, generator=gen, out=out)
    print("wrote", out, "-", dump_name, len(zone_rows), "areas,", len(goals), "goals; wago", wago.product, wago.build)
    return paths


def main(argv=None):
    """Navzeroth: exits non-zero when a download fails or the data does not match."""
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    p.add_argument("--refresh", action="store_true", help="ignore the cache and fetch everything again")
    p.add_argument("--offline", action="store_true", help="use only cached downloads")
    p.add_argument("--no-tooltips", action="store_true", help="accepted for a uniform command line; no effect")
    p.add_argument("--out", default=str(envelope.OUTPUT), help="output root (default data-generators/output)")
    p.add_argument("--wago-product", default="wowf", choices=sorted(db2.EDITIONS), help="client data for area names")
    p.add_argument("--wago-build", help="wago.tools build (default: the product's newest)")
    args = p.parse_args(argv)
    if args.refresh and args.offline:
        sys.exit("--refresh and --offline exclude each other")
    if args.refresh:
        http.configure(refresh_cache=True)
    if args.offline:
        http.configure(offline_only=True)
    paths = []
    try:
        paths = build(args.out, args.wago_product, args.wago_build)
    except (http.FetchError, KeyError, ValueError) as e:
        print(f"gen_fishing failed: {e}", file=sys.stderr)
        return 2
    finally:  # the downloads are listed even when the run fails
        record_written(args.out, "gen_fishing", paths)
    return 0


if __name__ == "__main__":
    sys.exit(main())
