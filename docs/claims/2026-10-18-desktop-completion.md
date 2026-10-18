# Continuation 8 — desktop workflow and installer completion

Owner/sole writer: Codex `/root`; 2026-10-18. Baseline `896539e`, guide/inventory
PR #197 pending. Integrate resulting main before final review/merge.
Branch/worktree: `feat/continuation-desktop-completion`, `Navzeroth-desktop`.
Read STATUS and guide/delivery/media/lifecycle handoffs; preserve original checkout
and `.claude/`. No implementation delegation.

Complete explicit installation/product/locale selection, durable choices,
progress/inventory/conflicts and selected input/media/guide workflows. Add opt-in
change detection using bounded polling/debounce, staging notifications or research
candidates without automatic activation. Finish installer/first-run choices while
keeping desktop UI English and explicitly selected installed content locales.

Preserve opaque selections, revision-bound pages <=200, stale-response guards and
private paths/assets. Mutable cache reads still require native closed-game guard;
no assumption that the game remains closed from the previous operator window.
Tests cover polling/debounce and opt-in persistence, locale/resume fingerprints,
selection/progress/conflicts/recovery, first-run/installer behavior and accessibility
as applicable. Independent read-only review and exact-head checks before merge;
ordinary Windows/interactive operator evidence stays unverified if unavailable.
