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
