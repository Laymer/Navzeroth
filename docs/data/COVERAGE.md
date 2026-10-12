# Existing dataset coverage

Generated with `validate-content --input content/forever --json` on baseline 08f6daa plus R8.

`forever-atlas-r1` passes structural validation; this does not verify live content.

| Collection | Records |
| --- | ---: |
| evidence | 6 |
| items | 0 |
| acquisitions | 0 |
| quests | 0 |
| map_spaces | 1 |
| nodes | 0 |
| edges | 0 |
| objectives | 0 |
| rules | 0 |
| map_images | 1 |
| map_embeddings | 0 |

Errors: 0. Coverage gaps: 3.

- `unknown_map_transform`: 1
- `unverified_evidence`: 2

The atlas calibration is inferred; pins are reference locations, not player unlocks. No playable quest segment or acquisition catalog is claimed. Demo atlas/contract fixtures remain synthetic and are reported as demo when passed explicitly.

CLI accepts one dataset file or a directory containing `datasets/*.json` (otherwise direct `*.json`). It never recurses. Limits: 128 files, 16 MiB per file, 64 MiB total; regular UTF-8 files only. `--json` returns deterministic full coverage reports; human output lists counts and fixed finding codes. Gaps alone succeed; decode/validation errors fail with exit 1; malformed command options use exit 2.
