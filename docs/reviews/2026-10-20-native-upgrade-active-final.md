# Independent final native upgrade-driver repair review

Root and tooling independently reviewed PR259 final
7bffde54f7414497ee560e906719637da417ea6f, code
a3ff4bfbe3fc9fe3dc70c47ededdee5f3ac82dc8, baselineee26. Sole writer desktop,
claim39e25512; normal parent integrationb615d6f944748b474dce4e0ebd8f1af2dce66f68.
V11 signed identities/raw actual failure were archivedb0d9c442 BEFORE integration;
actual coverage verdict/native receipts were retained0fd50561.

Actual failure evidence supports the fixed600ms placeholder race while the native
get_active_content request took636ms; its response body was not retained, so actual
product data loss remains unproven. Synthetic content scope must persist; admitted
real-bundle version resets are not an exception. The corrected driver validates
exact character/full context/fixture against list_profiles, uses the actual issued
stored-64hex ID for active-content IPC and UI selection, requires exact revision/
full synthetic scope, then waits for matching rendered inspection. One30s deadline
and late-completion guards prevent subsequent reads/UI after failure. An already
issued browser wait is independently bounded, not claimed cancellable.

Original9a817375 wrongly used character as ProfileDto.id, and its manual mock hid
that incompatibility. Root/tooling independently blocked it before integration.
Normal child54258821 corrects issued identity; a3ff4bfb adds late-completion guards
and a positive using the actual Rust-written governance DTO golden. Published
v0.9.2 and rc.1 writers were directly checked: same opaque stored-id/display_name
contract. Wrong/missing revision/scope/fixture/profile, IPC/UI errors, duplicate
profiles and timeouts remain failures. A650ms response case demonstrates the old
600ms placeholder failure and verifies completed identity before UI.

Author/tooling each ran43affected Node tests at final7bffde54, all pass. Root ran
43affected Node tests again on exact integratedb615; all pass,0fail/0skip. Commands:
node --test scripts/windows/native-smoke-lib.test.mjs scripts/windows/native-command-boundaries.test.mjs scripts/windows/stable-workload-lib.test.mjs
and node --check scripts/windows/Native-Smoke.mjs. Root initially ran41helper/
workload cases and separately2command-boundary cases before exact integration43.
Diff checks passed. Runtime Rust/UI/addon/content/Cargo bytes remain1ddaa8d6; only
outer workflow line and these three test-driver/helper files differ in production.
Actual functional proofef3c7098 retains1dd/b9 execution; no new benchmark/Windows
pass is claimed.

Final review CLEAR for this repair. Fresh all-ten protected/source/Windows checks
on the renewed signed release composition remain required. V11 package FAILED and
all eight stable cases were SKIPPED. No owner smoke, tag or publication exists.
