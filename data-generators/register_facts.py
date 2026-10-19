"""Registers knowledge_v1 fact documents as retained source material (ADR 0007 §7).

Pipeline stage "register" (task V2.8c). Input is the folder written by
navzeroth_datagen/to_facts.py:

    <facts>/<generator>/<origin edition>/<origin build or unknown-build>/part-NNNN.json

For each (generator, edition, build) folder this script

  * copies every document to <objects>/<sha256> (an existing object must hold the
    same bytes; it is never overwritten);
  * upserts one admission-registry source `gen-<generator>-<edition>-<build>` whose
    files list each document's exact sha256, byte count and `domain.field` set, with
    rights copied from the upstream sources the generator used (GENERATOR_UPSTREAMS,
    plus the registry ids the world-data shards cite) and ADR 0007 §7 as the access
    evidence for local retention (`manual_local`, bounded to the exact closure);
  * upserts the matching `knowledge_v1` inputs of the source-build plan;

then removes generated sources, plan inputs and objects that no longer exist, sets
`registry_sha256` in the source-updates policy to the new registry hash and rewrites
the delimited generated sections of the material NOTICE and of .gitattributes (which
stores exactly the generated fact objects in Git LFS; other objects stay plain).

The output is deterministic: same inputs, same bytes. JSON files keep the repository
style, json.dumps(indent=2, ensure_ascii=False) plus a newline.

    python3 data-generators/register_facts.py --facts data-generators/output/facts \\
        --registry docs/sources/source-admission-v1.json --plan content/source-build-v1.json \\
        --objects content/source-materials/objects --updates docs/sources/source-updates-v1.json \\
        --notice content/source-materials/NOTICE.md
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
PREFIX = "gen-"
TRANSFORMATION = "knowledge_v1"
OBJECT_URL = "https://github.com/Laymer/Navzeroth/blob/main/content/source-materials/objects/{}"
ADR = ROOT / "docs" / "decisions" / "0007-world-data-automation.md"
ADR_URL = "https://github.com/Laymer/Navzeroth/blob/main/docs/decisions/0007-world-data-automation.md"
ADR_SCOPE = ("ADR 0007 §7: the research catalog registers every generated fact; the retained "
             "fact documents are Navzeroth generator output kept in Git LFS under "
             "content/source-materials/objects and registered by data-generators/register_facts.py "
             "(manual local acquisition, exact sha256 and byte bounds). source-sync never fetches "
             "them. Pinned bytes: ADR 0007 with LF line endings (revision is the Git blob id).")
NOTICE_BEGIN = "<!-- BEGIN generated fact sources (data-generators/register_facts.py) -->"
NOTICE_END = "<!-- END generated fact sources -->"
ATTR_BEGIN = "# BEGIN generated fact objects (data-generators/register_facts.py): Git LFS"
ATTR_END = "# END generated fact objects"
ATTR_LINE = "content/source-materials/objects/{} filter=lfs diff=lfs merge=lfs -text"

# Mirrors crates/core/src/source_admission.rs; refused here with a clear message
# before the Rust validator refuses the whole registry.
MAX_FILE_BYTES = 8 * 1024 * 1024
MAX_FILES = 1024
MAX_FIELDS = 256
MAX_REQUESTS = 1024
MAX_RETRIEVAL_BYTES = 2 * 1024 * 1024 * 1024
MAX_SOURCES = 128
MAX_EVIDENCE = 64
UNKNOWN_BUILD = "unknown-build"
SEGMENT = re.compile(r"[a-z0-9_-](?:[a-z0-9._-]*[a-z0-9_-])?")  # AssetPath segments

# Upstream registry sources whose rights a generator's output carries, by origin
# edition ("*" for any). World-data shards may cite more (e.g. CMaNGOS spawns cite
# wago.tools for uiMap positions); those are added when --world is given.
GENERATOR_UPSTREAMS = {
    "gen_world_cmangos": {"*": ["cmangos-classic-db"]},
    "gen_world_db2": {"wow_forever": ["wago-tools-wowf"], "wow_classic_era": ["wago-tools-classic-era"]},
    "gen_professions": {"wow_forever": ["talod-generators", "wowhead-forever"],
                        "*": ["talod-generators", "wowhead-classic"]},
    "gen_enhancements": {"wow_forever": ["talod-generators", "wowhead-forever"],
                         "*": ["talod-generators", "wowhead-classic"]},
    "gen_fishing": {"*": ["talod-generators"]},
}


class RegisterError(Exception):
    pass


def dumps(value):
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def load(path):
    return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))


def write_if_changed(path, text):
    path = pathlib.Path(path)
    data = text.encode("utf-8")
    if not path.exists() or path.read_bytes() != data:
        path.write_bytes(data)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def git_blob_id(data):
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def source_id(generator, edition, build):
    name = generator[4:] if generator.startswith("gen_") else generator
    raw = f"{PREFIX}{name}-{edition}-{build}".lower()
    return re.sub(r"-+", "-", re.sub(r"[^a-z0-9.]", "-", raw)).strip("-")


def upstreams_for(generator, edition):
    table = GENERATOR_UPSTREAMS.get(generator)
    if table is None:
        raise RegisterError(f"{generator}: no upstream source mapping (add it to GENERATOR_UPSTREAMS)")
    ids = table.get(edition, table.get("*"))
    if ids is None:
        raise RegisterError(f"{generator}: no upstream source mapping for edition {edition}")
    return list(ids)


def shard_citations(world):
    """{(generator, edition, build folder): [registry-looking source ids in first-seen order]}."""
    cited = {}
    if world is None or not pathlib.Path(world).is_dir():
        return cited
    for path in sorted(pathlib.Path(world).rglob("part-*.json")):
        rel = path.relative_to(world).parts
        if rel[0] == "facts" or any(p.startswith(".") for p in rel):
            continue
        doc = json.loads(path.read_text(encoding="utf-8"))
        build = (doc["origin"].get("build") or {}).get("value") or UNKNOWN_BUILD
        key = (doc["generator"]["name"], doc["origin"]["edition"], build)
        ids = cited.setdefault(key, [])
        for s in doc.get("sources", []):
            if s.get("id") not in ids:
                ids.append(s.get("id"))
    return cited


def scan(facts_root):
    """[(generator, edition, build, [(relative path, bytes, fields)])] sorted."""
    facts_root = pathlib.Path(facts_root)
    if not facts_root.is_dir():
        raise RegisterError(f"{facts_root}: facts folder not found")
    groups = {}
    for path in sorted(facts_root.rglob("*.json")):
        rel = path.relative_to(facts_root)
        parts = rel.parts
        if any(p.startswith(".") for p in parts):
            continue
        if len(parts) != 4 or not re.fullmatch(r"part-\d{4}\.json", parts[3]):
            raise RegisterError(f"{rel.as_posix()}: expected <generator>/<edition>/<build>/part-NNNN.json")
        if len(rel.as_posix()) > 256 or not all(SEGMENT.fullmatch(p) for p in parts):
            raise RegisterError(f"{rel.as_posix()}: registry paths are lowercase [a-z0-9._-] segments")
        if path.is_symlink():
            raise RegisterError(f"{rel.as_posix()}: symbolic links are refused")
        data = path.read_bytes()
        if not data or len(data) > MAX_FILE_BYTES:
            raise RegisterError(f"{rel.as_posix()}: {len(data)} bytes is outside 1..{MAX_FILE_BYTES}")
        doc = json.loads(data)
        if doc.get("schema_version") != 1 or not isinstance(doc.get("facts"), list) or not doc["facts"]:
            raise RegisterError(f"{rel.as_posix()}: not a non-empty knowledge_v1 document")
        fields = sorted({f"{f['domain']}.{f['field']}" for f in doc["facts"]})
        if len(fields) > MAX_FIELDS:
            raise RegisterError(f"{rel.as_posix()}: {len(fields)} fields exceed {MAX_FIELDS}")
        groups.setdefault(tuple(parts[:3]), []).append((rel.as_posix(), data, fields))
    return [(g, e, b, files) for (g, e, b), files in sorted(groups.items())]


def merge_rights(upstream):
    """One rights block carrying every upstream source's terms and evidence."""
    licenses = [s["rights"]["license"] for s in upstream]
    if len(set(licenses)) == 1:
        license = licenses[0]
    else:
        license = next(x for x in ("owner_accepted", "share_alike", "permissive") if x in licenses)

    def joined(field, sep):
        values = []
        for s in upstream:
            v = s["rights"].get(field)
            if v and v not in values:
                values.append(v)
        return sep.join(values) if values else None

    evidence, seen = [], set()
    for s in upstream:
        for e in s["rights"]["evidence"]:
            if (e["url"], e["sha256"]) not in seen:
                seen.add((e["url"], e["sha256"]))
                evidence.append(e)
    if len(evidence) > MAX_EVIDENCE:
        raise RegisterError(f"{len(evidence)} evidence entries exceed {MAX_EVIDENCE}")
    ids = ", ".join(s["id"] for s in upstream)
    return {
        "status": "admitted",
        "license": license,
        "license_identifier": joined("license_identifier", " AND "),
        "reviewer": f"Generated by data-generators/register_facts.py (V2.8c, ADR 0007 §7); rights copied from {ids}: "
                    + joined("reviewer", "; "),
        "attribution": joined("attribution", "; "),
        "obligations": joined("obligations", " "),
        "evidence": evidence,
    }


