# Private local archive research workflow

The [standalone implementation prompt](../../localdata.md) defines the finished
product. This implementation supports local CASC table extraction, bounded WDC
research decoding, private immutable catalogs and desktop inspection. It does
not establish playable content or complete 1.0.0 inventories.

## CLI

Use explicit native paths on Windows or mounted paths under Linux/WSL. Replace
angle-bracket placeholders; quote paths with spaces. Create an empty private
destination first (owner-only permissions on Unix). It must be separate from
the game installation, and neither directory may contain the other.

```sh
cargo run --locked -p azeroth-cli -- local-data inspect --installation <selected-directory> --product wow_classic_beta
cargo run --locked -p azeroth-cli -- local-data plan --installation <selected-directory> --product wow_classic_beta --destination <private-directory> --preset max
cargo run --locked -p azeroth-cli -- local-data extract --installation <selected-directory> --product wow_classic_beta --destination <private-directory> --preset max
cargo run --locked -p azeroth-cli -- local-data verify --destination <private-directory> --catalog <catalog-sha256>
cargo run --locked -p azeroth-cli -- local-data coverage --destination <private-directory> --catalog <catalog-sha256>
```

Optional `extract --tables Map,ItemSparse,SpellName,UiMap,TaxiNodes` selects an
explicit subset and records that gap. Otherwise every pinned manifest table name
is processed, including DBC-only identifiers and names without known FileDataIDs.
Names/identifiers come from reviewed metadata; no arbitrary asset signature scan
or CDN discovery occurs. Files not present locally remain missing.

Commands emit JSON. Exit0 means a successful inspection/verification report;
exit1 means refusal/failure; exit2 means unsupported command/options; exit3 means
a preliminary plan or completed research-only extraction. An extracted research
catalog never becomes gameplay-ready or active simply because decoding succeeds.
`status` takes the same destination/catalog options as `verify`. `coverage`
reports dispositions and unknown target completeness, not an imported-count
completeness percentage. Durable job control and research activation are described below. Gameplay diff,
projection/media delivery binding and receipt *preparation* remain separate work;
research activation never enables planning.

## Durable jobs, saved selections and research activation

The private store now includes `lifecycle.sqlite3`, a separate version-1 SQLite
index with transactional checkpoints and activation history. Existing catalogs
and projections keep their original formats and bytes. Unknown database, worker
and checkpoint versions refuse. No character database is migrated or shared.

```sh
cargo run --locked -p azeroth-cli -- local-data select --installation <selected-directory> --product wow_classic_beta --destination <private-directory> --preset small
cargo run --locked -p azeroth-cli -- local-data selections --destination <private-directory>
cargo run --locked -p azeroth-cli -- local-data extract --destination <private-directory> --selection <opaque-selection-id> --job <chosen-job-id>
cargo run --locked -p azeroth-cli -- local-data jobs --destination <private-directory>
cargo run --locked -p azeroth-cli -- local-data status --destination <private-directory> --job <job-id>
cargo run --locked -p azeroth-cli -- local-data cancel --destination <private-directory> --job <job-id>
cargo run --locked -p azeroth-cli -- local-data resume --destination <private-directory> --job <job-id>
cargo run --locked -p azeroth-cli -- local-data forget --destination <private-directory> --job <terminal-job-id>
cargo run --locked -p azeroth-cli -- local-data active --destination <private-directory> --catalog <catalog-sha256>
cargo run --locked -p azeroth-cli -- local-data activate --destination <private-directory> --catalog <catalog-sha256> --expected <generation>
cargo run --locked -p azeroth-cli -- local-data rollback --destination <private-directory> --catalog <previously-active-catalog-sha256> --expected <generation>
```

Direct `extract --installation … --product …` remains supported. `--job` is
optional for new extraction; otherwise the CLI generates and reports its ID.
`status --catalog …` retains its historical catalog-verification behavior.
`--selection` uses only a previously explicitly saved path/product/preset and
cannot be combined with another installation/product. Explicit `--preset`
overrides that run; saved selections are changed by `select` or desktop review.
Resume uses the saved request; it cannot change product, tables or resource preset.

