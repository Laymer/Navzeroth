"""The world-data pipeline (ADR 0007): one command from download to draft PR.

    python3 data-generators/pipeline.py                 # every stage
    python3 data-generators/pipeline.py --offline       # from the cache only
    python3 data-generators/pipeline.py --pages-dir ~/Downloads/wowhead
    python3 data-generators/pipeline.py --only content,gate --no-publish

Stages, in order (each one's outputs are the next one's inputs):

  detect    client build from .build.info -> release/v1/world-data-target.json
  generate  run_all.py (+ import_wowhead_pages.py with --pages-dir) -> data-generators/output/
  verify    azeroth-cli verify-world-data on the output
  register  to_facts.py, then register_facts.py: fact documents into
            content/source-materials/objects (LFS), the admission registry and
            content/source-build-v1.json; then source-sync and source-build
  content   world_to_content.py -> content/world/<build>/ (catalog.json, sources.json,
            conflicts.json), retain exact admitted originals/corresponding source,
            rebind source metadata, then build-content-v2 and verify-pack
  gate      release readiness with release/v1/accepted-origins.json
  publish   branch data/world-<build>-<date>, commit (LFS), push, draft PR

Interfaces between stages (owned by the V2.8 sub-tasks):

  register_facts.py --facts DIR --registry FILE --plan FILE --objects DIR
                    --updates FILE --notice FILE --target FILE
  world_to_content.py --input DIR --target FILE --policy FILE --authored DIR --out DIR

Each stage logs to data-generators/.cache/logs/pipeline-<stage>.log (the cache
folder follows NAVZEROTH_DATAGEN_CACHE). The run ends with a summary table and
exits non-zero when a stage failed. A gate that is not ready is reported, not a
failure. Publishing never pushes to main and never force-pushes: a later run on
an existing data branch adds a merge commit whose parents are the branch tip and
the current HEAD.
"""
import argparse
import datetime
import json
import hashlib
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
STAGES = ["detect", "generate", "verify", "register", "content", "gate", "publish"]
TARGET = ROOT / "release" / "v1" / "world-data-target.json"
POLICY = ROOT / "release" / "v1" / "accepted-origins.json"
LOCAL = HERE / "local.json"
RUN_ALL = HERE / "run_all.py"
IMPORT_PAGES = HERE / "import_wowhead_pages.py"
TO_FACTS = HERE / "navzeroth_datagen" / "to_facts.py"
# Generators that need no Wowhead page (the weekly CI run).
NETWORK_FREE = ["gen_fishing", "gen_world_cmangos", "gen_world_db2"]
# Paths the publish stage stages, relative to ROOT.
PUBLISHED = [".gitattributes", "data-generators/output", "content/world", "content/source-materials",
             "content/source-build-v1.json", "docs/sources/source-admission-v1.json",
             "docs/sources/source-updates-v1.json", "release/v1/world-data-target.json"]
LFS_POINTER = b"version https://git-lfs.github.com/spec/v1"

# WoW Forever client products seen in .build.info (Product column).
FOREVER_PRODUCTS = ("wow_classic_beta", "wowf")
INSTALL_NAMES = ("World of Warcraft",)


def workspace_version():
    """The workspace package version; release-readiness refuses any other version."""
    try:
        m = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(encoding="utf-8"), re.M)
    except OSError:
        m = None
    return m.group(1) if m else "1.0.0"


def probe_roots():
    """Install folders probed after --client, in order (WSL mounts, then native Windows)."""
    roots = []
    for drive in ("c", "d"):
        for base in ("Program Files (x86)", "Program Files", ""):
            for name in INSTALL_NAMES:
                parts = [p for p in (f"/mnt/{drive}", base, name) if p]
                roots.append(pathlib.Path(*parts))
    if os.name == "nt":
        for drive in ("C:\\", "D:\\"):
            for base in ("Program Files (x86)", "Program Files", ""):
                for name in INSTALL_NAMES:
                    roots.append(pathlib.Path(drive, *(p for p in (base, name) if p)))
    return roots


# --------------------------------------------------------------------------- context