def adr_evidence(adr_path, objects):
    data = pathlib.Path(adr_path).read_bytes().replace(b"\r\n", b"\n")
    digest = sha256(data)
    store(objects, digest, data)
    return {"url": ADR_URL, "revision": git_blob_id(data), "sha256": digest, "reviewed_scope": ADR_SCOPE}, len(data)


def store(objects, digest, data):
    target = pathlib.Path(objects) / digest
    if target.is_symlink():
        raise RegisterError(f"{target}: symbolic links are refused")
    if target.exists():
        if target.read_bytes() != data:
            raise RegisterError(f"{target}: existing object differs from its name; refusing to overwrite")
        return
    temporary = target.with_name(f".pending-{os.getpid()}-{digest}")
    temporary.write_bytes(data)
    os.replace(temporary, target)


def build_source(generator, edition, build, files, registry_by_id, cited, access, objects, evidence_bytes):
    ids = upstreams_for(generator, edition)
    for extra in cited.get((generator, edition, build), []):
        if extra in registry_by_id and not extra.startswith(PREFIX) and extra not in ids:
            ids.append(extra)
    upstream = []
    for i in ids:
        s = registry_by_id.get(i)
        if s is None or s["rights"]["status"] != "admitted":
            raise RegisterError(f"{generator}: upstream source {i} is not an admitted registry source")
        upstream.append(s)
    rights = merge_rights(upstream)
    entries, total = [], 0
    for rel, data, fields in files:
        digest = sha256(data)
        store(objects, digest, data)
        entries.append({"path": rel, "url": OBJECT_URL.format(digest), "sha256": digest, "bytes": len(data),
                        "fields": fields, "transformation": TRANSFORMATION})
        total += len(data)
    if len(entries) > MAX_FILES:
        raise RegisterError(f"{generator}/{edition}/{build}: {len(entries)} files exceed {MAX_FILES}")
    # The closure counts each distinct object once: files, rights evidence, access evidence.
    closure = {e["sha256"]: e["bytes"] for e in entries}
    for e in rights["evidence"] + [access]:
        if e["sha256"] not in closure:
            closure[e["sha256"]] = evidence_bytes(e["sha256"])
    closure_bytes = sum(closure.values())
    if len(closure) > MAX_REQUESTS or closure_bytes > MAX_RETRIEVAL_BYTES:
        raise RegisterError(f"{generator}/{edition}/{build}: closure of {len(closure)} objects, "
                            f"{closure_bytes} bytes exceeds the registry bounds")
    revision = sha256("".join(f"{e['path']} {e['sha256']}\n" for e in entries).encode())
    return {
        "id": source_id(generator, edition, build),
        "revision": revision,
        "publisher": f"Navzeroth data generators ({generator}); https://github.com/Laymer/Navzeroth",
        "origin": {"edition": edition, "build": None if build == UNKNOWN_BUILD else build},
        "rights": rights,
        "retrieval": {"method": "manual_local", "max_total_bytes": closure_bytes, "minimum_interval_ms": 1000,
                      "max_requests": len(closure), "access_evidence": access},
        "files": entries,
    }


