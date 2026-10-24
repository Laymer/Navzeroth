# Strict catalog load and recovery performance repair

Owner tooling_review, baseline2466b7440fec64e346b7d0eba7e8fefcb1a1fa6e,
claimbc62c694. World owns the duplicate-cache repair, claim recorded
in its [handoff](2026-10-21-validated-duplicate-cache.md). One writer per scope.
Reviewed code union419fcaecfcb208867e280140215c6e5e01891e27; no generated data,
release contracts, receipt identities, source grants, budgets or gates changed.

Three separately reviewed changes retain all strict boundaries:

- Tooling fd722928 owns the stored String, strict wrapper and core decoder, then
  drops the String before capacity/provenance canonicalization. The core decoder
  already validates its complete graph; the unchanged graph is not validated a
  second time. Arbitrary typed staging still validates. The test counter measures
  only redundant storage-side typed validation (zero after strict decode, one for
  typed staging), not an instrumented core call count. Invalid decoded graphs,
  unknown fields and invalid provenance still fail. No public unchecked API exists.
- Tooling33674568 streams the EXISTING sorted canonical writer into SHA-256.
  Snapshot Vec API and exact bytes/errors remain. Fixed canonical bytes, scalar
  escaping, false/zero/null/floats, short sink writes and all failure boundaries
  are tested, along with the independent pre-change SQLite document/hash/coverage
  golden. The full serde Value allocation remains; no typed serializer was added.
- Worlda9ce8e46/f30a5c0e retains only actual SQL-validated duplicate Arcs under the
  existing original generation stamp and owned IMMEDIATE transaction. New inserts
  never cache caller input. Default activation can reuse that validated Arc;
  content/DDL/external/temp/rollback invalidation and historical checks remain.
  Duplicate-only unused document/report serialization is deferred until insertion.

Root independently reviewed all three changes and executed matched tests; world
independently reviewed the hash and ran2stream/10catalog tests; desktop reviewed
the full union and exercised cache/assets/consistent-read tests. These matched
binary executions are not independent compiler builds. Root also independently
rederived the exact production LCOV counts below using unchanged gate functions.

Actual local checks:3decoded/capacity units,2streaming units,10catalog integration
(including fixed pre-change bytes/hash/coverage),19snapshot tests, all-target
storage Clippy-Dwarnings, fmt and diff checks passed. The final ONE combined
app+storage LLVM execution passed475tests,0failed,3ignored across64test/example
binaries. Exact current-source production lines, excluding only existing
rustTestRanges: content374/412=90.7767%, catalog363/381=95.2756%, preferences89/93=
95.6989%, generation52/52 and hash51/51=100%. Integer90/100/no rounding unchanged.
No old-worktree rows were substituted. Two earlier LLVM builds failed ENOSPC
(the second also linker SIGBUS); neither ran tests or establishes coverage. Root
removed confirmed idle incremental caches only, preserving binaries/profiles/
evidence, then the same CARGO_INCREMENTAL=0 profile passed. Initial derivation
could not import TypeScript in this isolated worktree; byte-identical existing
root gate/inventory modules with installed dependency produced the same checks.
Initial development compile errors in the invalid-graph fixture, helper-name
shadowing and polymorphic error adapter were corrected before passing tests.

Fresh full-pack composed-catalog load diagnostic, identical copied prior backup:
baseline2466 genuinely rebuilt storage10.189s, firstfd7229.238s, union4198.937s;
peakRSS5115384/5115640/5115644KiB respectively. These are one shared Linux sample
per version with uncontrolled cache, not a lower-memory or Windows claim.
All260100 core graph identities and strict stored identity/hash/coverage/
provenance checks passed. This is not a new release inventory count.

Then genuinely rebuilt optimized app dependencies at union419 in46.90s. The new
standalone diagnostic executable links that new app/storage, with compile-time
runtime419 and harness3447 identities. Harness5713bytes SHA256
6e94b838250f2319e3f61258db3dd11f72d6af9c784d3c12f339bb768eeda10c is BYTE-IDENTICAL
to the prior parameterized harness. The unchanged OLD V13 signed bundle/source
identity is printed explicitly and never relabelled as new production.

Actual whole diagnostic passed287.959653614s, peak childRSS12304676KiB (~11.735GiB),
exit0/no timeout under the unchanged1200s outer bound. Previous d636 diagnostic
was347.804999857s/12508668KiB (~11.929GiB). All original assertions remain: manual
engineering profile/settings125, inferred choice false, backup/inspect, restore
receipt cleanup complete/originals retained/nonempty directory, ready/errornull,
exact profiles/settings/content/full scope/default marker/choice, full integrity,
explicit close/reopen through signed initializer, persistence and final integrity.
Actual backup plan kind/request/guard/summary are all null. Final phase timings
and all raw output are retained; restored/reopened state reads349/358/358ms,
inspect31348ms, restore77106ms, checks28603ms/28429ms,
reopen78862ms. Initial startup41034ms(load33703/activation7191), false-travel
activation787ms. Startup status after restore reports the original process
startup measurement; it is not a newly measured restore subphase.

This one shared Linux/WSL comparison has uncontrolled cache/host and is not a
Windows prediction, ordinary-PC claim, statistical benchmark or proof that the
hosted20-minute recovery workload fits. V15 hosted recovery timeout remains a
failure at its original source. No installer/owner observation is promoted.
Runtime functional proof must be freshly executed by root after integration;
final protected-main/source/stability/Windows gates and exact draft smoke remain.

Compact raw harness/runner/output, measurement, preexecution hashes, exact
production coverage, checks and all three fresh-load audits are archived in
[files](../../release/history/2026-10-21-storage-validation-recovery/files.json).
Full local LLVM log/LCOV and both ENOSPC logs remain external at
/tmp/navzeroth-storage-validation-union-{coverage.log,coverage-enospc.log,
coverage-enospc-second.log} and /tmp/navzeroth-storage-validation-union.lcov,
with hashes in archived checks. Actual executable
/tmp/navzeroth-v16-fullpack-recovery-diagnostic SHA256
6471a04ea17dcbfb1b23e387eace2eea644dd28188ccc1218f722b57e85fad39;
app/storage preserved as /tmp/libnavzeroth_v16_{app,storage}.rlib. Historical
executables/raw audits remain unchanged; shared original library paths may be
rebuilt, so historical library digests remain their recorded execution identities.

Optimized target ownership was released to root after the measured process ended.
Next: one coherent draft repair PR, independent final archive/diff review and normal
integration only after root approval, followed by fresh actual functional proof,
public review, prepared identities before any signing, and all applicable gates.
No tag/publication, key access, account/quota change or network acquisition here.
