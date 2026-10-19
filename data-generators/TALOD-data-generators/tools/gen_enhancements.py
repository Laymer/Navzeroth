"""Builds EnhanceData.lua: every permanent or temporary gear improvement the
professions offer (enchants, armor kits, shield spikes, counterweights,
spurs, scopes, sharpening stones, weightstones, wizard / mana oils), with
the slot, effect, required profession skill, tools, reagents, and how each
reagent is made (recursively: an Iron Bar is smelted at Mining 125, ...).

Source: Wowhead Classic (https://www.wowhead.com/classic). The profession
list pages embed every recipe (skill, reagents, created item, where it is
learned); per-spell / per-item tooltips give the slot, tools and effect text.
Tooltips are cached in tools/cache/ (git-ignored), so a re-run only fetches
what is new.

    python tools/gen_enhancements.py            # fetch (cache) and write EnhanceData.lua
    python tools/gen_enhancements.py --refresh  # ignore the cache, fetch everything again

Run it again whenever the data may have changed (new content patch, a
reported wrong reagent). See docs/DATA.md.
"""
import html
import json
import pathlib
import re
import sys
import time
import urllib.request

ADDON = pathlib.Path(__file__).resolve().parent.parent
CACHE = ADDON / "tools" / "cache"
OUT = ADDON / "EnhanceData.lua"
BASE = "https://www.wowhead.com/classic/spells/professions/"
TOOLTIP = "https://nether.wowhead.com/classic/tooltip/{kind}/{id}"
PROFESSIONS = {"enchanting": "Enchanting", "leatherworking": "Leatherworking", "blacksmithing": "Blacksmithing",
               "engineering": "Engineering", "mining": "Mining", "tailoring": "Tailoring"}
MAX_CLASSIC_SPELL = 40000      # higher IDs on Wowhead Classic are Season of Discovery additions
SOURCES = {1: "crafted", 2: "drop", 3: "pvp", 4: "quest", 5: "vendor", 6: "trainer", 7: "discovery", 10: "starter"}

# Crafted items that are used on gear, by name.
ITEM_PATTERNS = [
    (r"Armor Kit$", "kit"), (r"Shield Spike$", "spike"), (r"Counterweight$", "counterweight"), (r"Spurs$", "spurs"),
    (r"Scope$", "scope"), (r"Sharpening Stone$", "stone"), (r"Weightstone$", "stone"), (r"Wizard Oil$", "oil"),
    (r"Mana Oil$", "oil"),
]
TEMPORARY = {"stone", "oil"}

# Reagents bought from vendors (not crafted, gathered or dropped). Checked by hand.
VENDOR_ITEMS = {
    2320: "Coarse Thread", 2321: "Fine Thread", 4291: "Silken Thread", 8343: "Heavy Silken Thread", 14341: "Rune Thread",
    2880: "Weak Flux", 3466: "Strong Flux", 3371: "Empty Vial", 3372: "Leaded Vial", 8925: "Crystal Vial",
    18256: "Imbued Vial", 4470: "Simple Wood", 6217: "Copper Rod", 4289: "Salt", 2324: "Bleach", 2604: "Red Dye",
    2605: "Green Dye", 6260: "Blue Dye", 4340: "Gray Dye", 4341: "Yellow Dye", 4342: "Purple Dye", 10290: "Pink Dye",
    3857: "Coal", 17020: "Arcane Powder", 4399: "Wooden Stock", 4400: "Heavy Stock", 2678: "Mild Spices",
    # Tools and reagents Wowhead's lists name but price nowhere / other vendor goods (checked 2026-10-04).
    4471: "Flint and Tinder", 5956: "Blacksmith Hammer", 2325: "Black Dye", 6261: "Orange Dye",
    2692: "Hot Spices", 3713: "Soothing Spices", 2665: "Stormwind Seasoning Herbs", 159: "Refreshing Spring Water",
    1179: "Ice Cold Milk", 2596: "Skin of Dwarven Stout", 2894: "Rhapsody Malt", 4536: "Shiny Red Apple",
    9260: "Volatile Rum", 787: "Slitherskin Mackerel", 4592: "Longjaw Mud Snapper", 4593: "Bristle Whisker Catfish",
    4594: "Rockscale Cod", 6530: "Nightcrawlers", 10647: "Engineer's Ink", 10648: "Blank Parchment",
    11291: "Star Wood", 17034: "Maple Seed", 17035: "Stranglethorn Seed", 17194: "Holiday Spices",
    17196: "Holiday Spirits",
    18567: "Elemental Flux",   # Thorium Brotherhood (reputation) vendor
}

