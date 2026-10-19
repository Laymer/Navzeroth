# World data v1 (`navzeroth-world-data`)

Generated world reference data: professions, fishing, items, NPCs, objects, spawns,
quests, vendors, trainers, spells, talents, travel, loot, dungeons and graveyards.
The Python generators under `data-generators/` write it; Rust validates it
(`azeroth_core::world_data`, task V2.7). The sources and their admission are in
[ADR 0006 §14](../decisions/0006-v1-program.md) and
[source admission](../sources/source-admission-v1.md); credits are in
[data-generators/NOTICE.md](../../data-generators/NOTICE.md).

A valid shard is structurally sound. It is not a rights grant by itself, not a
verified Forever fact and not release content: release `DOMAINS` and acceptance
gates are unchanged.

## Envelope

Every shard is one JSON object with unknown fields refused:

| Field | Meaning |
| --- | --- |
| `format`, `schema_version` | `"navzeroth-world-data"`, `1` |
| `dataset` | Dotted lowercase name, at most 128 bytes, e.g. `professions.recipes`, `world.spawns` |
| `record_kind` | The one kind every record in the shard has |
| `origin` | `edition`, `build` (an observation), optional `product`, `locale`: where the data came from |
| `target` | `edition` (`wow_forever`) and `build`; unknown in generated shards (the packs carry the detected client build, ADR 0007 §6) |
| `confidence` | One of `verified_on_build`, `publisher_documented`, `limited_sample`, `cross_edition_unverified`, `inferred`, `disputed_or_stale` |
| `sources` | 1 to 64 references: admission-registry `id`, `https` `url`, optional `revision`, `retrieved_at_utc_ms`, `snapshot_sha256` |
| `generator` | `name`, `version` and the required `credit` text |
| `shard` | `index` and `count`; `count` is 1 to 4,096 |
| `records` | 1 to 250,000 records, each tagged with `kind` |

## Record kinds

`item`, `recipe`, `profession_rank`, `profession_note`, `enhancement`,
`fishing_area`, `fishing_goal`, `timed_fish`, `fishing_event`, `area`, `ui_map`,
`ui_map_assignment`, `npc`, `game_object`, `spawn`, `quest`, `vendor_item`,
`trainer_spell`, `spell`, `talent`, `taxi_node`, `taxi_path`, `loot`, `dungeon`,
`graveyard`. The field lists are in the schema below.

## Unknown, unsupported and zero

An absent optional field means the source did not provide it: **unknown**, never
zero, false or empty. The generators drop `None` and keep `false`, `0`, `""` and
`[]`. Where the reason a value is missing matters, the field is an
[`Observation`](../DOMAIN.md) (`known` with a value, `unknown`, `unsupported`,
`restricted`, `not_applicable`):

- `origin.build` and `target.build`;
- `profession_rank.cost`;
- `spawn.position`, `taxi_node.position`, `graveyard.position`: the uiMap position
  (`unsupported` when the map has no uiMap, such as an instance);
- `dungeon.entrance`.

World coordinates must be finite; known map positions are normalized to 0..1. Text
is 1 to 8,192 bytes without control characters (newlines allowed).

## Origins, editions and confidence

Origin editions are `wow_forever`, `wow_classic_era` and `vanilla_1_12`. A shard
names exactly one origin and is never relabelled as another edition.

