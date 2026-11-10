# V110.4c — bounded progression companion persistence

Owner/sole writer Codex `/root`; baseline `9e1158e0`, isolated PR #297 branch.
Scope: additive SQLite migration 14, immutable snapshot-bound companions with
separate imported/manual provenance, current-snapshot filtering, deletion/backup/
restore integration and focused regression tests. Plans and application UI follow.
Checks: migration/storage/recovery suites, Clippy, formatting, focused per-file
coverage and independent read-only review. No actual client or release claim.
