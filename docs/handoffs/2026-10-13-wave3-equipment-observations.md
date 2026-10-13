# Task 9 — conservative equipment capture and position diagnostics

Owner: Codex Worker A (/root/addon), sole writer. Baseline 8ffd2c6, task6 PR52;
includes task2 PR48 and coordinator PR47. Branch feature/wave3-equipment-observations;
worktree /tmp/navzeroth-wave3-equipment. Separate claim recorded before implementation.

## Delivered

Manual capture probes GetInventoryItemLink('player', slot) for explicit numeric
slots 1..19. Keys inventory-slot-N describe API inputs, not verified authored slot
semantics. Readable bounded links produce canonical item IDs and capture metadata.
Variants, instances and modifiers remain Unknown. Missing links never mean Empty;
exceptions/malformed links remain Unknown and missing API yields Unsupported slots.
Fixed reason counts persist separately after successful capture. Raw item links,
labels and client errors are not exported. This does not establish a complete Gear
baseline: variants and semantic slot correspondence remain explicit gaps.

/anavdiag position separately guards recorded 70338 and tests C_Map readability.
Even readable zero coordinates leave snapshot location Unknown. No map/floor/
instance/axis/transform identity is invented, and no coordinate values are stored.
The explicit command preserves existing exported snapshots. No background reads.

## Actual checks

- Locked Lua harness: 92 tests passed, zero failures, including three new equipment/
  position regressions and actual Lua snapshot bytes imported by SnapshotApplication.
- Scoped all-target Lua-harness Clippy and cargo fmt --all --check passed.
- Lua 5.1 scratch luacheck: 15 files, zero warnings/errors.
- git diff --check passed.

## Limits and next task

All new bindings are experimental. Actual 70338 inventory links, API slot scope,
link serialization, asynchronous loading, position shape and combat/restricted
behavior remain UNVERIFIED. Missing link may mean pending/unreadable/empty; no such
meaning is inferred. Diagnostic readability never means a faithful position.
No semantic slot map or variant interpretation is supplied, so live baseline
configuration matching can remain unresolved as required.

Next: coordinator integration/live checklist and candidate packaging after all
previous PRs. Obtain sanitized recorded-client evidence for quests, equipment,
position diagnostics, plan paste/HUD and session reload/export/import. Real authored
content, validated stage mappings and full live acceptance remain later work.

Integration follow-up: merged task6 cached-observation fix734b2e4 preserving both
README additions (only textual conflict). Own worktree target checks establish the
correct compile-time addon root; final harness93 passed, scoped Clippy/fmt,15-file
Lua lint and post-resolution diff-check passed. Cached persisted equipment stays
prior observation with original age/unknown variant states, never a new live read.
