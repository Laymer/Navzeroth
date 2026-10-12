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