Each completed table checkpoints its inventory, retained/decoded object hashes and
gaps. Resume compares the exact source, extractor/dependency profile, requested
subset, locale/table sequence and preset; it verifies every reused object before
opening the archive. Changed inputs, corrupt objects or unknown versions refuse
without reinterpreting the checkpoint. Source checks now stream SHA-256 over all
selected archive segments before and after extraction (4 GiB per-segment bound),
in addition to metadata/config/index hashes and file stamps. This performs full
archive reads and can materially increase startup/resume time. It has not yet
been measured on ordinary Windows. No source files are modified.

Desktop cancellation persists the terminal state then kills its worker. CLI
cancellation is cooperative at table boundaries; it cannot interrupt a native
read in another process. A crash is classified as interrupted only after the
private destination's writer lock proves that no worker can still commit.
Checkpoint writes and completion refuse after cancellation. Extracted objects or
an unactivated sealed catalog may remain after a failure; previous activation is
unchanged. Partial table results are never checkpointed.

There are at most 16 saved selections, 8 retained jobs and 1,024 activations per
build/locale scope. The index is capped at 64 MiB with 4 KiB pages, DELETE journal
and FULL synchronization. Store quotas conservatively reserve another 128 MiB
for the index and rollback journal, including existing file sizes. Capacity or
SQLite failures preserve committed state. `forget` deletes terminal job records
only; retained catalogs, projections and activation history are never removed.

Activation first verifies the full immutable catalog closure under the store
writer lock. The immediate SQLite transaction compares the displayed generation
and appends the new research selection atomically. Rollback requires an actually
previously active digest in the same exact product/version/build/locale scope.
Stale generations, missing/corrupt objects and cross-scope rollback refuse. No
activation changes characters, plans, playable content, release readiness or
`verified_for_planning` (which remains false). Projection/media binding and
validated gameplay consumer activation are continuation 6, not this selection.

In the desktop Local Data panel, use **Review saved extraction state** to restore
an installation and its preset or inspect a saved job. Interrupted, failed and
cancelled jobs offer exact-source resume. Terminal history can be forgotten
without deleting artifacts. A complete job offers research activation review;
choose an earlier complete job to select a rollback target. All operations use
typed native commands and opaque IDs; no arbitrary paths are sent to browser UI.

Explicit receipt steps (task V1.2) take named files only; nothing is generated,
searched for, activated or printed from a key:

```sh
cargo run --locked -p azeroth-cli -- local-data receipt-public-key --secret-key <key-file>
cargo run --locked -p azeroth-cli -- local-data receipt-sign --payload <payload.json> --policy <policy.json> --key-id <id> --secret-key <key-file>
cargo run --locked -p azeroth-cli -- local-data receipt-assemble --payload <payload.json> --operator <operator.sig> --reviewer <reviewer.sig>
cargo run --locked -p azeroth-cli -- local-data receipt-verify --receipt <receipt.json> --policy <policy.json> --candidate <tested-commit> --production <sha256>
```

A key file is 64 lowercase hex characters (32 bytes), optionally followed by a
newline; keep it private and outside the repository. `receipt-assemble` prints the
canonical compact receipt, which is the only encoding `verify` accepts.

## Research-only projections (task V2.2)

`project` turns one verified catalog into typed records for one declared locale;
`projection-verify` checks a sealed projection:

```sh
cargo run --locked -p azeroth-cli -- local-data project --destination <private-directory> --catalog <catalog-sha256> --locale enUS
cargo run --locked -p azeroth-cli -- local-data projection-verify --destination <private-directory> --projection <projection-sha256>
```

