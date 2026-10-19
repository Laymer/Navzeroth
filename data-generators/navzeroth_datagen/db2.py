"""Client DB2 tables as CSV from wago.tools, pinned to one product build.

WoW Forever is product `wowf` on wago.tools; Classic Era is `wow_classic_era`.
The build is resolved once from https://wago.tools/api/builds (or given) and
passed on every request, so all tables of a run come from the same build.

    w = Wago("wowf")                        # pins the newest wowf build
    rows = w.table("TaxiNodes")             # list of dicts (strings)
    w.origin()                              # envelope origin {edition, build, product}
"""
import csv
import io

from . import envelope, http

BUILDS = "https://wago.tools/api/builds"
CSV = "https://wago.tools/db2/{table}/csv?product={product}&build={build}"
EDITIONS = {"wowf": envelope.FOREVER, "wow_classic_era": envelope.CLASSIC_ERA}


class Wago:
    def __init__(self, product="wowf", build=None):
        if product not in EDITIONS:
            raise ValueError(f"unknown wago product {product}")
        self.product = product
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
            if e.status in (400, 403, 404):  # wago answers 400 for a table the build lacks
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
