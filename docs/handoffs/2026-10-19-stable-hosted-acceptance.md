# STABLE-HOSTED-ACCEPTANCE handoff

Owner/sole writer `/root/tooling_review`; isolated
`/tmp/navzeroth-stable-hosted-acceptance`, branch
`feat/stable-hosted-acceptance-harness`. Initial baseline `a37abc6`; claim `e1923a9`
preceded implementation. Explicit native/reader prerequisite `084b76b` and source
objective projection `13b5d88` merged normally. No push or real release keys/signing.

Implemented the eight actual hosted case producers, source-compatible manual-entry
native workload, resource/source/baseline checks, bounded offline control/restoration,
exact PID parent memory observations, explicit in-memory startup load/activation
measurements, and stable workflow assembly/producer/evidence/gate wiring. Source
stage and declared travel edge must produce a nonempty native plan and travel hop.
See docs/releases/STABLE_HOSTED_ACCEPTANCE.md for scope and metric definitions.

Found/repaired a startup responsiveness defect while integrating full-pack timing:
initialization held the live host write lock through verification/staging. It now
initializes the private opened application and publishes once; checking status stays
readable and operations refuse partial state. A controlled channel test proves this
without a hanging test. Legacy native smoke's default 30-second readiness allowance
is unchanged; stable full-bundle workflows explicitly select 20 minutes. Existing
RC/demo goldens and historical acceptance evidence were not regenerated/promoted.

Actual checks on final implementation:
- Portable continuous LLVM app all-target run: 274 passed. Production LCOV with the
  existing cfg(test) exclusion: bundled_content.rs 245/254=96.46%; recovery.rs
  415/452=91.81%. No coverage threshold/inventory relaxation. Pinned llvm-cov0.9.1,
  Linux relocation + continuous profile settings from repository collector.
- Desktop full suite: 541 passed; existing per-file 90% gate passed. Changed
  RecoveryPanel: 36/39 statement-start lines=92.31%. TypeScript typecheck and ESLint
  passed using installed pinned tools. Dependencies were reused by local symlink;
  pnpm's attempted automatic install rejected a non-real hoist directory, then
  commands ran; direct tool invocations also passed. No dependency files modified.
- Node native-smoke/command-boundary/workload regressions: 26 passed. New workload
  tests alone: 8 passed, pure workload library97.01% line coverage. Actual Windows
  driver body remains environment-dependent/unverified, not a portable native pass.
- Native Windows PowerShell5.1 mocked producer tests:10 passed; existing phase/
  preparation tests:42 passed. New scripts parsed with native PS5.1 parser. Real
  adapter cmdlet parameter metadata inspected read-only; no adapters changed.
- App all-target Clippy -D warnings, format/diff checks, workflow YAML unique-key
  parse and JavaScript syntax checks passed.
- Root independently read WIP code with no blocker, ran8 workload tests and3 Rust
  startup tests. Root will record exact reviewed source/commit after this handoff.

Initial failures are not passes: a wrong local pnpm binary path was corrected;
focused Vitest coverage correctly failed the whole-source per-file gate, then the
full suite passed. A missing-backup mock initially reused deliberately corrupted
recovery state and got the correct settings-refusal first; independent mock fixed
that test. No production gate was weakened to make a synthetic case succeed.

Actual hosted Windows install/start/upgrade/downgrade/resources/persistence/recovery/
offline/performance are UNVERIFIED here. PowerShell7 runtime, native CDP/body and
large actual catalogs require the hosted run. Baseline network download, adapter
control and installer execution were not performed. Signed bundle assembly CLI and
dynamic installed-tree verifier are root-owned prerequisite changes, not supplied
by this branch. No owner smoke, signature issuance or publication occurred.

Next: independent final-head review and root bundle-binding integration; adapt the
native coverage collector to the verified assembled bundle/identity (stable source
must not bypass missing-build-identity refusal). Then run fresh source checks,
hosted full-pack cases, independently downloaded draft verification and owner
smoke. Source/code changes invalidate an earlier installer smoke result.