def closure_hashes(source):
    hashes = {f["sha256"] for f in source["files"]}
    hashes |= {e["sha256"] for e in source["rights"]["evidence"]}
    if source["retrieval"].get("access_evidence"):
        hashes.add(source["retrieval"]["access_evidence"]["sha256"])
    return hashes


def notice_section(sources):
    lines = [NOTICE_BEGIN, "", "## Generated fact sources (ADR 0007 §7)", "",
             "Written by `data-generators/register_facts.py`; do not edit by hand. Each source is",
             "the knowledge_v1 fact documents one generator wrote for one origin edition and build",
             "(`data-generators/navzeroth_datagen/to_facts.py`). The objects are stored through Git",
             "LFS. Their rights, attribution and obligations are those of the upstream sources",
             "named below, as recorded in the admission registry; the origin is never relabelled.", ""]
    access = {s["retrieval"]["access_evidence"]["sha256"] for s in sources}
    for digest in sorted(access):
        lines += [f"Access evidence for every generated source: `docs/decisions/0007-world-data-automation.md` "
                  f"(§7) → [`{digest}`](objects/{digest}).", ""]
    if not sources:
        lines += ["No generated fact documents are registered.", ""]
    for s in sources:
        r = s["rights"]
        facts_bytes = sum(f["bytes"] for f in s["files"])
        lines += [f"### {s['id']}", "",
                  f"Publisher: {s['publisher']}. Origin: `{s['origin']['edition']}`, build "
                  f"`{s['origin']['build'] or 'unknown'}`. Revision `{s['revision']}`.", "",
                  f"Licence: {r['license_identifier']}.", f"Attribution: {r['attribution']}.", "",
                  f"{len(s['files'])} documents, {facts_bytes} bytes:", ""]
        for f in s["files"]:
            lines.append(f"- `{f['path']}` → [`{f['sha256']}`](objects/{f['sha256']}) ({f['bytes']} bytes)")
        lines.append("")
    lines.append(NOTICE_END)
    return "\n".join(lines) + "\n"


