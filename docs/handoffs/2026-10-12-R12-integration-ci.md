# R12 integration CI and browser checks

Owner: Codex root; single shared-file writer. Claimed 2026-10-12.
Baseline: 91fd7c5 (integration/wave2-foundations combining R1-R10 prerequisites).
Branch: feature/wave2-r12; stacked base integration/wave2-foundations.
Scope: explicit fixture/content/Lua CI checks and mocked command browser coverage.
No main or PR merge; native window/live checks remain unverified.
Checks pending implementation.

Combined prerequisite revision: 258172b. Root resolved the R3/R10 AtlasScreen
merge by retaining both external watcher refreshKey and manual atlas refresh.
No other implementation conflict. Source review covered R1 timing disposition,
R4 invalidation/NA correction, R7 identity/sealing/conflicts, R9 analytics and
R11 bounded synthetic compiler/fixed-point Lua arithmetic. Peer review of R5/R6
and R10 found missing regressions, stale captured balance and refresh recovery;
the owning task branches contain corrections and recorded checks.

Implemented explicit Rust CI steps for 73 contract/33 transport fixture checks,
reference-content validation and Lua predicate/gear parity; added Linux Chromium
browser job. Pinned @playwright/test 1.64.0 and lockfile; isolated unit-test glob
prevents Playwright specs being executed by Vitest. Four browser cases exercise
malformed import recovery, duplicate/older state, opaque profile selection,
selected-file/watch controls, dense atlas filtering/keyboard zoom/pan/refresh,
and backend age requery while a stored view remains open. Native command adapter
is mocked through its existing invoke seam in tests only; no product test hooks.

Actual combined local checks: fmt and locked workspace all-target Clippy excluding
azeroth-desktop passed; workspace tests 463 passed/0 failed/1 existing ignored;
explicit Lua predicate/gear parity passed (192 predicate cases, 32 score component
comparisons, 157 arithmetic vectors). validate-fixtures passed 73+33; reference
content passed with gaps separately retained. Desktop typecheck/lint/build and
64 unit tests passed; four Playwright Chromium cases passed. Frozen pnpm install
passed; contracts:types/check 73 passed; export-schemas unchanged; four committed
LFS pointers checked in a temporary git cat-file copy. diff --check passed.

Browser development corrections: JSON fixture import attributes/path, StrictMode
initial query count, and pan assertion along the fixture's overflowing vertical
axis. A bounded horizontal pan correctly has no effect when the image is narrower
than the viewport. Browser/server execution required approved sandbox escalation.
Live WoW, native window interaction and packaging remain UNVERIFIED. Hosted CI
pending this task's draft PR/workflow dispatch; final integration record will
supply checked code head/run and all task PR links. Next: real client evidence,
party snapshot persistence and planner/export UI integration.

Coordinator final verification: hosted [run 38310280809](https://github.com/Laymer/Navzeroth/actions/runs/38310280809) at a08f5d7 completed SUCCESS; all eight jobs passed, including native Windows build/tests and mocked Chromium. Final integration documentation records the complete PR stack. No native interaction/live/packaging claim.

Merge follow-up 2026-10-13: local merge of protected main 68aae9d resolved the
combined branch ancestry cleanly, retaining the PR #33 UTC range correction.
Affected desktop typecheck/lint/build, 65 unit tests, four Chromium workflows,
Rust formatting and diff checks passed. Fresh hosted CI follows before merge.

Merge gate verified: synchronized 7e90dc1 passed [run 38318614268](https://github.com/Laymer/Navzeroth/actions/runs/38318614268), all eight jobs, before main merge f2f64a3.
