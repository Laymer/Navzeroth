# Continuation 2 — durable extraction lifecycle

Owner and sole implementation writer: Codex `/root`.
Baseline: `167a4545edb0166da3ac81aaac51c002a9e1172a` (origin/main).
Branch: `feat/continuation-extraction-lifecycle`; isolated worktree `Navzeroth-lifecycle`.
Original checkout, existing worktrees and `.claude/` preserved.

Scope: versioned private job/checkpoint and selection/preset persistence, exact
source/profile-bound resume, status/cancellation, immutable revision staging and
transactional explicit research activation/rollback. CLI and typed desktop
commands use the same orchestration and validation. Activation never establishes
playable availability or release acceptance.

Expected files: core lifecycle contracts; storage private lifecycle index;
local-data extraction/worker integration; app service; CLI and Tauri adapters;
focused tests and workflow documentation.
Checks: format, focused Rust lifecycle/storage/worker/CLI/app tests, affected TS
checks, workspace Clippy/tests, independent read-only review and exact-head CI.
Missing live/ordinary-Windows checks remain unverified.
