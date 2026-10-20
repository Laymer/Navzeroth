# Workflow gate test success exit under the actual GitHub wrapper

Owner /root/desktop_review; sole writer in fix/stable-workflow-gate-exit,
/tmp/navzeroth-stable-workflow-gate-exit. Baseline
f02964ec4af98a5b8dbc79e89362acfbc0fac387. Code repair
a74e16042245481f3f12efcf85cda83aff3c924d changes only the terminal three lines of
scripts/windows/Test-StableWorkflowGate.ps1 (comment and exit0).

Actual package39397316809/job118543149504 progressed beyond the repaired generic
reader but failed its next native PowerShell5.1 workflow-gate step: all12assertions
printed PASS, then the GitHub wrapper exited1. Expected negative verifier tests
leave global LASTEXITCODE nonzero even when their expected-refusal assertions pass.
The script previously never normalized its successful terminal process status.

Reproduced with native Windows PowerShell5.1.26100.9711 and the actual GitHub form:
- powershell.exe -Command dot-sources a runner wrapper; the wrapper calls the test
  script and propagates LASTEXITCODE as GitHub does.
- Old unchanged f029 script: all12checks PASS, process exit1, matching hosted CI.
- Repaired script: all12checks PASS, process exit0.
- Owned fixture with deliberately wrong first gate expectation: assertion throws,
  process exit1, terminal success is unreachable.
- Owned fixture with controlled finally/cleanup failure after resource removal:
  process exit1 even after PASS12printed; terminal success is unreachable.

A preliminary direct-File/dot-source wrapper exposed the raw stale exit9 (the
negative verifier's selected code) rather than GitHub command-mode's final1.
The first local harness expected1and rejected that raw9; switching to the exact
GitHub -Command shape reproduced1. Neither form is a passed test status before
the fix. No error preference, gate assertion, workflow implementation or exception
handler was loosened. Success exit executes only after assertions and finally
cleanup complete. Existing Test-ReleaseGate uses this same terminal-exit pattern.

Read-only audits of remaining helper steps through the same GitHub wrapper:
Test-StablePreparation42checks passed/exit0; Test-StableAcceptanceHelpers10checks
passed/exit0. No changes to those files. Whitespace/diff check passed. PowerShell7
is unavailable locally; actual hosted packaging remains required. These are
synthetic verifier/helper tests, not installer or hosted acceptance observations.

Logs outside Git: /tmp/navzeroth-workflow-gate-command-{before,after,assertion-failure,
cleanup-failure}.log; owned fixture trees under /tmp/navzeroth-workflow-gate-exit-repro.
No Git bridge, installer execution, source/packs/policy pins, other-owner coverage
files, keys, signatures or acceptance statuses changed. Previous package failure
remains actual failure; new checks do not relabel it.

Next: independent review and separate delivery repair PR, normal merge and fresh
all10checks/source/platform/package verification. This test-script change affects
the production fingerprint; final review/receipt/source bindings must be renewed
only after the complete repair union. In-progress earlier full-pack checks remain
historical/provisional; do not restamp them onto this candidate.
