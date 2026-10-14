# Registered source identity migration prerequisite

Owner: Codex /root, shared recovery claim. Baseline main76802f5;
source6509a3c is isolated here before dependent feature PRs are integrated.
Migration10 widens local submission source IDs from128 to the existing core
256-byte SourceId contract. Migration9 remains immutable; all row columns,
payloads, erased tombstones, counters, signed timestamps and foreign keys survive.
Recovery now bounds migration-history reads by latest_version+1, so an extra future
history row cannot disappear behind the previous fixed LIMIT10.

Actual checks:25 storage tests passed (12 migrations,6 recovery,6 local foundation,
1 source-identity migration), followed by strict storage Clippy. First compilation
of the history helper used usize for SQLite; corrected checked-safe i64 binding.
The final positive test reconstructs the trusted old9 table, proves old9 rejects
registered256-byte sources, upgrades without row loss and rejects257-byte sources.
No failed attempt is counted as a pass. Logs /tmp/navzeroth-wave7-schema10-final-
{tests,clippy}.log. Dependent feature confirms129/256 ASCII/256 UTF8 typed source
persistence; that evidence belongs to its separate submission PR.

Next: exact-head hosted checks, protected-main integration, then dependent task
PRs and typed populated recovery/provenance integration before the preview release.
