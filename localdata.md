# Local client-data extraction — standalone implementation prompt

Implement Navzeroth's independent local-data subsystem. Read this whole prompt,
AGENTS.md, docs/STATUS.md, the latest local-data handoff, initial.md and the
architecture, source aggregation, content, transport, testing and release sections
of AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md before changing code. This prompt
specifies the intended finished product; inspect the handoff/code for actual
implemented capabilities. Do not report planned capabilities as implemented.

## Purpose, ownership and baseline

Read the user's installed World of Warcraft: Forever client without modifying it.
Extract supported data into private local storage, combine explicitly selected
relevant caches and permitted addon exports, and expose validated projections to
the existing desktop workflows. Remove reliance on ForeverDB's extraction
infrastructure. Its [verification description](https://foreverdb.net/verify) is a
capability/provenance reference, not an admitted upstream dataset or code source.
Client record presence never proves playable availability or server completeness.

Use small coherent PRs. Claim one task, owner, exact baseline and isolated branch
before implementation; one writer per task, no subagents without explicit
authorization. Preserve unrelated changes. Record checks, limitations and the
next concrete task in a handoff. Review merge candidates and resulting main with
all nine required checks plus applicable source checks; cancelled/skipped jobs
do not pass. Main pushes, merges and releases need applicable user authorization.

Planning baseline: main `0e04042ec1397887df008c153aa5f0c568831e6d`, after PR153's
0.9.0 preparation. Reconciliation on 2026-10-16 found latest published v0.8.1,
baseline Windows packaging/source workflows successful and CI cancelled. Keep
0.9.0's outstanding publication procedure separate; recheck actual release state.
Version declarations alone do not establish release publication.

Authorized 1.0.0 target: **Forever 1.60.1 / build70374 / interface16001**.
Planning installation metadata reports **1.60.1.70425**, product
`wow_classic_beta`; selected caches also report70425. This is installation
evidence, not new live acceptance. Develop against available70425 inputs, stage
them independently and never relabel them70374 or reconstruct missing target data.

## Binding decisions and limits

Dedicated `azeroth-local-data` Rust workspace crate, not a submodule. Both CLI
automation and desktop integration. Rust implementation with one narrowly wrapped
native archive library permitted. Native Windows and Linux/WSL. All discoverable
client tables; normalize only implemented, supported semantics. Full available
map/icon inventory within quotas. All locally installed locales, no language
downloads. Explicit cache selection and existing permitted addon importers.
Retain selected originals in content-addressed private storage. Local-only data
by default; explicit activation and rollback; validated partial projections may
activate with explicit gaps. Unsupported inputs must not stop unrelated supported
work. Unavailable content is research-only. Stable archives may be read during
play; mutable cache import waits until the game closes. Never close it automatically.

| Preset | Worker upper limit | Whole-worker memory budget | Total storage quota |
| --- | ---: | ---: | ---: |
| Small | 1 | 1 GiB | 8 GiB |
| Balanced | 2 | 2 GiB | 32 GiB |
| Large | 4 | 4 GiB | 64 GiB |
| Max (default) | 8 | 8 GiB | 128 GiB |

Limits are ceilings, not preallocation or performance promises. Memory applies
to the whole extraction worker, not each thread. Storage counts retained originals,
normalized artifacts, media, indexes and temporary staging. Preserve gameplay
limits of250,000 identities,250,000 evidence records and256MiB encoded catalog,
plus all existing record/planning limits. Overflow refuses without truncation.

Never access process memory, inject, decode packets, automate input, recover
restricted combat information or introduce an addon socket/HTTP/live bridge.
Live observations remain in the permitted addon. Imported Lua/scripts/templates
are bounded inert data, never executed. No ForeverDB addon/uploader/extractors or
data-endpoint scraping. No Battle.net credentials/session tokens/authenticated
downloads, CDN fallback, key acquisition or protected-content recovery. Missing
and encrypted local content remain gaps. Preserve unknown, unsupported,
restricted, stale, false, zero and justified empty separately. Parser/metadata
licences do not grant client-data distribution rights. Exclude private client
tables, caches, text/media and signing keys from public releases unless separately
admitted for distribution.

## Architecture and reviewed dependencies

Keep pure contracts/validation in `azeroth-core`, orchestration in `azeroth-app`,
persistence adapters in `azeroth-storage`, CLI dispatch in `azeroth-cli`, and
presentation in Tauri/React. New crate owns installation inspection, archive
access, bounded table/cache/media decoding, immutable private artifacts, worker
progress/cancellation/resumption, fingerprints and consistency. Core must not
depend on filesystem, native, SQLite, network or Tauri. Update the release
validator's explicit internal package list and its fixtures when adding the crate.

Pin and review, retaining originals/hashes/licences/notices:

- [CascLib](https://github.com/ladislav-zezula/CascLib/tree/38a34665624b8775bb875274b36191b21c38d97b),
  revision `38a34665624b8775bb875274b36191b21c38d97b`, MIT. Static build from local
  sources; no build.rs/CMake downloads, shared library or production upstream tests.
  Windows Unicode paths. Explicit product, local opening only; no online,
  allow-download or force-download flags. Strict archive verification; reject
  download callbacks. Audit unexpected writes/network fallbacks. Private RAII
  handles; serialize native calls unless thread safety is demonstrated. Restrict
  unsafe to documented FFI. No second production archive backend initially.
- [WoWDBDefs](https://github.com/wowdev/WoWDBDefs/blob/996e53f029c0213b223aecb94d754f6b4b9efb49/LICENSE.md),
  revision `996e53f029c0213b223aecb94d754f6b4b9efb49`; distinguish share-alike
  definitions from BSD code. `wowdbdefs-rs =0.1.0` behind a bounded adapter.
  Validate actual syntax against retained definitions. Map, ItemSparse, SpellName,
  UiMap and TaxiNodes listing70374/70425 does not prove all table support.
- [DBCD](https://github.com/wowdev/DBCD/tree/e732093f8864240fc5884bd1bba6b02f3dfc0d56),
  revision `e732093f8864240fc5884bd1bba6b02f3dfc0d56`, development differential
  oracle only; no production .NET dependency.
- [`wow-blp =0.7.0`](https://docs.rs/wow-blp/0.7.0/wow_blp/), bounded byte-oriented
  media adapter after format/licence review.
- [`ed25519-dalek =2.2.0`](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/),
  reviewed receipt signing implementation.

Metadata bundle: version, immutable upstream revisions, original selected files,
licences, exact hashes, product/build/layout support, table names/FileDataIDs,
implemented transform IDs, field semantics/uncertainty/dependencies and supported
cache/media formats. Transform IDs select Rust functions, never executable code.
Select definitions by exact recorded build and verified layout, no nearest-build
fallback. Metadata updates are separately opted in, bounded/immutable/licensed,
staged through source admission; new transforms require an application release.

Run extraction outside desktop in the same executable's internal worker entry,
dispatched before normal CLI/Tauri startup. Launch without shell construction;
inherited pipes only, no listening socket. Versioned bounded messages carry
request/operation IDs, progress, cancellation and typed failures. Enforce memory,
decompression, allocation and storage budgets before allocation. Windows Job
Objects; Linux applicable process limits plus internal budget (document address
space versus resident memory). Bounded queues/native access/Rust decoding; report
actual concurrency. Cancellation exits cleanly. Crashes preserve active catalog.
Default logs omit absolute source paths and imported text.

## Inspection, extraction and source separation

Require explicit installation path/folder picker and unambiguous active product.
Remember selection privately. Read only relevant `.build.info`, product flavour,
build/config/root/index files, installed locales, relevant archive segments and
explicit caches. Never scan accounts/chat/unrelated SavedVariables/drives.
Descriptor: product, edition, version, build, locales, storage identity and
consistency. Absolute paths belong only in private config/run records.

Before extraction fingerprint relevant inputs, confirm a stable metadata interval,
open local storage, plan work/space and validate destination permissions and
source/destination separation. Verify logical checksums and source file identity,
size/time plus build/config/root/index fingerprints during/after extraction.
Refuse candidate publication on source change. Checkpoints bind identical inputs;
patched installations cannot resume an old checkpoint. Serialize writers per
installation. Queue mutable cache work during play, never control game/launcher.

Enumerate tables from reviewed metadata and storage names/FileDataIDs, not arbitrary
asset signature scans. Every candidate has a disposition: decoded; retained but
unsupported; not locally present; unavailable/encrypted; definition missing;
layout mismatch; invalid/corrupt; policy excluded. Processing all candidates never
means complete game data. Implement encountered modern WDC/cache formats with
headers/sections/storage/record IDs, inline/non-inline IDs, signed/bit fields,
common/pallet/copy/relationship/sparse records, localized string boundaries,
table/layout hashes and encrypted sections. Unknown formats may remain private
raw inputs; never normalize their records into gameplay.

Cache allowlist: reviewed hotfix DBCache.bin, quest, creature, game-object and
item caches when supported/present. Exclude combat/damage/chat/account/unrelated
caches. Verify build/locale/format/size; retain bytes and separate provenance.
Modification time is not trusted observation time. Keep base and hotfix
add/replace/delete operations separate, exact-build/table/record overlay only.
Unreadable/missing hotfix state is unknown, not no hotfixes.

Reuse explicit snapshot/session importers. Add observations only with permitted
APIs/deliberate exports; version extensions compatibly. No hidden bulk-ID queries
or automated interactions. Contextual observations do not prove all spawns,
vendor inventories, prerequisites, drop rates, universal eligibility or complete
server content; a turn-in milestone is not a full quest model.

## Private artifacts and gameplay normalization

Separate storage from the character-snapshot database: (1) content-addressed
retained originals, (2) research rows/status/provenance/indexes, (3) validated
gameplay projections. Canonical authoritative artifacts are distinct from
rebuildable SQLite indexes. Versioned manifest binds product/edition/build/locales,
extractor/source/native/metadata versions, input hashes/object refs, table/media
dispositions, transforms/profiles, base/hotfix/observation lineage, canonical output
hashes, validation/gaps and local-only classification. Exclude run IDs, absolute
paths and incidental timestamps from identity; separate private run records.
Canonical UTF-8, deterministic ordering/numbers; identical semantic hashes across
OS/presets. Atomic sealing/activation, rollback history, pinned acceptance/release
inputs excluded from automatic cleanup. Partial projections identify gaps.

Field provenance binds original build/locale, source, record/field, input hash,
definition/layout/transform revision, evidence and availability. Precedence by
fact type: valid matching hotfixes over base values; archives supply static client
facts; caches/addon observations supply only observed facts. Keep other admitted
candidates and conflicts. Never global freshest-wins or cross-build overwrite.
No positive availability evidence means research-only, no actionable routes,
acquisition recommendations or preparation claims.

Normalize all nine domains without inventing missing facts:

| Domain | Client contribution | Separately evidenced requirements |
| --- | --- | --- |
| Quests | text/IDs/objectives/supported relations | availability, branches, prerequisites, sharing/resets |
| Equipment | definitions/slots/attributes/effects/requirements | server availability, variants |
| Acquisition | explicit rewards/crafting, observations | complete drops/vendors/entitlement/rates |
| Geography | maps/floors/regions/UI relations/media | access, calibration |
| Travel | explicit taxi nodes/paths/links | unlocks/costs/waits/availability |
| Dungeons | map/instance/journal facts | full chains/loot/checklists |
| Class/roles | classes/spells/talents/proficiencies | documented models/validated recommendations |
| Party | eligibility/preparation support | workflows and actual duo acceptance |
| Reminders | conditions/supporting facts | priorities/suppression/restricted/stale behavior |

Enumerate full available map/icon assets via metadata/virtual paths/references,
not models/audio/cinematics. Bound dimensions/mips/offsets/compressed lengths and
pixels before decoding; decoder cannot open neighbours. Deterministic PNG/tiles,
originals and conversion provenance; unsupported encodings stay explicit. Prefer
tiles over huge stitched images. Hash-check media before serving via opaque IDs
in existing local asset protocol, never arbitrary paths. Keep coordinate frames,
floors/instances distinct; only calibrated/evidenced/tested transforms get overlays.

Tooltips: typed bounded allowlisted parser/evaluator, no Lua/JS/general execution.
Explicit level/attributes/applicability; missing input unknown; preserve base and
scaling values/templates/supporting records and mark derived text. Bound nesting,
references/work, detect cycles/unsupported tokens. No character defaults. Operator
checks compare representative results with actual client displays.

## Interfaces, installation and updates

CLI family (explicit inputs, JSON, documented distinct exit codes):
`local-data inspect`, `plan`, `extract`, `status`, `cancel`, `resume`, `verify`,
`diff`, `coverage`, `activate`, `rollback`, `metadata-check`, `prepare-receipt`,
`sign-receipt`, `verify-receipt`. Partial completion differs from corruption,
unsupported versions, cancellation/capacity refusal. Extraction never activates;
receipt preparation never signs. Unsupported commands must fail explicitly.

Data and Compatibility > Local Data: select installation/product; inspect;
choose preset/destination; select archives/caches/addon inputs; review estimates
and gaps; extract/progress/cancel; inspect tables/media/conflicts/domain coverage;
review diff; explicitly activate/rollback. Typed commands, opaque selection IDs,
existing job conventions, revision-bound pages <=200 rows. No React parsing or
authoritative calculations.

Windows interactive install offers four presets, Max default; first run, Settings
and CLI too. Preserve settings on upgrade; silent installs neither overwrite
selection nor extract. Preflight real free space/work. Unsuitable settings do not
prevent ordinary use. Preserve per-user install, addon consent, data/uninstall.
Client-change watching is separate opt-in from metadata checks: selected relevant
metadata only,60s fallback poll,30s debounce, one writer, cache queue while playing,
separate changed-build staging, explicit activation, no game update triggering.

## Separately reviewed 1.0.0 release-contract redesign

Version delivery policy to support explicit `local_catalog` alongside bundled
content. Ship software/reviewed parser metadata/licences, no extracted data.
Users build privately. Still require a complete independently evidenced private
target inventory across all nine domains, actual live/interactive Windows
acceptance and candidate/publication separation. Partial useful catalogs cannot
satisfy1.0.0. Hash/count imports alone are not independent completeness proof.

Version inventory/evidence/readiness contracts binding exact private inventory/
catalog hashes/counts/unit IDs, independent enumeration, full reference graph,
source/transform closure, scenario dependencies and reviewed coverage. Public
reports say private data was reviewed locally, unavailable for hosted revalidation.

Canonical bounded domain-separated Ed25519 receipts bind candidate source commit,
production fingerprint/policy hash/target build, extractor/native/profile/metadata
versions, private input closure/inventory/catalog/media hashes, full validation,
mandatory acceptance and performance workload/results hashes, distinct trusted
operator/reviewer identities and signatures. Production-fingerprinted policy
contains trusted public keys/revocations; private keys outside Git/artifacts.
Receipts cannot supply their own trust. Verify canonical encoding before crypto.
Reject tampering, unknown/revoked/duplicate/missing signatures, missing reviewer,
stale fingerprints, build/policy mismatch, incomplete inventory/results and current
candidate failures. Hosted CI verifies binding/signatures, not private raw data.
Historical formats retain original rules; unknown versions/modes fail closed.
No packaging bypass flags. Update policy, candidate/final packaging/verifiers,
runbooks and initial.md together. Keep0.9.0 procedure separate.70374 remains
blocked without actual private70374 inputs, complete evidence and bound acceptance.

## Sequence, tests and completion

1. Standalone prompt and actual baseline/release reconciliation.
2. Pure versioned contracts and reviewed metadata admission/licence closure.
3. Offline archive worker/isolation/Unicode/cancel/consistency.
4. Generic table staging/dispositions/deterministic artifacts.
5. Exact-build caches/hotfixes and separate addon observations.
6. Domain projections/tooltips and guarded consumers.
7. Full bounded private map/icon pipeline/calibrated use.
8. End-to-end CLI/desktop/presets/opt-in automation/recovery.
9. Separately reviewed local-catalog release policy/receipts/packaging.
10. Real extraction/Windows/Linux/operator checks and independent review.
11. Complete private target inventory/all acceptance/final verified1.0.0 assets.

Public tests use generated synthetic fixtures, no private client samples. Test
truncation/corruption/overflow/offset overlap/excess counts/strings/dimensions/
ratios, signed/zero/false/empty/unavailable, all supported field encodings,
unknown/ambiguous layouts, hotfix ordering, mixed builds/locales, missing local
files/network attempts, Unicode/symlinks/reparse points, changed sources,
cancel/crash/quota/resume, no installation writes/excluded personal reads.
Test cross-platform/preset/repeated determinism, checkpoint invalidation,
>1024 staging rows without truncation, old activation preserved on refusal,
pagination/planning bounds, measured declared corpus/hardware, responsive UI.
Check CLI/desktop equivalence, traceability, research action suppression, missing
facts, private asset containment, preset persistence/silent install behavior and
no private bytes/keys/account paths in packaging/reports. Adversarial release
tests reject altered closure/hashes/trust/policy/missing signatures, synthetic
substitution, permissive bundled gates, stable candidate acceptance and partial
inventory promotion. Run affected documented Rust/Lua/schema/TS/desktop/browser/
PowerShell/packaging checks; record actual results, not expectations.

Extractor completion requires all supported local inputs end to end without
external service or installation writes, explicit gaps/dispositions, safe partial
activation/rollback, full quota-bounded media, provenance/determinism/interfaces/
presets/opt-in updates and private-free public artifacts. Do not mark step10/11
complete from partial handling. Missing real data/hardware/operator/review stays
unverified; finish independent engineering without inventing server facts/rights.

Final handoff: exact baseline/result commits, pins, actual formats/unsupported
cases/checks/measurements, private artifact locations without personal paths,
coverage/missing units, source/licence decisions, receipt signatures/review state,
release blockers and next concrete task. Reconcile initial.md, STATUS and release
docs before production freeze. Both root prompt files are production-fingerprinted;
post-acceptance edits require evaluating the new candidate. No false completion.