def replace_section(text, begin, end, section):
    """Replace the begin..end block (inclusive, through its newline) or append it."""
    start = text.find(begin)
    if start < 0:
        return text.rstrip("\n") + "\n\n" + section
    stop = text.find(end, start)
    if stop < 0:
        raise RegisterError(f"unterminated generated section {begin!r}")
    stop += len(end)
    if text[stop:stop + 1] == "\n":
        stop += 1
    return text[:start] + section + text[stop:]


def attributes_section(hashes):
    return "\n".join([ATTR_BEGIN] + [ATTR_LINE.format(h) for h in sorted(hashes)] + [ATTR_END]) + "\n"


def register(facts, registry_path, plan_path, objects, updates_path, notice_path, world=None, adr=ADR,
             attributes=ROOT / ".gitattributes"):
    """Register every fact document; return a summary dict."""
    objects = pathlib.Path(objects)
    objects.mkdir(parents=True, exist_ok=True)
    registry = load(registry_path)
    plan = load(plan_path)
    updates = load(updates_path)
    groups = scan(facts)
    cited = shard_citations(world)
    kept = [s for s in registry["sources"] if not s["id"].startswith(PREFIX)]
    old_generated = [s for s in registry["sources"] if s["id"].startswith(PREFIX)]
    by_id = {s["id"]: s for s in kept}
    access, _ = adr_evidence(adr, objects)

    def evidence_bytes(digest):
        path = objects / digest
        if not path.is_file():
            raise RegisterError(f"evidence object {digest} is missing from {objects}")
        return path.stat().st_size

    generated = [build_source(g, e, b, files, by_id, cited, access, objects, evidence_bytes)
                 for g, e, b, files in groups]
    ids = [s["id"] for s in generated]
    if len(set(ids)) != len(ids) or set(ids) & set(by_id):
        raise RegisterError("generated source ids collide")
    generated.sort(key=lambda s: s["id"])
    if len(kept) + len(generated) > MAX_SOURCES:
        raise RegisterError(f"{len(kept) + len(generated)} sources exceed {MAX_SOURCES}")
    registry["sources"] = kept + generated
    registry_text = dumps(registry)

    inputs = [i for i in plan["inputs"] if not i["source"].startswith(PREFIX)]
    inputs += sorted(({"source": s["id"], "file": f["path"], "adapter": TRANSFORMATION}
                      for s in generated for f in s["files"]), key=lambda i: (i["source"], i["file"]))
    plan["inputs"] = inputs

    updates["registry_sha256"] = sha256(registry_text.encode("utf-8"))

    # Objects only the previous generated sources referenced are removed; anything a
    # current source (generated or not) needs stays.
    live = set().union(*(closure_hashes(s) for s in registry["sources"])) if registry["sources"] else set()
    stale = set().union(*(closure_hashes(s) for s in old_generated)) - live if old_generated else set()
    stale.discard(access["sha256"])
    for digest in sorted(stale):
        path = objects / digest
        if path.is_file() and not path.is_symlink():
            path.unlink()

    write_if_changed(registry_path, registry_text)
    write_if_changed(plan_path, dumps(plan))
    write_if_changed(updates_path, dumps(updates))
    notice = pathlib.Path(notice_path).read_text(encoding="utf-8")
    write_if_changed(notice_path, replace_section(notice, NOTICE_BEGIN, NOTICE_END, notice_section(generated)))
    lfs = {f["sha256"] for s in generated for f in s["files"]}
    if attributes is not None:
        attr = pathlib.Path(attributes).read_text(encoding="utf-8")
        write_if_changed(attributes, replace_section(attr, ATTR_BEGIN, ATTR_END, attributes_section(lfs)))
    return {"sources": ids, "documents": sum(len(s["files"]) for s in generated),
            "bytes": sum(f["bytes"] for s in generated for f in s["files"]),
            "removed_objects": sorted(stale), "registry_sha256": updates["registry_sha256"]}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--facts", required=True, help="to_facts.py output folder")
    p.add_argument("--registry", required=True, help="docs/sources/source-admission-v1.json")
    p.add_argument("--plan", required=True, help="content/source-build-v1.json")
    p.add_argument("--objects", required=True, help="content/source-materials/objects")
    p.add_argument("--updates", required=True, help="docs/sources/source-updates-v1.json")
    p.add_argument("--notice", required=True, help="content/source-materials/NOTICE.md")
    p.add_argument("--world", default=str(HERE / "output"),
                   help="world-data shards, for the source ids they cite (default: data-generators/output)")
    p.add_argument("--adr", default=str(ADR), help="ADR 0007, the access evidence (default: docs/decisions/...)")
    p.add_argument("--attributes", default=str(ROOT / ".gitattributes"),
                   help="the .gitattributes whose generated block lists the LFS objects")
    args = p.parse_args(argv)
    try:
        summary = register(args.facts, args.registry, args.plan, args.objects, args.updates, args.notice,
                           world=args.world, adr=args.adr, attributes=args.attributes)
    except (RegisterError, OSError, ValueError, KeyError) as e:
        print(f"register_facts: {e}", file=sys.stderr)
        return 1
    print(f"{len(summary['sources'])} generated sources, {summary['documents']} documents, "
          f"{summary['bytes']} bytes; {len(summary['removed_objects'])} stale objects removed; "
          f"registry {summary['registry_sha256']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
