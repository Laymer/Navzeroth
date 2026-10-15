# ForeverDB inert quest staging

Owner/sole writer: Codex `/root`; baseline `93c361d`; claim `ea74d45`.
Branch `feat/foreverdb-inert-staging`, isolated worktree. Builds on PR143 review.

Implemented `azeroth_ingest::foreverdb` and the explicit read-only CLI command
`stage-foreverdb --input <directory> --manifest <member-manifest>`. It reuses the
existing bounded literal parser and component-checked local source reader.
No Lua engine, process execution, downloader, write/activation path or new
dependency is added to ingestion. CLI errors are static and omit source text.

Strict 0.34.0 metadata and complete expected namespace assignments must parse.
Unknown/duplicate assignments, keys and manifest fields, executable suffixes,
unsafe paths, unexpected versions, malformed compact quest rows and mismatched
file hashes refuse the complete operation. Limits: 8MiB/file, depth32, 64KiB/string,
one million literal values per file and 100,000 quest IDs per source table.
Only three fixed filenames are read from a selected manifest; no discovery.

Private raw staging preserves nil, false, zero, empty and missing values.
Reports expose source IDs/table sizes, not quest text, and are not playable
catalogs. The archive bundle's build is separate from unknown record capture
builds; Classic rows never inherit a Forever build. Overlapping IDs are retained
in both inventories. Target required IDs remain null, rights unreviewed, target
validation unverified and publication disallowed even with a 70374 metadata label.
Manifest integrity explicitly does not authenticate the selected manifest or the
identity of a complete archive. No rights/coverage gate is bypassed.

Actual local execution verified the three selected members against the PR143
member manifest from archive SHA256
`197af97c02444077e8edd7883eeeb6328828cf9b2f06c540d5708d89894a17d0`.
It parsed 557 Forever-labelled and 3,491 Classic-derived quest records, with 373
overlapping IDs. These are source counts, not target completeness. The committed
result summary records table counts and report/file hashes only; full local
source files and ID report remain outside Git.

Actual checks:

- All72 ingest tests pass, including five new staging tests and three existing
  non-execution/dependency guards. Tests include cross-assignment aggregate
  limits, depth/size refusal and executable-looking literals/suffixes.
- All17 selected CLI tests pass: four new staging/boundary tests, ten Questie
  regressions and three argument/banner checks. Source and manifest symlinks,
  malformed/duplicate metadata, changed bytes and unsafe paths are exercised.
- Strict ingest/CLI all-target Clippy passes. Its first run identified the
  `as_chunks` convention; the corrected run passed. Formatting/whitespace pass.
- Source-admission worker independently reviewed implementation and source audit,
  verified all176 member pins, and found no concrete staging blocker. No broad
  tests were repeated by that reviewer. Licence wording corrections are recorded
  in the source-review continuation.

This is a partial raw adapter. Equipment/travel/model normalization, per-field
quest semantics, corresponding-source assessment, actual70374 validation and the
independent target inventory remain unavailable or unfinished. Native/live
acceptance remains unverified. The prepared publisher inquiry remains unsent;
the coordinator asked who should send it and which reply address to use.

Next: hosted checks and integration of this independent engineering, resolve the
source/export inquiry, review admissible file/field/transform scope, then extend
normalization without erasing the origin labels or claiming invented coverage.
