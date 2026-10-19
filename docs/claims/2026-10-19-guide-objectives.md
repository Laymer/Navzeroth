# V2.3 — exact objective accounting in inert guide replay

Owner/sole writer: Codex `/root/guide_validation`; 2026-10-19.
Baseline: `826aa21` (`origin/main`); branch `task/guide-validation-hardening`,
isolated worktree `/tmp/navzeroth-guide-validation`.

Read shared AGENTS.md, STATUS, ADR 0006 (including amendments), V2.3/V2.5 TODO,
implementation-plan §11.2 and the latest gameplay-facts/guides-inventory/private
inspection handoffs. Preserve unrelated worktrees and private/untracked inputs.

Repair objective completion accounting: invalid objective indexes must never
substitute for missing valid objectives at turn-in. Add adversarial coverage for
sparse, repeated, invalid, zero, unknown and maximum-size objective sets. Retain
existing warning severity, optional semantics and research/evidence boundaries.
No live-client, authored-content completeness or acceptance claim.
