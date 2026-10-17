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
completeness percentage. Durable CLI job control/resume/diff/activation and
receipt *preparation* are not implemented; those commands fail closed.

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
private store, under its writer lock. It refuses, without creating anything,
when the catalog is not already in the store. It prints a summary (digest,
catalog digest, build, locale, per-domain counts and source completeness, gaps)
and exits3: the projection is research-only, `activated` and
`verified_for_planning` are always false, and nothing consumes it yet.
`projection-verify` exits0 after checking the projection hash, canonical
encoding, structural invariants, the source catalog and each bound decoded
object. When the current projector profile matches the sealed one it also
re-projects and requires identical bytes (`reproduced: true`).

Domains and sources (each pinned to the one decoded layout the profile was
written against for 70374/70425; any other layout is a
`projection_layout_unsupported` gap):

| Domain | Tables | Records |
| --- | --- | --- |
| equipment | ItemSparse, Item | item identity, name, quality, item level, required level, inventory type, candidate slots, class/subclass, class/race masks, required skill/rank, bonding, raw stat-budget slots |
| geography | Map, UiMap, UiMapAssignment | map, UI map and assignment identities; parents; raw UI/world coordinate frames |
| travel | TaxiNodes, TaxiPath, TaxiPathNode | flight nodes; directed paths with cost and ordered waypoints |
| class_roles | ChrClasses, ChrRaces | class and race identities and their input facts |

Every value is a `Fact`: `known` (zero is `known` 0), `unknown` (the decoder
reported it unknown), `missing` (field or joined row absent) or `unsupported`
(type/range refused, or ItemSparse and Item disagree). Tables that are absent,
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
depend on stored row order. The total record budget is1million (including
waypoints) and the encoding is bounded at256MiB. Activation, rollback,
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

Before and after extraction, relevant build/config/index fingerprints and archive
file identities/sizes/times are compared. Inspection must match the captured
build-info fingerprint. Changes refuse sealing. Symlink/reparse preflight and
regular-file checks reject unsafe selections. These checks do not establish an
OS sandbox against hostile concurrent directory replacement; native read-only
restrictions and post-read consistency checks are additional layers.

Private storage contains content-addressed `objects/`, immutable `catalogs/`,
sealed research `projections/` and an OS writer lock. Per-installation locks
also serialize alternate destinations.
Quota accounting includes existing objects/catalogs and staging. Free space is
checked before writing; no quota is preallocated. Writes sync staging before
atomic sealing, and extraction never changes active gameplay state. Cancellation
terminates the child, releases OS locks and leaves previous catalogs intact;
unsealed staging may remain and currently needs manual review/cleanup. There is
no checkpoint resumption or automated cleanup yet.

Supported research readers cover WDC3/4/5 headers, bounded sections, inline/
non-inline IDs (including valid zero IDs), integer/sign/bit storage, common and
pallet data/arrays, copy chains, relationships, normal strings and supported
sparse inline strings. Exact build/layout metadata is required, with no nearest
build fallback. Unavailable encrypted sections produce explicit gaps. Unsupported
storage/layouts and failed checksum/offset/count/string validation never become
normalized gameplay facts. Limits include64MiB per logical table,1million total
rows,2million scalar/array cells,64MiB accumulated decoded text and256MiB encoded
table output. Capacity refusal does not truncate data or replace an active catalog.

The pinned wowdbdefs-rs0.1.0 parser needed a retained local patch for u32 builds,
EOF layout retention and complete build/layout list parsing. Original changed
files are preserved. Definitions without supported syntax/layout stay unsupported.
Original metadata, licences and per-file hashes remain separate from client data.

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
projection activation/rollback and consumers, row search and indexes, durable jobs/
checkpoints, persistent presets/selection and installer choices, opt-in update
detection, receipt preparation and the completed private-inventory packaging/readiness
contracts. All nine complete target inventories and actual candidate-bound
70374 live/interactive acceptance remain missing.70425 development extraction
cannot substitute for those inputs or reviews.