- Data from another edition than the target must be `cross_edition_unverified`
  (or `disputed_or_stale`). Classic Era and 1.12 data stays there in the shards;
  ADR 0007 §3 lets the release gate accept those origins by policy (see "Release
  gate" below) without relabelling the data.
- `verified_on_build` needs the same edition **and** a known build equal to the
  target build. While the target build is unknown, nothing is verified on build.
- Generators default to `publisher_documented` for Forever origins and
  `cross_edition_unverified` otherwise.

## Shards and limits

One dataset from one origin is written to

    <dataset with dots as />/<origin edition>/<origin build or unknown-build>/part-NNNN.json

for example `world/spawns/vanilla_1_12/1.12.1.5875/part-0000.json`. Parts are
numbered `0000` to `count-1`, one record per line with sorted keys, so a re-run from
the same cache is byte-identical. The Python writer keeps parts under 8 MiB; Rust
refuses any shard over 16 MiB (`MAX_SHARD_BYTES`) before parsing. Generated output
lives in `data-generators/output/` and is committed through Git LFS.

## Run manifest

A generator run may write `run-manifest.json` at the output root:

```json
{
  "format": "navzeroth-world-data-run",
  "schema_version": 1,
  "generators": [
    {"name": "gen_world_cmangos", "status": "ok",
     "files": [{"path": "world/spawns/vanilla_1_12/1.12.1.5875/part-0000.json",
                "sha256": "<64 hex>", "bytes": 123, "records": 4}]}
  ],
  "sources": [{"url": "https://...", "status": "ok", "sha256": "<64 hex>", "bytes": 456}]
}
```

`status` of a generator is `ok`, `failed` or `skipped`. A source `status` is a word
or an HTTP status code; `sha256` and `bytes` are optional. Unknown fields and
duplicate keys are refused. File paths are relative to the output root, use `/`,
and contain no `..`, `.`, empty component, drive or backslash.

## `verify-world-data`

    cargo run --locked -p azeroth-cli -- verify-world-data --input data-generators/output [--manifest <file>]

The command reads only local files and never fetches or executes anything:

1. Walks `--input` recursively for `*.json`, skipping `run-manifest.json`, every
   `facts/` folder (the knowledge_v1 fact documents) and dot-folders. Symbolic
   links and non-UTF-8 names are errors; nesting is limited to 32 levels and
   65,536 files.
2. Refuses a file over 16 MiB before reading it, then parses and validates each
   shard with `azeroth_core::world_data::parse`.
3. Per folder: every part has the same dataset, record kind, origin, target,
   confidence and generator; one shard count; indexes exactly `0..count-1`; file
   names `part-NNNN.json` matching the index; and the folder path above.
4. With `--manifest`, or `<input>/run-manifest.json` when it exists: every listed
   file is a shard on disk with the listed SHA-256, byte count and record count;
   every shard on disk is listed; no generator `failed`; no path listed twice;
   sources have `https` URLs and lowercase SHA-256 values.

It prints a deterministic JSON summary (`files`, `records`, records per dataset,
per-folder origin and counts, `error_count` and the first 1,000 errors with
relative paths) and exits 0 only without errors, 1 on errors, 2 on bad arguments.
The development aid `cargo run -p azeroth-core --example validate_world_data --
<file|dir>...` only parses files.

## Schema

The draft 2020-12 schema is generated from the Rust types and committed at
[`schemas/world-data/v1/world-data.schema.json`](../../schemas/world-data/v1/world-data.schema.json).
Regenerate it after a contract change with

    cargo run -q -p azeroth-core --example export_world_data_schema > schemas/world-data/v1/world-data.schema.json

Rust validation stays authoritative: the schema cannot express the edition,
confidence, record-kind and shard rules above.

## Retrieval bounds

The registry's `https_rate_limited` entries declare at least 1000 ms between
requests, matching `HOST_LIMITS` in `data-generators/navzeroth_datagen/http.py`.
The registry caps `max_requests` at 1024; a full professions run makes more
tooltip requests than that. The cap binds the source pipeline's own fetching,
which never runs for these sources (`source-sync` refuses them), so the generators'
cache and per-host limits are the effective bound. Run them opt-in, and prefer
`--offline` once the cache is warm.

## Source pipeline limits

ADR 0007 §7 registers every generated fact in the research catalog
(`data-generators/register_facts.py`, task V2.8c). Measured on 2026-10-19 from
the committed shards (`to_facts.py`, then `register_facts.py`, `source-sync` and
two `source-build` runs of the debug CLI on Linux):

| Measure | Value |
| --- | ---: |
| Generated fact documents | 76 (68 CMaNGOS `vanilla_1_12` 1.12.1, 8 wago.tools `wow_forever` 1.60.1.70245) |
| Generated facts | 2,862,735 (2,566,760 + 295,975); 2,864,420 with the hand-admitted adapters |
| Fact document bytes | 620,893,524 (561,833,509 + 59,060,015); largest document 8,323,101 |
| Largest per-source closure | 561,888,003 bytes, 72 objects (CMaNGOS) |
| Retained objects folder | 621,228,274 bytes |
| Pretty `catalog.json` | 3,355,337,694 bytes |
| Build time and peak memory | 205 to 213 s and 3.97 GB per debug build |

Limits, at about twice the measured size:

| Limit | Before | Now | Where |
| --- | ---: | ---: | --- |
| Facts per build (`MAX_FACTS`) | 250,000 | 6,000,000 | `crates/core/src/source_pipeline.rs` |
| Facts per document (`MAX_DOCUMENT_FACTS`) | 250,000 | 250,000 (unchanged; `to_facts.py` writes to it) | same |
| Encoded catalog (`MAX_CATALOG_BYTES`), compact candidates and pretty catalog | 256 MiB | 8 GiB | same |
| Retained closure and per-source bytes (`MAX_RETRIEVAL_BYTES`) | 1 GiB | 2 GiB | `crates/core/src/source_admission.rs` |

Unchanged because the data fits: 8 MiB per file, 1,024 files and requests per
source (largest: 72), 128 sources (19), 256 fields per file (largest: 36), 1 MiB
registry (101 KB), 1,024 plan inputs (80).

Memory: before V2.8c the build cloned every candidate for its identity check and
encoded the catalog in memory, peaking at 14 GB. Convergence now sorts indices
instead of cloning, and the catalog is streamed to its file while it is hashed and
counted, so the peak is 3.97 GB, inside a 16 GB CI runner. The catalog hash is
the same either way.

## Release gate (ADR 0007, task V2.8e, 2026-10-19)

This section supersedes the statements above that 1.12 and Classic Era data stays
unverified until a frozen target exists and that the acceptance gates are unchanged.

- **Schema 3 release manifests.** `release/v1/coverage-inventory.json` and
  `acceptance-evidence.json` are schema 3. The target is read from
  `release/v1/world-data-target.json` (`build` `major.minor.patch.build`,
  `interface` decimal); inventory and evidence must equal it. The 15 `LIVE-*` rows
  and the live slice matrix are no longer required; the 10 `WIN-*` rows are.
- **Accepted origins.** `release/v1/accepted-origins.json` is decoded strictly
  (`azeroth_core::origin_policy`). Evidence binds its exact bytes
  (`origin_policy` FileRef, hashed like every evidence file). A pack source lists
  its true origin and the target edition, for example
  `["vanilla_1_12", "wow_forever"]`; its origin is every edition other than the
  target. Release evidence names the same origin (`origin_edition`), and the gate
  refuses a pack source whose editions do not include it.
- **What the gate accepts.** For a v2 pack the strict `coverage_gaps` count is
  partitioned (`origin_policy::classify`) into:
  - accepted origins: unverified-evidence findings whose evidence is all
    `cross_edition_unverified` from a source of an accepted origin for the record's
    domain, or `inferred` when `accept_inferred` is set;
  - accepted unknowns (with `accept_unknowns`): unknown, unsupported and restricted
    observations, unknown map transforms and image calibrations, and positions with
    no imagery. They stay unknown and are listed, never written as zero;
    `not_applicable` stays distinct;
  - remaining gaps: everything else, including `limited_sample`, conflicting,
    unregistered or non-accepted-origin evidence. The release requires zero.
  Evidence that claims `verified_on_build` or `publisher_documented` from a source
  of non-target origin is a relabel and blocks. Content v2 already refuses
  `disputed_or_stale` and conflicted provenance.
- **Target validation.** A release source whose origin edition every citing domain
  accepts passes target validation without target evidence; rights, files,
  licences, attribution and permission evidence stay mandatory.
- **Report.** `release-readiness` reports `report_version` 3 and a `world_data`
  object: target, policy SHA-256, accepted origins per domain, the policy flags,
  origin-accepted sources, and per-pack accepted origins, accepted unknowns,
  remaining gaps and relabels (each list capped at 1,000, counts complete).
- **Limits.** `release::MAX_RECORDS` is 2,000,000 required or shipped record
  identities (whole-world pack plus the authored pack, with headroom).
- **Several packs.** Release evidence may list several packs (for example the
  authored slice pack and the world pack); their records combine and a duplicate
  identity is refused. The desktop composes stored v2 revisions of one scope into
  one revision with `SnapshotApplication::activate_content_set`.
- **Inferred travel.** The travel planner excludes `unverified` edges unless the
  caller lists them in `RouteOptions::inferred_edges` (`travel::inferred_edges`
  selects non-stateful edges whose evidence is all conflict-free `inferred`).
  The desktop does so only for the revision activated with
  `accept_inferred_travel = true`; costs must still be known.

## Content limits

The whole-world content v2 pack built from these shards, and the content-v2 limits
raised for it (ADR 0007 §7), are recorded in [WORLD_CONTENT.md](WORLD_CONTENT.md).


## Stable dungeon metadata extension

World-data schema 1 adds optional dungeon `entrance_estimate` and
`excluded_reason` fields. Existing shards remain readable and preserve their
bytes when these fields are absent. `entrance_estimate` is a planar world
coordinate (`map_id`, `x`, `y`) plus mandatory derivation and uncertainty text;
there is deliberately no `z`. Coordinates must be finite. The CMaNGOS generator
v2 retains ghost-entry XY as estimates, never claims they establish a live portal
or entry access, and records map169's unused/test exclusion from zero source
capacity/levels/ghost fields. Exact source portal points remain separate.
See [world content](WORLD_CONTENT.md#stable-dungeon-expansion-converter-v3) for
projection preference, preparation heuristics and the consumer path.
