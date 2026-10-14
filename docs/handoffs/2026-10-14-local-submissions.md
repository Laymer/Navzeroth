# Local opt-in submissions

Owner: Codex /root/atlas_dense (sole writer). Baseline: published v0.6.0,
739a82229a5b7369cab2a6d83c4b2a935bd0ba2e. Branch feat/local-submissions.
User authorizes isolated parallel feature PRs and granular pushes.

Scope: bounded local review queue, explicit opt-in, closed minimal quest-state,
price and acquisition-outcome declarations, exact immutable dataset/source/evidence
links, CAS decisions, payload-erasing withdrawal and terminal revocation, explicit
permission-gated local JSON export. Local acceptance does not activate content,
upgrade evidence confidence, authenticate consent or upload anything.

Owned: new core/storage/app submissions modules and tests; feature-specific
additive lib/native/SDK/demo registration; ContentScreen child panel and tests.
Root owns migration9/recovery registration; content governance worker owns
content.rs and disposition enforcement. No competing edits to those modules.

Frozen API: schema1 closed Reported values preserve unknown, unsupported,
restricted, stale, false and zero. Canonical document SHA256 is the opaque row ID
and duplicate fingerprint. Safe-integer CAS revisions, 16KiB documents,
1024 total rows including erased tombstones, 32-row pages, 32KiB local export.
Export requires the accepted exact guard and active exact base; recorded source
permission must be Permitted with license/attribution, edition, exact revision,
allowed payload token and local_observation_export.v1 transformation token.
Initial/final consistent reads surround compilation; final handoff preserves
source policy/content hash/CAS. No new exchange format or transport.

Checks: pending implementation. Native/live environment acceptance remains
unverified. Next: normal merge root foundation, implement/test bounded storage,
app service and explicit local UI; record actual checks and limitations here.

## Implementation and reviewable source

Granular commits45905da claim,5fc1f6c pure declarations/policy,41f577d storage/app,
ef30950 feature UI/native/SDK, e13c2bf storage regressions,31f46f5 app regressions,
15d62cb UI/browser regressions,e53ec2d existing Content mock compatibility,
29c38ed disposition-revision guard/docs. Normal foundation merge3abe684 imports
dfcd37b; normal governance API mergeeafa5ef imports9b0a651. Main coordination
normal merge257f97d preserves the published baseline claim and all histories.
Draft PR94: https://github.com/Laymer/Navzeroth/pull/94. No main merge performed.

Core keeps minimal declarations pure and refuses unknown nested fields, duplicate
known JSON fields, future schema, excessive input and successes>opportunities.
No raw snapshot/session/name/chat/credential field is accepted. Storage verifies
full context, fixture, base, registered source/evidence and target citation; every
read revalidates canonical document identity and bounded metadata. Tombstones
retain disclosed fingerprints/base/source/decision/time and consume queue capacity.
Revoked payload erasure preserves terminal revocation. Rows are never deleted to
make capacity available or silently resurrected by a duplicate submission.

Export binds the exact locally accepted guard, active eligible exact base,
manifest SHA, source-policy SHA and disposition revision. Recorded Permitted,
nonblank license/attribution, edition, exact revision and closed payload/transform
policy tokens are all required. Initial/final immediate reads surround bounded
serialization; final reservation spans validation and handoff. Restrict/release/
reactivate ABA refuses a pending export; a newly requested export after review of
the current eligible base remains possible. Declaration timestamps remain nullable
signed-safe values, with no inferred trusted clock/freshness or probability.

The child Content view provides explicit opt-in, bounded queue pages, guarded local
decisions, payload-erasing withdrawal, terminal revocation/erasure and manual local
JSON export inspection. Deferred responses and existing exports are cleared after
queue context/refresh changes. Native additive command registration preserves the
actual-webview reference-only gate and generated typed function-pointer annotation.
Native DTOs are re-exported from the app crate, requiring no new native dependency.
See docs/data/LOCAL_SUBMISSIONS.md for the local format and exact policy tokens.

## Actual checks and limitations

On source29c38ed (main257f97d adds coordination docs only):

- `cargo test --locked -p azeroth-core -p azeroth-storage -p azeroth-app`:563 passed,
  0 failures/ignored, including3 new pure tests,6 storage tests and8 app tests.
  Capacity/pagination exercised actual1024 rows including a withdrawn tombstone;
  corruption bypassed SQL CHECKs and was refused by the typed validator. A real
  second connection tested stale CAS preservation. Deterministic private app hooks
  tested withdrawal/base activation during compile and restrict/release/reactivate
  ABA without sleeps. Unknown/Conditional/NotPermitted and each missing source
  constraint refused export. False/zero, unavailable states and signed/null/zero
  timestamps stay distinct. Log /tmp/navzeroth-submissions-rust-full.log.
- Strict core/storage/app all-target Clippy, fmt and diff checks passed.
  Log /tmp/navzeroth-submissions-clippy.log.