# How raw materials are gathered, by name.
GATHER_PATTERNS = [
    (r"(Dust|Essence|Shard)$", "Disenchanting"), (r"Ore$", "Mining"), (r"(Stone|Rough Stone|Coarse Stone|Heavy Stone|Solid Stone|Dense Stone)$", "Mining"),
    (r"^(Light|Medium|Heavy|Thick|Rugged) (Leather|Hide)$", "Skinning"), (r"Scale$", "Skinning or drop"), (r"^Raw ", "Fishing"),
    (r"(Linen|Wool|Silk|Mageweave) Cloth$|^Runecloth$|^Felcloth$", "Cloth (drops from humanoids)"),
    (r"Pearl$", "Clams (fishing, drops)"),
    (r"^(Arthas' Tears|Peacebloom|Silverleaf|Earthroot|Mageroyal|Briarthorn|Bruiseweed|Wild Steelbloom|Kingsblood|Liferoot|Fadeleaf|Goldthorn|Khadgar's Whisker|Wintersbite|Firebloom|Purple Lotus|Sungrass|Blindweed|Ghost Mushroom|Gromsblood|Golden Sansam|Dreamfoil|Mountain Silversage|Plaguebloom|Icecap|Black Lotus|Stranglekelp|Grave Moss|Swiftthistle)$", "Herbalism"),
    (r"(Emerald|Sapphire|Ruby|Diamond|Opal|Citrine|Jade|Malachite|Tigerseye|Shadowgem|Moss Agate|Lesser Moonstone|Aquamarine|Star Ruby|Blue Sapphire|Large Opal|Huge Emerald|Azerothian Diamond|Arcane Crystal|Elemental Earth)$", "Mining (gem) or drop"),
]

UNQUOTED = re.compile(r'([{,])([A-Za-z_][A-Za-z0-9_]*):')



def makes_of(creates):
    """Average yield of Wowhead's creates = [itemID, min, max]. Wowhead lists
    0 / 0 for transmutes and oils, which make one: never below 1."""
    lo = creates[1] if len(creates) > 1 and creates[1] else 0
    hi = creates[2] if len(creates) > 2 and creates[2] else lo
    avg = (lo + hi) / 2
    if avg < 1:
        return 1
    return int(avg) if avg == int(avg) else round(avg, 1)

def fetch(url, cache_name, refresh):
    path = CACHE / cache_name
    if path.exists() and not refresh:
        return path.read_text(encoding="utf-8")
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (TALOD data generator)"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                text = r.read().decode("utf-8", errors="replace")
            break
        except Exception as e:  # network hiccup: retry
            if attempt == 2:
                raise
            time.sleep(2)
    CACHE.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    time.sleep(0.25)
    return text


def parse_page(text):
    m = re.search(r"var listviewspells = (\[.*?\]);\n", text, re.S)
    spells = json.loads(UNQUOTED.sub(r'\1"\2":', m.group(1)))
    items = {}
    for im in re.finditer(r"WH\.Gatherer\.addData\(3, \d+, (\{.*?\})\);", text, re.S):
        for k, v in json.loads(im.group(1)).items():
            items[int(k)] = v
    descs = {}
    for sm in re.finditer(r"WH\.Gatherer\.addData\(6, \d+, (\{.*?\})\);", text, re.S):
        for k, v in json.loads(sm.group(1)).items():
            descs[int(k)] = v.get("description_enus") or ""
    return spells, items, descs


def strip(fragment):
    text = re.sub(r"<br\s*/?>", "\n", fragment)
    text = re.sub(r"<[^>]+>", "", text)
    return html.unescape(text).replace("\xa0", " ").strip()


