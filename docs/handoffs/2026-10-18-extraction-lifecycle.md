# Continuation 2 — durable extraction lifecycle

Owner and sole implementation writer: Codex `/root`; 2026-10-18.
Baseline: `167a4545edb0166da3ac81aaac51c002a9e1172a` (PR #191 merged).
Branch: `feat/continuation-extraction-lifecycle`; isolated worktree.
[Claim](../claims/2026-10-18-extraction-lifecycle.md).

## Implemented

- Pure version-1 checkpoint, job and research-activation contracts in core.
- Separate bounded private SQLite lifecycle index in storage; existing immutable
  catalogs/projections and character databases retain their formats/data.
- Durable explicit installation/product/preset selections; per-table checkpoint
  transactions; exact source/profile/subset/preset resume with reused-object checks.
- Full streaming archive-segment content hashes before/after extraction, plus
  metadata/config/index hashes and stamps; source fingerprints cannot be reused
  after a same-size segment rewrite that restores modification time.
- Status, cooperative CLI cancel, hard desktop cancel, writer-lock-based crash
  recovery and explicit terminal job forgetting. Competing/refused workers cannot
  terminalize an owned attempt. Checkpoint/completion refuse after cancellation.
- Explicit scoped research activation and historical rollback with generation CAS,
  closure validation and transactional history. No gameplay availability promotion.
- Shared CLI/app orchestration, typed Tauri/TS commands, saved-state/resume and
  research activation controls with safe error/stale-response behavior.

## Decisions and limitations

Research activation selects a catalog only. Delivery bindings and validated partial
projection consumers remain continuation 6. It never changes planning trust.
Resume is table-granular; work in the currently interrupted table is discarded.
CLI cancel cannot kill another process and takes effect at the next source-file or table boundary.
SQLite index cap is 64 MiB; quotas reserve 130 MiB for index/journal in addition to
existing sizes. Eight retained jobs, sixteen selections, 1,024 activation events
per scope; terminal job forgetting preserves artifacts and history. Full archive
hashing adds I/O; ordinary Windows workload performance is unverified.

## Actual validation (Linux/WSL)

Frozen final code: `9b0884edb7ae29457aa5f5eda99dd91ed7942f73`.
PR: https://github.com/Laymer/Navzeroth/pull/192.

- `cargo fmt --all --check`: passed.
- Workspace Clippy, all targets except desktop, `--locked`, `-D warnings`: passed.
- `cargo test --locked --workspace --exclude azeroth-desktop`: 170 suites,
  1,244 passed, zero failed, four ignored. Incremental and debug info disabled.
- Desktop typecheck, lint, unit tests: passed, 389 tests across 45 files.
- Initial quota regression test used an old eight-byte quota without the new
  lifecycle reservation; corrected its budget while retaining the same eight-byte
  object allowance. The initial SQLite-full fixture used an unsupported in-memory
  journal mode; corrected it to a real private file. Both final tests pass.
- Independent read-only reviewer `/root/review_lifecycle` found a competing-worker
  terminalization bug. Failure recording now exists only inside the owned attempt;
  the same-ID worker regression passes. Full segment hashing and a restored-mtime
  mutation test close the source-binding gap. Two stale documentation paragraphs
  were corrected; no remaining actionable code findings were reported.
- The initial unoptimized real-client attempt was stopped during its first full
  archive fingerprint. It is not a successful extraction or a performance pass.
  This exposed delayed cancellation before startup progress. Added a durable
  `preparing` state before hashing, cancel checks between source files/reused
  objects and regressions for cancellation winning checkpoint binding. Unbound
  preparation has no reusable rows. Reviewer rechecked the state transitions.
- The explicitly selected installation inspects as build 70450/enUS. On code
  `a3a9f39`, CLI cancellation during preparation persisted `cancelled` with zero
  tables in 0.011 s (acknowledgement only, not worker-stop latency); the worker
  then refused at its next source-file check.
- The optimized final-code four-table interruption/cancellation/resume-versus-fresh
  comparison runs in owner-only temporary stores, outside Git. Its subsequent
  completion receipt, exact digests, timings, cleanup outcome and resulting-main
  integration will be recorded in [PR #192](https://github.com/Laymer/Navzeroth/pull/192).
  Completion must not be inferred until that receipt is present. This link avoids
  relabelling a running check as passed in a commit written before it finishes.
- Final-code workspace checks still pass: 1,244 Rust tests, four ignored, and
  Clippy; final desktop checks pass (389 tests, typecheck/lint/production build).
- All nine hosted checks passed on `a3a9f39` (CI 39115819126, packaging
  39115819144), and source workflow 39115819215 passed. The later final-head
  activation hardening and these documentation updates require fresh hosted
  checks before merge; exact outcomes are recorded in the linked PR receipt.

Ordinary Windows performance, native interactive desktop behavior and live addon
acceptance remain unverified. Hosted exact-head checks are required before merge.

## Next task

Continuation 3: explicitly selected cache/hotfix/observation input adapters with
exact build/locale lineage, bounded reviewed formats and reliable closed-game
preconditions for mutable caches. Expanded gameplay projections follow those inputs.
