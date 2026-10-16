# Local archive research extraction continuation

Owner and sole writer: Codex `/root`; clean baseline
`789f39272dc00c3f72ec6881a4a69fc1383a1948`, continuation branch
`feat/local-data-extraction`, claim `96a29d0`. No subagents, main merge, release,
game action, addon update, cache/account scan or external messaging. This extends
draft PR154's foundation, not a completed extractor/1.0.0 acceptance claim.

## Implemented

- Retained CascLib38a34665624b8775bb875274b36191b21c38d97b source subset,
  originals/hashes/notices, static offline build and private serialized RAII FFI.
  Build overlays reject network/master/partial providers and native writes/
  creation; the wrapper selects local storage/product/build/locales, enables
  strict data checks and cancels download callbacks. Windows Unicode build;
  narrowly fixed upstream Linux LPDWORD portability typo. No CMake, build-time
  downloads, shared-library/upstream-test build, new keys or encrypted zero-fill.
  Out-of-scope Overwatch/OpenSSL and unused SHA1 sources are excluded.
  Retained originals keep upstream whitespace/CRLF bytes; .gitattributes exempts
  those snapshots/notices from whitespace rewriting/checks while source SHA256
  checks enforce their exact bytes. Authored implementation files remain checked.
- Retained WoWDBDefs996e53f029c0213b223aecb94d754f6b4b9efb49 manifest,
  1,161 original definitions explicitly listing70374/70425, exact file hashes,
  full CC-BY-SA/BSD licence distinction and attribution. All1,346 manifest table
  names receive dispositions:1,270 DB2 IDs,31 DBC-only IDs,45 names without known
  FileDataIDs. Missing IDs stay null. This is a table-name denominator, not all
  game data or all playable content. Selected subsets are explicit gaps.
- Retained wowdbdefs-rs0.1.0 with originals and a small documented local patch:
  u32 build numbers, final layout retention and complete comma/range parsing.
  Unpatched u16 builds cannot represent70374/70425. DBD selection requires exact
  explicit build/layout; unknown/unsupported syntax never falls back. Parser
  source hashes are checked during builds. Root vendor package is excluded from
  internal workspace ownership; repository adapter tests exercise retained data.
- Self-worker entry in CLI and desktop before normal startup, bounded version1
  inherited-pipe frames, request/job IDs, typed progress/failure/completion,
  serialized writers, parent lifetime protection, Windows memory-limited Job
  Objects and Linux RLIMIT_AS. Actual concurrency1 on every preset; memory is
  whole-worker. No shell or socket. Cancellation terminates the child safely;
  unsealed staging may remain. No durable resume/checkpoints yet.
- Bounded WDC3/4/5 research reader: header/section/layout validation, normal and
  supported sparse records, valid zero IDs, inline/non-inline IDs, signed/packed
  integers, common/pallet/array storage, copies, relationships and strings.
  Missing relationship data remains unknown. Unavailable encrypted sections
  stay gaps. Copy resolution is bounded graph traversal; array cells and decoded
  text are counted before unbounded growth. Sparse compression combinations not
  implemented remain unsupported. No private real fixture is committed.
- Content-addressed private retained/decoded objects and immutable research
  manifests, quota/free-space checks, atomic sealing, existing-object verification
  and source consistency rechecks. Semantic identity excludes paths, timestamps,
  run IDs and presets, and includes native/metadata/parser/profile bindings.
  Per-installation locks serialize even alternate destinations. Previous active
  gameplay state is never touched. Retained objects are hash-verified with bounded
  streaming during desktop inventory verification.
- CLI `extract`, `verify`, `status`, `coverage`; explicit JSON/exit codes documented
  in [the workflow](../data/LOCAL_EXTRACTION.md). Existing inspect/plan remain
  read-only, with actual free-space reporting and unknown required-space estimates.
  Every extraction is research-only, exit3, no activation. No implicit media,
  cache, addon, metadata-update or network work.
- Desktop Compatibility > Local Data: explicit folder picker, opaque selections,
  preset/review flow, progress/cancel and revision-bound <=200-row inventory pages.
  Rust owns orchestration/parsing; native decoding is outside Tauri. Research
  catalog inventories do not enter planning, gear, routes or preparation.
- Versioned delivery/receipt contracts, domain-separated canonical Ed25519
  primitives, two distinct trusted operator/reviewer identities and strict
  candidate/policy/native/profile/metadata/closure/acceptance bindings. All nine
  inventory-domain proof bindings and25 actual-method acceptance bindings are
  mandatory. Tamper/revocation/unknown/duplicate signer, missing reviewer,
  unsupported format/mode and noncanonical encoding refuse. Counts/hashes are
  bindings, not independent completeness proof. No real key or receipt generated.