class Context:
    """What the stages share: options, paths, the subprocess runner and the log folder.

    `runner(cmd, cwd, env, log)` returns a subprocess.CompletedProcess with text
    stdout/stderr; tests replace it."""

    def __init__(self, args, runner=None, root=ROOT, probes=None, local=LOCAL, now=None):
        self.args = args
        self.root = pathlib.Path(root)
        self.runner = runner or run_process
        self.probes = probes if probes is not None else probe_roots()
        self.local = pathlib.Path(local)
        self.now = now or datetime.datetime.now(datetime.timezone.utc)
        self.cli = getattr(args, "cli", None)
        self.notes = {}
        self.log_handle = None
        cache = os.environ.get("NAVZEROTH_DATAGEN_CACHE")
        self.logs = (pathlib.Path(cache) if cache else HERE / ".cache") / "logs"

    def log(self, message):
        print(message, flush=True)
        if self.log_handle:
            self.log_handle.write(message + "\n")
            self.log_handle.flush()

    def run(self, cmd, env=None, check=True, quiet=False):
        """Run cmd from the repository root; output goes to the stage log."""
        cmd = [str(c) for c in cmd]
        if self.log_handle:
            self.log_handle.write("$ " + " ".join(cmd) + "\n")
            self.log_handle.flush()
        if not quiet:
            print("  $ " + " ".join(cmd), flush=True)
        result = self.runner(cmd, cwd=self.root, env=env, log=self.log_handle)
        if check and result.returncode != 0:
            raise StageError(f"{command_label(cmd)} exited {result.returncode} (see the stage log)")
        return result


class StageError(Exception):
    pass


def command_label(cmd):
    """'run_all.py', 'azeroth-cli verify-world-data', 'git push' for error messages."""
    scripts = [pathlib.Path(c).name for c in cmd if c.endswith(".py")]
    if scripts:
        return scripts[0]
    return " ".join([pathlib.Path(cmd[0]).name, *cmd[1:2]])


def run_process(cmd, cwd, env=None, log=None):
    result = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)
    if log:
        if result.stdout:
            log.write(result.stdout if len(result.stdout) < 1 << 20 else result.stdout[: 1 << 20] + "\n...\n")
        if result.stderr:
            log.write(result.stderr)
        log.flush()
    return result


# --------------------------------------------------------------------------- detect


def parse_build_info(text):
    """Rows of a .build.info table as dicts keyed by bare column names."""
    lines = [line for line in text.splitlines() if line.strip()]
    if not lines:
        return []
    header = [col.split("!", 1)[0].strip() for col in lines[0].split("|")]
    rows = []
    for line in lines[1:]:
        cells = line.split("|")
        if len(cells) != len(header):
            continue
        rows.append(dict(zip(header, (c.strip() for c in cells))))
    return rows


def choose_row(rows):
    """The active WoW Forever row: Active=1 and a Forever product or a 1.60.x version.
    A 1.60.x version ranks first, then the known product names in FOREVER_PRODUCTS order."""
    candidates = []
    for row in rows:
        if row.get("Active") != "1":
            continue
        product, version = row.get("Product", ""), row.get("Version", "")
        forever = product in FOREVER_PRODUCTS
        if not (forever or version.startswith("1.60.")):
            continue
        rank = (0 if version.startswith("1.60.") else 1,
                FOREVER_PRODUCTS.index(product) if forever else len(FOREVER_PRODUCTS))
        candidates.append((rank, row))
    candidates.sort(key=lambda c: c[0])
    return candidates[0][1] if candidates else None


def interface_of(version):
    """1.60.1.70450 -> "16001" (major*10000 + minor*100 + patch)."""
    m = re.match(r"^(\d+)\.(\d+)\.(\d+)\.(\d+)$", version or "")
    if not m:
        raise ValueError(f"unrecognised client version {version!r}")
    major, minor, patch = (int(x) for x in m.groups()[:3])
    return str(major * 10000 + minor * 100 + patch)


def locale_of(row, default):
    m = re.search(r"\b([a-z]{2}[A-Z]{2}) text\?", row.get("Tags", ""))
    return m.group(1) if m else default


def build_info_in(folder):
    """The .build.info of an install folder, or of a product subfolder's parent."""
    folder = pathlib.Path(folder)
    for candidate in (folder / ".build.info", folder.parent / ".build.info"):
        if candidate.is_file():
            return candidate
    return None


