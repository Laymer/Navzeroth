# R10 atlas search and filters

Owner: Codex root; claimed before implementation 2026-10-12.
Baseline: 08f6daa. Branch: feature/wave2-r10.
Scope: reference pin search/filter, keyboard list navigation and refresh preservation.
Relevant M6.1 handoff and M6 plan read. Checks pending implementation.

Implemented case-insensitive reference label/ID search, point-kind filtering,
filtered map/list counts, hidden-selection explanation and atlas refresh.
Existing road/flight layer toggles retained. Pin-list ArrowUp/Down/Home/End moves
focus; native button activation selects. Selection/search/layers survive a
successful same-image refresh. Reference geography stays separate from unlocks.
Dense synthetic 200-pin regression checks filtering, map/list consistency,
keyboard navigation, empty results and refresh preservation.

Actual checks: desktop typecheck/lint/build passed; unit tests 57 passed;
Rust fmt and locked workspace Clippy/tests excluding desktop passed (one existing
ignored generator); contracts:types/check passed for 73 fixtures, no schema drift;
git diff --check passed. Initial strict TS errors fixed (PinKind tagged shape and
bounded indexes). Native keyboard/window and live WoW remain unverified.
Hosted CI pending draft PR. Next: R12 browser navigation checks.

## Correction claim
Ownership transferred to Codex worker B by coordinator on 2026-10-12.
Correction baseline: cf0a381, feature/wave2-r10. Scope: retain ready atlas after
refresh errors, prove completed refresh, keyboard wrap checks, and distinguish
removed selections from filtering. No manifests, dependencies or shared docs.

## Focused correction delivered
A refresh failure now displays a separate recoverable error while retaining
the previous ready atlas and its component state. Search, point type, selected
pin and layer toggles survive both the failure and retry. A selected pin removed
from a refreshed dataset is explicitly distinguished from a filtered-out pin.
The dense fixture test awaits a changed displayed dataset revision before
asserting selection/search persistence and covers ArrowUp/Down wrapping in
addition to Home/End. New tests cover failed-refresh/retry preservation and
removed-selection wording.

Actual correction checks: desktop typecheck/lint/build passed; seven unit-test
files, 59 tests passed; `cargo fmt --all --check` and `git diff --check` passed.
No Rust implementation, contract, dependency or lockfile changed in this
correction, so the existing package's Rust/contract checks were not repeated.
Initial implementation cf0a381 hosted run passed all seven jobs on PR 29.
Updated-head hosted verification follows below when complete. Native interaction
and live client remain unverified. Next: merge the R3 refreshKey dependency into
the integration branch, then R12 browser refresh/failure checks.
