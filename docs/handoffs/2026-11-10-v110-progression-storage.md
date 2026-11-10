# V110.4c — immutable companion persistence

Owner/sole writer Codex `/root`; baseline `9e1158e0`; isolated PR #297 branch.
Claim: `docs/claims/2026-11-10-v110-progression-storage.md`.

## Implemented

Additive migration 14 stores immutable progression companions separately by imported
or manual origin. Saving validates exact selected predecessor metadata and the
existing canonical persisted snapshot hash inside an immediate transaction. Imported
captures require addon provenance; manual captures require manual provenance;
known nested observations cannot mix sources or lose provenance. Synthetic fixture
records remain explicitly synthetic. Same-ID changes refuse; exact repeats are no-ops.

Reads revalidate documents, digests, identity, origins and predecessor binding.
Queries reuse a caller-owned transaction when present, allowing parent and
companion to share one consistent read. Current queries return only records for the current character snapshot, ordered
by ID without assuming recency. Changing the selected snapshot hides predecessor
companions. Profile deletion and snapshot pruning cascade. Limits: 128 records
and 16 MiB per profile, existing 4 MiB per document; capacity refusal is atomic.

Recovery recognizes the new table and validates all retained companions, including
noncurrent history. Backup/restore, restart, schema13 upgrade, stale-parent refusal,
manual/import separation, capacity, identity conflicts and corruption are tested.
Historical downgrade-test setup removes the new table when reconstructing older
schemas; migration expectations now name 14. No historical schema was rewritten.

## Actual verification

- Full `cargo test --locked -p azeroth-storage`: 190 tests passed, including migration/recovery.
  Final consistent-read integration then passed the six focused tests and Clippy.
- Six focused progression tests passed; new storage module llvm-cov line coverage
  159/169 = 94.08%. Coverage is focused engineering evidence, not full release proof.
- Storage all-target Clippy with warnings denied, fmt and whitespace passed.
- Application snapshot_workflow and local_state_recovery tests: 10 passed.
- Desktop `pnpm --dir apps/desktop typecheck` passed with expanded exchange types.
- Earlier failures: new test passed a table name to a SQL-count helper; fixed.
  Historical downgrade fixtures initially retained migration14's table, and one
  schema-version assertion still expected13; corrected before passing full rerun.

## Independent review

Read-only `/root/progression_review` inspected progression persistence, origin
checks and transaction reuse: no blockers; no independent test/coverage run.
Read-only `/root/recovery_review` separately inspected migration registration,
recovery copy/foreign-key/full-history validation and corruption/downgrade tests:
no blockers; whitespace passed, no independent test/coverage run. Combined review
covers both subsystems; neither review establishes live or release acceptance.

## Remaining and next task

Storage methods are not yet exposed by application/Tauri commands or UI. Companion
records do not yet include saved plans. Consumers still need field freshness,
equipment identity and authored fact/ruleset checks. Manual inputs do not mutate
addon observations; completion must never assert game-state changes. No live
capture, Lua parity, Windows acceptance or release readiness is established.

Next: V110.4d application import/manual-input services and generated DTOs, then
saved plan dependencies and progression screen; continue V110.3 typed facts/content
and exact-build interface evidence independently. Existing 1.0.0 receipts do not
bind this changed product source. No merge/tag/publication in this task.
