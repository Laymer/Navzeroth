"""Runs the Navzeroth data generators in parallel and writes a run manifest.

    python3 data-generators/run_all.py --jobs 6            # fetch what is not cached, then build
    python3 data-generators/run_all.py --offline           # rebuild from the cache only
    python3 data-generators/run_all.py --only gen_fishing,gen_professions --refresh

Stages: a prefetch fetches the shared inputs once (the CMaNGOS classic-db dump,
the wago.tools build list), so the generators start on a warm cache; then each
generator runs as `python3 -I <script> --out <out> [--refresh|--offline]` in its
own process, at most --jobs at a time (navzeroth_datagen.http's cross-process
host limits keep the sources' load polite). With --facts, to_facts.py converts
the output afterwards. Each generator's log is .cache/logs/<name>.log.

<out>/run-manifest.json lists every generator's status, its shards (path,
sha256, bytes, records) and the downloads they used (url, status, sha256,
bytes). It holds no timestamps or durations: an --offline re-run over the same
cache writes the same bytes. A generator reports its shards and downloads in
<out>/.written/<name>.json (removed after the run); for one that does not, its
shards are found by the generator name in their headers and its sources stay
unlisted. Exits non-zero when a generator failed.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from navzeroth_datagen import db2, envelope, http, sqldump  # noqa: E402

MANIFEST_FORMAT = "navzeroth-world-data-run"
MANIFEST_VERSION = 1
TALOD = HERE / "TALOD-data-generators" / "tools"
NAVZEROTH = HERE / "Navzeroth-data-generators"
# name -> (script, shared inputs it reads: "cmangos", "wago")
GENERATORS = {
    "gen_enhancements": (TALOD / "gen_enhancements.py", ()),
    "gen_fishing": (TALOD / "gen_fishing.py", ("cmangos", "wago")),
    "gen_professions": (TALOD / "gen_professions.py", ()),
    "gen_world_cmangos": (NAVZEROTH / "gen_world_cmangos.py", ("cmangos", "wago")),
    "gen_world_db2": (NAVZEROTH / "gen_world_db2.py", ("wago",)),
}
TO_FACTS = HERE / "navzeroth_datagen" / "to_facts.py"


def prefetch(needs, log=print):
    """Fetch the shared inputs once; a failure is logged and left to the generators."""
    if "cmangos" in needs:
        try:
            _, name, _ = sqldump.load_classic_db()
            log(f"prefetch: CMaNGOS {name}")
        except Exception as e:  # noqa: BLE001 - reported, the generator fails on its own
            log(f"prefetch: CMaNGOS dump failed: {e}")
    if "wago" in needs:
        try:
            log(f"prefetch: wago.tools wowf {db2.Wago('wowf').build}")
        except Exception as e:  # noqa: BLE001
            log(f"prefetch: wago.tools builds failed: {e}")


def child_env(mode):
    env = dict(os.environ)
    env["NAVZEROTH_DATAGEN_CACHE"] = str(http.cache_root())
    env["NAVZEROTH_DATAGEN_OFFLINE"] = "1" if mode == "offline" else "0"
    env["NAVZEROTH_DATAGEN_REFRESH"] = "1" if mode == "refresh" else "0"
    return env


def run_one(name, script, out, mode, logs):
    """Run one generator; returns (name, status, seconds)."""
    if not script.exists():
        return name, "skipped", 0.0
    written = out / ".written" / f"{name}.json"
    if written.exists():
        written.unlink()
    cmd = [sys.executable, "-I", str(script), "--out", str(out)]
    if mode in ("refresh", "offline"):
        cmd.append(f"--{mode}")
    logs.mkdir(parents=True, exist_ok=True)
    start = time.monotonic()
    with open(logs / f"{name}.log", "w", encoding="utf-8") as log:
        log.write("$ " + " ".join(cmd) + "\n")
        log.flush()
        code = subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, env=child_env(mode)).returncode
    return name, "ok" if code == 0 else "failed", time.monotonic() - start


def file_entry(out, rel):
    data = (out / rel).read_bytes()
    try:
        records = len(json.loads(data).get("records", []))
    except ValueError:
        records = 0
    return {"path": rel, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data), "records": records}


def shards_by_generator(out):
    """{generator name: [relative shard paths]} from the shard headers."""
    found = {}
    for path in sorted(out.rglob("part-*.json")):
        rel = path.relative_to(out).as_posix()
        if rel.startswith(("facts/", ".written/")):
            continue
        try:
            name = json.loads(path.read_bytes()).get("generator", {}).get("name")
        except ValueError:
            continue
        found.setdefault(name, []).append(rel)
    return found


def source_entry(meta):
    entry = {"url": meta["url"], "status": meta["status"]}
    for key in ("sha256", "bytes"):
        if meta.get(key) is not None:
            entry[key] = meta[key]
    return entry


def load_previous(out):
    """The manifest already in out (generators and sources), or None."""
    path = pathlib.Path(out) / "run-manifest.json"
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
        return {g["name"]: g for g in doc["generators"]}, {x["url"]: x for x in doc["sources"]}
    except (OSError, ValueError, KeyError, TypeError):
        return None


def build_manifest(out, statuses, ran=None, previous=None):
    """statuses: {name: status}. Reads and then removes <out>/.written.
    A partial run (ran = the generators started) keeps the entries and sources of the
    others from the previous manifest, so --only never orphans their shards."""
    out = pathlib.Path(out)
    written_dir = out / ".written"
    scanned = None
    generators, sources = [], dict(previous[1]) if previous and ran is not None else {}
    for name in sorted(statuses):
        files = []
        if ran is not None and name not in ran and previous and name in previous[0]:
            generators.append(previous[0][name])
            continue
        report = written_dir / f"{name}.json"
        if statuses[name] != "skipped" and report.exists():
            doc = json.loads(report.read_text(encoding="utf-8"))
            files = doc.get("files", [])
            # A failed run's downloads too (403s, refusals). Generators report them either as a
            # "sources" list or as "used", the {url: meta} dict of http.used().
            metas = list(doc.get("sources", [])) + list((doc.get("used") or {}).values())
            for meta in metas:
                sources[meta["url"]] = source_entry(meta)
        elif statuses[name] == "ok":
            scanned = scanned if scanned is not None else shards_by_generator(out)
            files = scanned.get(name, [])
        if statuses[name] != "ok":
            files = []  # shards left by a failed run are not this run's output
        generators.append({"name": name, "status": statuses[name],
                           "files": [file_entry(out, rel) for rel in sorted(set(files))]})
    if written_dir.exists():
        for f in written_dir.iterdir():
            f.unlink()
        written_dir.rmdir()
    return {"format": MANIFEST_FORMAT, "schema_version": MANIFEST_VERSION, "generators": generators,
            "sources": [sources[u] for u in sorted(sources)]}


def write_manifest(out, manifest):
    path = pathlib.Path(out) / "run-manifest.json"
    path.write_text(json.dumps(manifest, sort_keys=True, indent=1) + "\n", encoding="utf-8")
    return path


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    p.add_argument("--jobs", type=int, default=4, help="generators run at once (default 4)")
    p.add_argument("--only", help="comma-separated generator names")
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--refresh", action="store_true", help="ignore the cache and fetch everything again")
    mode.add_argument("--offline", action="store_true", help="use only cached downloads")
    p.add_argument("--out", default=str(envelope.OUTPUT), help="output root (default data-generators/output)")
    p.add_argument("--facts", action="store_true", help="convert the output with to_facts.py afterwards")
    args = p.parse_args(argv)

    names = sorted(GENERATORS)
    if args.only:
        names = [n.strip() for n in args.only.split(",") if n.strip()]
        unknown = [n for n in names if n not in GENERATORS]
        if unknown:
            p.error(f"unknown generator(s): {', '.join(unknown)}; known: {', '.join(sorted(GENERATORS))}")
    mode_name = "refresh" if args.refresh else "offline" if args.offline else "cache"
    http.configure(refresh_cache=args.refresh, offline_only=args.offline)
    out = pathlib.Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    logs = http.cache_root() / "logs"

    previous = load_previous(out)
    selected = [n for n in names if GENERATORS[n][0].exists()]
    # With --refresh each generator re-downloads what it reads, the shared inputs
    # included (http.py keeps no per-run memory of what was refreshed), so the
    # prefetch would only add one more download.
    if mode_name != "refresh":
        prefetch({need for n in selected for need in GENERATORS[n][1]})
    statuses = {n: "skipped" for n in GENERATORS}
    seconds = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        futures = [pool.submit(run_one, n, GENERATORS[n][0], out, mode_name, logs) for n in names]
        for f in concurrent.futures.as_completed(futures):
            name, status, secs = f.result()
            statuses[name], seconds[name] = status, secs
            print(f"{name}: {status} ({secs:.1f} s)", flush=True)

    facts_failed = False
    if args.facts:
        if TO_FACTS.exists():
            code = subprocess.run([sys.executable, "-I", str(TO_FACTS), "--input", str(out),
                                   "--output", str(out / "facts")], env=child_env(mode_name)).returncode
            print(f"to_facts: {'ok' if code == 0 else 'failed'}")
            facts_failed = code != 0
        else:
            print("to_facts: skipped (navzeroth_datagen/to_facts.py not found)")

    manifest = build_manifest(out, statuses, ran=set(names), previous=previous)
    path = write_manifest(out, manifest)
    print(f"\n{'generator':<20} {'status':<8} {'seconds':>8} {'files':>6} {'records':>9}")
    for g in manifest["generators"]:
        secs = f"{seconds[g['name']]:.1f}" if g["name"] in seconds else "-"
        print(f"{g['name']:<20} {g['status']:<8} {secs:>8} {len(g['files']):>6} "
              f"{sum(f['records'] for f in g['files']):>9}")
    print(f"manifest: {path} ({len(manifest['sources'])} sources); logs: {logs}")
    failed = [g["name"] for g in manifest["generators"] if g["status"] == "failed"]
    if failed:
        print(f"failed: {', '.join(failed)} (see the logs)", file=sys.stderr)
    return 1 if failed or facts_failed else 0


if __name__ == "__main__":
    sys.exit(main())
