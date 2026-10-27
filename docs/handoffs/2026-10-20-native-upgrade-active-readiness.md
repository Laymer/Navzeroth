# Native synthetic upgrade readiness repair

Owner desktop_review; isolated baseline ee26d119b8fc63454771e963cfd28ae6cb1b6aab.
Claim39e25512; code9a817375. No runtime/application UI, workflow, source packs,
canonical release bindings, private keys or acceptance statuses changed.

Actual V11 package39441834744 at PR merge e3fdd58a9fe969f972d2796c47f4b114e5b0d661
failed the blocking native regression gate: candidate smoke32 passed, upgrade6 passed/
1 failed (`Active content was lost`). Eight stable cases and recorder never ran.
Only the2.6MiB native-smoke artifact was downloaded, to
/tmp/navzeroth-v11-native-smoke-39441834744; full log is
/tmp/navzeroth-v11-package-39441834744.log. No current installer download or rerun.

The driver counted ContentScreen's null placeholder after its generic600ms sleep.
The actual issued get_active_content request took636ms (200262→200898ms in verify
IPC tail). ContentScreen renders the same placeholder until the separate async
active-content read completes. The artifact does not retain its response body or
pointer; actual data loss is therefore not established. Baseline populate5 checks
passed. The profile is synthetic demo/UNVERIFIED_DEMO, distinct from shipped real
reference/70450; authorized application-version bundled resets do not excuse its loss.

The repair first awaits actual list_profiles and get_active_content IPC, requiring
the exact character/revision/fixture/context from the two fixtures used by populate.
It then selects that issued profile and waits for its exact rendered revision/scope.
A single30s deadline covers hung IPC and UI. Null, wrong revision/scope/profile,
IPC errors, UI failures and timeouts fail. Diagnostics contain only two presence
booleans plus a300-character bounded error, and success records bounded expected
identity. No arbitrary sleep enlargement or missing-content acceptance.

Actual Linux Node checks:41 passed,0 failed across native-smoke-lib,
native-command-boundaries and stable-workload-lib tests. New regression delays the
issued active response650ms and demonstrates the old600ms placeholder condition,
then verifies completion; negatives cover absence, malformed/wrong revision,
fixture/namespace/build mismatches, absent/duplicate profiles, IPC/UI errors and
hung IPC/UI. Existing workflow already invokes the modified helper test file.
`git diff --check` passed. Log /tmp/navzeroth-upgrade-readiness-tests.log.
These are synthetic driver checks, not actual repaired native Windows execution.

Next: independent review, routine feature draft PR, parent normal integration with
renewed release bindings, then fresh actual Windows package/regression and all
applicable protected/source gates. Eight hosted cases and owner smoke remain pending.

## Independent review correction

Root and tooling found a blocking error in initial code9a817375: it used character
as ProfileDto.id. Actual snapshots.rs171–180/346–355 hashes the character/context/
fixture tuple as stored-64hex and exposes the character in display_name. The initial
passing tests incorrectly modeled character as id and therefore did not prove the
real contract. No native repaired pass was claimed; initial implementation is superseded.

Normal child54258821 resolves exactly one issued profile by display_name(character),
full context and fixture=true, requires its actual stored-64lowerhex id and uses
that returned id for active-content IPC, UI selection and diagnostic identity.
Both published v0.9.2 and v1.0.0-rc.1 writers have this same stored-id/display-name
contract (read directly with git show). Corrected DTO-shaped tests have hashed ids
unequal to character and reject wrong character/context/fixture, malformed id and
duplicate matching profiles. Fresh actual41Node checks pass,0fail; diffcheck passes.
Log /tmp/navzeroth-upgrade-readiness-corrected-tests.log. Independent re-review and
actual Windows regression are still required; draft PR259 remains unapproved here.

Final robustness follow-up: the shared deadline now marks verification expired and
checks after each asynchronous completion before issuing subsequent IPC/UI. A hung
operation cannot complete late and trigger a new readActive or UI selection after
failure. The UI wait itself remains separately bounded; an already-issued browser
operation is not claimed cancellable. Tests resolve profile/active requests after
timeout and prove no further calls. An additional positive directly loads the
Rust-written content-governance/synthetic-flow.json profiles and inspection golden:
actual stored-1c4548c6... id, demo-r1/full synthetic scope, id distinct from character.
Fresh affected suite43 passed,0 failed; diffcheck passed. Log
/tmp/navzeroth-upgrade-readiness-final-tests.log. No Windows repaired run inferred.
