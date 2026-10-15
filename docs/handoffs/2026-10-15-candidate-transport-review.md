# Acceptance candidate transport review and tests

Owner/sole writer: Codex `/root/acceptance`. Claimed 2026-10-15 before code.
Baseline `853eb56`, branch `test/acceptance-candidate-transport`, isolated worktree
`/tmp/navzeroth-release-candidate-tests`. Own new transport tests and this handoff;
coordinator owns existing candidate readiness/packaging sources (read-only here).
Read status, release-materials and guided-kit handoffs, release contract and M9 /
definition-of-done requirements. No source or live game evidence is fabricated.

## Independent review and actual tests

Reviewed coordinator `853eb56` typed pending-observation/initial-binding changes,
CLI exit/report selection, unchanged integrity/source/content checks, candidate
marker, default stable refusal, tag refusal and workflow dispatch. No remaining
blocker/important finding identified. The CLI continues to inspect ignored addon
inputs before candidate readiness; the PowerShell early candidate branch does not
waive that check. Stable/tag workflows never fall back to candidate readiness.

Added `Test-AcceptanceCandidate.ps1`, with only controlled Cargo readiness/material
responses stubbed inside a temporary synthetic checkout. Actual Git source binding,
addon/material ZIP construction, checksums and independent artifact verification
execute. No complete production corpus or real source grant is simulated as truth.

Actual checks on Linux PowerShell 7: all 14 transport checks passed, covering the
positive candidate path, exact canonical marker bytes, repeated readiness checks,
no publication dispatch, independent opt-in verification, default refusal, tampered
marker despite rebuilt checksums, inventory mismatch, stripped-marker full gate,
and both tag environment variables refusing before Cargo/output mutation.
Rust focused suites also passed: 14 core release tests and 14 CLI readiness tests.
Native PowerShell 5.1/Windows/live/corpus acceptance remain unverified. Coordinator
will integrate this test and add Windows CI calls before exact-head final review.

## PR7 live-observation prerequisite audit

At `d59aa91` and coordinator `853eb56`, the 70374 evidence log records only a
user-reported build tuple and error-free diagnostics with addon 0.8.1. It explicitly
has no reply for the later source/hash-bound candidate, field readability or event
observations. `Core/Snapshot.lua:172` deliberately reads accepted/boolean completed
quest state only; objectives remain empty, turned-in unknown, and stage/history
mapping unavailable. `Diagnostics.lua:221` injects manual/cached snapshot freshness
into plan controls without authoritative dataset/ruleset dependencies.
`Core/PlanControls.lua:27` correctly reports objective mapping unavailable.
`Core/Hud.lua:20` requires positive participant-scoped objective evidence; quest-log
absence cannot replace that evidence. Position diagnostics preserve unknown map
interpretation, so they cannot establish route deviation.

`Core/Session.lua:108` and `Diagnostics.lua:701` bind manual sample only to UnitLevel
and unclassified activity; no verified route/quest milestones exist. Existing pure
advancement/session logic is not missing permission to assume observations.
No new live bindings are justified from the recorded evidence. PR7 remains blocked
by actual permitted target-client observations and PR4's authored stage/model/
dependency contracts. Next obtain source-bound sanitized diagnostics, observed
quest-stage changes and permitted event behavior, then implement only supported
mappings; do not relabel historical 70338 evidence or infer combat-restricted data.
