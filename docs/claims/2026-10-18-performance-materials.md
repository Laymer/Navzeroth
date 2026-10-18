# Continuation 9 — reproducible performance and release materials

Owner/sole writer: Codex `/root`, 2026-10-18. Baseline `d34b417` (PR #198
pending exact-head checks); branch `feat/continuation-performance-materials`,
clean worktree `Navzeroth-performance`. Preserve original checkout and `.claude/`.
Integrate resulting main before final review/merge. No implementation delegation.

Read STATUS, desktop/media/delivery handoffs, historical search measurements and
implementation plan section 16 before changes. Define reproducible synthetic slice
workloads before measuring optimized source. Exercise planning/cancellation,
lookup, imports and repeated-use memory in available environments. Publish exact
source/build/fixture hashes, machine/environment, sample distributions and limits;
ordinary Windows measurements remain unverified if unavailable. Synthetic or WSL
measurements do not establish gameplay, live acceptance or broad Windows support.

Audit notices, retained source/hash closure and packaging exclusions for private
stores/exports/media/cache/keys. Use existing validation harnesses and retain their
actual results; no private input is copied. Version/notes, per-file coverage,
stability and verified development RC publication stay separate later PRs.
Independent read-only review and all applicable exact-head checks precede merge.
Record actual measurements, limits and next task in the handoff.
