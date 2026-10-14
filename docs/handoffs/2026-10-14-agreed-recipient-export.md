# Agreed recipient export

Owner: Codex /root/atlas_dense, sole feature writer. Claim 8808571 precedes code;
released baseline f 525911/v 0.5.0. Root explicitly amended ownership to include
recipient-specific additive app/native/SDK/demo registration and a scoped storage
consistent-read helper. Root's guard helper 47ef 7d 0 was normally cherry-picked as
648f 614; normal main cb 7e 03d merge 3f 127f 0 retains both histories. Root owns other
features, existing guard implementation and final integration/releases.

API: export_agreed_party_plan(text, now) -> PlanExport. Request JSON is bounded
4096 bytes and denies unknown fields: party_id, exact expected ProposalGuard,
recipient_profile_id (nonempty, at most 128 bytes). No frontend report is trusted.
The app reads the stored proposal, requires explicit acceptance keys exactly equal
to every participant including proposer, checks exact CAS identity/revision and
recipient membership, and captures compatible current cohort/snapshot identities,
fixture and active revision. It recomputes the original stored joint request,
compares full result SHA-256, selected objective order/total/context/fixture, and
compiles the actual joint choices without invoking a solo optimizer.

Schema 1 ANAV1 retains all cohort snapshot dependencies as provenance, but each
step applies exactly to the requested recipient character. Other input characters
have no applicable steps. Original selected order, routes/stopping, alternatives,
manual contributions, exclusions, time assumptions and actual edge/objective/quest
evidence IDs are preserved. Each original choice explanation identifies its
minimum-member utility; it does not aggregate that into a cluster priority.
Explicit ParticipantState predicates are never rewritten: missing/restricted remote
facts stay Unknown in Lua. Local unqualified predicates remain local. The text
explains that imported cohort readiness is not live party synchronization or
remote authenticated consent. No exchange/dataset identity changed.

Plan/step identity includes exact proposal/revision/recipient, canonical original
joint request/result, context/fixture, cohort snapshot hashes, content hash and
joint ruleset/model identifiers; generation time is excluded for stable retries.
Expiry is the minimum of generated+30 minutes and the guard's earliest finite
inclusive typed-class/capture upper bound, used conservatively as exclusive expiry.
No finite bound or missing trusted generation time is refused. Limits preserve
32KiB transport,12-deep/1024-node rule limits,8192 decoder nodes,4096-byte strings,
8 alternatives and collection/depth bounds. No truncation. Acquisition, stateful
travel and unavailable required gates are refused if not faithfully representable.

Capture and final validation each use a SQLite IMMEDIATE reservation in addition
to the app mutex. The new with_immediate_read helper excludes other connections'
writes through the read/handoff, forbids nesting with a typed safe StorageError,
and explicitly rolls back rejected callbacks; rollback/commit errors map safely.
Compilation runs outside both locks. Final publication rechecks exact complete
proposal digest/acceptances/CAS, then the existing guard's current snapshot hashes,
membership/content and typed freshness classes/late clock, then exclusive export
expiry, before constant-time handoff. This does not claim hostile-process
confinement. Other preexisting planning paths are unchanged by this task's helper.

The PartyAgreement UI requires an agreed state and explicit recipient selection,
shows manually copyable bounded text and expiry, and clears it on selection,
agreement mutation, refresh, client/party changes or backend refusal. Late async
responses cannot reinstate obsolete bytes. Backend export failure refreshes the
authoritative agreement; failed refresh disables the stale state.

Actual checks, source 58b 39a 1 plus focused browser testc 5d 2d 04/main merge 3f 127f 0:

- cargo test --locked -p azeroth-app -p azeroth-storage -p azeroth-lua-harness:
  315 passed,0 failures. Includes two prepared/final-phase regressions (withdrawal
  and independent field expiry after compilation), eight initial export cases,
  two cross-connection/nested/rollback transaction cases, two actual app-produced
  Lua cases and the existing suites.
- Added stored proposal context/fixture/result corruption case at 8d 54261:
  affected recipient integration suite 9 passed; strict all-target app/storage/Lua
  Clippy passed again. Formatting and whitespace checks passed.
- Desktop full 135 unit tests passed (four new export cases), typecheck/lint/build
  passed. Final focused browser source typecheck/lint passed.
- Focused proposals.spec.ts Chromium 1 passed: exact current guard/recipient/native
  argument, manual export bytes, refresh removes obsolete bytes/disables export.
  Browser server stopped; port 1420 confirmed free.
- Native cargo check attempted and blocked by missing system dbus-1.pc/libdbus;
  native build/runtime interaction and live game operation remain unverified.

Local logs: /tmp/navzeroth-recipient-full-rust.log, -final-affected.log,
-final-clippy.log, -ui.log, -browser.log and -native.log. Rust commands reused
--target-dir /tmp/navzeroth-peer-planning-a 4c 7a 93/target only for compilation.
Earlier fixture/check attempts were corrected before these results: private error
field access and moved include paths/alias, unchanged snapshot sequence produced a
history-only import, unresolved acquisition reference, large repeated rule hit
existing evaluation step/reward-dedup limits, and raw empty party rows were invalid.
Final tests use proper sequence 13, supported long preference predicates and an
isolated SQL probe table; no failed run is reported as a passing check.

Next: root exact-source/security review and all fresh task CI checks, integration
with the separately owned clustering/route-acquisition batch, and combined checks.
Native/live limitations require external evidence; no tag/release is created here.