- Desktop156 unit tests across27 files passed; latest affected6 tests passed.
  Typecheck/lint/production build passed. Logs /tmp/navzeroth-submissions-ui-full.log,
  -ui.log, -typecheck.log, -lint.log and -build.log.
- One mocked Chromium opt-in→local accept→source-policy refusal→withdraw→erased
  duplicate flow passed. It is command-boundary mock evidence, not native storage
  or game execution. Log /tmp/navzeroth-submissions-browser.log. Port1420 released
  after the run, verified by ss. Final affected existing Content/browser rerun is
  pending the coordinated next lease.
- Native `cargo check --locked -p azeroth-desktop` was attempted and blocked by
  missing system dbus-1 development metadata before desktop compilation. Log
  /tmp/navzeroth-submissions-native.log. Native interaction/live client acceptance
  remains unverified; no real source permission or observed game data was invented.

Failed development attempts were corrected before passing checks: the first core
fixture builder used the envelope's absent build field rather than the recorded
context; the first app test called a nonexistent constructor; Rust SQL u64 conversion,
and a desktop test unused import/namespace typing were corrected. Those attempts
are not included in passing test totals.

Recovery registration remains root-owned and deliberately fail closed for populated
local_submissions until root calls public
`local_submissions::validate_local_submissions(&SqliteStore)` after canonical content
validation. This branch supplies and directly tests that validator. Next: final
coordinated affected browser checks, independent source review, exact-head hosted
checks and root reconciliation/validator registration before any release.

## Final dependency reconciliation

Normal merge d27a2db integrates governance runtime677f068; four additive conflicts
were resolved by preserving both complete command registrations/client entries/
command-map entries and both Content child-panel mocks. Source ownership was not
transferred and no governance behavior was rewritten. Its final f5798e6 handoff is
normally merged as well. Fresh merged-source checks passed569 core/storage/app
Rust tests (0 failures/ignored), strict all-target Clippy,160 desktop tests,
typecheck/lint/build/fmt/diff. Logs /tmp/navzeroth-submissions-{rust,clippy,ui,
typecheck,lint,build}-final.log. Own feature runtime remains29c38ed.

The final coordinated clean browser rerun passed both affected cases2/2: local
submission/review/refusal/withdrawal/tombstone, and existing Content activation/
planning. Log /tmp/navzeroth-submissions-browser-final-clean.log. It ran before
the final governance UI dependency merge; merged-source browser coverage belongs
to root's forthcoming combined integration run. Port1420 was released and ss
confirmed no listener. Root's Gear/Today browser leases were preserved.

One browser attempt was invalidated by mistakenly starting a normal dependency
merge while its Vite server was starting. That produced temporary merge markers;
the merge was aborted, the failed attempt ended, and the clean sequential rerun
passed. Log /tmp/navzeroth-submissions-browser-final.log records the failed attempt.
It is not counted as successful product evidence. Subsequent merge resolution and
checks were sequential, with no browser server running.

Ready for independent source review and exact-head hosted checks. Root still owns
recovery validator registration and main/release integration; populated recovery
remains deliberately refused until those validators are registered together.

## Registered SourceId bound follow-up

Root requested the existing256-byte core SourceId contract after independent
review reproduced a valid129-byte registered source blocked by the intentionally
frozen minimum128-byte feature limit. Core now relies on the checked SourceId
primitive rather than adding a narrower cutoff; the bounded SQL read accepts256
UTF8 bytes. New129/256 ASCII and256 UTF8 positives plus257/258-byte negatives
preserve source identities. Root owns a separate migration10 widening the column
without rewriting committed schema9. Positive persistence checks depend on that
frozen foundation commit; no migration/recovery code was edited here.

## SourceId dependency verification

Normal merge6509a3c imports root's separately owned migration10 widening source_id
and its preservation/history checks; schema9 is unchanged. On the combined source,
129/256-byte ASCII and256-byte UTF8 registered sources actually stage, submit,
read and deduplicate. Core rejects257/258-byte IDs. All7 storage and8 app submission
cases are included in the fresh full Rust run;4 pure cases passed. Full totals and
strict checks are recorded in the next final check note. Logs
/tmp/navzeroth-submissions-{rust,clippy}-schema10.log. Affected6 desktop tests,
typecheck/lint/fmt/diff passed after full context labels were added to queue rows.
The queue explicitly spans all local contexts; edition/build/interface/namespace/
fixture are displayed before a local decision. No migration/recovery file was
edited by this worker.

Independent read-only quarantine review atf5798e6 passed10 storage+2 app focused
cases and strict Clippy, but found a valid129-byte revision blocked by that module's
128-byte guard reads. Separate external Rust repro confirmed successful staging/
raw read followed by rejected guard/activation. The sole writer fixed all tuple
bounds to256 in71d6983, final8cced745. Independent11-case rerun and original repro
now pass against that exact corrected head. No peer files were edited. Other
fallback/CAS/terminal/immutable-source/recovery-validator checks found no concrete
blocker. Logs /tmp/navzeroth-peer-quarantine-*.log and
/tmp/navzeroth-quarantine-long-identity-{repro,fixed}.log.
