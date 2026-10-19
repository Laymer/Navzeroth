# TALOD data generators

Python 3 scripts (standard library only) that build TALOD's reference data files.

| Script | Writes | Source |
|---|---|---|
| `tools/gen_enhancements.py` | `EnhanceData.lua` (enchants, kits, stones, oils...) | Wowhead Classic |
| `tools/gen_professions.py` | `ProfessionData.lua` (every recipe, reagents, trainers) | Wowhead Classic |
| `tools/gen_fishing.py` | `FishingData.lua` (zone fishing levels, goal fish, schedules) | CMaNGOS classic-db, wago.tools |

`gen_professions.py` imports helpers from `gen_enhancements.py`, so keep them in the same folder.
Each script writes its output to the folder *above* `tools/` and caches downloads in
`tools/cache/`. Pass `--refresh` to ignore the cache. See each script's docstring for details.

    python tools/gen_professions.py
    python tools/gen_enhancements.py --refresh
    python tools/gen_fishing.py

**In Navzeroth** the scripts are adapted (changes marked `# Navzeroth:`): instead of the Lua
files they write `navzeroth-world-data` v1 JSON (datasets `professions.*`, `enhancements.*`,
`fishing.*`) to `data-generators/output/`, cache downloads in `data-generators/.cache/`, and
are run together by `data-generators/run_all.py` (see `../README.md`). They also take
`--offline` and `--out DIR`.

The data these scripts download belongs to its sources (Wowhead, CMaNGOS, wago.tools) and is
subject to their terms. The permission in PERMISSION.md covers the scripts only.
