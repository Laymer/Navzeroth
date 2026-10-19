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
| `target` | `edition` (`wow_forever`) and `build`, unknown until the 1.0.0 target is frozen (ADR 0006 §1) |
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
  (or `disputed_or_stale`). Classic Era and 1.12 data stays there until it is
  validated against the frozen Forever target.
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

## Content limits

The whole-world content v2 pack built from these shards, and the content-v2 limits
raised for it (ADR 0007 §7), are recorded in [WORLD_CONTENT.md](WORLD_CONTENT.md).
