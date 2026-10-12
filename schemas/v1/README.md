# Exchange contracts, schema version 1

These files describe the documents Azeroth Navigator components exchange:
character and party snapshots, route plans, evidence bundles, reference datasets
and gear recommendation sets. The Rust definitions in `crates/core/src/contract/`
are the source of truth. Structural validity never establishes in-game truth.

| File | Origin |
| --- | --- |
| `exchange.schema.json` | Generated from Rust by `cargo run --locked -p azeroth-cli -- export-schemas --output schemas/v1` |
| `exchange.ts` | Generated from the schema by `pnpm contracts:types` |
| `../../fixtures/contracts/v1/` | Synthetic fixtures and `manifest.json` with each expected outcome |

Never edit the generated files by hand. After changing a contract type, run both
generators and commit the results. CI fails on drift: `cargo test` compares the
committed schema byte for byte, and the pnpm jobs regenerate `exchange.ts` and
run `git diff --exit-code`. `.gitattributes` pins LF endings for these files.

The `ANAV1` text transport (plan section 7.1) is defined in
[ANAV1 text transport](#anav1-text-transport) below. Its SavedVariables carrier
is documented there and is not a contract.

## Envelope

Every document is one JSON object with these members:

| Member | Required | Meaning |
| --- | --- | --- |
| `format` | yes | Always `"azeroth-navigator"`. |
| `kind` | yes | `character_snapshot`, `party_snapshot`, `route_plan`, `evidence_bundle`, `reference_dataset` or `gear_recommendations`. Must match the payload. |
| `schema_version` | yes | `1`. Any other unsigned integer is rejected as `UNSUPPORTED_SCHEMA` before the payload is read. |
| `fixture` | yes | `true` for synthetic engineering data. Production planning must reject fixture documents. |
| `producer` | yes | `component` (`addon`, `desktop`, `cli`, `test_fixture`) and `version`, the producer application's version. Optional `session_id`; optional `sequence`, which requires `session_id`. |
| `game` | yes | `edition`, `build`, `interface`, `locale`. Every payload context must use this build. |
| `namespace` | yes | `demo` or `reference`. Every payload context must use this namespace. |
| `provenance` | yes | `source` and `clock_trust` for the document; optional `created_at_utc_ms`. Absent means no creation time was recorded. |
| `dependencies` | per kind | `dataset_revision`, `ruleset_version`, `score_model_version`, `snapshots`. Absent members and an empty list mean "no such dependency". |
| `payload` | yes | The kind's record (below). |
| any other member | no | An **extension**: retained and re-serialized in key order. At most 64; keys are 1–256 bytes without control characters and never a reserved member name; each value at most 16 KiB of JSON text and 64 KiB in total, checked before parsing; numbers within ±(2^53 − 1). |

Unknown members *inside* the payload are accepted and dropped, matching the M1.2
domain records. Consumers must not rely on them surviving a round trip.

Schema, dataset revision, ruleset, score-model and application versions are
independent. Timestamps are integer UTC milliseconds (`*_utc_ms`) everywhere.
This deliberately differs from the illustrative RFC 3339 string in plan
section 6.4, so the domain and Lua keep one time format.

### Envelope rules

- A `demo` namespace document must have `fixture: true`.
- `fixture` is true exactly when `provenance.source` is `synthetic_fixture`.
  `test_fixture` producers must mark fixtures. A fixture may use the `reference`
  namespace to exercise reference code paths; it stays labeled as a fixture.
- Nested observation metadata must agree with the flag. A non-fixture document
  cannot contain any `synthetic_fixture` source. A fixture cannot contain
  `addon_snapshot`, `manual_entry` or `official_api` sources. `unrecorded` and
  `reference_data` are allowed in both.
- Fixture evidence can never claim `verified_on_build`. This holds in either
  namespace.
- A payload whose edition, build, interface or namespace differs from the
  envelope fails with `BUILD_MISMATCH`.

## Kinds

| Kind | Payload | Dependencies | Additional checks |
| --- | --- | --- | --- |
| `character_snapshot` | `CharacterSnapshot` | none | Producer and capture `session_id`/`sequence` must agree when both are present. |
| `party_snapshot` | `PartySnapshot` | none | Participants keep their own capture metadata and timestamps; they may come from different producers. |
| `route_plan` | `RoutePlan` | `dataset_revision`, `ruleset_version`, `score_model_version` and `snapshots` must equal the plan's own values and inputs | At most 100 steps, 8 alternatives per step and prerequisite/invalidation rule depth 12 (plan section 7.1). |
| `evidence_bundle` | `{game_build, evidence, providers}` | none | `game_build` key and locale equal `game`. Evidence and provider IDs are unique. Capability, provider and conflict evidence references resolve inside the bundle. |
| `reference_dataset` | `{revision, evidence, items, acquisitions, quests, map_spaces, nodes, edges, objectives, rules, map_images?, map_embeddings?}` | none | See [Reference resolution](#reference-resolution) and [Coordinates and imagery](#coordinates-and-imagery). |
| `gear_recommendations` | `{snapshot, dataset_revision, score_model_version, recommendations}` | `dataset_revision`, `score_model_version` and `snapshots: [snapshot]`; no ruleset | Every recommendation uses the set's snapshot, revision and score model. An empty list means none were produced. |

A step's alternatives are distinct objectives other than the step's own.
Party membership of a plan is the set of its input participants; no separate
party ID is carried in version 1.

A plan is declarative data. It carries no code, commands, URLs or paths; the
predicate language is the closed whitelist from plan section 6.6. Plan expiry
and revalidation triggers (`dataset_revision_changed`, `build_changed`,
`snapshot_superseded`, `ruleset_changed`) never prove that game state changed.

### Reference resolution

Within a reference dataset, each collection holds at most 1,024 records with
unique identifiers (items by edition-scoped `ItemKey`, map spaces by `MapKey`).
The following must resolve inside the same dataset, otherwise decoding fails
with `DATASET_INCOMPLETE`:

- every evidence reference, including evidence conflicts;
- acquisition items and quest rewards, against item definitions;
- quest locations and travel-node positions, against map spaces;
- travel-edge endpoints, against a node with the same ID and map key;
- objective targets: a quest plus one of that quest's stages, an acquisition
  source, or a service node.

- map images, against map spaces; control points, against a node on the
  image's map; embeddings, against both map spaces. Embedding cycles and chains
  deeper than 4 are `INVALID_INPUT`.

Not resolved here (later content work, M3): quest, objective or item references
that appear only inside predicates; objective participants; travel-graph cycles,
which are legitimate; image `source` registry IDs (docs/data/SOURCES.md); and
references to other datasets or bundles.

### Coordinates and imagery

`map_images` and `map_embeddings` are additive version 1 members (ADR 0002);
absent or empty means no imagery or embeddings are recorded, and the canonical
encoding omits them when empty. A `Position` has its origin at the map's
top-left corner with x east and y south in [0,1]; `map_id` is the uiMapID as a
decimal string. A `CoordinateTransform` applies `target = scale * normalized +
offset` per axis; image calibrations use units `pixel` and target the full
image frame. Layer `file` values are `AssetPath`s (relative, lowercase,
traversal-free) resolved only against the content root of the dataset's
namespace: `content/forever` for `reference`, `content/demo` for `demo`.
Consumers must still refuse symlinks that escape that root. A known calibration
requires evidence; its confidence is read from those evidence records
(`inferred` draws an approximate marker, `limited_sample` needs in-game control
points, `verified_on_build` needs a client record). Digests are declared, not
computed, in the contract.

## ANAV1 text transport

`azeroth-ingest` (`crates/ingest`) is normative for this grammar. The addon
encoder and the desktop importer must both follow it; the addon never defines a
second one. The envelope above is unchanged: the transport only frames it.

```text
transport = ws* "ANAV1:" document ws*
ws        = SP | HT | LF | CR
document  = one JSON exchange envelope object
```

- Encoding is UTF-8 with no byte order mark. A byte order mark, any other
  leading or trailing character (including NBSP and form feed), a second
  prefix, a lowercase or differently versioned prefix, or whitespace between the
  prefix and the JSON is rejected with `INVALID_INPUT`. CRLF and LF are both
  tolerated around the document; the encoder emits neither.
- There is exactly one encoding: the prefix directly followed by the compact
  JSON (`encode_text` writes the canonical serialization). No base64,
  compression, chunking or alternate variants exist in version 1.
- Limits are applied before any JSON is parsed: raw text at most 4 MiB plus
  1 KiB of framing slack and the document at most 4 MiB (`IMPORT_TOO_LARGE`
  above that), JSON bracket nesting at most 32 (`INVALID_INPUT`). The 4 MiB
  ceiling holds for every kind, including reference datasets, which the contract
  decoder alone would allow to reach 16 MiB. After that the document goes through
  `decode_any`, so anything after the closing brace is rejected by the JSON
  parser.
- A transport accepts a caller-chosen set of kinds. The default import options
  accept character and party snapshots only; any other known kind fails with
  `INVALID_INPUT` before its payload is decoded.
- Text, a file holding the text, and a SavedVariables string all reach one
  validation function, `import::normalize`, and produce the same result. The
  result surfaces `fixture`, `namespace`, build and producer facts so production
  callers can reject synthetic and demo data, and per-snapshot freshness computed
  from a clock the caller supplies. Without a trusted clock freshness is
  `uncertain`. Imported state is never labelled live. No identity hash, ordering
  or storage is decided here.
- Error messages are static: they never include imported content, paths or
  excerpts.
- The desktop reads text only from an explicit user action or an explicitly
  chosen file; it never polls the clipboard.

### SavedVariables carrier (layout UNVERIFIED)

The addon cannot write files; the client writes its SavedVariables file. The
parser reads that file as data and never evaluates it: no Lua engine, no `load`,
no `dofile`. It accepts top-level `Name = value` assignments whose values are
tables, strings, decimal numbers and booleans, and rejects everything that would
need evaluation: functions, calls, name references, `nil`, `..`, operators
(other than a minus attached to a number), long strings and comments, hexadecimal
numbers, duplicate keys and names, Lua keywords as names, and trailing garbage.
String escapes `\\ \" \' \a \b \f \n \r \t \v \ddd \xXX \z \u{X}` are
decoded; an invalid escape, a raw line break in a string, or a decoded string
that is not valid UTF-8 is rejected, never converted lossily. `--` line comments
are skipped. Limits: 8 MiB per file, 32 nested tables, strings up to 4 MiB plus
1 KiB, 262,144 values and 64 top-level assignments.

The extractor takes the global name and key as parameters. The assumed layout
`AzerothNavigatorDB = { ["export"] = "ANAV1:{...}" }` is an engineering
assumption owned by the addon (`docs/compatibility/ADDON_SAVEDVARIABLES.md`,
when it lands). The real serialization (escape style, key order, number formats
such as `1e+15`, comments, line endings) is UNVERIFIED until a live client run
at M0.5, and the accepted subset may need widening then.

Synthetic cases live in `../../fixtures/transport/` with their own
`manifest.json`; `cargo run --locked -p azeroth-cli -- validate-fixtures` checks
it and the contract fixtures against their declared outcomes.

## Decoding and errors

Use `azeroth_core::contract::decode_envelope::<P>` for an expected kind, or
`decode_any` to dispatch on `kind`. Decoding proceeds in this order:

1. Size. `ImportTooLarge` above 4 MiB, or 16 MiB for reference datasets.
2. Header: `format`, `schema_version`, `kind`.
3. JSON nesting depth: at most 32 for both snapshot kinds and route plans
   (plan section 7.1). Other kinds use the parser's recursion limit of 128.
4. Typed structure plus domain invariants.
5. Envelope and payload rules.

Transports must bound their reads before building the input string. Error
messages never include imported content.

| Code | Cause |
| --- | --- |
| `UNSUPPORTED_SCHEMA` | `schema_version` is not 1 |
| `IMPORT_TOO_LARGE` | Size limit exceeded |
| `BUILD_MISMATCH` | Payload context differs from the envelope build or namespace |
| `DATASET_INCOMPLETE` | A bundle or dataset reference does not resolve |
| `INVALID_INPUT` | Anything else: format, kind, structure, nesting, domain or envelope rule |

Domain violations nested inside a snapshot or plan record surface as
`INVALID_INPUT`, as with `json::decode_json`.

## What the schema cannot express

The schema is a structural approximation. Rust validation is authoritative.
Fixtures marked `semantic_invalid` pass the schema but fail in Rust:

- Identifier and text limits count UTF-8 bytes in Rust but code points in JSON
  Schema.
- Rust rejects integers written in fractional form, such as `22.0`, and Copper
  values above `u64`.
- Duplicate object keys are rejected by Rust; `JSON.parse` keeps the last one.
- Nesting depth, extension limits and every cross-field or cross-record rule
  (contexts, fixture/provenance, dependencies, plan limits, reference
  resolution, inverted ranges, uniqueness) exist only in Rust.

Generation strips schemars' non-standard integer `format` annotations such as
`int16`; explicit `minimum`/`maximum` keep the range, and default validators
can load the schema.

Generated TypeScript types omit unknown members. Copper is a decimal string:
never convert it to `number`. Other integers are within ±(2^53 − 1).

## Compatibility

Additive optional members get documented defaults and keep schema version 1.
Removing a member, making it required, or changing its meaning requires a new
schema version and directory. Consumers reject versions they do not support.

## Cross-runtime checks

`pnpm contracts:check` type-checks a strict TypeScript consumer of
`exchange.ts`, validates every fixture with Ajv against its declared outcome,
and narrows valid fixtures through exhaustive switches. Rust tests check the
same manifest, including error codes and round trips. No Lua addon exists yet,
so Lua decoding and evaluation parity remain deferred to M2. Shared field names
alone do not establish parity.
