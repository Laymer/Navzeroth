# Disposable hosted Windows offline adapter probe

Owner tooling_review, claim33eb1b6b, baseline764fcac347f0c75ee628df619e9bd224ebf00b6f.
Frozen code e986f955022cfb8d82988a43e80c4afdd7bbec29. Only a NEW workflow, wrapper
and mocked wrapper tests; desktop separately owns the real helper/tests. Before
actual execution, normal-merge independently reviewed final helper4d9a000d.
The Invoke-StableOffline API is unchanged; controlled adapter identities are now
canonical GUID strings. No protected workflow/gate/job name, product build,
resource, release contract, receipt, budget or runtime is changed here.

The probe calls the real helper with a tiny local marker workload that independently
observes zero Up adapters, including hidden adapters. It checks the exact original
Up GUID set, strict returned disable/restore flags, completed marker and fresh
post-return per-GUID Up status. No previously-down adapter is enabled by the probe.
Helper cleanup stays in its own finally block. Probe output failure cannot mask
the primary helper error. Its JSON engineering report is at most64KiB; errors are
bounded; unestablished network/restore flags remain null. No case/owner observation
or acceptance success is produced, even when this engineering probe passes.

The wrapper refuses ordinary/self-hosted environments before control calls; it
requires disposable GitHub-hosted Windows metadata, exact issued Git SHA and clean
checkout, and a new output directory below RUNNER_TEMP. This is not a filesystem
sandbox or a guarantee of restoration after forced process/job termination.
The distinct nonprotected job has a finite10-minute outer limit and concurrency
cancellation disabled so another push cannot intentionally interrupt restoration.
Only contents:read; no secrets, elevation workaround, signing, install, acquisition,
account/quota change or product workload. Small diagnostics upload is attempted
always after the helper returns; lost connectivity can prevent upload and does
not establish a pass.

Actual checks at e986:11wrapper checks PASS on LinuxPS7.6.6 and native WindowsPS5.1.
ALL adapter operations are mocked; none controlled this WSL/Windows host network.
Native PS5 uses an external read-only WSL Git bridge because native Git is absent;
its initially blocked script execution was retried with process-only ExecutionPolicy
Bypass. Existing VSCode path-translation warnings did not fail checks. Source/run
identity, positive marker/restoration, wrong GUID/duplicate/typed flags, unrestored
post-state, no Up adapter, self-hosted refusal, wrong SHA, no-clobber output, and
report-write/primary-error preservation are covered. Logs:
/tmp/navzeroth-offline-probe-mock-{ps7,ps51}.log. Actionlint and diff checks pass.
Root independently ran11checks/actionlint and reviewed code. World independently
ran11checks/actionlint and an external interaction test with the REAL final helper
and REAL wrapper but MOCKED CIM controls: duplicate descriptions do not collide,
originally-down adapters remain untouched, zero-Up marker and restoration pass.
No actual hosted adapter, offline product or installer result exists yet.

GitHub documents that workflow_dispatch requires default-branch registration:
[manual workflow documentation](https://docs.github.com/actions/managing-workflow-runs/manually-running-a-workflow).
A push event uses workflows in its event commit and can run unmerged workflows:
[push documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#push).
Therefore the NEW workflow has only opt-in push branch verify/offline-adapters-*
and future workflow_dispatch triggers. Normal repair branch pushes cannot run it.
Root alone explicitly pushes a reviewed combined helper/probe commit to a UNIQUE
verify/offline-adapters-<sha> branch. No main registration merge or protected gate
bypass is needed. No actual trigger, retry or network control was executed here.

Next: root final Git-blob review, normal helper/probe union in its isolated task,
then the explicitly authorized one tiny hosted probe. Inspect actual primary and
per-GUID restoration diagnostics before any next full signed candidate. V16's
original offline failure remains failed; its five prior cases and recovery timing
retain their original source. Missing RC1 upgrade/downgrade/owner results remain
unverified. Do not infer that GUID addressing proves the original hidden cause.
