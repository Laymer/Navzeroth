# Session import and persistence (wave 3 task 4)

Owner: Codex Worker C (/root/sessions), sole feature writer; migration v5 exclusive.
Claimed baseline 32cec0f, coordination PR #47; branch feature/wave3-session-import;
worktree /tmp/navzeroth-wave3-sessions. Coordinator supplies shared exports and
SnapshotApplication/native/command wiring. Existing worktrees remain preserved.

Scope: bounded local session transport, literal SavedVariables import, immutable
profile/context/fixture scoped SQLite storage, migration v5, analytics services.
No exchange-schema kind or live observations are added. Actual checks, decisions,
limitations and next task will be recorded before draft PR publication.

## Implemented and decisions

Dedicated local SessionDocument/ANAVSESSION1 parser, version1, 32 KiB/depth32;
selected-file literal SavedVariables uses session_export. Existing character
profile is required; full context/fixture scope checked server-side. Migration v5
adds immutable canonical events, hash and derived analytics. Reads revalidate and
compare all three. Imports sort sequences/deduplicate identical events, retain
128 sessions/profile, reject conflicting identity/capacity atomically, expose
explicit deletion. Summary threshold 300s and UTC tolerance5s are fixed and
recorded by core methodology; no inferred activity, new schema kind or calibration.
Coordinator supplied exports, shared profile-store helper, native blocking commands
and client/aggregate command wiring; all included in this feature commit.

## Actual checks

- cargo test --locked -p azeroth-ingest -p azeroth-storage -p azeroth-app:
  153 passed, zero failures. Two further boundary/context regressions were added;
  current focused session transport/storage tests: 11 passed, zero failures.
- Populated v4 upgrade retains snapshot documents/hash/current, false preference,
  active content and party. Existing migration rollback/refusal tests pass at v5.
- Locked affected-crate all-target Clippy with -D warnings and workspace fmt pass.
- Desktop frozen install, typecheck and lint pass. Unit/build results below.
- git diff --check passes. No native window or live WoW test was performed.
- Read-only cross-review of quest PR48 tip4044a8e: no blocker/important finding;
  both focused quest_capture harness tests reproduced passing against exact source.

Initial compile found SQL COUNT typed usize; corrected to i64. Existing v3 fixture
reconstruction dropped only migration4 and left migration5; updated reconstruction
to remove all later empty tables/markers. Shared cross-worktree target cache caused
stale module metadata; cleaned task-local packages and reran successfully. These
failed attempts are recorded, not presented as successful checks.

## Limitations and next task

Selected profile requires a prior character import. Whole SavedVariables files
retain the existing 8 MiB bound while extracted documents cap32KiB. File-format
anav1_text is reused for plain text selection but this command requires the
ANAVSESSION1 prefix. Recorder/session screen arrive separately in task8 after
quest/task4 prerequisites. Native interaction, client event/clock readability,
Windows packaging and hosted CI remain unverified locally; coordinator records
current-head CI/review/merge evidence. Real content, calibration, inferred play
categories and live acceptance remain deferred.

Desktop 78 unit tests and production build passed. Shared wire boundary remains
mock/native-compilation scoped; actual native interaction was not exercised.

Recorder integration found and reproduced a UTC uncertainty defect: a trusted
first event followed by an uncertain or missing UTC still mapped the whole
session. Added regression failed before correction. Mapping now requires trusted
UTC on every event, preserving monotonic coverage regardless. Focused core and
service checks were rerun; this does not change exchange schemas.

Final fixture/source audit reproduced a document labelled synthetic with an
addon_snapshot observation being accepted (failing regression). Shared pure
SessionDocument identity validation now rejects captured-source metadata in
fixtures and synthetic_fixture metadata in nonfixtures, in both ingest and direct
storage paths. Current15focused ingest/storage/app tests pass. Initial helper
incorrectly treated SessionEvent ItemKey as an observed equipment slot; corrected
based on actual type (ItemKey has no observation metadata). Clippy needless-borrow
was corrected before final passing check; no schema kind/version changed.