def spell_tooltip(spell_id, refresh):
    data = json.loads(fetch(TOOLTIP.format(kind="spell", id=spell_id), f"spell_{spell_id}.json", refresh))
    tip = data.get("tooltip", "")
    req = re.search(r'wowhead-tooltip-requirements">Requires ([^<]+)<', tip)
    tools = []
    tm = re.search(r"Tools:(.*?)(Reagents:|<div class=\"q\">|$)", tip, re.S)
    if tm:
        tools = [int(x) for x in re.findall(r"item=(\d+)", tm.group(1))]
    desc = re.search(r'<div class="q">(.*?)</div>', tip, re.S)
    return (req.group(1).strip() if req else None), tools, (strip(desc.group(1)) if desc else "")


def item_tooltip(item_id, refresh):
    data = json.loads(fetch(TOOLTIP.format(kind="item", id=item_id), f"item_{item_id}.json", refresh))
    tip = data.get("tooltip", "")
    use = re.search(r"Use: (.*?)</a>", tip, re.S) or re.search(r"Use: (.*?)</span>", tip, re.S)
    level = re.search(r"Requires Level <!--rlvl-->(\d+)", tip)
    return (strip(use.group(1)) if use else ""), (int(level.group(1)) if level else None)


# Slot IDs: 1 head .. 19 tabard (see Gear.lua). "kind" narrows weapon slots.
def slots_for_enchant(req):
    r = (req or "").lower()
    if "bracer" in r: return [9], None
    if "chest" in r: return [5], None
    if "cloak" in r or "back" in r: return [15], None
    if "boot" in r or "feet" in r: return [8], None
    if "glove" in r or "hand" in r: return [10], None
    if "shield" in r: return [17], "shield"
    if "two-hand" in r or "2h" in r: return [16], "twohand"
    if "weapon" in r: return [16, 17], "weapon"
    if "head" in r: return [1], None
    if "leg" in r: return [7], None
    if "shoulder" in r: return [3], None
    return None, None


def slots_for_item(kind, use):
    u = use.lower()
    if kind == "kit": return [5, 7, 10, 8], None
    if kind == "spike": return [17], "shield"
    if kind == "counterweight": return [16], "twohand"
    if kind == "spurs": return [8], None
    if kind == "scope": return [18], "bowgun"
    if kind == "stone":
        return [16, 17], ("blunt" if "blunt" in u else "bladed" if ("blade" in u or "sharp" in u) else "weapon")
    if kind == "oil": return [16, 17], "weapon"
    return None, None


def lua(v, indent=""):
    if isinstance(v, bool): return "true" if v else "false"
    if v is None: return "nil"
    if isinstance(v, (int, float)): return repr(v)
    if isinstance(v, str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v, list): return "{ " + ", ".join(lua(x) for x in v) + " }"
    if isinstance(v, dict):
        parts = []
        for k, x in v.items():
            key = f"[{k}]" if isinstance(k, int) else (k if re.match(r"^[A-Za-z_]\w*$", k) else f"[{json.dumps(k)}]")
            parts.append(f"{key} = {lua(x)}")
        return "{ " + ", ".join(parts) + " }"
    raise TypeError(type(v))