`project` verifies the catalog closure, reads the hash-verified decoded objects
and seals the canonical projection bytes at `projections/<sha256>` in the same
private store, under its writer lock (a concurrent extraction or projection
refuses). It refuses, without creating anything, when the catalog is not
already in the store. Its storage quota is the Max preset's, whatever preset
produced the catalog. It prints a summary (digest,
catalog digest, build, locale, per-domain counts and source completeness, gaps)
and exits3: the projection is research-only, `activated` and
`verified_for_planning` are always false, and nothing consumes it yet.
`projection-verify` checks the projection hash, canonical encoding, structural
invariants, the source catalog closure and each bound decoded object
(`integrity_verified`). When the sealed projector profile equals the current
one it re-projects and requires identical bytes: `reproduction: "reproduced"`,
`verified: true`, exit0. A projection sealed by a different projector profile
is integrity-checked only: `reproduction: "skipped_profile_mismatch"`,
`verified: false`, exit4; re-run `project` with the current binary to verify.
Any refusal (including a same-profile projection that does not re-derive)
exits1.

The projector profile hash covers only the projector code
(`crates/local-data/src/projection.rs` and
`crates/core/src/local_data_projection.rs`, which holds the record types,
validation, slot mapping and `PROJECTION_PROFILE`), not the rest of the
extractor, so unrelated extractor edits don't invalidate projections.
`PROJECTION_PROFILE` and the slot-mapping identifier are bumped whenever the
mapping's meaning changes.

Domains and sources (each pinned to the one layout WoWDBDefs lists for
70374/70425/70450, which is the same layout block for all three builds; any
other layout is a `projection_layout_unsupported` gap; no 70374 or 70450
catalog has been projected yet):

| Domain | Tables | Records |
| --- | --- | --- |
| equipment | ItemSparse, Item | item identity, name, quality, item level, required level, inventory type, candidate slots, class/subclass, class/race masks, required skill/rank, bonding, raw stat-budget slots |
| geography | Map, UiMap, UiMapAssignment | map, UI map and assignment identities; parents; raw UI/world coordinate frames |
| travel | TaxiNodes, TaxiPath, TaxiPathNode | flight nodes; directed paths with cost and ordered waypoints |
| class_roles | ChrClasses, ChrRaces | class and race identities and their input facts |

Every value is a `Fact`: `known` (zero is `known` 0), `unknown` (the decoder
reported it unknown), `missing` (field or joined row absent, including every
fact from a source table that is a whole-table gap; `sources` says which) or
`unsupported` (type/range refused, or ItemSparse and Item disagree). ItemSparse
is authoritative for inventory type; Item only fills a value ItemSparse lacks,
never an unknown or refused one. Tables that are absent,
encrypted, undecoded or in an unsupported layout appear in `sources` as gaps
with the catalog disposition and reason; encrypted sections inside decoded
tables are gaps too, and either makes the domain's `sources_complete` false.
Join and reference problems (items in only one table, unmapped inventory
types, paths with unresolved endpoints or unordered waypoints, waypoints for
unknown paths) are counted gaps. Slots are authored candidates
(`navzeroth-inventory-type-v1`) without proficiency or dual-wield rules; stat
amounts, coordinate-frame interpretation, unlocks/costs/waits and role models
are standing gaps.

The projection binds the catalog digest, product, version, build, locale,
catalog extractor/metadata/profile hashes and the projector version/profile
hash. Record order is canonical (by identity), so the bytes and digest do not
depend on stored row order; duplicate identities refuse. The budget is1million
decoded rows across projected sources (waypoints included) and the encoding is
bounded at256MiB. Activation, rollback,
cache/hotfix import, desktop display and planner consumption remain open.

## Worker, storage and formats

CLI and desktop launch their own executable's internal version1 worker entry
with bounded inherited pipes. No shell, listening socket, addon bridge or game
process access is involved. Requests/events carry version and request/job IDs;
unknown versions and oversized/truncated messages refuse. Windows attaches a
memory-limited, one-process Job Object before sending a request; Linux applies
RLIMIT_AS before archive opening and ends workers if their parent dies. Address
space/committed memory ceilings are not resident-memory promises. Concurrency is
currently1 for every preset; preset worker numbers remain upper bounds. Native
calls are serialized, handles RAII-owned, and unsafe code is confined to the
archive and OS boundary modules.

