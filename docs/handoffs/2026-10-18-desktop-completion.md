# Continuation 8 — desktop workflow and installer completion

Owner/sole writer Codex `/root`; baseline `896539e`, branch
`feat/continuation-desktop-completion`. Claim:
[desktop completion](../claims/2026-10-18-desktop-completion.md).
Engineering is implemented in PR #198; exact-head hosted checks and merge remain pending.

Implemented additive private index schema 3, versioned CAS workflow preferences,
first-run choices, selected locale/resource persistence and opaque selected input
paths (32 retained selections, explicit forget frees capacity without deleting
artifacts). Extraction request protocol 3 binds explicit known installed locales;
old requests retain their canonical encoding/readers. CLI accepts sorted unique
`--locales`; desktop requires an explicit selection before new extraction.

Opt-in detection reads only selected `.build.info`, polling every 500 ms with
750 ms stable bytes/size/mtime and three staging attempts. Unix also binds
file identity; Windows file-ID replacement detection remains unverified. Cancellation
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

Explicit delivery preparation now binds selected media/input revisions and optional
research guide facts with version-1 choices. Even empty dependency lists use the
selected resource ceiling; the old API remains for historical callers. Artifact
history pages list candidates separately from integrity, and retained reference
selection verifies the current catalog before inspection/diff/activation. Exact
nine-domain inventory supports selected original authored packs and keeps required
counts unknown independently of imports. Selected private guide replay is inert,
with at most 200 findings per replay-hash-bound page; continuation requires a valid
matching digest and refuses changed guides/facts. Tooltip rendering exposes gaps
and does not execute markup. First-run saved workflows reopen on subsequent
launches; Settings can change the next-launch choice without performing data work.
The installer explicitly stays English; its existing addon prompts default to No.
[User workflow/contracts](../data/LOCAL_DESKTOP_WORKFLOW.md).

Actual checks on the implemented source:

- Full Rust workspace excluding native desktop: 1,311 passed, four ignored,
  181 result suites. Workspace Clippy with warnings denied passed.
- Desktop: 411 unit tests in 53 files, typecheck, lint and production build passed.
- Six focused Chromium checks passed: populated synthetic choices at 320/1100 px,
  200% text, keyboard focus and axe; four production CSP/font/navigation checks.
- Native Windows PowerShell 5.1 addon-installer harness passed all 30 synthetic
  install/update/removal/WhatIf/refusal checks, including running-game protection.
- Native Linux desktop check was attempted but stopped in libdbus-sys because
  dbus-1 development metadata is missing. Native desktop/ordinary-Windows
  installation and live-client acceptance are not claimed.

Two independent read-only reviewers confirmed fixes for delayed preferences
overwriting edits, same-installation locale desynchronization, legacy-preset
watcher restarts, stale opt-out reconciliation, material capacity recovery,
projection-only resource-preset bypass and missing continuation replay tokens.
Both reviewed media/binding/inventory/guide boundaries without remaining actionable
findings. Final review also fixed startup remounts reapplying next-launch navigation:
restoration is now consumed once per App/client, with an error/recovery regression.
That regression was mutation-checked by temporarily restoring unguarded navigation;
it failed as expected, and all 411 tests passed after restoring the fix. Windows
file-ID replacement detection was explicitly narrowed to unverified; no guarantee
beyond stable bytes/size/mtime is claimed there. A real browser test exposed an incorrect first-run destination (Content
instead of Compatibility and coverage); corrected and the final six tests passed.
The native desktop exact-head hosted checks remain authoritative for its adapter.

Continuation 7 merged at `8da8301`; its source/packaging/check receipts are in its
handoff. Integrate resulting main before final task-8 review/merge. Next independent
task: reproducible performance slices, notices/retained-source closure and release
materials (continuation 9), then the substantial per-file coverage PR, version/notes,
verified development RC and Pages in their separate PRs. No continuation release
has been published. Ordinary Windows and live acceptance remain unverified; no
further mutable cache read was performed in this task. This work does not clear
content completeness, availability, signing, launch-target or stable-release gates.

Hosted checks on `d34b417` caught TypeScript test-fixture errors before native
compilation: unsupported Testing Library `exact` role options and a cross-runtime
timer-mock return type. The recovery regression now drives the existing bounded
database checking-to-ready poll instead of mocking timers, preserves the same
behavior assertion, and rejects the unguarded-navigation mutation. Typecheck,
lint, all ten App tests and the full 411-test suite passed after correction.
The exact new head must rerun every applicable hosted check; no failed head merges.


## Integration receipt

Merged PR #198 at `9d046b73b04bef548e2809533b5867f9e891620f`,
2026-10-18T13:17:04Z, reviewed head
`abc639560228d06081c8f1c3042680421a60c99d`. All nine applicable checks
passed: CI 39129854804, Windows package 39129854787, source workflow
39129854792 (both platforms and parity). Independent help correction PR #199
was merged first and is present in this exact head. Full browser suite passed;
ordinary Windows/operator acceptance and live checks remain separate/unverified.
Next: continuation 9 performance/materials, then substantial per-file coverage.