def main():
    refresh = "--refresh" in sys.argv
    recipes, items, descs, creates = {}, {}, {}, {}
    for slug, prof in PROFESSIONS.items():
        print("profession", prof)
        spells, page_items, page_descs = parse_page(fetch(BASE + slug, f"page_{slug}.html", refresh))
        items.update(page_items)
        descs.update(page_descs)
        for sp in spells:
            sid = sp["id"]
            if sid >= MAX_CLASSIC_SPELL or not sp.get("reagents") and not sp.get("creates"):
                continue
            r = {"name": sp["name"], "prof": prof, "skill": sp.get("learnedat") if sp.get("learnedat") != 9999 else None,
                 "reagents": [[a, b] for a, b in sp.get("reagents", [])],
                 "src": [SOURCES.get(x, str(x)) for x in sp.get("source", [])]}
            if sp.get("trainingcost"): r["cost"] = sp["trainingcost"]
            if sp.get("colors"): r["grey"] = sp["colors"][3]
            if sp.get("creates"):
                r["creates"], r["makes"] = sp["creates"][0], makes_of(sp["creates"])
                creates.setdefault(sp["creates"][0], []).append(sid)
            recipes[sid] = r

    enhancements = []
    for sid, r in sorted(recipes.items()):
        if r["prof"] == "Enchanting" and r["name"].startswith("Enchant ") and not r.get("creates"):
            req, tools, desc = spell_tooltip(sid, refresh)
            # The name says the slot ("Enchant Bracer - ..."); the tooltip's first
            # requirement can be a level or "Armor".
            slots, kind = slots_for_enchant(r["name"])
            if not slots:
                slots, kind = slots_for_enchant(req)
            if not slots:
                print("  skip (slot?)", r["name"], req)
                continue
            r["tools"] = tools
            enhancements.append({"type": "enchant", "spell": sid, "slots": slots, "kind": kind,
                                 "effect": desc or descs.get(sid, ""), "name": r["name"].replace("Enchant ", "", 1)})
        elif r.get("creates"):
            name = items.get(r["creates"], {}).get("name_enus", r["name"])
            for pattern, kind in ITEM_PATTERNS:
                if re.search(pattern, name):
                    use, level = item_tooltip(r["creates"], refresh)
                    slots, wkind = slots_for_item(kind, use)
                    e = {"type": "item", "item": r["creates"], "spell": sid, "slots": slots, "kind": wkind,
                         "effect": use, "name": name, "temporary": kind in TEMPORARY}
                    if level: e["level"] = level
                    ml = re.search(r"items? level (\d+) and above", use)
                    if ml: e["minItemLevel"] = int(ml.group(1))
                    enhancements.append(e)
                    req, tools, _ = spell_tooltip(sid, refresh)
                    if tools: r["tools"] = tools
                    break

    # Every recipe an enhancement depends on, through its reagents.
    keep_recipes, keep_items = set(), set()
    def need_item(item_id, depth=0):
        keep_items.add(item_id)
        if depth > 6: return
        for sid in creates.get(item_id, []):
            if sid in keep_recipes: continue
            keep_recipes.add(sid)
            for rid, _ in recipes[sid]["reagents"]:
                need_item(rid, depth + 1)
            for tid in recipes[sid].get("tools", []):
                need_item(tid, depth + 1)
    for e in enhancements:
        keep_recipes.add(e["spell"])
        if e.get("item"): keep_items.add(e["item"])
        r = recipes[e["spell"]]
        for rid, _ in r["reagents"]:
            need_item(rid)
        for tid in r.get("tools", []):
            need_item(tid)

    out_items = {}
    for iid in sorted(keep_items):
        it = items.get(iid, {})
        name = it.get("name_enus") or VENDOR_ITEMS.get(iid) or f"Item {iid}"
        entry = {"name": name, "q": it.get("quality", 1)}
        if iid in VENDOR_ITEMS:
            entry["get"] = "Vendor"
        elif iid not in creates:
            for pattern, how in GATHER_PATTERNS:
                if re.search(pattern, name):
                    entry["get"] = how
                    break
        if iid in creates:
            entry["made"] = [s for s in creates[iid] if s in keep_recipes]
        out_items[iid] = entry
    out_recipes = {sid: recipes[sid] for sid in sorted(keep_recipes)}

    lines = [
        "-- Generated by tools/gen_enhancements.py from Wowhead Classic. Do not edit by hand:",
        "-- re-run the generator (see docs/DATA.md).",
        f"-- Generated {time.strftime('%Y-%m-%d')}: {len(enhancements)} enhancements, {len(out_recipes)} recipes, {len(out_items)} items.",
        "local ADDON_NAME, ns = ...",
        "ns.EnhanceData = {",
        f"    generated = {lua(time.strftime('%Y-%m-%d'))},",
        "    source = \"https://www.wowhead.com/classic\",",
        "    enhancements = {",
    ]
    for e in enhancements:
        lines.append("        " + lua(e) + ",")
    lines.append("    },")
    lines.append("    recipes = {")
    for sid, r in out_recipes.items():
        lines.append(f"        [{sid}] = " + lua(r) + ",")
    lines.append("    },")
    lines.append("    items = {")
    for iid, it in out_items.items():
        lines.append(f"        [{iid}] = " + lua(it) + ",")
    lines.append("    },")
    lines.append("}")
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{OUT}: {len(enhancements)} enhancements, {len(out_recipes)} recipes, {len(out_items)} items")


if __name__ == "__main__":
    main()
