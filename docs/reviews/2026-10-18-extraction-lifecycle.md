# Independent read-only review — extraction lifecycle

Baseline: `167a4545edb0166da3ac81aaac51c002a9e1172a`.
Reviewer: Codex `/root/review_lifecycle`, explicitly requested by continuation plan.
Reviewed working-tree implementation and fixes; no reviewer edits.

## Findings resolved

1. Important: worker entry marked any colliding job failed even when that worker
   never obtained the destination lock. Moved failure recording into an owned
   extraction guard, after lock acquisition and successful checkpoint begin.
   Concurrent same-ID CLI-worker regression passes.
2. Important: metadata-only segment stamps could miss a same-length rewrite with
   preserved modification time on platforms lacking reliable change identity.
   Source fingerprints now stream full segment SHA-256 before/after extraction;
   restored-mtime byte replacement regression passes. Extra I/O is documented.
3. Documentation: older workflow paragraphs contradicted durable resume and saved
   selections. Updated those paragraphs while retaining unfinished gameplay
   projection/media activation and consumer work as explicit limitations.

## Final assessment

No remaining actionable code defects reported. Review covered private index/path
bounds, ownership/drop order, resume prefix/object verification, lock-gated crash
recovery, guarded activation/history, typed commands and UI response guards.
Tests cover transactional failure, SQLite full, binding changes, same-ID race,
corruption/rollback, recovery, saved presets and guarded UI operations.
Ordinary Windows performance and interactive/live acceptance remain unverified.
Exact-head hosted checks are a separate merge prerequisite.

## Preparing-state follow-up

The private run exposed that source hashing preceded durable job creation.
Implementation now exposes a `preparing` job under the destination writer lock
before fingerprinting. Unbound preparation carries no reusable rows. Cancellation
cannot be overwritten by binding. Reviewer rechecked these transitions and version
refusals, then requested cancellation checks during reused-object verification;
those checks and corruption/cancellation regressions were added.
