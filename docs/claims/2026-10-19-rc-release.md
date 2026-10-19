# Development RC preparation and artifact verification

Owner and sole writer: Codex `/root`. Isolated worktree:
`/tmp/navzeroth-rc-release`, branch `release/1.0.0-rc.1`.
Initial tooling baseline: `c8577da03ace3c101c682dce75d9d3238dd2e9bf`.

Scope: draft/public download verification tooling, then version and release notes,
committed-tree readiness/material receipts, and exact-source release validation.
Lifecycle PR #204 and SemVer PR #206 must merge before version preparation and
opening the release PR. Integrate their resulting main before those changes.
Independent read-only review is required. Preserve all ten required checks and
strict per-file coverage. No acceptance-candidate or stable promotion.

The original checkout, existing worktrees, `.claude/` and private inputs remain
untouched. Ordinary Windows performance and live acceptance remain unverified.