def find_build_info(ctx):
    """(path, how it was found, folders probed); path None when nothing was found."""
    probed = []
    if ctx.args.client:
        found = build_info_in(ctx.args.client)
        if not found:
            raise StageError(f"--client {ctx.args.client}: no .build.info there or in its parent")
        return found, "--client", [str(ctx.args.client)]
    for root in ctx.probes:
        probed.append(str(root))
        found = build_info_in(root)
        if found:
            return found, "probe", probed
    if ctx.local.is_file():
        try:
            client = json.loads(ctx.local.read_text(encoding="utf-8"))["client"]
        except (ValueError, KeyError, TypeError) as e:
            raise StageError(f"{ctx.local.name}: expected {{\"client\": \"<dir>\"}} ({e})")
        probed.append(f"{ctx.local.name}: {client}")
        found = build_info_in(client)
        if not found:
            raise StageError(f"{ctx.local.name} names {client}, which holds no .build.info")
        return found, "local.json", probed
    probed.append(f"{ctx.local.name} (absent)")
    return None, None, probed


def target_document(current, row, info_path, how):
    version = row["Version"]
    install = info_path.parent.name
    doc = dict(current)
    doc.update({
        "edition": "wow_forever",
        "build": version,
        "interface": interface_of(version),
        "locale": locale_of(row, current.get("locale", "enUS")),
        "detected_from": f".build.info of the '{install}' install ({how}; product "
                         f"{row.get('Product', 'unknown')}, active row)",
    })
    return doc


def stage_detect(ctx):
    target = ctx.root / "release" / "v1" / "world-data-target.json"
    found, how, probed = find_build_info(ctx)
    if not found:
        ctx.log("no WoW client .build.info found; probed:\n  " + "\n  ".join(probed))
        ctx.log(f"keeping the committed {target.relative_to(ctx.root)} unchanged")
        return "skipped", "no client found; committed target kept"
    rows = parse_build_info(found.read_text(encoding="utf-8", errors="replace"))
    row = choose_row(rows)
    if not row:
        products = ", ".join(sorted({r.get("Product", "?") for r in rows})) or "none"
        raise StageError(f"{found}: no active WoW Forever row (products: {products})")
    current = json.loads(target.read_text(encoding="utf-8"))
    doc = target_document(current, row, found, how)
    text = json.dumps(doc, indent=2, ensure_ascii=False) + "\n"
    if text == target.read_text(encoding="utf-8"):
        return "ok", f"build {doc['build']} (unchanged)"
    target.write_text(text, encoding="utf-8")
    ctx.log(f"wrote {target.relative_to(ctx.root)}: build {doc['build']}, interface {doc['interface']}")
    return "ok", f"build {doc['build']}"


def read_target(ctx):
    from navzeroth_datagen import bounded
    return bounded.json_document(bounded.read_file(ctx.root / "release" / "v1" / "world-data-target.json", bounded.MAX_SHARD_BYTES))


# --------------------------------------------------------------------------- generate / verify


def mode_flags(args):
    return ["--offline"] if args.offline else ["--refresh"] if args.refresh else []


def stage_generate(ctx):
    py = sys.executable
    if ctx.args.pages_dir:
        ctx.run([py, "-I", IMPORT_PAGES, ctx.args.pages_dir])
    cmd = [py, "-I", RUN_ALL, *mode_flags(ctx.args)]
    if ctx.args.no_network_sources:
        cmd += ["--only", ",".join(NETWORK_FREE)]
    env=None
    if ctx.args.weekly_verified:
        from navzeroth_datagen import bounded
        policy=bounded.json_document(bounded.read_file(ctx.root/'release/v1/source-normalization-policy.json',bounded.MAX_SHARD_BYTES))
        rules=[rule for rule in policy['sources'] if rule['id']=='wago-tools-wowf']
        if len(rules)!=1 or rules[0]['origin']['edition']!='wow_forever' or not re.fullmatch(r'[0-9]{1,9}(?:\.[0-9]{1,9}){3}',rules[0]['origin']['build']):
            raise StageError('weekly source policy has no exact Wago build')
        build=rules[0]['origin']['build']
        env=dict(os.environ,NAVZEROTH_DATAGEN_WAGO_WOWF_BUILD=build,NAVZEROTH_DATAGEN_LOGS=str(ctx.logs))
        ctx.log('weekly policy-pinned Wago wowf build: '+build)
        cmd.append('--clean-declared')
    ctx.run(cmd,env=env)
    return "ok", manifest_summary(ctx.root / "data-generators" / "output") or ""


