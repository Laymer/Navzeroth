# Actual Linux full-pack recovery diagnostic baseline

Sole writer `/root/tooling_review`; isolated benchmark baseline
`d271bba67e8fa274e44953c9f78198b1d8fc5f6d`; claimfdc0c520;
baseline harness41f013c6. This is diagnostic engineering evidence, not a new
functional proof, Windows result, acceptance result or restamp of actualef3.

Actual baseline execution:1test PASS,0failed;721.4337s outer/721.39s Rust;
maximum child resident set11946608KiB (approximately11.39GiB), Linux
resource.RUSAGE_CHILDREN; whole1200-second bound not reached. Original attempt
failed127 before execution because GNUtime was absent; that setup failure remains
separate. No package was installed. Python time.monotonic/subprocess timeout and
resource captured the actual run afterward.

| Phase | Actual ms |
|---|---:|
| Initial signed-bundle startup |71148|
| Manual engineering profile/settings125 |24|
| Inspect active/choose inferred false |17334|
| Initial exact state |572|
| Backup463335424bytes |409|
| Inspect/reconstruct backup |123727|
| Restore and signed initializer |151689|
| Restored ready/error-null |0|
| Restored profiles/settings/content/choice |15462|
| Restored database check |66012|
| Drop restored host |245|
| Explicit existing-database reopen/initializer |200546|
| Reopened profiles/settings/content/choice |13847|
| Reopened database check |59985|

Strict restore receipt requires cleanup-not-pending, retained originals and a
nonempty recovery directory. Exact profiles, settings125, active revision/scope,
inferredfalse and default marker equality pass after restore and reopen; both
checks pass. Read-only SQLite query of the actual backup confirms current_plan
kind/request/guard/summary allNULL. The initial initializer reports load56474ms
and activation14562ms. Restored status retains that initial report; it is not a
new restore-phase breakdown. Existing-database reopen reports initializer91268ms,
activation0, leaving about109278ms outside the initializer. Reconstruction and
full semantic validation dominate copy, not an established hang or OOM.

The harness was freshly compiled with rustc--test but linked the existing optimized
app dependencies; no new app compiler build is claimed. Actual runtime
`1ddaa8d66b0fe6ccfdbef2dbf57263afcab03611` crate tree equals d271 byte-for-byte.
App library libazeroth_app-56cefb75fabdc75b SHA256
10f3e9f4eb6fe3ffc5be823060277ac1e983494c0dea95e676857fb6363c5e92;
baseline diagnostic binary SHA256
f84ef8b0baf656573206272c07dfd24c37372156d88e7b1dbc0961559da308e8.
The original verified V13 bundle remains separate signed productiona8c292/tested716;
a later runtime diagnostic must not relabel it as signed by that later production.

External artifacts: /tmp/navzeroth-fullpack-recovery-diagnostic.log,
/tmp/navzeroth-fullpack-recovery-diagnostic-measurement.json,
/tmp/navzeroth-fullpack-recovery-baseline-audit.json, baseline binary
/tmp/navzeroth-fullpack-recovery-diagnostic, isolated463MB database/backup state
/tmp/navzeroth-fullpack-recovery-diagnostic-data. Audit hashes bind actual phase
log, test source, binary and library. Original missing-tool failure is retained as
/tmp/navzeroth-fullpack-recovery-diagnostic-initial-setup-failure.log.

V13 Windows per-workload20-minute recovery failure remains failed/unlocated at the
individual IPC level; only four hosted rows passed, persistence/recovery failed,
and offline/upgrade/downgrade never ran. Linux measurements do not prove an ordinary
Windows timing or budget fit. Root owns fresh functional proof after reviewed
runtime union; world owns no-plan duplicate-validation repair; desktop owns strict
allocation and hosted-checkpoint repairs. Next: parameterized diagnostic identity,
independent frozen-code reviews, then rebuild the actual changed runtime dependencies
and run the same diagnostic sequentially, with original V13 bundle identity explicit.
No runtime, policy, gate, budget, key, canonical proof or release was changed here.

