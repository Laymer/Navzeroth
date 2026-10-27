# Native upgrade same-selection interaction

Owner desktop_review; baseline728a9198cb2d72212d39cf06771f807a412967f3;
claim eec12a96; code377f9de7 in /tmp/navzeroth-native-upgrade-selection.

Actual V12package39454144270 sourcefdf83a2961eb66ac04021e6de6721cab6a188dba
failed the blocking native-upgrade gate. Candidate native smoke passed; upgrade
verification6passed/1failed: Synthetic content verification timed out, diagnostic
profile_found=true/active_present=true. ALL8stable cases and recorder were skipped.
Full log /tmp/navzeroth-v12-package-39454144270.log; only small2.6MiB native-smoke
artifact downloaded to /tmp/navzeroth-v12-native-smoke-39454144270. No installer.

Actual UI active read244675→245376ms; strict direct read245405→245409ms. Wrong
revision/scope would have produced a distinct immediate error; observed timeout
therefore follows successful exact IPC verification into the UI wait. No new
get_active_content request occurs before timeout275333ms. The repaired259 driver
unconditionally selected the already-selected profile. Actual ContentScreen.tsx88
onChange clears active state, while unchanged profile means the effect44–50 does
not refetch. These observations did not establish lost data.

A controlled Chromium regression now renders UNMODIFIED real ContentScreen in a
test-only dev HTML/React entry using the Rust-written governance golden DTO and
explicitly synthetic IPC. Old unconditional selectOption reproduces the cleared
inspection and no new active read. The production driver's shared helper first
reads inputValue and selects ONLY a different issued id, then retains exact
revision/scope UI wait and empty-state refusal. Actual browser tests prove same
profile zero changes/preserved inspection, different profile one change/refetch/
matching inspection, and wrong revision, wrong scope or absent inspection fail
within the bound. Strict issued-profile/active-IPC validation and shared30s deadline
remain unchanged. No UI assertion was removed and no bundle reset policy applied
to synthetic baseline data.

Actual validation:
- Playwright dev Chromium3passed,0failed,6.7s; includes old behavior control.
- Existing nativehelper/boundary/workload Node43passed,0failed.
- Existing ContentScreen unit7passed,0failed.
- TypeScript, focused ESLint, both driver/helper syntax, gitdiffcheck passed.
- Existing Playwright setup also built the ordinary production bundle successfully;
  new fixture paths do not occur in dist. No production build/entry/config changes.

Logs /tmp/navzeroth-native-selection-{browser-final,node,content-unit,typecheck}.log.
Initial browser discovery failed before tests because Node JSON import lacked an
attribute; corrected test uses readFileSync. Final actual browser results above are
dev-only component regression, NOT production-CSP, native WebView2, installed
Windows, acceptance or owner results. Existing prod-CSP project remains unchanged.
No runtime Rust/UI, source packs, canonical receipts or private keys changed.

Next independent review, routine draft repair PR, parent normal integration and
renewed exact-source release closure, then fresh protected/source/native/hosted
checks. V12failed result and all8skipped stay historical; no publication claim.
