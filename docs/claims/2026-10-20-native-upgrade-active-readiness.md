# Native upgrade active-content readiness

Owner: desktop_review. Baseline: ee26d119b8fc63454771e963cfd28ae6cb1b6aab.
Task: repair the native synthetic upgrade regression driver, independently of stable acceptance.
Sole writer in /tmp/navzeroth-native-upgrade-active. Authorized files: Native-Smoke.mjs,
small I/O-independent helper/tests and this claim/handoff; workflow invocation only if needed.

Actual V11 run39441834744 failed its blocking native gate. Candidate smoke32 passed;
upgrade6 passed/1 failed; eight stable cases never ran. Actual get_active_content took636ms,
while settle sleeps600ms and ContentScreen renders its null placeholder before completion.
No retained response body proves content loss. Synthetic demo scope must remain preserved;
it is distinct from version-reset defaults for admitted nonfixture bundled content.

Implementation and checks pending. No runtime, source packs, canonical receipts or keys changed.
