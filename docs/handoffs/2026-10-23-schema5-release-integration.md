# Schema 5 release integration in progress

Coordinator: root; baseline V19 `9cc774e4f2656f26d544071d6bd174367ccca16a`.
Main remains `807ef674ee13c15ece90e64618ccd5f122afcbdb`; no new release candidate
has been pushed, tagged or published by this continuation.

Normal local merges retain published history. PR271 (ADR0009/build-watch) merged
at8a11f882; PR272 schema5 contract at a1a33fc5 and generator followup at1b6e764b
are integrated; PR273 Windows transport at3003c65d and display-label followup
c4c4284c are integrated; PR274 UI driver finald1990d90 is integrated. Current
release-process docs were reviewed independently by world_review and committed
at9d30d716. Exact normal merge commits remain in Git history. These local merges
do not establish required remote checks or authorize publication.

PR243 was closed as superseded, preserving its historical generated run. Issue270
was acknowledged and closed with the owner's authorization: reported70532 is a
future compatibility investigation, not admitted data or a target change. See
https://github.com/Laymer/Navzeroth/issues/270#issuecomment-6358669314.
The connector could read the issue but returned403 for writes; the authenticated
repository CLI completed the authorized comment and closure.

Independent reviews and actual coordinator checks:

- Schema5 contract cbf2/8bf/409 production changes and final a1/1b followups were
  read and cleared. The matching author-built CLI binary's eight draft evidence
  tests passed independently; this is not an independent compiler build. The
  inventory generator's four Python regressions also passed independently.
- Tooling independently cleared Windows transport final3003 and cosmeticc4c.
  Its Linux PS7 runs passed24 draft,49 preparation,27 bounds, three installed-tree
  tests and actionlint. Root additionally passed native PS7 draft24 and native
  PS5 draft18; six PS7-only cases remain explicitly unexecuted under PS5.
- Root reviewed the new UI driver8666 and correctiond199, and independently ran
  seven Node tests and three real-React mocked-browser tests (4.7s, retries0).
  Review corrected an inherited travel-edge disclosure: this draft scenario is
  explicitly same-node manual geometry, while its objective comes from admitted
  source data. Mocked browser results are never installer acceptance.
- Root reviewed recovery2659 and diagnosticsfa11. The rebuilt exact storage
  binary SHA2567c92c7d0f018a105c73598e959789afcc11b2d8a9fbb2ca61fc135fb6f0b4728
  passed all six new regressions independently; Node diagnostics28 also passed.
  An earlier six-test execution preceded deletion of an unused helper; it is
  retained as earlier-source execution, not mislabeled as the final binary.

The coverage audit found the new executable schema exporter missing from the
collector. Tooling owns the narrow closured552, with both supported selectors and
expected panic refusals run under instrumentation. Root read that change and ran
all three collector wiring/instrumentation tests successfully. The first attempt
had Cargo absent from PATH (two tests passed, one setup failure); rerunning with
the documented Cargo path passed. Exporter coverage itself still needs its actual
sixth-file report before integration. Existing five-file Rust coverage claims were
scoped correctly but did not constitute closure for this new example.

Desktop's same-harness full-pack Linux comparison passed: baseline9cc recovery
305.786843632s, repair2659 recovery210.229088706s; peak process RSS12174412 versus
12239288KiB. Both use explicitly historical V13 signed bundle inputs and manual
engineering state on shared Linux/WSL2 with uncontrolled OS cache. These are
observed rehearsals, not Windows acceptance, controlled speedup, reduced-memory or
ordinary-PC claims. All backup/inspect/restore/check/reopen and state assertions
passed. Runtime coverage reports479 passed/3 ignored and each of its three changed
production files above90%; see the runtime writer's forthcoming exact handoff.

Next: integrate reviewed runtime and exporter closure; review/run the bounded
hosted Windows recovery probe. A separate writer prepares a supplemental Windows11
0.8.1 backup/upgrade launcher; no local installer or private profile has been read
or changed for that task. Renew functional/source/inventory/review/receipt bindings
after all production changes are committed. Canonical observations stay pending;
old schema4 evidence and signed V19 bytes remain archived. Fresh protected checks,
source parity, resulting-main stability, exact draft automation and independently
reviewed completed API evidence are mandatory before publication. Public-download
verification and website deployment follow publication; none has occurred yet.

## Integrated runtime and schema5 inventory renewal

Local integration now reaches `ca94ec5d749519b7b656f34f9c9a3739b322020e`.
PR275 final749977a0 merged atb601bb8f; PR276 finalc29725c8 merged at3f935a2d.
The schema-exporter closure final873d1fc6 merged atf589d196. Actual continuous
LLVM instrumentation covers all six affected Rust files above90%, including the
new exporter8/8 lines; the committed structural schemas regenerated byte-for-byte.
The runtime writer's actual checks are recorded in
[the recovery handoff](2026-10-23-recovery-validation-v20.md).

Supplemental backup/upgrade helpers passed fifteen inert checks independently on
LinuxPS7, nativeWindowsPS5 and nativeWindowsPS7. The launcher uses ordinary
current-user installation rather than forcing elevation. Commitca94 wires these
helpers into both package-workflow PowerShell jobs; desktop independently reviewed
that wiring and passed the workflow-style tests. No local personal installation,
backup or profile operation has occurred.

