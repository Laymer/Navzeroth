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
CLI cancel cannot kill another process and takes effect at the next table boundary.
SQLite index cap is 64 MiB; quotas reserve 128 MiB for index/journal in addition to
existing sizes. Eight retained jobs, sixteen selections, 1,024 activation events
per scope; terminal job forgetting preserves artifacts and history. Full archive
hashing adds I/O; ordinary Windows workload performance is unverified.

## Actual validation (Linux/WSL)

- `cargo fmt --all --check`: passed.
- Workspace Clippy, all targets except desktop, `--locked`, `-D warnings`: passed.
- `cargo test --locked --workspace --exclude azeroth-desktop`: 170 suites,
  1,241 passed, zero failed, four ignored. Incremental and debug info disabled.
- Desktop typecheck, lint, unit tests: passed, 388 tests across 45 files.
- Initial quota regression test used an old eight-byte quota without the new
  lifecycle reservation; corrected its budget while retaining the same eight-byte
  object allowance. The initial SQLite-full fixture used an unsupported in-memory
  journal mode; corrected it to a real private file. Both final tests pass.
- Independent read-only reviewer `/root/review_lifecycle` found a competing-worker
  terminalization bug. Failure recording now exists only inside the owned attempt;
  the same-ID worker regression passes. Full segment hashing and a restored-mtime
  mutation test close the source-binding gap. Two stale documentation paragraphs
  were corrected; no remaining actionable code findings were reported.
- The explicitly selected installation inspects as build 70450/enUS. A private
  real-client interruption/cancellation/resume-versus-fresh run is pending on
  frozen implementation. No live WoW acceptance is inferred.

Ordinary Windows performance, native interactive desktop behavior and live addon
acceptance remain unverified. Hosted exact-head checks are required before merge.

## Next task

Continuation 3: explicitly selected cache/hotfix/observation input adapters with
exact build/locale lineage, bounded reviewed formats and reliable closed-game
preconditions for mutable caches. Expanded gameplay projections follow those inputs.
