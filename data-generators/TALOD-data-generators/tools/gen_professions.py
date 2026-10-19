"""Builds ProfessionData.lua: every recipe of every crafting profession and
secondary skill (Alchemy, Blacksmithing, Cooking, Enchanting, Engineering,
First Aid, Leatherworking, Tailoring, and Mining's smelting) with what the
leveling planner (Plan.lua) needs:

  - difficulty thresholds (orange / yellow / green / grey skill),
  - skill-up points per craft, reagents, tools, specialization (hand rule),
  - where the recipe is learned (trainer and its cost, or a pattern),
  - every reagent's name, quality, icon, vendor price and Wowhead's auction
    average, and how it is made (recursively) or gathered,
  - the training ranks (Apprentice .. Artisan) with cost and required level.

Source: Wowhead Classic (https://www.wowhead.com/classic). The profession
list pages embed every recipe and item; the recipe-item lists say which
recipes a pattern teaches (and its vendor price); per-spell tooltips add tools. Everything is cached in tools/cache/ (git-ignored), so a
plain re-run only fetches what is new; --refresh fetches everything again.

    python tools/gen_professions.py            # fetch (cache) and write ProfessionData.lua
    python tools/gen_professions.py --refresh  # ignore the cache: pull current data from Wowhead
    python tools/gen_professions.py --no-tooltips   # skip the per-spell tooltips (tools)

Run it again whenever the data may have changed (new content patch, a
reported wrong reagent or skill color). See docs/DATA.md.
"""
import json
import pathlib
import re
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from gen_enhancements import (fetch, parse_page, lua, SOURCES, VENDOR_ITEMS, GATHER_PATTERNS,  # noqa: E402
                              MAX_CLASSIC_SPELL, TOOLTIP)

ADDON = pathlib.Path(__file__).resolve().parent.parent
OUT = ADDON / "ProfessionData.lua"
BASE = "https://www.wowhead.com/classic/spells/"

# Wowhead list path -> (the name the game's Skills tab uses, skill line ID).
# Each page is filtered by the skill ID: a wrong path falls back to a list
# of every profession, which must not relabel other recipes.
PROFESSIONS = {
    "professions/alchemy": ("Alchemy", 171), "professions/blacksmithing": ("Blacksmithing", 164),
    "secondary-skills/cooking": ("Cooking", 185), "professions/enchanting": ("Enchanting", 333),
    "professions/engineering": ("Engineering", 202), "secondary-skills/first-aid": ("First Aid", 129),
    "professions/leatherworking": ("Leatherworking", 165), "professions/tailoring": ("Tailoring", 197),
    "professions/mining": ("Mining", 186),
}
# Leveled by gathering, not by recipes: the planner explains instead of planning.
GATHERING = {"Mining": "mining ore veins (smelting also gives skill)", "Herbalism": "picking herbs",
             "Skinning": "skinning beasts", "Fishing": "fishing"}

# Training ranks. Wowhead gives the rank spells' skill to learn at and cost;
# the character level each needs and the secondary skills' special steps are
# vanilla rules, checked by hand (Classic Era; [VERIFY] on Forever).
RANK_ORDER = ["Apprentice", "Journeyman", "Expert", "Artisan"]
RANK_MAX = {"Apprentice": 75, "Journeyman": 150, "Expert": 225, "Artisan": 300}
RANK_LEVEL = {"Apprentice": 5, "Journeyman": 10, "Expert": 20, "Artisan": 35}
RANK_NOTES = {
    ("First Aid", "Expert"): "a book: Expert First Aid - Under Wraps (vendor in Arathi Highlands / Dustwallow Marsh)",
    ("First Aid", "Artisan"): "the quest Triage (Theramore / Hammerfall, First Aid 225)",
    ("Cooking", "Expert"): "a book: Expert Cookbook (vendor in Booty Bay / Gadgetzan area)",
    ("Cooking", "Artisan"): "the quest Clamlette Surprise (Dirge Quikcleave, Tanaris, Cooking 225)",
}
# Secondary skills have no character-level requirement for their ranks.
SECONDARY = {"Cooking", "First Aid", "Fishing"}

# Specialization recipes taught by the specialization trainer. Wowhead has no
# field for it (the tooltips describe the created item), so: a hand rule.
# Blacksmithing / Leatherworking specialization recipes are plans and
# patterns from quests or drops, already marked as patterns.
SPEC_RULES = [(r"^Gnomish ", "Engineering", 200, "Gnomish Engineer"),
              (r"^Goblin ", "Engineering", 200, "Goblin Engineer"),
              (r"^The Big One$", "Engineering", 200, "Goblin Engineer"),
              (r"^Dimensional Ripper - Everlook$", "Engineering", 200, "Goblin Engineer"),
              (r"^Ultrasafe Transporter - Gadgetzan$", "Engineering", 200, "Gnomish Engineer"),
              (r"^World Enlarger$", "Engineering", 200, "Gnomish Engineer")]
