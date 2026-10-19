# Navzeroth data generators

Python 3.10+ scripts (standard library only) that build Navzeroth's world reference
data as `navzeroth-world-data` v1 JSON shards
(`schemas/world-data/v1/world-data.schema.json`, `crates/core/src/world_data.rs`).

| Generator | Datasets | Source (origin edition) |
|---|---|---|
| `TALOD-data-generators/tools/gen_professions.py` | `professions.ranks`, `.notes`, `.recipes`, `.items` | Wowhead Classic (`wow_classic_era`, build unknown) |
| `TALOD-data-generators/tools/gen_enhancements.py` | `enhancements.enhancements`, `.recipes`, `.items` | Wowhead Classic (`wow_classic_era`, build unknown) |
| `TALOD-data-generators/tools/gen_fishing.py` | `fishing.areas`, `fishing.goals` | CMaNGOS classic-db (`vanilla_1_12`, 1.12.1) and wago.tools AreaTable names |
| | `fishing.timed`, `fishing.events` | TALOD's hand lists with cited wiki pages (`wow_classic_era`, build unknown) |
| `Navzeroth-data-generators/gen_world_db2.py` | client DB2 tables (areas, uiMaps, ...) | wago.tools `wowf` (`wow_forever`, pinned build) |
| `Navzeroth-data-generators/gen_world_cmangos.py` | NPCs, spawns, quests, loot, items (`item_template`, the fallback for items the Forever table lacks), ... | CMaNGOS classic-db (`vanilla_1_12`) |

## Running

    python3 data-generators/run_all.py --jobs 6          # all generators, at most 6 at once
    python3 data-generators/run_all.py --offline         # rebuild from the cache only
    python3 data-generators/run_all.py --only gen_fishing,gen_professions
    python3 data-generators/run_all.py --refresh --facts # fetch again; then convert with to_facts.py

`run_all.py` prefetches the shared inputs (CMaNGOS dump, wago.tools build list), then
runs each generator as `python3 -I <script> --out <out>`. A generator whose script is
missing is `skipped`. Each generator also runs alone with `--refresh`, `--offline`,
`--no-tooltips` and `--out DIR`. Tests: `python3 -m unittest discover -s data-generators/tests`.

- Cache: `data-generators/.cache/<host>/` (git-ignored; `NAVZEROTH_DATAGEN_CACHE` moves
  it). Failed answers are cached too, so a refused page stays refused until `--refresh`.
  Logs: `.cache/logs/<generator>.log`.
- Output: `data-generators/output/<dataset path>/<origin edition>/<origin build>/part-NNNN.json`
  and `output/run-manifest.json` (`navzeroth-world-data-run` v1: per generator its status
  and shards with sha256, bytes and records; the downloads used). Neither holds timestamps:
  an `--offline` re-run over the same cache writes the same bytes.

## Pipeline

`pipeline.py` (ADR 0007) runs everything from download to a draft pull request:

    python3 data-generators/pipeline.py                         # every stage
    python3 data-generators/pipeline.py --offline --no-publish  # from the cache, no branch or PR
    python3 data-generators/pipeline.py --pages-dir ~/Downloads/wowhead
    python3 data-generators/pipeline.py --only content,gate --no-publish

| Stage | Does |
|---|---|
| `detect` | Reads the client's `.build.info` and writes `release/v1/world-data-target.json` (build, interface, locale). |
| `generate` | `run_all.py` (`--offline`/`--refresh` passed on); with `--pages-dir`, `import_wowhead_pages.py` first; with `--no-network-sources`, only `gen_fishing`, `gen_world_cmangos`, `gen_world_db2`. |
| `verify` | `azeroth-cli verify-world-data --input data-generators/output`. |
| `register` | `to_facts.py`, `register_facts.py` (which also sets the target of `content/source-build-v1.json` to the edition and build of `release/v1/world-data-target.json`), then `source-sync` and `source-build` (output `dist/source-knowledge-world`). |
| `content` | `world_to_content.py` to `content/world/<build>/`, then `build-content-v2` and `verify-pack` (pack in `dist/world-pack/<build>`). |
| `gate` | `release-readiness --root . --version 1.0.0`; the JSON report goes to `content/world/<build>/release-readiness.json`. Not ready is reported, not a failure. |
| `publish` | Commits the generated paths to `data/world-<build>-<YYYYMMDD>` (`--branch` overrides), checks LFS pointers, pushes and opens or updates a draft PR. |

- Client detection: `--client DIR`, else the usual WSL installs (`/mnt/c` and `/mnt/d`,
  `Program Files (x86)`, `Program Files` or the drive root, `World of Warcraft`), else
  `data-generators/local.json` holding `{"client": "<install folder>"}` (git-ignored).
  A product subfolder such as `_classic_beta_` resolves to its parent. The active row
  whose version starts with `1.60.` (products `wow_classic_beta`, `wowf`) wins; the
  interface is major·10000 + minor·100 + patch (1.60.1 → 16001). With no client found,
  the committed target stays unchanged.
- The CLI is built once with `cargo build --locked -p azeroth-cli` (`--cli PATH` uses a
  built one). `--only`/`--skip` select stages; the order is fixed. A failed stage stops
  the run and the exit code is non-zero. Logs: `.cache/logs/pipeline-<stage>.log`.
- Publishing commits through a private index, so your index and working tree are left
  alone. It never pushes to `main` or force-pushes; on an existing data branch it adds a
  merge commit (parents: branch tip, current HEAD). Nothing is published when the
  generated paths equal HEAD (or the branch); a changed readiness report alone does not
  count. LFS-tracked files must be staged as pointers (`git lfs install --local`).
- `register` and `content` need `register_facts.py` (V2.8c) and `world_to_content.py`
  (V2.8d); until those are merged the stage fails and names the missing script.

The weekly workflow `.github/workflows/world-data.yml` runs
`pipeline.py --no-network-sources --skip detect --branch data/world-weekly` (reusing the
committed target) and opens or updates a draft PR only when the output changed. **The
owner must enable "Allow GitHub Actions to create and approve pull requests"**
(repository Settings > Actions > General > Workflow permissions), or the PR step fails.
A branch pushed and a PR opened with the default `GITHUB_TOKEN` do not trigger
`ci.yml`, so the weekly draft PR shows no checks until the owner pushes a commit to it
or the workflow is changed to use a personal access token.

## Editions

Every shard names the edition and build its data comes from; nothing is relabelled as
WoW Forever. Data from another edition stays `cross_edition_unverified` until checked
on the target build. Wowhead `/classic` pages are Classic Era with no build stated.
CMaNGOS classic-db is a community reconstruction of 1.12.1 server data. wago.tools
`wowf` is the WoW Forever client, pinned to one build per run.

Wowhead's `/forever` list pages answer HTTP 403 to scripts, and its `/classic` pages
may answer with a bot check (HTTP 202, empty body). Both are refusals: the generators
report them and fail; they are not worked around.

## Credits

- Data generators based on work by NotInTheBand (TALOD), used with permission
  (`TALOD-data-generators/PERMISSION.md`).
- Wowhead (https://www.wowhead.com/classic): list pages and tooltips, under Wowhead's terms.
- CMaNGOS classic-db (https://github.com/cmangos/classic-db), GPL-2.0.
- wago.tools (https://wago.tools): exports of Blizzard client data; the data belongs to
  Blizzard Entertainment.
