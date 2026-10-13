# Session recorder and Sessions screen (wave 3 task 8)

Owner: Codex Worker C (/root/sessions), sole feature writer. Claimed baseline
800cd7c (task4 draft PR51, including main90026ed); quest prerequisite2b4e912
merged before implementation. Branch feature/wave3-session-recorder; isolated
worktree /tmp/navzeroth-wave3-recorder. Worker A owns Diagnostics.lua bindings;
coordinator owns TOC/navigation/aggregates. This task sends wiring requests to
them rather than overlapping writes.

Scope: opt-in explicit start/sample/pause/resume/stop/export recorder, bounded
runtime/session-relative timing, SavedVariables export retention on failure;
desktop history/analytics screen with text and selected-file import/deletion.
No background sampling, inferred productive activities or persisted timeline.
Actual checks, decisions, limitations and next task will be recorded before PR.

## Implemented and decisions

Pure Core/Session.lua injects monotonic/UTC/read callbacks. Only explicit manual
commands sample. Each start has a fresh independent identity/counter; phases are
idle/recording/paused/stopped. Running spans default unclassified, pause spans are
gaps. Stops visibly at event/JSON bounds or monotonic reversal/read failure.
Producer caps128events and existingJSON16KiB/2048nodes (consumer32KiB/1024events).
UTC inconsistencies use5s tolerance; snapshots and recording sequences separate.
Fresh manual level reads preserve observation status. No quest-turn-in inference,
equipment-acquisition inference or productive-play category is supplied.

Export requires stopped state to avoid an intermediate export conflicting with
immutable final events under the same identity. Reload never resumes a timeline;
prior successful export survives malformed/new failed exports. Worker A supplied
Diagnostics.lua binder; coordinator supplied TOC/global declaration, navigation
and harness ingest dev-dependency/lock. Feature includes these shared changes.
Sessions screen imports text/explicit file, lists scoped history, details measured
coverage/unclassified/gaps/progression/UTC, and deletes explicitly. Superseded
async details cannot restore another profile's selection. Synthetic mocks labelled.

## Actual checks and reviews

- Five focused recorder harness tests pass: manual sample/pause/resume/stop/export,
  coverage40000unclassified+20000gap, measured0, observedlevelchange1; automatic
  stops/bounds/reversal, new/reload identity, failed export retention, no background
  sample, buildguardbeforeclockread, independent successful-start counter.
- Actual Rust-encoded session bytes load via Lua decoder/reencoder and desktop
  session loader; actual Lua-generated stopped recording imports via app/storage.
- Full lua-harness:94passed, zero failures. Affected all-target Clippy/fmt pass.
- Desktop typecheck/lint/build and80unit tests pass. Eight mocked Chromium flows
  pass, including isolated Sessions import/duplicate/analytics/detail/delete flow.
- Scratch Lua5.1 luacheck:16files, zero warnings/errors after5long lines fixed.
- Tests initially wrote synthetic globals under strict sandbox; corrected helpers
  to namespace-local test tables, then5recorder tests passed. Missing ingest devdep
  was supplied by coordinator before successful compilation.
- Read-only review task6 tip734b2e4: no new blocker/important issue;3plan_controls
  and7snapshot_export tests reproduced passing. Cached restore is bounded,
  context/source scoped, preserves original age and explicitly labels cached state.
- Read-only review task9 tip7136a17: no blocker/important issue;3equipment_capture
  tests reproduced passing; unknown variants/absent links and positionzero retained.

## Limitations and next task

All tests are pure or mocked. No live client, native window, or Windows packaging
check was performed locally. GetTime, recorder commands/level reads, restart and
SavedVariables behavior on70338 remain experimental. Recorder only captures level;
turn-ins/equipment changes need permitted explicit evidence before producing those
events. Calibration, real content and full release acceptance are deferred.
Next: coordinator reviews/integrates current-head task8, checks hosted CI and
builds candidate; consolidated live checklist validates installed revision.

Review limitation: if an external minute refresh changes refreshKey while import
is pending, generation guards may suppress that mutation's receipt/onChanged.
The immutable backend import still completes; the next periodic/manual history
refresh exposes it. No corruption or cross-profile response is introduced. This
minor feedback limitation is recorded; coordinator review did not require a fix.
After merging final cached-plan734b2e4,28focused load/plan/snapshot/recorder tests
passed. After fixture/source guard68630ae was merged,5recorder tests and affected
all-target Clippy passed again. Both task trees were left clean for coordinator
latest-main ancestry updates and shared conflict resolution.

Coordinator review follow-up: manual recorder stop emitted Logout, despite no
observed game logout. Added a focused transport regression; it failed before the
fix. Stop now emits the existing recording_paused event as recording-disabled
terminal evidence, while runtime phase remains stopped/manual_stop. Running stop
retains unclassified coverage; stop during pause preserves the paused gap. The
regression checks both states, no Logout claim, final event, zero measured play,
coverage and gap totals. Six recorder tests, affected all-target Clippy, fmt,
16-file Lua static checks (zero warnings/errors) and diff-check pass. Task4 and
shared enums/exchange schemas were untouched. Handbook clarifies the distinction.

Coordinator interop follow-up: Decode.null was cloned into an empty table, so a
valid Rust optional utc=None became [] on Lua reencoding and desktop rejected it.
Extended actual Rust→Lua→desktop regression with absent UTC/uncertain clocks and
zero UTC/elapsed values plus fixture=false; it failed before the fix. Recorder
plain-copy now omits optional null fields, preserving their SessionBatch None
semantics while false/zero remain values. Six recorder tests, all-target Clippy,
fmt, 16-file Lua static checks (zero warnings/errors) and diff-check pass. The
shared diagnostic JSON encoder and schemas are unchanged.
