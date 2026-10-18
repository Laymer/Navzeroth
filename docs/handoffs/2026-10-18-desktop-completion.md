# Continuation 8 — desktop workflow checkpoint

Owner/sole writer Codex `/root`; baseline `896539e`, branch
`feat/continuation-desktop-completion`. Claim:
[desktop completion](../claims/2026-10-18-desktop-completion.md).
This is a partial implementation checkpoint; task 8 remains active.

Implemented additive private index schema 3, versioned CAS workflow preferences,
first-run choices, selected locale/resource persistence and opaque selected input
paths (32 retained selections, explicit forget frees capacity without deleting
artifacts). Extraction request protocol 3 binds explicit known installed locales;
old requests retain their canonical encoding/readers. CLI accepts sorted unique
`--locales`; desktop requires an explicit selection before new extraction.

Opt-in detection reads only selected `.build.info`, polling every 500 ms with
750 ms stable byte/size/mtime/file identity and three staging attempts. Cancellation
wakes the worker. Immutable private candidates preserve source/profile lineage;
acknowledgment changes the detection baseline, never extracts or activates.
Reconciliation reads current preferences under its registry lock, preventing
stale concurrent reads from undoing opt-out. Legacy preview presets cannot cause
watcher restart loops. Defaults remain off; no mutable cache watching.

Selected input UI stages explicit cache/addon files with separate lineage,
unknown/conflict counts and unsupported dispositions. Cache reads retain the
existing native closed-game guard and write-sharing refusal. Record pages are
revision-bound and at most 200. Paths stay native. Media UI discovers bounded
candidates, selects at most 128 unique IDs and invokes the isolated media worker.
Verified PNG previews use `/research/<revision>/<opaque-id>` under the existing
atlas scheme/CSP, with GET-only, no queries, no-store and nosniff. All dispositions,
read refusals, fixture labels and uncalibrated status remain visible.

Actual checks: full Rust workspace excluding native desktop passed 1,309 tests,
four ignored, 181 result suites before the final media adapter expansion. Latest
workspace Clippy with warnings denied passed. Desktop unit suite passed 403 tests
in 50 files; typecheck, lint and production build passed. Native Linux check was
attempted but stopped in libdbus-sys because dbus-1 development metadata is missing;
no native desktop success is claimed. Affected Rust lifecycle/storage/app checks
are rerun after the final adapter changes; record final result before closing.

Two independent read-only reviewers found and confirmed fixes for delayed
preferences overwriting edits, same-installation locale desynchronization,
legacy-preset watcher restarts, stale opt-out reconciliation and material capacity
recovery. Both reviewed media boundaries without further actionable findings.
Runtime/browser/native hosted checks remain pending for this task.

Next: integrate resulting main after PR #197; finish explicit delivery input/media/
guide-facts bindings, persisted revision history selection, nine-domain inventory,
selected authored packs/guide replay and tooltip presentation. Verify existing
installer opt-in choices and harnesses. Then final review and all applicable
exact-head checks. Performance/materials, substantial per-file coverage expansion,
version/notes/verified development RC and Pages remain subsequent separate PRs.
No continuation release has been published. Ordinary Windows and live acceptance
remain unverified; no further mutable cache read was performed in this task.
