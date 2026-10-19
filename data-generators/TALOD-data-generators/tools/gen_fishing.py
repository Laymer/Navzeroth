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
"""
import collections
import csv
import datetime
import gzip
import io
import json
import pathlib
import re
import sys
import urllib.request

ADDON = pathlib.Path(__file__).resolve().parent.parent
OUT = ADDON / "FishingData.lua"
CACHE = ADDON / "tools" / "cache"
CMANGOS_LIST = "https://api.github.com/repos/cmangos/classic-db/contents/Full_DB"
CMANGOS_RAW = "https://github.com/cmangos/classic-db/raw/master/Full_DB/"
WAGO = "https://wago.tools/db2/{}/csv?product=wow_classic_era"

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


def fetch(url, name, refresh, binary=False):
    path = CACHE / name
    if path.exists() and not refresh:
        return path.read_bytes() if binary else path.read_text(encoding="utf-8")
    print("fetch", url)
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (TALOD data generator)"})
    with urllib.request.urlopen(req, timeout=120) as r:
        data = r.read()
    CACHE.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return data if binary else data.decode("utf-8")


def parse_values(body):
    """Rows of an INSERT ... VALUES body, as lists of strings."""
    rows, i, n = [], 0, len(body)
    while i < n:
        if body[i] != "(":
            i += 1
            continue
        row, cur, i = [], "", i + 1
        while i < n:
            c = body[i]
            if c == "'":
                i += 1
                s = []
                while body[i] != "'":
                    if body[i] == "\\":
                        i += 1
                    s.append(body[i])
                    i += 1
                cur = "".join(s)
            elif c == ",":
                row.append(cur.strip())
                cur = ""
            elif c == ")":
                row.append(cur.strip())
                rows.append(row)
                i += 1
                break
            else:
                cur += c
            i += 1
    return rows


def table(sql, name):
    m = re.search(r"CREATE TABLE `%s` \((.*?)\n\)" % name, sql, re.S)
    cols = re.findall(r"^\s*`(\w+)`", m.group(1), re.M)
    out = []
    for body in re.findall(r"INSERT INTO `%s` VALUES (.*?);\n" % name, sql, re.S):
        out.extend(dict(zip(cols, r)) for r in parse_values(body))
    return out


def lua_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def lua(v):
    if v is None:
        return "nil"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, str):
        return lua_str(v)
    if isinstance(v, list):
        return "{ " + ", ".join(lua(x) for x in v) + " }"
    if isinstance(v, dict):
        parts = []
        for k, x in v.items():
            key = ("[%d]" % k) if isinstance(k, int) else (k if re.match(r"^[A-Za-z_]\w*$", k) else "[" + lua_str(k) + "]")
            parts.append("%s = %s" % (key, lua(x)))
        return "{ " + ", ".join(parts) + " }"
    raise TypeError(v)


def main():
    refresh = "--refresh" in sys.argv
    listing = json.loads(fetch(CMANGOS_LIST, "cmangos_full_db.json", refresh))
    dump = next(f["name"] for f in listing if f["name"].endswith(".sql.gz"))
    sql = gzip.decompress(fetch(CMANGOS_RAW + dump, dump, refresh, binary=True)).decode("utf-8", "replace")
    areas = {int(r["ID"]): r for r in csv.DictReader(io.StringIO(fetch(WAGO.format("AreaTable"), "wago_era_AreaTable.csv", refresh)))}
    assign = list(csv.DictReader(io.StringIO(fetch(WAGO.format("UiMapAssignment"), "wago_era_UiMapAssignment.csv", refresh))))

    def area_name(a):
        r = areas.get(a)
        return r["AreaName_lang"] if r else None

    def parent_name(a):
        r = areas.get(a)
        p = int(r["ParentAreaID"]) if r else 0
        return area_name(p) if p else None

    # Zone fishing levels.
    base = {int(r["entry"]): int(r["skill"]) for r in table(sql, "skill_fishing_base_level")}
    zone_rows = {}
    for a, skill in sorted(base.items()):
        name = area_name(a)
        if not name:
            print("skip area", a, "(not in the Classic Era AreaTable)")
            continue
        zone_rows[a] = [name, parent_name(a), skill]
    ui_maps = {}
    for r in assign:
        a, m = int(r["AreaID"]), int(r["UiMapID"])
        if a and a in zone_rows and int(r["OrderIndex"]) == 0:
            ui_maps[m] = a

    # Where each goal item is fished: walk the fishing loot (references included).
    refs = collections.defaultdict(list)
    for r in table(sql, "reference_loot_template"):
        refs[r["entry"]].append(r)
    where = collections.defaultdict(set)

    def walk(rows, a, depth=0):
        for r in rows:
            n = int(r["mincountOrRef"])
            if n < 0 and depth < 5:
                walk(refs[str(-n)], a, depth + 1)
            elif n >= 0:
                where[int(r["item"])].add(a)
    by_area = collections.defaultdict(list)
    for r in table(sql, "fishing_loot_template"):
        by_area[int(r["entry"])].append(r)
    for a, rows in by_area.items():
        walk(rows, a)

    items = {int(r["entry"]): r for r in table(sql, "item_template")}
    quests = table(sql, "quest_template")
    goals = []
    for item_id, group, expected in GOALS:
        it = items.get(item_id)
        if not it or it["name"] != expected:
            sys.exit("goal item %d is %r in the database, expected %r" % (item_id, it and it["name"], expected))
        g = {"id": item_id, "group": group, "name": it["name"], "quality": int(it["Quality"])}
        places = []
        for a in sorted(where.get(item_id, ()), key=lambda a: (parent_name(a) or area_name(a) or "", area_name(a) or "")):
            if area_name(a):
                places.append([area_name(a), parent_name(a)] if parent_name(a) else [area_name(a)])
        if places:
            g["where"] = places
        if item_id in WHERE_NOTES:
            g["whereNote"] = WHERE_NOTES[item_id]
        qs = []
        for q in quests:
            for k in range(1, 5):
                if int(q["ReqItemId%d" % k]) == item_id:
                    rewards = [items[int(q[c])]["name"] for c in
                               ["RewChoiceItemId%d" % i for i in range(1, 7)] + ["RewItemId%d" % i for i in range(1, 5)]
                               if int(q[c]) and int(q[c]) in items]
                    quest = {"id": int(q["entry"]), "title": q["Title"], "count": int(q["ReqItemCount%d" % k])}
                    if rewards:
                        quest["rewards"] = rewards
                    qs.append(quest)
        if qs:
            g["quests"] = sorted(qs, key=lambda x: x["count"])
        goals.append(g)

    for t in TIMED:
        if t["id"] not in items:
            sys.exit("timed fish %d not in the database" % t["id"])

    today = datetime.date.today().isoformat()
    out = [
        "-- Generated by tools/gen_fishing.py. Do not edit by hand: change the generator's",
        "-- lists or re-run it (see docs/DATA.md).",
        "-- Generated %s from:" % today,
        "--   CMaNGOS Classic DB %s (https://github.com/cmangos/classic-db):" % dump,
        "--     skill_fishing_base_level, fishing / reference loot, items, quests;",
        "--   wago.tools Classic Era DB2 (https://wago.tools/db2/AreaTable, /UiMapAssignment).",
        "-- Zone level rule (cmangos/mangos-classic GameObject.cpp): hooked when skill >= base",
        "-- and a 1-100 roll <= skill - base + 5; no fish gets away from base + 95.",
        "local ADDON_NAME, ns = ...",
        "-- Other fishing modules may add their own tables to ns.FishingData: keep the table.",
        "ns.FishingData = ns.FishingData or {}",
        "local D = ns.FishingData",
        "D.generated = %s" % lua(today),
        "D.zoneSource = %s" % lua("CMaNGOS Classic DB " + dump + " (skill_fishing_base_level)"),
        "-- [areaID] = { name, parent zone (nil for a zone), base skill }",
        "D.areas = {",
    ]
    for a, row in sorted(zone_rows.items()):
        out.append("    [%d] = %s," % (a, lua(row)))
    out.append("}")
    out.append("-- Classic UiMap ID -> areaID (outdoor zones and cities).")
    out.append("D.uiMapArea = { " + ", ".join("[%d] = %d" % (m, a) for m, a in sorted(ui_maps.items())) + " }")
    out.append("-- Goals: id, group (pagle / stv / rare), name, quality, where = { { area, zone } },")
    out.append("-- whereNote, quests = { { id, title, count, rewards } }.")
    out.append("D.goals = {")
    for g in goals:
        out.append("    " + lua(g) + ",")
    out.append("}")
    out.append("D.whereSource = %s" % lua(WHERE_SOURCE))
    out.append("-- Server hours [from, to) / seasons [from, to] as month * 100 + day; sources inline.")
    out.append("D.timed = {")
    for t in TIMED:
        out.append("    " + lua(t) + ",")
    out.append("}")
    out.append("D.extravaganza = " + lua(EXTRAVAGANZA))
    OUT.write_text("\n".join(out) + "\n", encoding="utf-8")
    print("wrote", OUT, "-", len(zone_rows), "areas,", len(ui_maps), "UiMaps,", len(goals), "goals")


if __name__ == "__main__":
    main()