# Recipes with a cooldown (days): never a way to level. Not on Wowhead's list.
# Transmute: Elemental Fire has none in vanilla.
COOLDOWN = [r"^Transmute: (?!Elemental Fire$)", r"^Mooncloth$"]
# Recipes Wowhead lists without an orange threshold, with the yellow one as
# "learned at": they are learned at skill 1 (trainer / starting recipes).
FROM_START = {2657: "Smelt Copper", 7418: "Enchant Bracer - Minor Health", 7428: "Enchant Bracer - Minor Deflect",
              2538: "Charred Wolf Meat", 2540: "Roasted Boar Meat", 818: "Basic Campfire", 3920: "Crafted Light Shot"}
ITEMS_BASE = "https://www.wowhead.com/classic/items/recipes/"
ITEM_SOURCES = {2: "drop", 4: "quest", 5: "vendor", 16: "fishing"}
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

def recipe_items(slug, refresh):
    """The profession's recipe items (Pattern: / Plans: / Formula: ...):
    { spell name: {item, src, price} }. The name after the prefix is the
    recipe's name; a vendor price means a vendor sells it."""
    text = fetch(ITEMS_BASE + slug, f"recipeitems_{slug}.html", refresh)
    m = re.search(r"var listviewitems = (\[.*?\]);\n", text, re.S)
    if not m:
        print(f"  no recipe item list for {slug}")
        return {}
    listed = json.loads(UNQUOTED.sub(r'\1"\2":', m.group(1)))
    extra = {}
    for im in re.finditer(r"WH\.Gatherer\.addData\(3, \d+, (\{.*?\})\);", text, re.S):
        for k, v in json.loads(im.group(1)).items():
            extra[int(k)] = v
    out = {}
    for it in listed:
        name = it.get("name", "")
        if ": " not in name:
            continue
        spell_name = name.split(": ", 1)[1]
        entry = {"item": it["id"], "src": [ITEM_SOURCES[x] for x in it.get("source", []) if x in ITEM_SOURCES]}
        # A price only when Wowhead lists a vendor as a source: buyprice alone is
        # set on raid-drop patterns too (Flask of the Titans).
        price = ((extra.get(it["id"]) or {}).get("jsonequip") or {}).get("buyprice")
        if price and "vendor" in entry["src"]:
            entry["price"] = price
        out.setdefault(spell_name, entry)
    return out


def spell_extra(spell_id, refresh):
    """Tools from the spell tooltip."""
    data = json.loads(fetch(TOOLTIP.format(kind="spell", id=spell_id), f"spell_{spell_id}.json", refresh))
    tip = data.get("tooltip", "")
    tools = []
    tm = re.search(r"Tools:(.*?)(Reagents:|<div class=\"q\">|$)", tip, re.S)
    if tm:
        tools = [int(x) for x in re.findall(r"item=(\d+)", tm.group(1))]
    return tools


