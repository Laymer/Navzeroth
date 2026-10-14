# Wave7 durable local content restrictions

Owner: Codex `/root/release_review`, sole feature writer. Claim `a0fca1d` before
code at published v0.6.0 `739a82229a5b7369cab2a6d83c4b2a935bd0ba2e`.
Branch feat/content-dispositions; /tmp/navzeroth-wave7-quarantine.
Frozen implementation/tests `677f068deea95b0be863f95ca73ab0f3c7f81697` after a normal
protected-main coordination merge. Schema9 foundation `dfcd37b` was merged normally
only after root released content.rs ownership. Root owns migration/recovery source;
this feature does not modify either. Root reconciles additive feature command wiring.

## Delivered behavior

Immutable revisions have durable eligible/quarantined/revoked dispositions.
State changes compare the exact revision key, immutable content hash, current state
and safe-integer decision counter inside one SQLite IMMEDIATE transaction. Every
actual transition increments the counter, retaining restrict/release ABA history.
Revocation is terminal. Quarantine can explicitly return to eligible only with
operator_review; release never activates content. Repeated/same-state operations,
stale guards, invalid reasons and exhausted decision counters are refused.
Changed timestamps are optional signed safe UTC values; no clock is invented.

Restricting the active revision first blocks it in that same transaction, then
considers only previously activated eligible candidates in the exact edition,
build, interface, namespace and fixture scope. Candidates are fully revalidated
against their immutable document, manifest, source records and coverage. Malformed
candidates are skipped. Tied activation ordinals never invent chronology; ambiguous
candidates are skipped and strict recovery validation refuses that history.
Staged-but-never-activated revisions are never automatic fallbacks.

No usable fallback clears the active pointer and commits the restriction. Exhausted
activation ordering also commits the restriction and clears active content rather
than rolling back the safety decision. Genuine SQLite failures roll back the whole
transaction. Typed LoadFailure distinguishes malformed retained data from real SQL
failures; private database errors are not classified by message text. Direct
activation and rollback refuse blocked content, and active reads return unavailable
for missing/blocked disposition state. Healthy active reads and usable guard checks
retain a SQLite reservation throughout validation. Duplicate staging never resets
restrictions. Raw immutable documents, evidence confidence and source rights stay
unchanged. This does not authenticate hostile filesystem/database modifications.

## Frozen APIs and integration boundaries

`azeroth_storage::content_dispositions` exports:

- ContentDisposition: full key, content_sha256, state, state_revision, closed
  reason_code, optional changed_at_utc_ms and optional last_activated_order.
- `.guard()` → DispositionGuard {key, content_sha256, state, state_revision}.
- SqliteStore::content_disposition(key), list_content_dispositions(scope).
- usable_content_revision(key) → missing None or eligible (ContentRevision,
  ContentDisposition); blocked/invalid data returns an error.
- validate_usable_content_guard(guard) → fresh ContentRevision or stale/blocked
  error. It reuses an existing immediate reservation instead of nesting one.
  Callers must retain the reservation through their final bounded handoff.
- change_content_disposition(mutation, optional_now) → disposition, nullable active
  key and fallback_applied. Guarded release does not autoactivate.
- public validate_content_dispositions(&rusqlite::Connection) → Result<()> strict
  recovery hook. Root registers this hook in its separate shared recovery task;
  registration is a remaining combined-release prerequisite, not claimed here.

Application RPCs: get_content_dispositions {profile_id} returns schema1 scope,
nullable active key and bounded revision descriptors. change_content_disposition
{text} parses at most8192bytes, schema1, deny-unknown-fields request
{schema_version,profile_id,mutation:{guard,state,reason_code}}. The profile resolves
the complete context/fixture scope; foreign decisions are refused before writing.

Content UI adds fixed reason choices, explicit confirmation and CAS controls.
Quarantine release and permanent revocation are explained; retained revisions and
never-activated status are visible. Changed profile/client, refreshed data and stale
failed decisions invalidate actionable descriptors. Late old replies are ignored.
Existing staging/activation controls are disabled while a decision is active.
Demo mutation services remain unavailable. Native command registration remains
inside the existing typed generated dispatcher and preserves the actual-reference
webview gate; reference can still invoke only get_reference_view.

Root assigned Today/PublicationGuard and submissions workers to capture/validate
these counters so async handoffs and saved descriptors reject restrict/release/
reactivate ABA. Those integrations are separate tasks, not edits by this worker.

## Actual checks at frozen source

- Full `cargo test -p azeroth-storage -p azeroth-app`: 267 passed,0failed,0ignored,
  39 targets including empty doctests. Log /tmp/navzeroth-wave7-quarantine-final-rust.log.
- `cargo clippy -p azeroth-storage -p azeroth-app --all-targets -- -D warnings`,
  `cargo fmt --all --check` and git diff check passed.