Root genuinely rebuilt the full-content functional binary at exactca94; SHA256
`d1745d726af2c24457973e1153134b4df24baa5c57d0861f48fd573d1496c5b1`.
Its actual run passed in76.789839612s, peak6205296KiB. World independently ran
the same verified binary in a separate clean ca94 checkout, passing in78.041175037s,
peak6205752KiB. Both produced identical15568-byte proofSHA256
`cbbdb705a2b1e6fea3fc3506205994dbf033c940efd667cf2e667899652ff13a`.
This is independent execution, not a second compiler build. SharedWSL/Linux,
32logicalCPU and32675956KiB reported memory with uncontrolled OS cache do not
establish Windows or ordinary-PC performance. The source-backed plans retain
unresolved eligibility and explicit manual engineering time/node assumptions.

The actual inventory generator completed using the rebuilt CLI. It retains
260113 shipped records and274059 accepted unknown fields. Independent semantic
comparison found the inventory unchanged except schema4→5; evidence changes only
the schema, inventory/four functional proof hashes, new final automated row and
cleared old source/production/bundle bindings. All nine observations remain
pristine unverified. The original six canonicalV19 files were archived atbde347d0,
with hashes and original Git blob identities verified before renewal. The ca94
proofs, measurements, logs and generator summary are separately retained under
`release/history/2026-10-23-v20-ca94-functional/` before final-source renewal.

The separate hosted recovery engineering probe is still under review. Review
found and is correcting cleanup ownership after rejected fresh-install preflight,
and a parent-exit case that left a descendant outside the selected watchdog.
Neither defect is accepted as a passing probe or product observation. Probe
production changes must be committed before selecting final source bindings;
final functional execution, independent public review, prepared identities and
external-key signatures remain next. RemotePR239 still holdsV19; all ten required
main checks were read back unchanged, and no open issues remain after the already
authorized270 closure. No new candidate push, main merge, tag or publication.

## Final integrated production and material preparation

Reviewed PR278 probe final80e3dfd5 is normally merged ata1ca248f; reviewed PR277
transport finald8d6ece0 is normally merged in the following integration. Root's
five-line probe wiring commitde2fd3e69bcce8db55c73d98fed492e2ee485261 adds the
actual native Git transport tests before compilation/fixture installation. Tooling
independently reviewed that exact diff and actionlint passed. Root and desktop
cleared transport CODEa2eb; root independently passed17Linux tests/one explicit
Windows-junction skip, then repeated after correcting the case-collision fixture
to construct ambiguous Git entries without relying on a case-sensitive filesystem.
Four direct native Windows process/junction tests passed separately. The full
native Git suite was unavailable locally and is checked by the hosted probe.

This closes a concrete public-download path gap: packaging's eight observations
cannot include future completed-draft provenance/review. The reviewed immutable
data-commit transport reads only bounded inert evidence subtrees, preserves tagged
HEAD/source, rejects links/reparse/case collisions and never executes archive code.
Public verification still requires full Rust readiness and writes separate public
receipts so it cannot overwrite its consumed draft proof. See the
[transport handoff](2026-10-23-public-reviewed-evidence-transport.md).

Exactde2 productionSHA256 is
`02c1012cfb6d413f93e6406c3327d2118769121f5655fa9a410956245999a713`
over3158 tracked entries excludingdocs/release. Runtime/content/Cargo bytes compared
identically toca94; new changes are the reviewed probe/transport/tests/workflows.
Cargo confirmed the cached functional binaryd1745d72 (not a new compiler build).
The actual final-source execution passed80.043903930s/peak6205684KiB on the same
disclosed sharedLinux host. Proof15568B SHA256
`fea08eda898c71a043a629eba69df66b53b9429f783d4b2bb8d381e772d53f7a`
differs from the independentca94 proof only in its recorded tested commit; world
independently verified that semantic comparison and exact production fingerprint.
Final execution remains attributed to root, not to the independent earlier replay.

The generator ran again with the final proof. Inventory bytes remain4d3b844e;
exactde2 source/production bindings are now recorded while all nine observations
stay pristine unverified. Actual optimized CLI material preview completed with
schema2 unsigned_preparation.ready=true for evidenceSHA256
`aa8884049a777e540187e767f89a5809a01e63b4ae694605998b1ba4f1c3cc7b`:
82selected files/two packs, preview55727B SHA256
`c0fdfd22a7d4b205b8d2aaefe4c5d0bba3cc165e3be72a0226d0536cbe3287dd`.
Actual logs, generator summary and preview are retained in
`release/v1/verification-v20/`; this authorizes preparation only.

One explicit opt-in push started [probe39916898751](https://github.com/Laymer/Navzeroth/actions/runs/39916898751)
on exactde2. Its guarded native process/ownership and native Git transport steps
completed successfully; individual counts/skips await completed logs. The combined
compilation/recovery step was still in progress when this entry was written; the
workload's start was not independently verified. No workload pass is inferred.
The old signed fixture remains distinct
from current runtime and final-installer acceptance. PR239's remote branch still
holdsV19; the opt-in engineering branch is not a signed release candidate.

Next: actual probe outcome, final independent V20 public review, committed prepared
identities and independent byte reproduction before external-key use. Then signed
closure and one frozen release-candidate push with every applicable fresh gate.