## Actual reviewed-union comparison

Runtime `d636c539ef90fed43d237d04bbacc1ab74071ad7` was genuinely rebuilt from the
held clean root worktree using `cargo build --locked --release -p azeroth-app`,
shared optimized target,2build jobs: success93s. Fresh parameterized harness3447bcbf
linked those NEW dependencies into a distinct binary. The original baseline
binary/log are unchanged. Old V13 signed productiona8c292/tested716 and receipt
are explicitly emitted separately; they are not signatures on runtime d636.

Actual comparison1test PASS,0failed;347.8050s outer/347.80s Rust, no1200s timeout.
Peak12508668KiB (approximately11.93GiB), HIGHER than baseline11946608KiB/11.39GiB.
Do not claim a peak-memory improvement. This sequential Linux sample took less
elapsed time, but OS caches, CPU and allocator conditions are not controlled and
it establishes neither Windows timing nor ordinary-PC performance/budget fit.

| Phase | Baseline ms | Reviewed union ms |
|---|---:|---:|
| Initial startup |71148|42897|
| Manual profile/settings125 |24|22|
| Inspect active/choose inferred false |17334|13020|
| Initial state |572|364|
| Backup463335424bytes |409|357|
| Inspect |123727|34557|
| Restore/initializer |151689|85721|
| Restored ready/error-null |0|0|
| Restored exact state |15462|8659|
| Restored database check |66012|33422|
| Drop host |245|54|
| Existing-database reopen/initializer |200546|86095|
| Reopened exact state |13847|9225|
| Reopened database check |59985|33085|

All original strict assertions passed, including retained originals, no pending
marker cleanup, ready/error-null, settings125, profiles, exact content revision/
scope/default marker/inferredfalse and both database checks. Union backup also
has all four current_plan fieldsNULL, confirmed through a read-only SQLite query.
Initial initializer load34566/activation8187; reopen load51793/activation0.
No canonical acceptance observation, fresh functional proof or historical failure
was changed. Root separately owns a genuine new full-pack functional proof.

Raw union artifacts and SHA256:
- /tmp/navzeroth-fullpack-recovery-union-diagnostic.log:
  3f3335d5067ce5e2ffda8c9996ed890485dd3d2386a6294bec610e2498387fe8
- /tmp/navzeroth-fullpack-recovery-union-diagnostic-measurement.json:
  1d62ed64688117ce6e25b2bd0db6a6bea951358f3a6c51642139715c05d3b687
- /tmp/navzeroth-fullpack-recovery-union-audit.json:
  3776e8788233a8d40e582882c51d30e12e002bea4fd93a52a701093d5659fd82
- New binary /tmp/navzeroth-fullpack-recovery-union-diagnostic:
  a0dca608645cb84bc0d1b592537a6051bfb3f35c86aa4e1cf0883db7aae72a1c
- Actual new app library libazeroth_app-56cefb75fabdc75b:
  83f326edb70b02d412a27451337bf1bf6d4d9ac757b5393bcd2f1b818ab59251

Harness source remains on the isolated diagnostic branch only, not proposed for
runtime integration. Archive-ready baseline/parameterized .rs.txt copies are at
/tmp/navzeroth-fullpack-recovery-{baseline,parameterized}-harness.rs.txt. Runtime
reviews separately cleared world8730 and desktopb3d; independent matched-binary
checks passed recovery7+guardedapp1 and allocation21core/1ignored+10catalog+2owned.
The shared storage unit binary no longer contained the new world counter test
when checked; that filtered0-test invocation is NOT an independent pass. Author
and root actual counter runs were separately recorded. No new app compiler build
was claimed for those matched-binary reviews.

Optimized-target ownership was released to root after the diagnostic process
ended. Next: root actual final-source functional proof, renewed review/receipt
binding, protected checks and exact installed Windows cases. V13 recovery timeout
remains failed until new hosted evidence, never inferred passed from this run.
