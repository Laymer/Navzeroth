# Hosted content preference persistence repair

Owner/sole writer `/root/world_review`; baseline
`ab16316bfbc1038976526489a847942527c3c408`; claim `6ef24cf3`.
Branch `fix/stable-hosted-persistence`, isolated worktree
`/tmp/navzeroth-stable-hosted-persistence`. Only workload library, dedicated tests,
state contract and this task's claim/handoff changed. Workflow wiring and material
reader tasks have separate owners.

The old workload disabled its defaults check once saved.active_revision existed;
it saved neither inferred preference nor defaults marker. Recovery similarly
checked context only. Controlled regression tests against immutable baseline code
accepted reset preferences/recovery changes (two Missing expected rejection
failures); both fail closed after this repair.

Use actual defaults implementation609f7624500fbf4869408758ef25ef9b9fcfd0b0
(PR223), not the standalone content-functional rehearsal which lacks that merge.
Its get_active_content exposes bundle_defaults{app_version,previous_app_version,
revision} and accepts_inferred_travel. Existing activate_content_set with the
single current revision preserves identity and commits the selected preference.
No new native API or source/game fact was invented.

[Workload state2](../releases/STABLE_HOSTED_WORKLOAD_STATE.md) saves/checks exact
revision, boolean inferred choice and all defaults fields. Fresh/retained RC upgrade
require true/current defaults first; normal workloads leave false selected and
check false plus full marker before restart/recovery/offline work. Recovery checks
again immediately after restore. Repeated engineering planning explicitly opts in
only after those persistence checks; finally restores false and verifies identity
and marker. Its disclosure distinguishes this temporary opt-in from the saved
choice. Failure produces no successful result. Version1, missing selected fields
and malformed baseline fallback are refused. No product persistence implementation,
acceptance status, source pin, generated content or workflow was modified.

Actual checks:
- Dedicated Node suite14pass0fail, library97.56% line/91.41% branch/100% functions
  using node --test --experimental-test-coverage. Includes false across restart,
  restore and offline, preference loss/reset, all marker/revision changes,
  fresh/RC-upgrade true defaults, old/missing state, ignored writes, changed toggle
  identity/marker and restoring false after controlled planning failure.
- Combined existing native smoke, command-boundary and workload suites32pass0fail.
- node --check and git diff --check passed.
- Controlled baseline copy with two new regression tests:0pass2fail, each Missing
  expected rejection. Temporary copy outside Git; source trees unchanged.
- An initial new upgrade-negative mock started with mismatched settings and
  correctly failed the earlier settings guard. Corrected the fixture to use its
  own retained baseline; final test now reaches the intended defaults refusal.

Logs: /tmp/navzeroth-hosted-persistence-{tests,combined-tests,before-tests}.log.
Native driver/Windows bodies are not covered by the portable library percentage.
No actual Windows installation/restart/recovery/offline run, signing, push or
acceptance promotion occurred. Next: independent root review, integrate with PR223
and hosted workflow, then exercise exact installed application on hosted Windows.
