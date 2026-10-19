"""Client DB2 tables as CSV from wago.tools, pinned to one product build.

WoW Forever is product `wowf` on wago.tools; Classic Era is `wow_classic_era`.
The build is resolved once from https://wago.tools/api/builds (or given) and
passed on every request, so all tables of a run come from the same build.

    w = Wago("wowf")                        # pins the newest wowf build
    rows = w.table("TaxiNodes")             # list of dicts (strings)
    w.origin()                              # envelope origin {edition, build, product}
"""
import csv
import os
import re
import io

from . import envelope, http

BUILDS = "https://wago.tools/api/builds"
CSV = "https://wago.tools/db2/{table}/csv?product={product}&build={build}"
EDITIONS = {"wowf": envelope.FOREVER, "wow_classic_era": envelope.CLASSIC_ERA}

# ITEM_MOD_* stat types (UnitDefines / ItemTemplate); others become stat_<id>. The client
# DB2 items (gen_world_db2) and the CMaNGOS item_template (gen_world_cmangos) share them.
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


def stat_name(stat_type):
    """ITEM_MOD_* name of a stat type, or stat_<id> for one without a name."""
    return STAT_NAMES.get(stat_type, f"stat_{stat_type}")


class Wago:
    def __init__(self, product="wowf", build=None):
        if product not in EDITIONS:
            raise ValueError(f"unknown wago product {product}")
        self.product = product
        if product == 'wowf' and os.environ.get('NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD'):
            pinned=os.environ['NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD']
            if not re.fullmatch(r'[0-9]{1,9}(?:\.[0-9]{1,9}){3}',pinned) or (build is not None and build != pinned):
                raise ValueError('explicit Wago build differs from policy pin')
            build=pinned
        if not build:  # {product: [{version, created_at, ...}, ...]}: newest by creation time
            builds = http.fetch_json(BUILDS, "builds.json")[product]
            build = max(builds, key=lambda b: b["created_at"])["version"]
        self.build = build

    def url(self, table):
        return CSV.format(table=table, product=self.product, build=self.build)

    def table(self, table):
        """Rows of one table; raises http.FetchError (400 or 404) if the build lacks it."""
        text = http.fetch_text(self.url(table), f"{self.product}_{self.build}_{table}.csv", timeout=300)
        return list(csv.DictReader(io.StringIO(text)))

    def has(self, table):
        try:
            self.table(table)
            return True
        except http.FetchError as e:
            if e.status in (400, 404):  # wago answers 400 for a table the build lacks
                return False
            raise

    def origin(self, locale="enUS"):
        return envelope.origin(EDITIONS[self.product], self.build, product=self.product, locale=locale)

    def source(self):
        """Call after the tables are read: the digest covers the CSVs fetched so far."""
        prefix = "https://wago.tools/db2/"
        return envelope.source(f"wago-tools-{self.product}", f"https://wago.tools/db2?product={self.product}",
                               revision=self.build, snapshot_sha256=http.snapshot_digest(prefix))


def num(value, kind=int):
    """'' -> None; numeric text -> int/float (floats with .0 stay float when kind=float)."""
    if value is None or value == "":
        return None
    return kind(float(value)) if kind is int else kind(value)
