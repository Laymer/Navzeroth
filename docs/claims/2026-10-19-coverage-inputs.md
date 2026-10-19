# COVERAGE-INPUTS — bounded coverage evidence inputs

Owner and sole writer: Codex `/root`.
Baseline: `826aa21`; branch `fix/release-evidence-audit`;
worktree `/tmp/navzeroth-release-audit`.

Scope: `scripts/coverage/gate.mjs`, its tests, and task documentation. Audit
found inventory bytes read before their size/type check, and raw-report parent
symlinks unchecked. Make evidence reads bounded and refuse linked path components;
expand adversarial tests without changing coverage thresholds or acceptance.

Read current STATUS, RC draft verifier and performance/materials handoffs,
ADR 0006, implementation plan sections 16, 18–20. Checks: focused Node gate
suite and full coverage-tool test suite. Native Windows/live checks are outside
this pure Node scope. Other parallel writers own desktop, guide, and local-data.
