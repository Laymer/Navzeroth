# Immutable catalog cache generation repair

Owner/sole writer world_review, isolated fix/stable-catalog-cache-generation.
Baseline35c0dc0137d4a5f38efa996408894b92140431c7; claim517941f8;
implementationac57050df4581048208452dfe65b6626ffde7d47.
The active rehearsal worktree, its generated data and other worktrees are untouched.

The parsed catalog and paging index previously keyed connection.total_changes().
Every ordinary snapshot/reminder/preference/settings write therefore discarded
both expensive immutable caches. A minimal external public-API regression using
labelled synthetic content reproduced a different Arc after an unrelated settings
write on the baseline; the identical assertion passes against this repair.
No actual full-pack speed or memory result is claimed from that small test.

The storage constructor now owns one SQLite update hook. It increments a monotonic
local generation only for dataset_revisions, dataset_manifests, source_records and
source_conflicts, including trigger writes. These canonical main tables are ROWID
tables. The same constructor behavior is installed for recovery's temporary store.
The hooks feature adds no dependency/version/lockfile or database schema change.
Existing loaded JSON, canonical hashes, coverage and provenance validation are
unchanged; neither a stored hash nor an epoch is trusted as a substitute on a miss.

Both cache identities include local generation, SQLite external data_version and
main/TEMP schema_version. Thus ordinary same-connection player writes reuse both
caches; external commits remain conservatively invalidating. Active pointers and
eligibility/disposition guards are still read and checked freshly. Actual content
staging invalidates; activation/rollback still select the exact current revision.

Only a store-managed consistent-read transaction with its unchanged entry stamp
may reuse or publish cache entries. Arbitrary write/savepoint transactions bypass
both caches. A managed callback that writes catalog rows or changes schema loses
cache eligibility too. This prevents uncommitted rows, ROLLBACK TO, reused IDs,
DDL rollback and error/unwind from leaking a transient catalog into the committed
cache. A terminal u64::MAX generation permanently disables caching; it never wraps.
Temporary same-name catalog shadows bypass caching, including WITHOUT ROWID tables
whose writes cannot be trusted to row hooks. An attached same-name table cannot
shadow the canonical main table under the supported schema.

SqliteStore has no public raw connection, SQL, hook-replacement or incremental
BLOB writer. Only this module installs update_hook. The hook must remain owned by
storage, and future catalog mutation APIs (including incremental BLOB writes or a
non-ROWID schema migration) must extend invalidation before introduction. This
cache is not an authentication boundary against a privileged process rewriting
SQLite internals. Every ordinary external committed change still invalidates via
data_version. Existing SQLite reservation/consistent-view behavior is retained.

Actual verification:

- Ten new scenarios passed: both Arc identities survive real snapshot/reminder/
  preference/settings writes; duplicate/failed staging; staging/activation/rollback;
  same-connection corruption of all four watched tables; DELETE without WHERE;
  INSERT OR REPLACE; trigger writes; external writes/corruption; main/TEMP DDL;
  attached/TEMP shadows; savepoint rollback and reused IDs; managed callback writes,
  DDL rollback and unwind; terminal overflow. Corruption fixtures remove protective
  triggers before warming the cache; production guards remain intact.
- Full storage + app Linux continuous-profile tests:432passed,0failed,3existingignored.
  No invalid profile is ignored. An initial ordinary-profile run also passed;
  final coverage uses the repository's continuous counter-relocation settings.
- Gate-style executable-source LCOV after exact repository rustTestRanges exclusion:
  catalog_generation50/50(100%), consistent_read16/16(100%), content_catalog339/358
  (94.69%), sqlite647/668(96.86%), recovery418/449(93.10%). Unchanged90% thresholds.
  The byte-identical repository inventory helper was imported from an existing
  desktop dependency installation because this new worktree has no TS modules.
- Storage all-target Clippy with warnings denied, formatting and whitespace checks
  passed. Early fixture compilation/forbidden sealed-insert errors were corrected;
  no validation or immutability guard was relaxed.

[Machine-readable source coverage and checks](../data/stable-catalog-cache-generation-v1.json)
retain source hashes and logs. Full raw evidence is /tmp/navzeroth-cache-generation*
(and the baseline/fixed public-API harness directories). No pushes, content receipts,
keys, installer verification, Windows test or publication occurred.

Next: root independent review, then integrate into the combined rehearsal. Re-run
actual full-pack installation/loading/browsing/planning/reminder persistence and
resource measurements before any performance or release acceptance claim. Cold
loads and content changes still require full validation; unrelated external commits
and arbitrary write transactions remain conservative cache misses.
