# Shared local state schema and recovery foundation

Owner: Codex /root, sole foundation writer. Claimcbe4b2b before code, published
baseline739a822/v0.6.0. Production sourcedfcd37b; final six testsfc87b93.
Coordination main d48f883 normally merged. Branch feat/local-state-schema.
Content.rs ownership released to /root/release_review after production freeze;
root retains migration/recovery registration and foundation-test ownership.

## Behavior and boundaries

Migration9 adds four bounded tables: content_dispositions, app_settings,
current_plan and local_submissions. Schema now has21 registered recovery tables.
Closed states/reasons/decisions, byte lengths, safe integer counters and exact
content/source foreign keys are enforced. Optional operation UTC values preserve
signed-safe core timestamps, includingzero; no clock is manufactured. Existing
content rows start locally eligible without evidence verification or permission
promotion. Only recorded current/previous pointers receive ordinal2/1; merely
staged rows keep unknown activation order and unknown changed UTC.

New staging inserts an initial disposition atomically; identical restaging never
resets restriction or counter. Existing activation/rollback refuse blocked/missing
eligibility before pointer mutation, and successful changed activation records a
checked per-scope ordinal. Exhaustion preserves previous active state. Active reads
fail closed toNone when eligibility is missing/blocked. Raw immutable content remains
inspectable. Pure policy/mutations, fallback selection and UI belong to the separate
content task, which consumes the frozen visibility helpers.

Current-plan clear uses an empty fixed-slot CAS tombstone; the migration seeds
revision1 with all payload columnsNULL. Recovery must replace that seed before
copying the source's exact row, preserving its revision and empty state. Old schema
backups migrate after their trusted data copy. Source files are read consistently
and never modified; imported schema/SQL is never executed.

Recovery explicitly registers all four tables. Until the feature modules' typed
validators are integrated, populated settings/plan/submission documents are refused
rather than trusted or silently discarded. This is a temporary foundation limit,
not delivered populated feature recovery. Root must replace these refusal gates
with each owning feature's typed validator before the combined next release.

## Actual checks and failed attempts

After legacy fixture corrections, all existing storage tests and strict all-target
storage Clippy passed. Combined app/storage passed259tests,0failures/ignored,
including the initial five new foundation cases. Final added source/context/decision
constraint test brings the focused foundation suite to6passing; strict all-target
storage Clippy passed again. Workspace fmt/diff pass. Logs:
/tmp/navzeroth-wave7-storage-final-baseline{,-clippy}.log,
/tmp/navzeroth-wave7-storage-combined.log,
/tmp/navzeroth-wave7-storage-foundation-final-{tests,clippy}.log.

The six cases cover populated v8 immutable-byte preservation/backfill and unknown
staged metadata, empty tombstone revision23 restoration/source preservation, invalid
feature-document refusal without destination or source mutation, blocked/missing
eligibility and duplicate preservation, rollback/ordinal exhaustion atomicity,
multibyte/partial/future/noninteger bounds, exact context/source foreign keys,
state/decision consistency, signed UTC and erased dedup tombstone constraints.
Low-level submission probes are schema tests, not accepted real observations.

Initial runs caught old v3/v4 schema simulations leaving newly added tables and a
v7-upgrade assertion hardcoding8. They now reconstruct the exact old schema and
assert actual supported latest version while retaining all preservation checks.
Strict Clippy caught a complex test tuple, factored into a named alias. An initial
shell sequence continued to Clippy after a failing test; actual failed test logs
were inspected and corrected, never counted as a passing suite. Final commands use
fail-fast sequencing. Independent earlier in-memory DDL review passed11probes;
final production peer review and fresh exact-head hosted jobs still gate merge.

## Next task and limits

Require fresh nine-job checks and protected integration. Three independent workers
consume this foundation and supply typed populated-state recovery validators.
Root then integrates those validators and verifies real backup round trips and
unknown/corrupt refusal. Feature API/module ownership stays separate. No package
tag, publication, native interaction or live-game result is created by this task.
