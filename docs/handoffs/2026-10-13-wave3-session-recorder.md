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
