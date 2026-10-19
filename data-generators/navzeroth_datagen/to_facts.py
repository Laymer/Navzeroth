"""Converts navzeroth-world-data shards into knowledge_v1 fact documents.

knowledge_v1 is the source pipeline's uniform input (crates/core/src/source_pipeline.rs,
docs/data/SOURCE_PIPELINE.md): {"schema_version": 1, "facts": [{domain, record,
field, datum, confidence, observed_at_utc_ms, clock_trust}]}, at most 8 MiB and
250,000 facts per document, one fact per (domain, record, field).

Only the nine release domains exist there, so professions ranks/notes, enhancements
and fishing stay world-data only (recipes do map to acquisition). Facts keep the
shard's confidence; the pipeline takes each candidate's origin edition/build from
the source registry, so documents are grouped by generator and origin:

    <output>/<generator>/<origin edition>/<origin build>/part-NNNN.json

Only fields a record actually has become facts (absent means unknown and is not
padded out); a non-known Observation (position, entrance, cost) becomes that
status. observed_at_utc_ms is null: download time is not an observation date.

    python3 data-generators/navzeroth_datagen/to_facts.py --input data-generators/output --output data-generators/output/facts
"""
import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from navzeroth_datagen import envelope  # noqa: E402

MAX_DOC_BYTES = 8 * 1024 * 1024 - 64 * 1024
MAX_FACTS = 250_000
MAX_VALUE_BYTES = 65536


def _key(*parts):
    return ":".join(str(p) for p in parts)


def _ref(r):
    return r.get("item") if r.get("item") is not None else f"ref{r.get('reference')}"


# record kind -> (domain, record key, fields). None fields = every field but the key ones.
MAPPING = {
    "quest": ("quests", lambda r: _key("quest", r["id"]), None),
    "item": ("equipment", lambda r: _key("item", r["id"]), None),
    "vendor_item": ("acquisition", lambda r: _key("vendor", r["npc_id"], "item", r["item_id"]), None),
    "recipe": ("acquisition", lambda r: _key("craft", r["spell_id"]), None),
    "loot": ("acquisition", lambda r: _key("loot", r["table"], r["entry"], _ref(r), "group", r.get("group", 0)), None),
    "area": ("geography", lambda r: _key("area", r["id"]), None),
    "ui_map": ("geography", lambda r: _key("ui_map", r["id"]), None),
    "ui_map_assignment": ("geography", lambda r: _key("ui_map_assignment", r["ui_map_id"], r["map_id"], r["order_index"]), None),
    "npc": ("geography", lambda r: _key("npc", r["id"]), None),
    "game_object": ("geography", lambda r: _key("game_object", r["id"]), None),
    "spawn": ("geography", lambda r: _key("spawn", r["entity"], r["guid"]), None),
    "taxi_node": ("travel", lambda r: _key("taxi_node", r["id"]), None),
    "taxi_path": ("travel", lambda r: _key("taxi_path", r["id"]), None),
    "graveyard": ("travel", lambda r: _key("graveyard", r["id"]), None),
    "dungeon": ("dungeons", lambda r: _key("dungeon", r["map_id"]), None),
    "spell": ("class_roles", lambda r: _key("spell", r["id"]), None),
    "talent": ("class_roles", lambda r: _key("talent", r["id"]), None),
    "trainer_spell": ("class_roles", lambda r: _key("trainer", r["npc_id"], "spell", r["spell_id"]), None),
}
KEY_FIELDS = {"kind", "id"}
OBSERVATIONS = {"position", "entrance", "cost", "build"}


def datum(field, value):
    if field in OBSERVATIONS and isinstance(value, dict) and "status" in value:
        return {"status": "known", "value": value["value"]} if value["status"] == "known" else {"status": value["status"]}
    return {"status": "known", "value": value}


def facts_for(record, confidence):
    """Facts of one world-data record, or [] for kinds outside the nine domains."""
    kind = record["kind"]
    if kind not in MAPPING:
        return []
    domain, key, fields = MAPPING[kind]
    rec = key(record)
    out = []
    for field in sorted(fields or record):
        if field in KEY_FIELDS or field not in record:
            continue
        d = datum(field, record[field])
        if d["status"] == "known" and len(envelope.dumps(d["value"]).encode()) > MAX_VALUE_BYTES:
            d = {"status": "unsupported"}  # too large for one fact (e.g. a long flight path)
        out.append({"domain": domain, "record": rec, "field": field, "datum": d, "confidence": confidence,
                    "observed_at_utc_ms": None, "clock_trust": "uncertain"})
    return out


def shards(root):
    for path in sorted(pathlib.Path(root).rglob("part-*.json")):
        rel = path.relative_to(root).parts
        if rel[0] in ("facts",) or any(p.startswith(".") for p in rel):
            continue
        yield path


def convert(input_root, output_root):
    """Write fact documents; return {"documents": [...], "facts": n, "duplicates": n, "conflicts": [...]}."""
    groups = {}
    for path in shards(input_root):
        doc = json.loads(path.read_text(encoding="utf-8"))
        o = doc["origin"]
        group = (doc["generator"]["name"], o["edition"], o["build"].get("value") or "unknown-build")
        facts = groups.setdefault(group, {})
        for record in doc["records"]:
            for f in facts_for(record, doc["confidence"]):
                k = (f["domain"], f["record"], f["field"])
                if k in facts:
                    if facts[k] != f:
                        facts[k]["conflict_with"] = True
                    continue
                facts[k] = f
    output_root = pathlib.Path(output_root)
    summary = {"documents": [], "facts": 0, "conflicts": []}
    for (gen, edition, build), facts in sorted(groups.items()):
        folder = output_root / gen / edition / build
        folder.mkdir(parents=True, exist_ok=True)
        for old in folder.glob("part-*.json"):
            old.unlink()
        lines = []
        for k in sorted(facts):
            f = facts[k]
            if f.pop("conflict_with", False):
                summary["conflicts"].append(_key(gen, *k))
            lines.append(envelope.dumps(f))
        docs, cur, size = [], [], 0
        for line in lines:
            n = len(line.encode()) + 2
            if cur and (size + n > MAX_DOC_BYTES or len(cur) >= MAX_FACTS):
                docs.append(cur)
                cur, size = [], 0
            cur.append(line)
            size += n
        if cur:
            docs.append(cur)
        for i, group_lines in enumerate(docs):
            path = folder / f"part-{i:04d}.json"
            path.write_text('{"facts":[\n' + ",\n".join(group_lines) + '\n],"schema_version":1}\n',
                            encoding="utf-8", newline="\n")
            summary["documents"].append(str(path.relative_to(output_root)).replace("\\", "/"))
        summary["facts"] += len(lines)
    return summary


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--input", default=str(envelope.OUTPUT))
    p.add_argument("--output", default=str(envelope.OUTPUT / "facts"))
    args = p.parse_args(argv)
    summary = convert(args.input, args.output)
    print(f"{summary['facts']} facts in {len(summary['documents'])} documents, "
          f"{len(summary['conflicts'])} conflicting duplicates (first value kept)")
    for c in summary["conflicts"][:20]:
        print("  conflict", c)
    return 0


if __name__ == "__main__":
    sys.exit(main())
