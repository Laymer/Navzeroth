# Local client-data foundation

Owner and sole writer: Codex `/root`. Clean baseline main
`0e04042ec1397887df008c153aa5f0c568831e6d`, isolated branch
`feat/local-data-foundation` in the original worktree. Claim `77f1bf1` precedes
implementation; standalone prompt commit `5af9450`. No subagents or independent
review claimed. No main integration, release, game action or addon update.
Implementation commit: `c4f4740fed1bd731f438d7a770ec510fb5313cdf`.
Pushed the isolated feature branch and opened draft
[PR154](https://github.com/Laymer/Navzeroth/pull/154). This remains a first
implementation slice, not completion of the whole supplied plan.

## Delivered scope

Standalone root `localdata.md` preserves the intended full implementation and
binding decisions, dependency pins, privacy boundaries, complete nine-domain
target, staged delivery and verification requirements. It is a continuation
prompt, not a statement that all its features exist. `initial.md` and STATUS
identify the changed source/delivery direction while preserving current gates.
Both root prompts are included by the existing production fingerprint rule.

Added Rust workspace crate `azeroth-local-data`0.9.0 (no submodule), internal
lock entry and explicit release-version validator/fixture enumeration. Core
contains pure format1 performance presets, inspection metadata, candidate
dispositions and bounded metadata structural contracts. Adapter uses no unsafe,
network, native library, SQLite, Tauri or game-process access. CLI dispatch
supports `local-data inspect` and preliminary `local-data plan`; the rest of the
prompt's command family is explicitly unsupported. See
[implemented interface](../data/LOCAL_DATA.md) for exact commands and exit codes.

Inspection reads only explicitly selected `.build.info` (256KiB/64 rows max).
It rejects ambiguity, duplicate columns, invalid required column types, malformed
keys/build/version, missing products, nonregular/oversize files and link/reparse
paths/ancestors. It compares bytes, length and file metadata across200ms; Unix
also compares device/inode/ctime. Default JSON/errors omit absolute paths/raw
input and make archive/edition/locale-file/availability/hotfix limits explicit.
Preliminary plans refuse source/destination overlap, create no output, report
unknown estimates as null and start zero workers. Max remains the default;
whole-worker memory and total-storage ceilings are explicit. Exit3 distinguishes
a useful preliminary plan from an extraction-ready success.

Metadata verification is library-only. It requires an application-owned reviewed
bundle digest and transform allowlist, strict duplicate/unknown-field/version
rejection, exact required upstream revision, bounded unique table/build/layout
identities, definition/licence-object closure, and every retained object's exact
size/SHA256. This establishes reviewed byte binding, not licence sufficiency or
format/parser support. No production transforms or metadata have been admitted.

## Actual checks

- `cargo check --offline -p azeroth-local-data -p azeroth-cli`: passed; only
  existing cached dependencies used, no network/build downloads introduced.
- `cargo test --locked --workspace --exclude azeroth-desktop`: **1,043 passed,
  0 failed, 4 existing ignored**,150 result summaries. Includes13 new local-data
  tests (5 core,1 adapter unit,5 adapter integration,2 CLI integration) and release
  readiness/package-enumeration regressions. Full log private in temporary storage.
- Initial sandbox run stopped at existing QuestieDB `mkfifo` fixture creation
  (`Operation not permitted`). Reran the same documented suite outside the
  sandbox with explicit approval; it passed. No test weakened or skipped.
- Strict all-target workspace Clippy excluding native desktop and Rust format
  check passed. Initial Clippy found constant `chunks_exact(2)` usage; replaced
  it with `as_chunks::<2>()` and reran successfully.
- `validate-fixtures`:73 contract fixtures (10 valid/63 rejected),33 transport
  fixtures (5 valid/28 rejected) passed. `git diff --check` passed.
- Committed-source `release-readiness --root . --version 0.9.0` passed version
  enumeration with the new package and reports66 mandatory blockers,
  `stable_ready=false`, `candidate_ready=false`, zero verified evidence files,
  zero verified gameplay packs. Preview-only `publication_allowed=true` remains
  unchanged; no release was published or authorized by this result.
- New tests cover malformed/ambiguous/oversize/versioned metadata, exact build
  preservation, preset limits, unknown estimates, no activation/writes, Unicode,
  links, source mutation between reads, strict JSON, compiled transform refusal,
  licence closure references, policy/hash/size tampering and missing objects.
  All public fixtures are generated synthetic data, not client bytes.
- Operator supplied the Windows installation selection. Actual WSL invocation
  inspected only its `.build.info`: product `wow_classic_beta`, version
  `1.60.1.70425`, declared `enUS`, stable over200ms; fingerprint
  `6b722f8c008e58da23a0705afbf0259b78f4d23392f4de6e0b32e609c283aa67`.
  Actual preliminary plan with a separate owner-only temporary destination
  returned exit3, Max8 workers/8GiB/128GiB ceilings, actual workers0,
  `extraction_ready=false`, null storage estimates and explicit prerequisites.
  No client bytes, account directories, caches or personal exports were retained.
  This is metadata-only installation evidence, not archive extraction/live acceptance.

## Source/release reconciliation

Read-only GitHub checks on2026-10-16: latest published release is stillv0.8.1,
no publishedv0.9.0. Baseline source workflow38803371318 and Windows packaging
38803371406 succeeded; CI38803371336 was cancelled. Keep the0.9.0 release task
separate. No cancellation/skipped check is a pass.

Required future pins recorded as constants/prompt only: CascLib
`38a34665624b8775bb875274b36191b21c38d97b`, WoWDBDefs
`996e53f029c0213b223aecb94d754f6b4b9efb49`, development-only DBCD oracle
`e732093f8864240fc5884bd1bba6b02f3dfc0d56`; adapter versions
wowdbdefs-rs0.1.0/wow-blp0.7.0/ed25519-dalek2.2.0 remain planned.
No upstream native sources, definitions, licences or these parser/crypto packages
have been vendored/admitted or run. No ForeverDB data was used.

## Limitations and next task

Step1 is delivered. Step2 is partial: contracts/guards exist; reviewed upstream
retention, licence closure and real metadata/parser admission do not yet exist.
Steps3–11 remain unfinished engineering/evidence. There is no CASC reader, isolated
worker/OS resource enforcement, table/cache/hotfix/media/tooltip decoder, retained
private catalog/index/activation, desktop/installer/update flow, `local_catalog`
release policy or signed receipt implementation. No receipt was prepared/signed;
no operator/reviewer identity was established. Existing bundled release gates
remain enforced. Do not report extractor completion or synthetic metadata as an
admitted production catalog.

Inspection preflight is not a race-proof extraction sandbox. Windows file
identity/reparse runtime checks, native Windows execution, cross-platform
determinism, actual archives and real format parity remain unverified. No UI/Lua/
installer behavior changed; their full native/browser checks were not run locally.
All nine target inventory denominators remain unknown; zero target completeness
or new live acceptance is asserted.70425 cannot replace70374 inputs. Full private
70374 inputs, independent inventory and candidate-bound human/reviewer acceptance
remain external release prerequisites in addition to the unfinished engineering.

Next concrete task: retain/review pinned CascLib/WoWDBDefs originals and all
licences/notices; implement/admit the bounded DBD adapter with exact70374/70425
layout tests and production-owned trust policy. Then implement the local-only
native worker with handle-based consistency, OS budgets, bounded pipes,
cancellation and synthetic fault tests before exposing any extraction command.
Require all nine hosted checks and applicable source checks before merge, plus
resulting-main checks after an authorized merge. Independent review is pending.
