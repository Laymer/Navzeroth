# Wave4 ordered storage foundations

Owner: Codex /root, sole writer. Baseline b2e10b7; branch
feature/wave4-storage-foundations. Prerequisite: coordination PR58.

Allocated migration6 for bounded per-profile reminder state and migration7 for
explicit per-session calibration annotations. Exact session/profile foreign keys
cascade annotations on deletion; existing immutable session documents are untouched.
Worker schema proposals were checked against actual SQLite keys and bounds.

Checks: locked storage suite passed (71 tests, 0 failures); storage all-target
Clippy with -D warnings passed; fmt and diff checks passed. Populated-v5 migration
regression preserves raw session sentinel bytes, rejects over-capacity documents
and nonexistent sessions, and checks independent session/profile cascades.
Existing historical v3/v4 schema reconstruction tests now remove newer empty
feature tables and migration markers before upgrading; test-only future migration
numbers follow the actual latest schema rather than assuming migration6 is unused.

Limitations: tables provide feature storage only; they do not validate game facts
or establish live/native acceptance. Current-head hosted CI and packaging follow.
Next: integrate this prerequisite before reminder/calibration application PRs.