- Production-fingerprinted `config/release-delivery-v1.json` retains bundled
  delivery and no trusted keys. Historical formats without this policy retain
  their old rules. Unknown policy versions/modes fail closed; local_catalog adds
  a mandatory blocker until its private validation/packaging integration exists.
  Existing content/acceptance gates are not bypassed.

## Actual verification and measurements

Foundation exact head789f392: all nine required hosted checks passed, plus Linux/
Windows source builds and platform parity. CI38874189994, source38874189942,
Windows packaging38874189920. The pull-request Draft GitHub release job skipped
because publication was not requested; it is not a required PR check. Superseded
c4f4740 runs were cancelled on new pushes; those are not counted as passes.
New continuation head needs its own hosted results; no resulting-main claim.
Implementation4c06203 passed both hosted source builds and parity. Its first
Windows Clippy run rejected a test builder mutated only under cfg(unix); made
the intended nonrecursive builder configuration explicit on both platforms.
Cancellation also treats a worker exiting between try_wait/kill as already
stopped. Fresh replacement-head checks are required; no failed/superseded check
is counted as a pass.

Actual selected-installation extraction used only archive/build/config/index
inputs for `wow_classic_beta`1.60.1.70425, declared enUS. Private artifacts and
raw run logs remain in task-owned temporary private storage outside Git; no
absolute installation/account path or raw client table is committed. Initial
all-locales selection exposed missing variant selection; corrected to per-locale
root opening. Strictly unreadable tables remain explicit gaps rather than
disabling checks. Actual TaxiNodes/UiMap files decode. Broader research run:
1,346 candidates processed,505 decoded on the measured development profile.

All four presets on that measured profile produced the identical semantic
catalog `bb906db2a72b23cfbbf7d5c65f90b73a26279046ce6929327a519c041a0f0e62`.
Warm-cache/reused-store Linux x86_64 runs on32 logical CPUs took33.954s Max,
34.224s Small,34.219s Balanced and34.197s Large; cumulative child maximum RSS
637,996KiB across the sequence. This is Linux research extraction, not the
full-corpus Windows product performance gate. Limits are not performance promises.
Final frozen-binary results and exact checked implementation commit are recorded
in the completion addendum after final verification.

Actual focused regressions cover malformed/oversize worker frames, worker failure/
cancellation preserving previous state, quota refusal/deduplicated writes, DBD
large builds/final lists, signed/common/pallet/sparse/string/copy/relationship
cases, zero IDs, >1,024 rows without truncation, long copies, array/cardinality
and offset/overflow/truncation refusal, canonical receipts and adversarial trust/
candidate changes. Desktop mocked tests cover explicit review, no activation,
opaque selection transport, bounded revision pages and path-safe errors.
Portable/full browser totals and final exact commands follow in the addendum.

Native Linux desktop compilation was attempted and is unverified because the
environment lacks DBus/GTK/WebKit development packages. Existing portable Rust,
mocked UI and CLI extraction do not substitute for native Windows/Linux operator
acceptance. Windows Job Objects/reparse/Unicode runtime behavior and DBCD
differential execution remain unverified locally; pinned DBCD source was read as
a development format oracle only, not shipped or run as a production dependency.

## Remaining work and release blockers

This is archive/table research engineering plus desktop and receipt foundations,
not completion of all requested extraction/release work. Cache/hotfix semantics,
full map/icon pipeline, tooltip evaluation, supported gameplay projections and
explicit activation/rollback, searchable row indexes, durable job/settings/
selection/checkpoint recovery, installer preset choices and opt-in updates remain
unfinished. Receipt CLI preparation/signing/verification and private inventory/
candidate/final packaging/readiness integration remain unfinished. No packaging
flag can waive those gaps. Bundled gates remain active; local_catalog cannot yet
authorize publication. No signature or independent review is fabricated.

Source consistency compares relevant fingerprints/stamps and logical checksums;
it is not an OS sandbox against a hostile process racing directory replacement.
Unsupported combinations/refusals preserve existing artifacts and publish no
gameplay projection. Rebuildable research indexes and efficient row search remain
separate from authoritative artifacts and public gameplay limits.

All nine70374 target inventories remain independently unknown and every actual
candidate-bound live/interactive check remains unverified.70425 data is staged
separately, never relabelled70374. Complete private70374 inputs, independent
inventory evidence and actual operator/reviewer acceptance remain external release
prerequisites, in addition to the unfinished engineering above.0.9.0 publication
remains a separate outstanding task; this branch publishes no release.

Next concrete task: inspect the recorded unsupported table/storage gaps with the
development oracle and add bounded cache/hotfix and private media adapters; then
implement research row indexing and validated field/domain projections without
inventing server availability. Finish durable job/settings and the separately
reviewed private-release validation/receipt/packaging contract. Require fresh
exact-head checks and independent review before any authorized protected merge.