def manifest_summary(out):
    try:
        doc = json.loads((out / "run-manifest.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    gens = doc.get("generators", [])
    records = sum(f.get("records", 0) for g in gens for f in g.get("files", []))
    shards = sum(len(g.get("files", [])) for g in gens)
    return f"{len(gens)} generators, {shards} shards, {records} records"


def ensure_cli(ctx):
    """The azeroth-cli binary: --cli, else built once with cargo and reused."""
    if ctx.cli:
        return ctx.cli
    ctx.run(["cargo", "build", "--locked", "-p", "azeroth-cli"])
    suffix = ".exe" if os.name == "nt" else ""
    ctx.cli = str(ctx.root / "target" / "debug" / f"azeroth-cli{suffix}")
    return ctx.cli


def stage_verify(ctx):
    cli = ensure_cli(ctx)
    result = ctx.run([cli, "verify-world-data", "--input", "data-generators/output"])
    try:
        report = json.loads(result.stdout)
        return "ok", ", ".join(f"{k} {report[k]}" for k in ("files", "records") if k in report)
    except (ValueError, TypeError):
        return "ok", ""


# --------------------------------------------------------------------------- register / content


def require(ctx, rel, owner):
    """The script ctx.root/rel; a missing one (another V2.8 task) fails the stage clearly."""
    path = ctx.root / rel
    if not path.is_file():
        raise StageError(f"{rel} is missing; it is written by task {owner}, which must be merged first")
    return path


def fresh(path):
    """Remove a pipeline-owned output folder under dist/ so the CLI can create it."""
    path = pathlib.Path(path)
    if path.exists():
        shutil.rmtree(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    return path


def stage_register(ctx):
    script = require(ctx, "data-generators/register_facts.py", "V2.8c (W2)")
    py = sys.executable
    ctx.run([py, "-I", TO_FACTS, "--input", "data-generators/output", "--output", "data-generators/output/facts"])
    ctx.run([py, "-I", script,
             "--facts", "data-generators/output/facts",
             "--registry", "docs/sources/source-admission-v1.json",
             "--plan", "content/source-build-v1.json",
             "--objects", "content/source-materials/objects",
             "--updates", "docs/sources/source-updates-v1.json",
             "--notice", "content/source-materials/NOTICE.md",
             "--target", "release/v1/world-data-target.json"])
    cli = ensure_cli(ctx)
    ctx.run([cli, "source-sync", "--registry", "docs/sources/source-admission-v1.json",
             "--cache", "content/source-materials/objects"])
    out = fresh(ctx.root / "dist" / "source-knowledge-world")
    ctx.run([cli, "source-build", "--registry", "docs/sources/source-admission-v1.json",
             "--plan", "content/source-build-v1.json", "--cache", "content/source-materials/objects",
             "--output", out.relative_to(ctx.root).as_posix()])
    return "ok", f"catalog in {out.relative_to(ctx.root).as_posix()}"


def stage_content(ctx):
    script = require(ctx, "data-generators/world_to_content.py", "V2.8d (W3)")
    build = read_target(ctx)["build"]
    out = f"content/world/{build}"
    retain = require(ctx, "data-generators/retain_world_sources.py", "STABLE-SOURCE-CLOSURE")
    mapping = [sys.executable, "-I", script,
               "--input", "data-generators/output",
               "--target", "release/v1/world-data-target.json",
               "--policy", "release/v1/accepted-origins.json",
               "--authored", "content/authored/slice-v1",
               "--out", out]
    # The first pass enumerates actual mapped fields. It is never packed/published
    # unless exact retained originals and admission bind the second pass.
    ctx.run(mapping)
    closure = f"{out}/source-closure.json"
    cache = os.environ.get("NAVZEROTH_DATAGEN_CACHE") or str(ctx.root / "data-generators/.cache")
    ctx.run([sys.executable, "-I", retain, "--root", ".",
             "--world", "data-generators/output", "--cache", cache,
             "--pack-sources", f"{out}/sources.json",
             "--registry", "docs/sources/source-admission-v1.json", "--output", closure,
             "--normalization-policy", "release/v1/source-normalization-policy.json"])
    ctx.run([*mapping, "--admission", "docs/sources/source-admission-v1.json",
             "--source-closure", closure,
             "--normalization-policy", "release/v1/source-normalization-policy.json"])
    cli = ensure_cli(ctx)
    pack = fresh(ctx.root / "dist" / "world-pack" / build).relative_to(ctx.root).as_posix()
    ctx.run([cli, "build-content-v2", "--input", out, "--output", pack])
    ctx.run([cli, "verify-pack", "--input", pack])
    return "ok", f"{out} -> {pack}"


# --------------------------------------------------------------------------- gate


def stage_gate(ctx):
    build = read_target(ctx)["build"]
    cli = ensure_cli(ctx)
    result = ctx.run([cli, "release-readiness", "--root", ".", "--version", ctx.args.version], check=False)
    try:
        report = json.loads(result.stdout)
    except (ValueError, TypeError):
        raise StageError(f"release-readiness exited {result.returncode} without a JSON report "
                         f"({(result.stderr or '').strip()[:300]})")
    path = ctx.root / "content" / "world" / build / "release-readiness.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    allowed = report.get("publication_allowed")
    ctx.notes["gate"] = "ready" if allowed else "not ready"
    blockers = report.get("blockers")
    detail = f"; {len(blockers)} blockers" if isinstance(blockers, list) else ""
    return "ok", f"{'ready' if allowed else 'NOT ready'}{detail}; report {path.relative_to(ctx.root).as_posix()}"


# --------------------------------------------------------------------------- publish


def branch_name(ctx, build):
    return ctx.args.branch or f"data/world-{build}-{ctx.now.strftime('%Y%m%d')}"


def commit_message(ctx, build, changed):
    lines = [f"data: world data for {build}", ""]
    summary = manifest_summary(ctx.root / "data-generators" / "output")
    if summary:
        lines.append(f"Generated: {summary}.")
    try:
        doc = json.loads((ctx.root / "data-generators" / "output" / "run-manifest.json").read_text(encoding="utf-8"))
        for g in doc.get("generators", []):
            records = sum(f.get("records", 0) for f in g.get("files", []))
            lines.append(f"- {g['name']}: {g.get('status')}, {len(g.get('files', []))} shards, {records} records")
    except (OSError, ValueError, KeyError):
        pass
    lines.append(f"Changed files: {len(changed)}.")
    if "gate" in ctx.notes:
        lines.append(f"Release readiness: {ctx.notes['gate']}.")
    lines += ["", "Written by data-generators/pipeline.py (ADR 0007)."]
    return "\n".join(lines) + "\n"


def stage_publish(ctx):
    build = read_target(ctx)["build"]
    branch = branch_name(ctx, build)
    if branch in ("main", "master") or not branch.startswith("data/"):
        raise StageError(f"refusing to publish to {branch!r}: data branches start with data/")
    paths = [p for p in PUBLISHED if (ctx.root / p).exists()]
    if not paths:
        return "skipped", "no generated paths exist"
    base = ctx.run(["git", "rev-parse", "HEAD"], quiet=True).stdout.strip()
    with tempfile.TemporaryDirectory() as tmp:
        # A private index: the owner's index and working tree are left as they are.
        env = dict(os.environ, GIT_INDEX_FILE=str(pathlib.Path(tmp) / "index"))
        ctx.run(["git", "read-tree", "HEAD"], env=env, quiet=True)
        ctx.run(["git", "add", "-A", "--", *paths], env=env)
        changed = [p for p in ctx.run(["git", "diff", "--cached", "--name-only", "-z", "HEAD"],
                                      env=env, quiet=True).stdout.split("\0") if p]
        if not material(changed):
            ctx.log("generated output equals the base commit (a readiness report alone does not count); "
                    "nothing to publish")
            return "skipped", "no changes versus HEAD"
        check_lfs_pointers(ctx, env, changed)
        tree = ctx.run(["git", "write-tree"], env=env, quiet=True).stdout.strip()
    remote_tip = None
    if ctx.run(["git", "fetch", "origin", f"refs/heads/{branch}"], check=False, quiet=True).returncode == 0:
        remote_tip = ctx.run(["git", "rev-parse", "FETCH_HEAD"], quiet=True).stdout.strip()
        differs = ctx.run(["git", "diff", "--name-only", "-z", remote_tip, tree, "--", *paths],
                          quiet=True).stdout.split("\0")
        if not material([p for p in differs if p]):
            ctx.log(f"origin/{branch} already holds this output")
            return "skipped", f"origin/{branch} already holds this output"
    parents = ["-p", remote_tip, "-p", base] if remote_tip else ["-p", base]
    message = commit_message(ctx, build, changed)
    commit = ctx.run(["git", "commit-tree", tree, *parents, "-m", message], quiet=True).stdout.strip()
    ctx.run(["git", "branch", "-f", branch, commit])
    ctx.run(["git", "push", "origin", f"{commit}:refs/heads/{branch}"])
    title = f"Generated world data for {build}"
    body = message + "\nThe owner reviews and merges; this pull request was opened by the pipeline.\n"
    found = ctx.run(["gh", "pr", "list", "--head", branch, "--state", "open", "--json", "number",
                     "--jq", ".[0].number"], check=False, quiet=True)
    number = (found.stdout or "").strip() if found.returncode == 0 else ""
    if number:
        ctx.run(["gh", "pr", "edit", number, "--title", title, "--body", body])
        return "ok", f"{branch} @ {commit[:9]}; updated PR #{number}"
    ctx.run(["gh", "pr", "create", "--draft", "--base", ctx.args.base, "--head", branch,
             "--title", title, "--body", body])
    return "ok", f"{branch} @ {commit[:9]}; draft PR opened"


def material(paths):
    """Changed paths that count: the gate report names the commit it ran on, so it
    changes on every run and alone never justifies a pull request."""
    return [p for p in paths if not p.endswith("/release-readiness.json")]


def check_lfs_pointers(ctx, env, changed):
    """Every staged LFS-attributed file must be a pointer blob (git-lfs installed)."""
    attrs = ctx.run(["git", "check-attr", "-z", "filter", "--", *changed], env=env, quiet=True).stdout
    fields = attrs.split("\0")
    lfs = [fields[i] for i in range(0, len(fields) - 2, 3) if fields[i + 2] == "lfs"]
    bad = []
    for path in lfs:
        size = ctx.run(["git", "cat-file", "-s", f":{path}"], env=env, check=False, quiet=True)
        if size.returncode != 0:
            continue  # deleted from the index
        if int((size.stdout or "0").strip() or 0) > 1024:  # pointers are about 130 bytes
            bad.append(path)
            continue
        blob = ctx.run(["git", "cat-file", "blob", f":{path}"], env=env, quiet=True)
        if not (blob.stdout or "").encode()[:len(LFS_POINTER)] == LFS_POINTER:
            bad.append(path)
    if bad:
        raise StageError(f"{len(bad)} LFS-tracked file(s) staged as full content, e.g. {bad[0]}; "
                         "install git-lfs (git lfs install --local) and run again")
    ctx.log(f"{len(lfs)} LFS-tracked file(s) staged as pointers")


# --------------------------------------------------------------------------- driver

STAGE_FUNCS = {"detect": stage_detect, "generate": stage_generate, "verify": stage_verify,
               "register": stage_register, "content": stage_content, "gate": stage_gate,
               "publish": stage_publish}


def parse_args(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--only", help="comma-separated stages (default: all, in order)")
    p.add_argument("--skip", help="comma-separated stages to leave out")
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--offline", action="store_true", help="use only cached downloads")
    mode.add_argument("--refresh", action="store_true", help="ignore the download cache")
    p.add_argument("--pages-dir", help="folder of Wowhead list pages saved by the owner")
    p.add_argument("--client", help="WoW client folder holding .build.info (else probed / local.json)")
    p.add_argument("--no-network-sources", action="store_true",
                   help="skip the Wowhead generators (the weekly CI run)")
    p.add_argument("--hydrate-weekly-manifest",action="store_true",help="hydrate and verify only prior run-manifest metadata then stop")
    p.add_argument("--weekly-verified",action="store_true",help="policy pinned generation, retention before verification, exact offline replay")
    p.add_argument("--no-publish", action="store_true", help="stop before the branch and draft PR")
    p.add_argument("--cli", help="a built azeroth-cli to use (default: cargo build once)")
    p.add_argument("--version", default=workspace_version(),
                   help="release version for the gate (default: the workspace version in Cargo.toml)")
    p.add_argument("--branch", help="publish branch (default data/world-<build>-<YYYYMMDD>)")
    p.add_argument("--base", default="main", help="pull request base branch (default main)")
    args = p.parse_args(argv)
    if args.weekly_verified and args.offline:
        p.error('weekly verification requires one acquisition pass followed by its own offline replay')
    args.stages = [s.strip() for s in args.only.split(",") if s.strip()] if args.only else list(STAGES)
    skip = [s.strip() for s in args.skip.split(",") if s.strip()] if args.skip else []
    unknown = [s for s in args.stages + skip if s not in STAGES]
    if unknown:
        p.error(f"unknown stage(s): {', '.join(unknown)}; stages: {', '.join(STAGES)}")
    if args.no_publish:
        skip.append("publish")
    args.stages = [s for s in STAGES if s in args.stages and s not in skip]
    if args.weekly_verified:
        if not args.no_network_sources or args.only or args.pages_dir or args.refresh:
            p.error('weekly verification requires full no-Wowhead stages and default cache mode')
        if not {'generate','verify','content','register','gate'}.issubset(args.stages):
            p.error('weekly verification requires every generate/verify/content/register/gate stage')
        args.stages=[stage for stage in ['detect','generate','verify','content','register','gate','publish'] if stage in args.stages]
    if not args.stages:
        p.error("no stage left to run")
    return args


def run_stages(ctx):
    """Run the selected stages in order; stops at the first failure. Returns [(stage, status, secs, note)]."""
    ctx.logs.mkdir(parents=True, exist_ok=True)
    results = []
    for stage in ctx.args.stages:
        print(f"== {stage}", flush=True)
        start = time.monotonic()
        with open(ctx.logs / f"pipeline-{stage}.log", "w", encoding="utf-8") as handle:
            ctx.log_handle = handle
            try:
                status, note = STAGE_FUNCS[stage](ctx)
            except StageError as e:
                status, note = "failed", str(e)
                ctx.log(f"{stage}: {e}")
            except Exception as e:  # noqa: BLE001 - reported in the summary
                status, note = "failed", f"{type(e).__name__}: {e}"
                ctx.log(f"{stage}: {note}")
            finally:
                ctx.log_handle = None
        results.append((stage, status, time.monotonic() - start, note))
        if status == "failed":
            for rest in ctx.args.stages[ctx.args.stages.index(stage) + 1:]:
                results.append((rest, "not run", 0.0, ""))
            break
    return results


def summary(results, logs):
    lines = [f"\n{'stage':<10} {'status':<9} {'seconds':>8}  note"]
    for stage, status, secs, note in results:
        lines.append(f"{stage:<10} {status:<9} {secs:>8.1f}  {note}")
    lines.append(f"logs: {logs}/pipeline-<stage>.log")
    return "\n".join(lines)


def hydrate_weekly_manifest(ctx):
    """Fetch one bounded metadata object, never original/raw/media wildcards."""
    from navzeroth_datagen import bounded
    relative='data-generators/output/run-manifest.json'
    size=ctx.run(['git','cat-file','-s','HEAD:'+relative],quiet=True).stdout.strip()
    if not size.isdecimal() or not 0<int(size)<=bounded.MAX_SHARD_BYTES:
        raise StageError('prior manifest Git blob exceeds bound')
    raw=ctx.run(['git','show','HEAD:'+relative],quiet=True).stdout.encode()
    match=re.fullmatch(rb'version https://git-lfs.github.com/spec/v1\noid sha256:([0-9a-f]{64})\nsize ([0-9]+)\n',raw)
    if match:
        expected_size=int(match[2])
        if not 0<expected_size<=bounded.MAX_SHARD_BYTES:raise StageError('prior manifest LFS object exceeds8MiB')
        ctx.run(['git','lfs','pull','--include='+relative,'--exclude='])
        actual=bounded.read_file(ctx.root/relative,bounded.MAX_SHARD_BYTES)
        if len(actual)!=expected_size or hashlib.sha256(actual).hexdigest()!=match[1].decode():raise StageError('hydrated prior manifest differs from exact Git pin')
    elif raw.startswith(LFS_POINTER):
        raise StageError('noncanonical prior manifest LFS pointer')
    else:
        actual=bounded.read_file(ctx.root/relative,bounded.MAX_SHARD_BYTES)
        if actual!=raw:raise StageError('prior manifest differs from committed metadata')
    if json.loads(actual).get('format')!='navzeroth-world-data-run':raise StageError('invalid prior manifest format')
    ctx.log(f'prior manifest metadata verified: {len(actual)} bytes; no other LFS selection')


def published_digests(ctx, paths=None, maximum=4*1024**3):
    """Exact source bytes; only the documented readiness report is nonmaterial."""
    from navzeroth_datagen import bounded
    result={}
    total=0
    build=read_target(ctx).get('build')
    if not isinstance(build,str) or not re.fullmatch(r'[0-9]{1,9}(?:\.[0-9]{1,9}){3}',build):
        raise StageError('replay requires an explicit canonical target build')
    volatile_report=f'content/world/{build}/release-readiness.json'
    for relative in PUBLISHED if paths is None else paths:
        base=ctx.root/relative
        if base.is_symlink():raise StageError("linked published root refused")
        if not base.exists():continue
        paths=bounded.input_files(base,'*') if base.is_dir() else [base]
        for path in paths:
            name=path.relative_to(ctx.root).as_posix()
            if name==volatile_report:continue
            info=path.lstat()
            if path.is_symlink() or not path.is_file():raise StageError('invalid published material file')
            total+=info.st_size
            if total>maximum or len(result)>=16384:raise StageError('published replay closure exceeds bounds')
            digest=hashlib.sha256()
            descriptor=os.open(path,os.O_RDONLY|getattr(os,'O_NOFOLLOW',0)|getattr(os,'O_NONBLOCK',0))
            with os.fdopen(descriptor,'rb') as source:
                stamp=lambda value:(value.st_dev,value.st_ino,value.st_size,value.st_mtime_ns,value.st_ctime_ns)
                if stamp(os.fstat(source.fileno()))!=stamp(info):raise StageError('published material changed before hashing')
                count=0
                while chunk:=source.read(min(1024*1024,info.st_size-count+1)):
                    count+=len(chunk)
                    if count>info.st_size:raise StageError('published material changed while hashing')
                    digest.update(chunk)
                if count!=info.st_size or stamp(os.fstat(source.fileno()))!=stamp(info) or stamp(path.lstat())!=stamp(info):raise StageError('published material changed while hashing')
            result[name]={'bytes':count,'sha256':digest.hexdigest()}
    return result


def verified_weekly(ctx):
    original_stages=list(ctx.args.stages)
    ctx.args.stages=[stage for stage in original_stages if stage!='publish']
    results=run_stages(ctx)
    if any(row[1]=='failed' for row in results):return results
    first=published_digests(ctx)
    products=published_digests(ctx,["dist/source-knowledge-world","dist/world-pack"],10*1024**3)
    first_logs=ctx.logs
    ctx.args.offline=True
    ctx.logs=first_logs/'offline-replay'
    replay=run_stages(ctx)
    results.extend(('offline-'+stage,status,seconds,note) for stage,status,seconds,note in replay)
    if any(row[1]=='failed' for row in replay):return results
    second=published_digests(ctx)
    second_products=published_digests(ctx,["dist/source-knowledge-world","dist/world-pack"],10*1024**3)
    if first!=second or products!=second_products:
        changed=sorted(name for before,after in [(first,second),(products,second_products)] for name in before.keys()|after.keys() if before.get(name)!=after.get(name))
        results.append(('reproducibility','failed',0.0,'exact material bytes differ: '+','.join(changed[:16])))
        return results
    receipt={'schema_version':1,'verified':True,'network_passes':1,'offline_passes':1,'files':first,'unpublished_build_products':products,
             'excluded_nonmaterial':[f"content/world/{read_target(ctx)['build']}/release-readiness.json",'pipeline/cache logs outside published paths']}
    (first_logs/'weekly-reproducibility.json').write_text(json.dumps(receipt,sort_keys=True,indent=2)+'\n',encoding='utf-8')
    ctx.logs=first_logs
    if 'publish' in original_stages:
        ctx.args.stages=['publish']
        results.extend(run_stages(ctx))
    return results


def main(argv=None, runner=None):
    args = parse_args(argv)
    ctx = Context(args, runner=runner)
    if args.hydrate_weekly_manifest:
        hydrate_weekly_manifest(ctx)
        return 0
    results = verified_weekly(ctx) if args.weekly_verified else run_stages(ctx)
    print(summary(results, ctx.logs))
    return 1 if any(r[1] == "failed" for r in results) else 0


if __name__ == "__main__":
    sys.exit(main())
