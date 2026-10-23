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
