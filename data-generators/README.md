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
| `Navzeroth-data-generators/gen_world_cmangos.py` | NPCs, spawns, quests, loot, ... | CMaNGOS classic-db (`vanilla_1_12`) |

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
