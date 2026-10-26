# V11 native upgrade failure and repair coordination

Owner/sole archive writer: `/root`; baseline `ee26d119b8fc63454771e963cfd28ae6cb1b6aab`.
Desktop specialist separately owns the isolated native-driver repair, with tooling
independent review. Read STATUS, stable release handoff and relevant implementation
plan before work. Original checkout and unrelated changes remain preserved.

Actual V11 CI39441834860 passed nine applicable jobs; stability was skipped.
Both whole-corpus source platforms and parity39441834737 passed; Pages39441834918
validated without deployment. Coverage39441834710 passed all three collectors and
the combined gate:276measured files meet exact90/100 without rounding. Both native
receipts are source-clean/tests-passed with no incomplete profiles; Windows CLI
executed18lines and desktop entry18. Source checkout is
`e3fdd58a9fe969f972d2796c47f4b114e5b0d661`, tree-identical to candidateee26.

Package39441834744 FAILED: blocking native gate reported smoke=success and
upgrade=failure. Actual native report32/0; baseline population5/0; upgraded verify6/1,
failed `verify: active content revision survived` with `Active content was lost`.
All eight stable hosted cases and their recorder were SKIPPED. No owner result,
tag or publication exists. Canonical nine rows remain unverified.

Root initially called the displayed native-smoke step passed before its blocking
gate and corrected that statement immediately. Final actual gate confirms smoke
success and upgrade failure. Continued-on-error display is not acceptance evidence.

Independent desktop and tooling diagnosis finds a supported timing-race explanation:
driver waits600ms then counts the initial null UI placeholder; actual active-content
request took636ms. No response body/pointer was retained, so product data loss is
NOT proven. The synthetic fixture uses UNVERIFIED_DEMO and must survive; bundled
default resets for admitted real scope do not justify replacing this assertion.

Next: desktop sole-writer repair must validate completed issued profile and exact
synthetic active-revision/scope IPC, then bounded matching UI; missing, wrong, failed
or timed-out results must still fail. Independently review meaningful delayed and
negative regressions. Archive signed V11 identities and actual raw failure before
integration; renew source/review/receipts after any production delta, then dispatch
fresh all-ten/source/Windows checks. Owner smoke remains after verified final draft.
