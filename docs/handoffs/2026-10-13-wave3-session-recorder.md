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
