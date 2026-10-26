# SQL-validated duplicate catalog retention

Owner world_review; claim73d2dfd5, baselinefd722928889a84dce6eba59c91cfeab4ba55fa6f
(tooling strict-decode repair frozen before this writer acquired content_catalog.rs).
Frozen implementationa9ce8e46d718b87e179be255c41341029d390513. Parent candidate2466;
this branch neither changes release bindings nor establishes acceptance.

Observed hosted V15 recovery restored successfully, then timed out with check_database
pending. Independently traced first active reads to cold catalog loads and duplicate
staging discarding the already SQL-validated composed catalog. Actual baseline Linux
microbenchmark by desktop: cold catalog10s, warm sameArc0ms, report~78ms/hash~210ms;
no immutable-DTO memoization was implemented. Windows versus Linux timings are not
interchangeable; no measured improvement is claimed for this repair yet.

Owned stage_content_v2_revision now starts IMMEDIATE and the existing managed read
scope before any content writes. Private duplicate-loader callback uses the existing
cached_catalog path: either the exact earlier SQL-validated Arc under unchanged
stamp, or a fresh strict SQL load including hash/coverage/source/conflict checks.
No cache is seeded from canonical input. Insert changes generation and remains cold;
other stage_canonical callers retain their original strict loader. Managed guard
drops before commit. Duplicate document/report serialization moves after duplicate
return because input capacity/domain/provenance validation already ran and those
bytes are unused for duplicates. The old catalog hash comparison remains unchanged;
no manifest digest shortcut or altered serialization is introduced.

Changed-version apply_bundle_default mirrors the existing guarded travel-activation
pattern, retaining SQL-validated Arc and dropping managed guard before preference/
pointer/marker writes. Same-version choices persist; new version resets defaults.
Existing trigger updates, local monotonic generation, external data_version,
main/TEMP schema and shadow checks remain. Arbitrary transaction/savepoint calls
still cannot acquire a nested managed reservation. Rollback cannot publish mutated
content under a fresh trusted stamp; no post-write/new-stamp trust was added.
SQLite connection ownership, private SQL/BLOB/hook assumptions are unchanged.
No additional filesystem-identity cache is introduced; reopening creates a new store.

Actual checks (PATH=/home/laymer/.cargo/bin:$PATH,
CARGO_TARGET_DIR=/tmp/navzeroth-cache-repro-target):
- cargo test -p azeroth-storage --lib catalog_generation::tests -- --nocapture:
  21PASS, including17 existing guards and4 new cases.
- cargo test -p azeroth-storage --lib --test catalog_v2 --test catalog_assets
  --test content --test content_dispositions --test consistent_read --test recovery:
  92PASS (41library,10catalog,7assets,14content,11dispositions,2consistent,7recovery).
- cargo clippy -p azeroth-storage --all-targets -- -D warnings; cargo fmt --all
  --check; git diff --check:PASS.
Initial test command named nonexistent target content_preferences and ran no tests;
corrected actual command above passed. Initial three new tamper tests were refused
by immutable-row triggers; test setup now explicitly removes guards for controlled
corruption, restoring canonical schema before historical integrity validation.
No production restriction was removed to make these tests pass.

New assertions prove cold duplicate needs one SQL decode then repeated duplicate,
default selection and first active return same Arc with zero additional full loads;
fresh insert does not seed cache. Conflicting input and source/conflict/manifest/
document corruption refuse. Injected trigger mutation/abort preserves pointer,
choice and marker, invalidates old stamp, and a corrupt noncurrent revision still
fails full recovery validation. Separate SQLite writer mutation invalidates warm
file-backed cache. Existing tests retain schema/TEMP/case-variant, arbitrary write,
savepoint rollback, overflow, trigger and external mutation guards.

Test executables are under /tmp/navzeroth-cache-repro-target/debug/deps, matched
sourcea9ce: azeroth_storage-14d99fd93d918ee0, catalog_v2-74e759949235ecb6,
catalog_assets-bf59cdda95558b2f, consistent_read-bd5c08e8a20b2a3e,
content-d6146c9ef8a3d252, content_dispositions-470e9e40ecbcf494,
recovery-1bba7640a69b7785. Logs /tmp/navzeroth-duplicate-cache-{tests,storage,clippy}.log.
Independent executions of these are matched-author-binary executions, not separate
compiler builds. Broader composition coverage and genuine full-pack before/after
benchmark are still pending and will be recorded separately; no90percent, speedup,
Windows, owner or release-pass claim follows from these targeted checks.

Next independent frozen-code review, single combined LLVM run on reviewed union
with tooling hash repair (avoid duplicate targets on constrained disk), genuine
rebuilt full-pack timing/functional proof, then normal repair PR integration and
renewed exact source/review/receipts before another hosted candidate. Production
20/60/170minute bounds and historical/destructive validation remain unchanged.
