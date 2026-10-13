# Diagnostic addon: SavedVariables layout and export format

Task M0.3. **Every layout, key, file name and write-timing statement here is
UNVERIFIED until M0.5** runs on a user-operated Forever client. This document is
normative for the SavedVariables global name `AzerothNavigatorDB` and its keys
(ADR 0003); the M2.3 SavedVariables parser takes the global name and the export key
as parameters and must not hard-code them. The keys are reconciled again in wave 2
(M2.1).

The diagnostic export is **not an ANAV1 envelope and not a contract kind**. The ANAV1
text grammar belongs to the transport package (`schemas/v1/README.md`, M2.2) and is
not defined here. The export has no `kind`, `schema_version` or producer fields and
is not validated by the contract schemas.

## Global and keys

The TOC directive `## SavedVariables: AzerothNavigatorDB` declares one global table.

| Key | Type | Written by | Meaning |
| --- | --- | --- | --- |
| `db_version` | integer, currently `1` | Settings | Layout version. A different value makes the addon leave the table untouched and skip storing an export. |
| `settings` | table | Settings | Only the keys below. Unknown or mistyped saved keys are dropped on load. |
| `settings.auto_probe` | boolean, default `true` | Settings | Run the probe on `PLAYER_LOGIN`. |
| `settings.chat_summary` | boolean, default `true` | Settings | Print one summary line per probe. |
| `diagnostic_export` | string | Export | One JSON document (below), at most 12288 bytes. Absent until a probe has stored one. |
| `diagnostic_export_error` | string | Export | Error code of the last failed store (closed list below). Cleared by the next success. |

Rules:

- The settings merge never touches `diagnostic_export` or `diagnostic_export_error`.
- A failed store never writes a partial string and keeps the previous
  `diagnostic_export` unchanged.
- If `AzerothNavigatorDB` exists but is not a table, or has another `db_version`, the
  addon does not overwrite it and stores nothing.
- Expected file shape (conventional WoW serialization, not observed for Forever):
  `AzerothNavigatorDB = { ["db_version"] = 1, ["diagnostic_export"] = "...", ... }`.
  A synthetic sample is `fixtures/addon/savedvariables-sample.synthetic.lua`. The file
  is a Lua literal and is **data**: parse it without executing it (plan section 7.2).
  This repository's only Lua interpreter use is the dev-only test harness, which runs
  the addon's own sources and never reads SavedVariables.

## Export document

A JSON object with these members (keys appear sorted, integers only):

| Member | Type | Meaning |
| --- | --- | --- |
| `format` | string | Always `"anav-diagnostic"`. |
| `version` | integer | Export format version, currently `1`. |
| `addon_version` | string | Addon `## Version`. |
| `toc_interface` | integer | The addon's own TOC interface number (`16001`, unverified). |
| `build` | object | `edition`, `edition_basis`, `version`, `build`, `interface`. |
| `capabilities` | array | One record per capability, sorted by `id`. |

`build.edition` is `"wow_forever"` with `build.edition_basis` `"assumed_target"`: the
probe cannot read the edition, so this is the addon's target, not an observation.
`build.version`, `build.build` and `build.interface` come from the client's
`GetBuildInfo` and are the string `"unknown"` when the call is missing, fails or
returns an unexpected shape. They map onto the `BuildKey` fields (`edition`, `build`,
`interface`) of `crates/core` in M2.1; how `version` and `build` combine into the
`BuildId` string is decided there.

Capability record:

| Member | Meaning |
| --- | --- |
| `id` | Stable snake_case id; equals the row id in `capability-observations.template.json`. |
| `status` | Only `"untested"` or `"failed"`. The probe never writes `verified available`, `verified unavailable` or `restricted`; those need an operator's recorded conditions. Vocabulary: [BUILD_CAPABILITIES.md](BUILD_CAPABILITIES.md). |
| `api_names` | Function names checked for existence (assumed mainline-style names, unverified). May be empty for manual-only capabilities. |
| `missing` | The subset of `api_names` that was not a function. |
| `detail_code` | `api_present`, `api_partial`, `api_missing`, `call_returned_values`, `call_unexpected_return`, `call_failed`, `lookup_failed`, `manual_check_required`, `needs_relog_verification`. |
| `detail` | Present only with `call_failed` or `lookup_failed`: a redacted excerpt of at most 96 ASCII bytes. |

Redaction is best effort: path-like and hyphenated tokens (`Name-Realm`, GUID-like),
quoted text and every byte outside a small ASCII set are replaced. It is applied only
to errors from the one call and the name lookups the probe performs.

## Encoder bounds

| Bound | Value | Error code when exceeded |
| --- | --- | --- |
| Container nesting depth | 32 (plan section 7.1; `MAX_ADDON_JSON_DEPTH`) | `json_depth` |
| Values visited | 2048 | `json_too_many_elements` |
| Entries in one array | 256 | `json_too_many_elements` |
| Keys in one object | 64 | `json_too_many_elements` |
| String or key length | 512 bytes, valid UTF-8 | `json_string_too_long`, `json_invalid_utf8` |
| Encoder output | 16384 bytes | `json_output_too_large` |
| Export document | 12288 bytes (below the 32 KiB addon text bound) | `json_output_too_large` |

Other encoder rejections: `json_type` (nil, function, userdata, thread, a table with a
metatable), `json_key` (non-string keys that do not form 1..n, mixed or sparse arrays),
`json_cycle`, `json_number_nan`, `json_number_infinite`, `json_number_not_integer`,
`json_number_out_of_range` (outside plus or minus 2^53 - 1), `json_internal`. Export
codes: `export_no_db`, `export_invalid_report`. Settings codes: `settings_no_db`,
`settings_db_version`. An empty Lua table encodes as `[]`. Floating-point numbers are
never emitted, so Rust and Lua float formatting cannot disagree.

## Data collected and not collected

Collected: the addon and TOC version, the client's reported version, build number and
interface number, and per-capability existence results. Not read at all: character or
account name, realm, GUID, guild, chat, mail, combat-log data, position, inventory,
quests, or any value the client restricts. The sources are scanned in tests for
dynamic code loading and restricted libraries.

## M2.1 additive snapshot keys (experimental)

Layout version remains 1; diagnostics/settings and older exports are preserved.

| Key | Type | Meaning |
| --- | --- | --- |
| snapshot_export | string | A separate v1 ANAV1 character snapshot, at most 32768 bytes including framing. |
| snapshot_export_error | string | Closed export failure code; a failed encode retains the previous complete snapshot. |
| snapshot_sequence | integer | Persisted logical capture counter, advanced only after successful storage; manual capture stops at 1,000,000,000. |

The manual producer uses snapshot-stream-<anonymous-profile-id> as its logical
stream identity and a monotonically increasing sequence, with no session elapsed
or UTC claims. It rejects corrupt counters and incompatible DB versions.
Snapshot export/import is separate from diagnostic_export. The desktop defaults to
AzerothNavigatorDB / snapshot_export, while custom globals/keys remain supported.
No actual snapshot persistence or real import is VERIFIED yet; the 70338 evidence
records only the user's diagnostic persistence example.