- New10 storage cases plus1 policy unit exercise exact guards, independent-process
  stale CAS, restart and ABA, permanent revocation, release/noactivation, retained
  immutable bytes, fallback order/unknown/bad candidates, never-active exclusion,
  foreign-build fallback exclusion, rollback refusal, safe counters/ordering,
  genuine SQL rollback and actual online-backup/reconstruct preservation.
  Strict hook tests reject blocked active, missing disposition and ambiguous order.
  Existing context/fixture baseline tests remain in the full passing suite.
- New2 application cases cover bounded/future/unknown-field/foreign-context/reason/
  unsafe-counter requests, cleared active state, stale decisions and explicit release.
- UPDATE_CONTENT_DISPOSITION_GOLDENS=1 generated
  fixtures/desktop/content-governance/synthetic-flow.json from actual application
  services; normal tests verify the same projection. No personal database was used.
- Desktop full156 tests/27files, typecheck/lint/build passed with pinned pnpm12.11.2.
  Logs /tmp/navzeroth-wave7-quarantine-final-{unit,typecheck,lint,build}.log.
- One focused command-mocked Chromium Content flow passed with the actual Rust
  golden: no automatic decisions, full guard requests, quarantine then release with
  active content remaining absent, no activation calls, stale failure clears the
  actionable guard and hides raw error text. Log /tmp/navzeroth-wave7-quarantine-browser.log.
  The server stopped; ss confirmed no1420 listener; lease released to root.
- Separate read-only foundation review at fc87b93 found no blocker and independently
  passed all6 local_state_foundation cases. Log /tmp/navzeroth-wave7-foundation-peer.log.

Development corrections: initial SQL prototype used u64 directly with rusqlite;
changed to checked i64 reads/safe casts. Strict Clippy required a raw-row alias and
moving unit tests last. An existing Content mock lacked the new getter and returned
INTERNAL instead of the expected staging error; added truthful empty governance
response. Test non-null assertions and closure narrowing were corrected. Two
corruption simulations were initially stopped by immutable-update triggers; tests
explicitly remove that trigger only on synthetic databases to simulate damaged
records. A mistyped in_memory test API was corrected to open_in_memory. Foundation
peer verification temporarily reused this worker's own build directory; returning
then picked an older storage rlib. Cleaned only this worker's storage outputs and
rebuilt exact feature source. Final checks use the isolated feature target and all
failed attempts are excluded from pass claims. Earlier broad checks preceded the
final ordering fix; final267 run covers the frozen source.

## Remaining checks and next work

Root recovery hook registration and Today/submission publication guard integration
must be reconciled before release. The feature does not repair corrupted documents;
startup recovery still requires a compatible fully validated backup. No native
window, installed restore or live game checks were run for this task. Local native
compilation remains unverified (the environment previously lacks dbus-1.pc);
all nine fresh hosted checks, including actual Windows jobs, are required on the
reconciled exact feature head. Local decisions do not establish source permission,
authenticity, confidence or verified gameplay coverage. No upload/network/account
creation, imported SQL/Lua execution or game bridge was added.

Next: independent source review and shared integration, fresh exact-head hosted
checks, protected-main reconciliation and the next versioned preview candidate.

## Peer correction: valid long identity tuples

Peer Atlas demonstrated an actual compatibility defect at f5798e6/source677f068:
a valid129-byte DatasetRevision stages and raw-reads, but disposition reads and
activation fail because metadata SQL used128-byte limits. Root normally merged
foundation main into8a23b42 before returning sole fix ownership. Claim55288e5
preceded the repair. New fulltuple regression reproduced the failure against the
unchanged SQL; log /tmp/navzeroth-quarantine-identity-baseline.log.

Fix71d6983f65823d46e0dad156bb413d18ec23977d raises only the four identity byte
bounds (edition/build/interface/revision) to the existing core256-byte contract,
retaining SQL CASE byte checks before allocating strings. Namespace/state/reason
bounds and all policy/API behavior are unchanged. Actual regression stages and
reads full tuples using129/256 ASCII bytes and256 UTF8bytes, activates/reads active
content, then restricts it. Core rejects257 ASCII bytes and258 UTF8bytes. This
uses valid synthetic documents without corrupt SQL. All11 disposition storage
cases passed; /tmp/navzeroth-quarantine-identity-fixed.log.

After the committed source repair, full app/storage checks passed274 tests,
0failed/0ignored across40 targets (including foundation6 and empty doctests), and
strict all-target app/storage Clippy plus workspace fmt/diff checks passed. Logs
/tmp/navzeroth-quarantine-identity-{full,clippy}.log. Desktop/native behavior did
not change; earlier frontend evidence remains scoped, while all nine fresh hosted
checks on the corrected exact head are still required. Recovery/schema files were
not edited by this worker. Source ownership returns to root after the pushed fix.
