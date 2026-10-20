# Historical acceptance-kit fixture and full packaging-helper audit

Owner /root/desktop_review; sole writer in fix/stable-legacy-acceptance-fixture,
/tmp/navzeroth-stable-legacy-acceptance-fixture. Baseline
 df62631fae0c6ba4e28808d321648f109fd40e2a. Code repair
9f2d57bfd88df71278f656ff6c436e18dd612b74.

Actual package39398642678/job118547546426 passed the repaired reader/exit steps
but failed guided acceptance-kit tests after two tool-root assertions: the test
read the current28MiB schema4 inventory through a16MiB generic reader. This was
not a reason to enlarge the historical helper's contract. Acceptance-Kit supports
only schemas1–3 and their original15live/10Windows or0live/10Windows runbooks;
schema4 has separately defined hosted/owner methods and producers.

Confirmed routing before editing: the actual stable package workflow invokes
Stable-Acceptance.ps1, not the historical Acceptance-Kit/Start-AcceptanceSession.
Only the legacy helper test borrowed the mutable current release. Production
Acceptance-Kit behavior is unchanged (only scope comment clarified); its16MiB
limits, schema/method refusal and unreviewed draft semantics remain intact.

Tests now own a bounded nine-domain/nine-unit synthetic legacy candidate and copy
the historical runbook. All schemas1/2/3 retain exact34/34/19 procedure counts,
including every unit; schema3 queues no live procedure. Schema4 explicitly refuses
with the exact unsupported-schema reason, not an unrelated size failure. Stale/
missing scenario coverage, evidence requirements and addon ZIP integrity checks
remain. New stable documentation points to the separate schema4 producer and exact
owner installer smoke rather than changing historical acceptance rules.

Actual native Windows PowerShell5.1.26100.9711, exact GitHub command-wrapper form:
- Repaired legacy suite21checks passed/exit0. An owned fixture with deliberately
  wrong schema1 procedure accounting failed its assertion/exit1; failure is not
  normalized away. Root independently reviewed exact9f2 and ran21checks/exit0.
- All12package helper suites ran in workflow order, each in a fresh wrapper that
  propagates LASTEXITCODE. All238assertions passed; every process exited0:
  addon30, release gate29, material transport19, contract bounds23 (default mode),
  candidate14, preparation42, hosted helpers10, workflow gate12, legacy kit21,
  native smoke helpers10, unsigned policy17 and synthetic signing11.
- Sequence/code identity, per-suite log hashes and result summaries are retained
  in release/v1/legacy-acceptance-helper-sequence.json. This default bounds run
  does not use ActualRoot or pretend current signed evidence still has an earlier
  fixed hash; earlier immutable actual-root29checks remain separate observations.

For synthetic tests that require Git, a task-external native executable forwards
actual Git operations through WSL because Windows Git is not installed locally.
PowerShell/file/ZIP/synthetic process execution is native; no product gate responses
were mocked beyond the suites' documented stubs. No real installer, network adapter,
live client, private export, key or signature was accessed. PowerShell7 and actual
hosted installation/performance remain pending despite portable/helper success.

Comprehensive consumer audit found no other current large inventory/evidence read
in package helper tests after this legacy coupling. Independent tooling also ran
native smoke10/policy17/signing11, first-start8 and40Nodechecks, with no additional
mutable-inventory helper coupling. No new source, corpus, pack, policy, version,
receipt or acceptance observation was changed. Actual failed delivery package
remains failure; the helper sequence is not stable Windows acceptance.

Next: independent final repair review, separate delivery PR and normal merge,
fresh exact-source protected checks and package run. Hold final keys/receipt
renewal until the production repair union is settled; do not restamp provisional
functional or material evidence onto a later source identity.
