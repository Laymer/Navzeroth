"""Writes navzeroth-world-data v1 shards (schemas/world-data/v1/world-data.schema.json).

One dataset is written to output/<dataset as path>/<origin edition>/<origin build>/
as part-NNNN.json files of at most MAX_SHARD_BYTES, one record per line, keys
sorted, so a re-run from the same cache is byte-identical. Old parts in that
folder are removed first. Records are plain dicts with a "kind" key; fields set
to None are dropped (absent means unknown), while False, 0 and "" are kept.
"""
import json
import pathlib

FORMAT = "navzeroth-world-data"
SCHEMA_VERSION = 1
OUTPUT = pathlib.Path(__file__).resolve().parent.parent / "output"
MAX_SHARD_BYTES = 8 * 1024 * 1024
TARGET_EDITION = "wow_forever"
CREDIT = "Data generators based on work by NotInTheBand (TALOD), used with permission."
NAVZEROTH_CREDIT = "Navzeroth data generators (https://github.com/Laymer/Navzeroth)."
# Origin editions. Never relabel one as another (AGENTS.md).
FOREVER, CLASSIC_ERA, VANILLA = "wow_forever", "wow_classic_era", "vanilla_1_12"


def known(value):
    return {"status": "known", "value": value}


def unknown():
    return {"status": "unknown"}


def unsupported():
    return {"status": "unsupported"}


def not_applicable():
    return {"status": "not_applicable"}


def build_observation(build):
    return known(build) if build else unknown()


def origin(edition, build=None, product=None, locale="enUS"):
    o = {"edition": edition, "build": build_observation(build), "locale": locale}
    if product:
        o["product"] = product
    return o


def target(build=None):
    """The 1.0.0 target build stays unknown until it is frozen after launch."""
    return {"edition": TARGET_EDITION, "build": build_observation(build)}


def confidence_for(origin_edition):
    return "publisher_documented" if origin_edition == TARGET_EDITION else "cross_edition_unverified"


def source(id, url, revision=None, retrieved_at_utc_ms=None, snapshot_sha256=None):
    return clean({"id": id, "url": url, "revision": revision, "retrieved_at_utc_ms": retrieved_at_utc_ms,
                  "snapshot_sha256": snapshot_sha256})


def generator(name, version="1", credit=CREDIT):
    return {"name": name, "version": version, "credit": credit}


def clean(value):
    """Drop None recursively (absent = unknown); keep False / 0 / "" / []."""
    if isinstance(value, dict):
        return {k: clean(v) for k, v in value.items() if v is not None}
    if isinstance(value, list):
        return [clean(v) for v in value]
    return value


def dumps(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False)


def dataset_dir(dataset, origin_value, out=None):
    build = origin_value["build"].get("value") or "unknown-build"
    return pathlib.Path(out or OUTPUT).joinpath(*dataset.split(".")) / origin_value["edition"] / build


def write(dataset, record_kind, records, *, origin, sources, generator, confidence=None, target_value=None,
          out=None, sort_key=None, max_bytes=MAX_SHARD_BYTES):
    """Write the shards and return their paths. Every record gets kind = record_kind."""
    if not records:
        raise ValueError(f"{dataset}: no records")
    lines = []
    for r in sorted(records, key=sort_key) if sort_key else records:
        r = clean(dict(r))
        r["kind"] = record_kind
        lines.append(dumps(r))
    header = {"format": FORMAT, "schema_version": SCHEMA_VERSION, "dataset": dataset, "record_kind": record_kind,
              "origin": origin, "target": target_value or target(), "confidence": confidence or confidence_for(origin["edition"]),
              "sources": sources, "generator": generator}
    overhead = len(dumps(header).encode()) + 256
    groups, current, size = [], [], overhead
    for line in lines:
        n = len(line.encode()) + 2
        if current and size + n > max_bytes:
            groups.append(current)
            current, size = [], overhead
        current.append(line)
        size += n
    groups.append(current)
    folder = dataset_dir(dataset, origin, out)
    folder.mkdir(parents=True, exist_ok=True)
    for old in folder.glob("part-*.json"):
        old.unlink()
    paths = []
    for i, group in enumerate(groups):
        head = dict(header, shard={"index": i, "count": len(groups)})
        text = dumps(head)[:-1] + ',"records":[\n' + ",\n".join(group) + "\n]}\n"
        path = folder / f"part-{i:04d}.json"
        path.write_text(text, encoding="utf-8", newline="\n")
        paths.append(path)
    return paths
