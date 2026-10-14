# Wave7 Today / Settings and guarded current-plan reference

Owner: Codex `/root/backlog_audit`; claim8631ea9 recorded before code.
Released baseline739a82229a5b7369cab2a6d83c4b2a935bd0ba2e,
v0.6.0 published2026-10-14T12:49:07Z (parent publication evidence).
Worktree `/tmp/navzeroth-wave7-today`, branch `feat/today-settings`.
Root owns schema/recovery integration; this task owns feature modules, job hooks,
additive command/UI registration, and the authorized disposition guard binding.

## Changes and decisions

- Unchanged solo/joint manual request DTOs now live in storage::today, with existing
  app public aliases. Recovery and calculation share one parsing model.
- Bounded singleton settings/current-plan storage, strict versioned parsing and
  monotonic CAS/tombstone clear. Local paths and manual defaults are explicit.
- Separate server-owned job pin descriptor preserves one-shot `take_result`.
  Successful publication, cancellation and pruning retain the existing registry
  lifecycle. No raw snapshots/Lua/full report duplicate is persisted.
- Original-clock guard reconstruction, full ordered cohort/snapshot/content
  identities, and original content eligibility counter. Existing synchronous
  planner/export/route adapters retain signatures via guarded internal callbacks.
  Recipient export and preparation bind disposition at consistent capture too;
  even all-stale explanatory preparation validates its original content counter.
- Empty/historical/current Today, explicit solo/joint pin controls, Settings,
  explicit application of planning defaults, and readonly bounded reference plan.
  Reference shows8 of up to128 original steps with explicit truncation.
- Public typed recovery validators plus `mark_recovered_plan_guard`. A recovered
  reference cannot be current merely because hashes still match; no automatic
  calculation. Coordinator must register/transform these before release.
- Presentation keeps missing clock/age, unknown observations, false and zero
  distinct. Fixed recorded context/revision/input references accompany advice.
  Local expiry suppresses next-action text, and calendar-unrepresentable safe
  integer timestamps remain inspectable as UTC milliseconds.

See [local intent contract](../data/LOCAL_INTENT.md).

## Reproduced correction

At merged50bc1c4, new deterministic test
`content_restrict_release_reactivate_aba_refuses_original_publication_and_pin`
failed: original `PublicationGuard.publish` returned success after quarantine,
release and explicit reactivation of the identical key/hash. Log
`/tmp/navzeroth-today-aba-baseline.log`. Fixed0ca8a83 retains/validates the original
`DispositionGuard`; saved descriptors do not reconstruct a current counter.
The regression then passed and leaves existing saved slot/revision intact on
failed replacement. Final guard tests also cover supplied late clock checks.

Normal merges preserve schema9 and final content governance dependency, including
8cced745's established256-byte identity boundary correction. No competing edits
to migrations/recovery/content governance modules were made.

## Actual checks and failed attempts

Backend frozen d2b8cf4432b498fabb5bc3ad15f9fe031e7ca86f:

- `cargo test -p azeroth-app -p azeroth-storage`:285 passed, no failures;
  `/tmp/navzeroth-today-rust-checked.log`.
- New Today integration11 passed: pin after one-shot read/restart, explicit local
  intent/unknown goal/false/zero, CAS/ABA, failed/canceled/foreign/pruned jobs,
  joint reversed request order/membership without acceptance, changed/deleted
  snapshot, original disposition counter, field expiry/future transitions/rollback/
  missing clock, recovered reference, bounds/future/nested fields, synthetic wire.
- Four planning guard units passed. Strict app/storage all-target Clippy,
  formatting and diff checks passed; `/tmp/navzeroth-today-clippy-checked.log`.
- Synthetic Today wire fixture uses fixed declared UTC and mathematically derived
  ages, rather than measured execution latency. Normal test compares the complete
  fixture. Explicit UPDATE_TODAY_GOLDEN/UPDATE_REFERENCE_GOLDEN regeneration ran;
  three reference tests pass with additive null plan/accessibility metadata.
- Frontend focused Today/Settings10 passed, then explicit Imports bridge added
  three no-side-effect/manual-edit/client-replacement cases:17 affected tests
  (TodaySettings13 + existing Imports4) pass. Typecheck/lint pass.
- Focused Chromium7 pass: Today/Settings3 at320px100/200% text plus historical
  advice; navigation4 at320/640px and16/32px text, all14 buttons/keyboard/reflow.
  `/tmp/navzeroth-today-browser-final.log`; server stopped/1420 released.

Earlier checks are not promoted: initial profile validator incorrectly assumed
64hex without actual `stored-` prefix; corrected after existing diagnostics and
new feature tests rejected selection. Test harness referenced nonexistent app
delete_profile; corrected using actual storage deletion in an isolated temp DB.
Original50ms final-clock unit setup expired under concurrent loaded checks before
its intended assertion; widened setup margin and supplied clock advance, preserving
the exact late-class regression. Initial fixture/UI typecheck, unused rule/comment,
Clippy formatting/range/conditional/test-initializer findings were corrected.
Two browser Settings cases initially timed out using exact `getByLabel` select
text; accessibility snapshot showed valid combobox names. Correct exact role
selectors passed all7; original log `/tmp/navzeroth-today-browser.log` is retained.

Final frontend bridge: explicit saved-file choice applies only editor inputs and
rejects old replies after manual edit/client replacement/unmount. Full desktop169
passed across28files; typecheck/lint/production build pass at final frontend source.
Log `/tmp/navzeroth-today-desktop-checked.log`. Additional exact-filtered Imports
Chromium1 passed (explicit path/format fill; no import/watch/window), log
`/tmp/navzeroth-today-import-browser.log`. Together affected browser evidence is8
cases, not a claim of a full browser rerun. Port1420 is stopped/free afterward.
Backend source remains d2b8cf4; final frontend source is recorded by the pushed PR.

Release version changes require explicit UPDATE_TODAY_GOLDEN regeneration too:
the fixed-clock Today fixture includes the versioned pure ReferenceView app version.
The fixture compares complete output, not an excluded nondeterministic field.

Native interactive Windows, native DPI/focus/
monitor behavior, ordinary-machine performance and live game checks are UNVERIFIED.
Next required integration: root typed recovery registration/recovered transform,
protected-main reconciliation and exact-head hosted gates before release. Real
catalogs, role models, Deadmines and official API evidence remain external blocks.