def main():
    refresh = "--refresh" in sys.argv
    tooltips = "--no-tooltips" not in sys.argv
    recipes, items, creates, ranks = {}, {}, {}, {}
    for slug, (prof, skill_id) in PROFESSIONS.items():
        spells, page_items, _ = parse_page(fetch(BASE + slug, f"page_{slug.split('/')[-1]}.html", refresh))
        items.update(page_items)
        n = 0
        for sp in spells:
            sid = sp["id"]
            if sid >= MAX_CLASSIC_SPELL or skill_id not in (sp.get("skill") or []):
                continue
            if sp.get("rank") in RANK_MAX and sp.get("name") == prof:
                rank = sp["rank"]
                entry = {"rank": rank, "max": RANK_MAX[rank], "at": 1 if sp.get("learnedat") == 9999 else sp.get("learnedat"),
                         "cost": sp.get("trainingcost") or 0}
                if prof not in SECONDARY:
                    entry["level"] = RANK_LEVEL[rank]
                if (prof, rank) in RANK_NOTES:
                    entry["note"] = RANK_NOTES[(prof, rank)]
                ranks.setdefault(prof, {})[rank] = entry
                continue
            colors = sp.get("colors")
            if not colors or not (sp.get("reagents") or sp.get("creates")):
                continue
            learned = sp.get("learnedat")
            skill = learned if learned and learned != 9999 else (colors[0] or colors[1] or 1)
            if sid in FROM_START:
                skill = 1
            r = {"name": sp["name"], "prof": prof, "skill": skill,
                 "colors": list(colors), "reagents": [[a, b] for a, b in sp.get("reagents", [])]}
            if (sp.get("nskillup") or 1) != 1:
                r["up"] = sp["nskillup"]
            src = [SOURCES.get(x, str(x)) for x in sp.get("source", [])]
            if sp.get("trainingcost") is not None and "trainer" not in src:
                src.insert(0, "trainer")
            r["src"] = src
            if sp.get("trainingcost"):
                r["cost"] = sp["trainingcost"]
            if sp.get("creates"):
                r["creates"], r["makes"] = sp["creates"][0], makes_of(sp["creates"])
                creates.setdefault(sp["creates"][0], []).append(sid)
            recipes[sid] = r
            n += 1
        # Taught by an item (pattern, plans, formula, ...) or, with no source
        # listed at all, by the trainer (Wowhead leaves e.g. Heavy Linen
        # Bandage's trainer out) or known from the start at skill 1.
        taught = recipe_items(slug.split("/")[-1], refresh)
        for sid, r in recipes.items():
            if r["prof"] != prof:
                continue
            item = taught.get(r["name"])
            if item and "trainer" not in r["src"]:
                r["src"] = ["pattern"] + [x for x in item["src"] if x not in ("pattern",)]
                r["pattern"] = item["item"]
                if item.get("price"):
                    r["patternPrice"] = item["price"]
            elif not r["src"]:
                r["src"] = ["starter"] if r["skill"] <= 1 else ["trainer"]
                if r["skill"] > 1:
                    r["guess"] = True
        # Ranks Wowhead does not list: fill from the vanilla table.
        for rank in RANK_ORDER:
            if rank not in ranks.get(prof, {}):
                entry = {"rank": rank, "max": RANK_MAX[rank], "at": {"Apprentice": 1, "Journeyman": 50, "Expert": 125, "Artisan": 200}[rank], "cost": None}
                if prof not in SECONDARY:
                    entry["level"] = RANK_LEVEL[rank]
                if (prof, rank) in RANK_NOTES:
                    entry["note"] = RANK_NOTES[(prof, rank)]
                ranks.setdefault(prof, {})[rank] = entry
        print(f"{prof}: {n} recipes")

    for r in recipes.values():
        for pattern, prof, at, spec in SPEC_RULES:
            if r["prof"] == prof and r["skill"] >= at and re.search(pattern, r["name"]) and "trainer" in r["src"]:
                r["spec"] = spec
        if any(re.search(pattern, r["name"]) for pattern in COOLDOWN):
            r["cooldown"] = True

    if tooltips:
        todo = sorted(recipes)
        print(f"tooltips for {len(todo)} recipes (cached ones are instant)")
        for i, sid in enumerate(todo):
            tools = spell_extra(sid, refresh)
            if tools:
                recipes[sid]["tools"] = tools
            if (i + 1) % 100 == 0:
                print(f"  {i + 1} / {len(todo)}")

    used = set()
    for r in recipes.values():
        used.update(rid for rid, _ in r["reagents"])
        used.update(r.get("tools", []))
        if r.get("creates"):
            used.add(r["creates"])
    out_items = {}
    for iid in sorted(used):
        it = items.get(iid, {})
        eq = it.get("jsonequip") or {}
        name = it.get("name_enus") or VENDOR_ITEMS.get(iid) or f"Item {iid}"
        entry = {"name": name, "q": it.get("quality", 1)}
        if it.get("icon"):
            entry["icon"] = it["icon"]
        # Wowhead's buyprice is the item's price field, not proof a vendor sells
        # it (herbs, leathers, raid materials have one): the hand list decides.
        if iid in VENDOR_ITEMS:
            entry["vendor"] = eq.get("buyprice") or 0
        if eq.get("avgbuyout"):
            entry["ah"] = eq["avgbuyout"]
        if eq.get("sellprice"):
            entry["sell"] = eq["sellprice"]
        if iid in creates:
            entry["made"] = creates[iid]
        elif "vendor" not in entry:
            for pattern, how in GATHER_PATTERNS:
                if re.search(pattern, name):
                    entry["get"] = how
                    break
        out_items[iid] = entry

    stamp = time.strftime("%Y-%m-%d")
    lines = [
        "-- Generated by tools/gen_professions.py from Wowhead Classic. Do not edit by hand:",
        "-- re-run the generator (see docs/DATA.md).",
        f"-- Generated {stamp}: {len(recipes)} recipes, {len(out_items)} items.",
        "local ADDON_NAME, ns = ...",
        "ns.ProfessionData = {",
        f"    generated = {lua(stamp)},",
        "    source = \"https://www.wowhead.com/classic\",",
        "    ranks = {",
    ]
    for prof, _ in PROFESSIONS.values():
        lines.append(f"        [{json.dumps(prof)}] = {{ " + ", ".join(lua(ranks[prof][r]) for r in RANK_ORDER) + " },")
    lines += ["    },", "    gathering = " + lua(GATHERING) + ",", "    recipes = {"]
    for sid, r in sorted(recipes.items()):
        lines.append(f"        [{sid}] = " + lua(r) + ",")
    lines += ["    },", "    items = {"]
    for iid, it in out_items.items():
        lines.append(f"        [{iid}] = " + lua(it) + ",")
    lines += ["    },", "}"]
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{OUT}: {len(recipes)} recipes, {len(out_items)} items")


if __name__ == "__main__":
    main()
