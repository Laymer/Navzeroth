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

Navzeroth: adapted with the author's permission (../PERMISSION.md). Instead of
ProfessionData.lua it writes navzeroth-world-data v1 JSON, the datasets
professions.ranks, professions.notes (TALOD's gathering notes),
professions.recipes and professions.items, with origin wow_classic_era and an
unknown build. Downloads go through navzeroth_datagen.http (shared cache in
data-generators/.cache); the tooltips use a small thread pool.

    python3 tools/gen_professions.py [--refresh | --offline] [--no-tooltips] [--out DIR]
"""
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
# Navzeroth: gen_enhancements also puts data-generators/ on sys.path for navzeroth_datagen.
from gen_enhancements import (fetch, parse_page, SOURCES, VENDOR_ITEMS, GATHER_PATTERNS,  # noqa: E402
                              MAX_CLASSIC_SPELL, TOOLTIP, cli, configure, in_parallel, item_record,
                              recipe_record, record_written, wowhead_sources)
from navzeroth_datagen import envelope, http  # noqa: E402

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

def recipe_items(slug):
    """The profession's recipe items (Pattern: / Plans: / Formula: ...):
    { spell name: {item, src, price} }. The name after the prefix is the
    recipe's name; a vendor price means a vendor sells it."""
    text = fetch(ITEMS_BASE + slug, f"recipeitems_{slug}.html")
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


def spell_extra(spell_id):
    """Tools from the spell tooltip."""
    data = json.loads(fetch(TOOLTIP.format(kind="spell", id=spell_id), f"spell_{spell_id}.json"))
    tip = data.get("tooltip", "")
    tools = []
    tm = re.search(r"Tools:(.*?)(Reagents:|<div class=\"q\">|$)", tip, re.S)
    if tm:
        tools = [int(x) for x in re.findall(r"item=(\d+)", tm.group(1))]
    return tools


def rank_entry(prof, rank, at, cost):
    """Navzeroth: TALOD's rank entry, built in one place; cost None = unknown."""
    entry = {"rank": rank, "max": RANK_MAX[rank], "at": at, "cost": cost}
    if prof not in SECONDARY:
        entry["level"] = RANK_LEVEL[rank]
    if (prof, rank) in RANK_NOTES:
        entry["note"] = RANK_NOTES[(prof, rank)]
    return entry


def rank_record(prof, entry):
    """Navzeroth: a TALOD rank entry as a world-data ProfessionRank record."""
    cost = envelope.known(entry["cost"]) if entry.get("cost") is not None else envelope.unknown()
    return {"profession": prof, "rank": entry["rank"], "max_skill": entry["max"], "learn_at_skill": entry["at"],
            "cost": cost, "character_level": entry.get("level"), "note": entry.get("note")}


def note_records():
    """Navzeroth: TALOD's GATHERING notes as ProfessionNote records."""
    return [{"profession": prof, "text": text} for prof, text in sorted(GATHERING.items())]


def build(out, tooltips):
    """Navzeroth: TALOD's main(), writing world-data shards; returns their paths."""
    recipes, items, creates, ranks = {}, {}, {}, {}
    for slug, (prof, skill_id) in PROFESSIONS.items():
        spells, page_items, _ = parse_page(fetch(BASE + slug, f"page_{slug.split('/')[-1]}.html"))
        items.update(page_items)
        n = 0
        for sp in spells:
            sid = sp["id"]
            if sid >= MAX_CLASSIC_SPELL or skill_id not in (sp.get("skill") or []):
                continue
            if sp.get("rank") in RANK_MAX and sp.get("name") == prof:
                rank = sp["rank"]
                # Navzeroth: a cost Wowhead does not list stays unknown (TALOD wrote 0).
                ranks.setdefault(prof, {})[rank] = rank_entry(
                    prof, rank, 1 if sp.get("learnedat") == 9999 else sp.get("learnedat"), sp.get("trainingcost"))
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
        taught = recipe_items(slug.split("/")[-1])
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
                at = {"Apprentice": 1, "Journeyman": 50, "Expert": 125, "Artisan": 200}[rank]
                ranks.setdefault(prof, {})[rank] = rank_entry(prof, rank, at, None)
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
        # Navzeroth: fetched on a thread pool; results come back in todo's order.
        for sid, tools in zip(todo, in_parallel(spell_extra, todo)):
            if tools:
                recipes[sid]["tools"] = tools

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
        # Navzeroth: quality stays unknown when Wowhead gives none (TALOD assumed 1).
        entry = {"name": name, "q": it.get("quality")}
        if it.get("icon"):
            entry["icon"] = it["icon"]
        # Wowhead's buyprice is the item's price field, not proof a vendor sells
        # it (herbs, leathers, raid materials have one): the hand list decides.
        # Navzeroth: a missing price stays unknown (TALOD wrote 0).
        if iid in VENDOR_ITEMS:
            entry["vendor"] = eq.get("buyprice")
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

    # Navzeroth: world-data shards instead of ProfessionData.lua.
    kw = dict(origin=envelope.origin(envelope.CLASSIC_ERA), sources=wowhead_sources(),
              generator=envelope.generator("gen_professions"), out=out)
    rank_records = [rank_record(prof, ranks[prof][r]) for prof, _ in PROFESSIONS.values() for r in RANK_ORDER]
    paths = envelope.write("professions.ranks", "profession_rank", rank_records, **kw)
    paths += envelope.write("professions.notes", "profession_note", note_records(), **kw)
    paths += envelope.write("professions.recipes", "recipe",
                            [recipe_record(sid, r) for sid, r in sorted(recipes.items())], **kw)
    paths += envelope.write("professions.items", "item", [item_record(iid, it) for iid, it in out_items.items()], **kw)
    print(f"{out}: {len(recipes)} recipes, {len(out_items)} items")
    return paths


def main(argv=None):
    """Navzeroth: exits non-zero when a download is refused or a page cannot be read."""
    args = cli(__doc__).parse_args(argv)
    configure(args)
    paths = []
    try:
        paths = build(args.out, not args.no_tooltips)
    except (http.FetchError, ValueError) as e:
        print(f"gen_professions failed: {e}", file=sys.stderr)
        return 2
    finally:  # the downloads are listed even when the run fails
        record_written(args.out, "gen_professions", paths)
    return 0


if __name__ == "__main__":
    sys.exit(main())
