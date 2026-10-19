# RC lifecycle transaction repair

Owner/sole writer Codex `/root`; baseline `38bbe4a`, branch
`fix/rc-lifecycle-transaction`. Claim recorded before implementation.

Identity, schema-version, page/journal validation and all additive migration steps
now run under one SQLite immediate transaction. Version is read after locking;
a competing opener sees the committed schema. RAII rollback preserves the original
schema and records if a later step fails. Connection-only foreign-key/synchronous
configuration remains outside the transaction. No schema/public contract changes.

Actual local checks: all 12 storage lifecycle tests passed, including 12 simultaneous
openers for each fresh/v1/v2/v3 database, preserved selection records, late migration
rollback, foreign-database and unsupported-version refusals. Storage all-target
Clippy with warnings denied passed; workspace formatting and diff checks passed.
Independent read-only review and final-head hosted checks remain pending.

PR #203 final-head success does not establish resulting-main success: main CI
39164374044, Coverage 39164373980 and package 39164374023 failed; source/parity
39164374011 passed. STATUS now records that distinction. No private inputs opened.
Next: independent review and exact-head checks; separately repair atomic Windows
fixture directories, bounded startup/structured IPC diagnostics, then SemVer upgrade
checks. RC publication, stability and Pages remain unfinished. 0.9.2 stays published;
no candidate or stable readiness is claimed.

## Integration receipt — 2026-10-19

PR #204 merged as `ac6037a44fb41a24dba7728a54cbf603eb364b87` after independent read-only approval of
final head `bbdd5e4bb499c84f7329884c14f54e16ac7dd254` and all ten required checks passing.
Successful runs: CI 39228695506, Coverage 39228695446, package 39228695449 and source/parity 39228695445.
Earlier failures/cancellations remain distinct from these passes.
Next: [development RC verification and preparation](2026-10-19-rc-release.md).
This repair does not establish stable/candidate readiness or release publication.