CascLib is statically built from retained pinned local sources, with no CMake or
build-script downloads. Unicode paths are enabled on Windows. The fixed wrapper
opens only the selected local product, verifies its reported build, selects
locally declared locales separately and enables strict logical-file checksums.
Download progress callbacks cancel. Build overlays additionally reject remote/
master/partial providers, force read-only local streams and reject native file/
directory creation and writes. No encrypted zero-fill option or key-management
interface is exposed. All retained originals and the small Rust DBD parser patch
have hashes, provenance and [third-party notices](../../crates/local-data/THIRD_PARTY.md).

### Archive read failures

A strict logical read either returns the whole verified file or one cause. CascLib
can stop part-way through a file and still report success with a short count; the
wrapper then reports the error CascLib recorded for the frame that stopped it,
not a generic failure. The partial buffer is discarded, never retained or
zero-filled. Catalog disposition and reason per cause:

| Cause | Disposition | Reason |
|---|---|---|
| No local file/name entry | `not_locally_present` | `not_locally_present` |
| End-of-data or missing file part while opening the file (BLTE header reads; open stage only) | `not_locally_present` | `archive_span_incomplete` |
| First frame encrypted with a key that is not available | `unavailable_encrypted` | `unavailable_encrypted` |
| Verified frames read, then a frame encrypted with an unavailable key | handled by the [partial read](#partial-reads-of-partially-encrypted-files-task-v22d) | (`unavailable_encrypted_frames` before V2.2d) |
| Partial read: header, field, pallet, common or section-directory bytes in an encrypted frame | `unavailable_encrypted` | `unavailable_encrypted_frames_in_table_metadata` |
| Partial read not decoded for another reason | that reason's disposition | `<reason>:partial_plain_frames` |
| String pointer into an undecoded section / copy source possibly in an unavailable section | `unavailable_encrypted` | `string_in_unavailable_section` / `copy_source_in_unavailable_section` |
| Unsupported BLTE/encryption encoding | `unavailable_encrypted` | `unsupported_archive_encoding_not_retained` |
| Unrecognized native failure | `unavailable_encrypted` | `local_archive_read_unavailable` |
| Short read with no recorded cause: most likely a raw data read failure after verified frames (for example an I/O or sharing error while the game runs), not encryption | `unavailable_encrypted` (used only because there is no unknown disposition) | `archive_short_read_unexplained` |
| `ERROR_FILE_CORRUPT`: frame checksum mismatch, malformed BLTE/encryption header or failed decompression | `invalid_corrupt` | `archive_verification_failed` |
| Logical size or allocation above the table limit | `excluded_by_policy` | `logical_file_or_allocation_exceeds_capacity_not_retained` |

Truncation found while reading (rather than opening) is not reported as
incomplete: local data streams zero-fill a short span, which then fails the
strict frame MD5 and is reported as `archive_verification_failed`. That is
conservative, never silent. On Windows, status 1006 is also the OS
`ERROR_FILE_INVALID`, so a genuine OS error 1006 at open would be reported as
`archive_span_incomplete`.

"Strict checksums" means CascLib's `CASC_STRICT_DATA_CHECK`, and only that: each
BLTE frame's encoded bytes are checked against the frame table's MD5. There is no
whole-file content-key check, and the BLTE frame table itself is not checked
against the file's encoding key. Plain and raw-ZLIB spans have no frame hashes
and are not hashed. CascLib zero-pads a `Z` frame, or a raw-ZLIB span, that
inflates to less than its declared size. These limits predate V2.2c and are
unchanged.

### Never decrypt (task V2.2d)

Navzeroth never decrypts archive data, whether or not a key would be known.
build.rs patches the `OUT_DIR` copy of `CascDecrypt.cpp`; the retained original
stays hash-pinned and byte-identical. `CascLoadEncryptionKeys` loads nothing, so
CascLib's built-in key table is never added to a storage. `CascDecrypt`, which
is CascLib's only decrypt entry point (called only from `CascReadFile.cpp`),
treats every key as unavailable, whatever the key map holds. This covers every
read: the ENCODING, ROOT and other storage files read while opening, and every
whole or partial table read. The wrapper calls no key-import API.
`nav_casc_decryption_disabled` checks this at run time with a key name from
CascLib's static table (no key material). Every archive open refuses unless the
check passes, and a test asserts it. Each encrypted frame therefore stops the
strict read: a leading one makes the table `unavailable_encrypted`, later ones
take the partial read below. Before V2.2d a whole read could decrypt frames whose
keys were in the built-in table. On build 70450 none did: apart from the 106
partially encrypted tables, every table's retained bytes are identical before
and after the change.

Only `archive_verification_failed` (and the decoder's `table_hash_mismatch`)
assert corruption. Encryption and incomplete local data are gaps. Until task
V2.2c (build 70450 catalogs before it), every short read was reported as
`invalid_corrupt`/`archive_verification_failed`. On the owner's 70450 client those
106 tables were all `unavailable_encrypted_frames`: DB2 files whose trailing BLTE
frames carry encrypted sections with unavailable keys (see
[the V2.2c handoff](../handoffs/2026-10-17-v2.2c.md)).

### Partial reads of partially encrypted files (task V2.2d)

When a strict whole-file read stops at a BLTE frame encrypted with an
unavailable key, the extractor reads the file frame by frame instead:

1. The wrapper loads the frame tables through CascLib's public span-info query
   and reports each frame's logical range from CascLib's frame records.
2. For every BLTE frame it reads the encoded bytes and checks them against the
   frame table's MD5 (strict). A mismatch is `archive_verification_failed`, and
   so is a frame whose expected MD5 is all zero (CascLib would accept any bytes
   for it). A file opened without integrity checking is refused as
   `local_archive_read_unavailable` (unreachable today: every open is strict). A frame whose verified
   encoded bytes start with mode `E` is marked encrypted; it is never decoded or
   decrypted, and its bytes are released at once.
3. Rust checks that the frames cover the logical file contiguously, with at
   least one plain and one encrypted frame (otherwise `archive_verification_failed`),
   merges consecutive frames into runs, and strictly re-reads only the plain runs
   through `CascReadFile` (per-frame MD5 again).

The result holds only verified plain bytes. Encrypted runs hold no bytes: they
are never zero-filled, synthesized or retained. The decoder reads through a
bounded view that refuses any access outside one plain range. Header, field,
pallet, common data and section directory must be plain, or the table is
`unavailable_encrypted`/`unavailable_encrypted_frames_in_table_metadata`. A
section is decoded only when all of its records, strings, IDs, copies, sparse
entries and relationships are plain. Otherwise the whole section is a gap:
`encrypted_section_records:<n>` when its tact key is non-zero, or
`unavailable_frame_section_records:<n>` when it is not key-flagged but lies in
an encrypted frame. Gap counts are totals per reason and table.

References from decoded records into sections that are not decoded are never
resolved. They are not reported as corruption either:

- A string pointer into such a section's string pool refuses the table as
  `unavailable_encrypted`/`string_in_unavailable_section`.
- A copy whose source chain starts at an ID listed for an encrypted section
  (the WDC4/5 encrypted ID lists) is a gap, `encrypted_copy_source_records:<n>`,
  and is never materialized.
- Any other unresolved copy in a table with a section in unavailable frames
  (whose IDs are unknown) refuses the table as
  `unavailable_encrypted`/`copy_source_in_unavailable_section`.
- A source that exists nowhere else stays `cyclic_or_missing_copy_source`.

These rules apply to whole files too.

Such a table is `decoded` with reason `research_decoded_from_plain_frames`. Its
decoded object's gaps also name each encrypted run as
`unavailable_encrypted_frames:<first>-<last>:bytes:<offset>+<length>` (frame
indices and logical byte range). Catalog gaps carry these strings prefixed with
the table name. Projection gaps carry them prefixed with domain and table, and
the domain's `sources_complete` is false. The retained original is a
deterministic container, not the client file: magic
`NAVZEROTH-PARTIAL-ORIGINAL-V1\0`, logical length (u64), range count (u32), one
25-byte record per range (state 1 plain / 2 encrypted, offset u64, length u64,
first frame u32, frame count u32; plain ranges carry `u32::MAX` and 0), then
the plain bytes in order. `encoded_bytes` is the logical length.

A partially read table that is not decoded (table hash, definition or layout
mismatch, or a decoder refusal) also retains that container. Its catalog gaps
still name every encrypted run, and its reason gains the suffix
`:partial_plain_frames` (for example `duplicate_relationship:partial_plain_frames`).
The exception is `unavailable_encrypted_frames_in_table_metadata`, which already
says so.

The decoded-cell budget (4,000,000, raised from 2,000,000 in V2.2d) counts the
records of decoded sections plus copies. Records of skipped encrypted or
unavailable sections are never materialized and do not count. The other bounds
are unchanged: 64 MiB per logical table, 1 million rows, 64 MiB decoded text,
and 256 MiB of encoded table output.

Memory, measured on build 70450: ItemSparse (19,231 rows, 2,000,024 cells)
encodes to 84 MiB of decoded JSON, inside the 256 MiB output bound. It decodes
under the `small` preset's 1 GiB worker address-space limit, with a worker peak
resident size of about 694 MiB; a full `max` run peaked at about 707 MiB. A
decoded cell costs roughly 100 to 150 bytes in memory, counting the row map entry
and its key. A worst-case table at the full 4,000,000 cells, with a large JSON
output, may therefore exceed the `small` limit. The worker process then aborts on
allocation failure and the extraction fails without sealing a catalog. The
table is never truncated or silently accepted. Larger presets allow 2 to 8 GiB.

Before and after extraction, relevant build/config/index fingerprints and archive
file identities/sizes/times are compared. Inspection must match the captured
build-info fingerprint. Changes refuse sealing. Symlink/reparse preflight and
regular-file checks reject unsafe selections. These checks do not establish an
OS sandbox against hostile concurrent directory replacement; native read-only
restrictions and post-read consistency checks are additional layers.

Private storage contains content-addressed `objects/`, immutable `catalogs/`,
sealed research `projections/`, the private lifecycle index and an OS writer lock. Opening a store (extract
or project) now creates `projections/`; binaries before task V2.2 refuse a store
containing it, so this is a one-way upgrade. Per-installation locks also
serialize alternate destinations.
Quota accounting includes existing objects/catalogs and staging. Free space is
checked before writing; no quota is preallocated. Writes sync staging before
atomic sealing, and extraction never changes active gameplay state. Cancellation
terminates the child, releases OS locks and leaves previous catalogs intact;
unsealed staging may remain and currently needs manual review/cleanup. Checkpoint resumption is available as described above; automated cleanup remains
unimplemented. Older binaries that do not recognize `lifecycle.sqlite3` refuse an
upgraded store rather than silently discarding its lifecycle state.

Supported research readers cover WDC3/4/5 headers, bounded sections, inline/
non-inline IDs (including valid zero IDs), integer/sign/bit storage, common and
pallet data/arrays, copy chains, relationships, normal strings and supported
sparse inline strings. Exact build/layout metadata is required, with no nearest
build fallback. Unavailable encrypted sections produce explicit gaps. Unsupported
storage/layouts and failed checksum/offset/count/string validation never become
normalized gameplay facts. Limits include64MiB per logical table,1million total
rows,4million scalar/array cells (2million before V2.2d),64MiB accumulated decoded text and256MiB encoded
table output. Capacity refusal does not truncate data or replace an active catalog.

The pinned wowdbdefs-rs0.1.0 parser needed a retained local patch for u32 builds,
EOF layout retention and complete build/layout list parsing. Original changed
files are preserved. Definitions without supported syntax/layout stay unsupported.
Original metadata, licences and per-file hashes remain separate from client data.

### Admitted builds and pinned metadata

Extraction and DBD selection admit exactly `1.60.1.70374`, `1.60.1.70425` and
`1.60.1.70450` (`metadata::ADMITTED_BUILDS`). All three are development inputs:
none is the 1.0.0 release target, artifacts keep their exact build and are never
relabelled, and any other build (for example 70449 or 70451) is refused with no
nearest-build fallback. 70450 was admitted on 2026-10-17 after the owner's
`wow_classic_beta` installation updated from 70425 (task V2.2b).

WoWDBDefs is vendored at upstream `b9a63096cdd99b26967f17f9515377ae62cadfd7`
(2026-10-16) with scope "manifest plus original definitions explicitly listing
1.60.1.70374, 1.60.1.70425 or 1.60.1.70450": the manifest, README, licence and
1,161 definitions, each bound by bytes and SHA-256 in
`crates/local-data/vendor/wowdbdefs/UPSTREAM.json`. The fetch was a one-time
maintenance step; builds and tests never access the network. Re-vendoring changes
`METADATA_SHA256`, so catalogs sealed under the earlier revision no longer verify
and must be re-extracted; that is the intended identity behaviour.

Semantic catalogs omit run IDs, absolute paths, timestamps, presets and incidental
indexes. They bind raw/decoded object hashes, source fingerprints, build/locales,
metadata/native revisions and the compiled extractor/profile source hash. Private
outputs are never release resources. Verification streams object hashes rather
than loading the entire corpus in the desktop process.

## Desktop and release contracts

Compatibility and source coverage > Local Data offers an explicit native folder
picker, metadata inspection, four resource choices (Max default), review of scope/
unknown estimates, extraction progress/cancellation and verified revision-bound
inventory pages of at most200 rows. Parsing/native decoding runs in Rust; React
does not execute or normalize client scripts. Backend selections are opaque IDs.
Research-only/unavailable records cannot feed plans, gear or preparation claims.
Native interactive Windows/Linux acceptance is separate from mocked UI checks.

The private-catalog release redesign has versioned contracts and canonical,
domain-separated Ed25519 primitives. Format 1 verification requires two distinct
trusted operator/reviewer identities. Format 2 policies and receipts (ADR 0006 §6,
task V1.3) let one owner identity hold one operator and one reviewer key; the keys
stay distinct, the payload must bind `independent_review_sha256` (the hash of an
independent review record), and signatures use a separate `V2` domain tag. That
independence is weaker than two people. Verification also requires exact candidate/policy/native/profile/metadata
bindings, all nine inventory-domain proof bindings and all25 mandatory acceptance
proof bindings. Tampering, revocation, unknown/duplicate signers, unsupported
versions/modes and noncanonical receipts refuse. Hashes/counts alone do not prove
independent completeness or actual acceptance. No real receipt has been signed.

Production-fingerprinted `config/release-delivery-v1.json` currently retains
`bundled_content` with no trusted signing keys. Historical repositories without
the policy keep their original rules. Explicit unknown policy formats/modes
refuse. `local_catalog` vocabulary exists, but selecting it adds a mandatory
release blocker until private validation, receipt preparation and packaging
integration are complete. Existing bundled gates are not made permissive.

Hybrid delivery (ADR 0006 §5, task V1.2) is policy format 2 with
`delivery_mode: "hybrid"` and a `local_content` binding: the extractor version,
native reader revision, profile and metadata hashes, and `expected_catalogs`,
one semantic catalog digest per build and locale that a user's local build must
reproduce. `release-readiness` then blocks until the digests are declared (they
are frozen after launch, task V4.2) and `release/v1/local-catalog-receipt.json`
verifies against the policy, the tested source commit and the production
fingerprint, with its private catalog among the declared digests. The committed
policy is still `bundled_content`; switching it is a later, reviewed change.

Remaining engineering: cache/hotfix import, full media and tooltip evaluation,
projection/media-bound gameplay activation and consumers, row search and indexes,
installer choices, opt-in update
detection, receipt preparation and the completed private-inventory packaging/readiness
contracts. All nine complete slice inventories and actual candidate-bound
live/interactive acceptance on the pending post-launch target remain missing.
70425/70450 development extraction cannot substitute for those inputs or reviews.
