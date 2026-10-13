# Wave 3 task 5: route inspection

Owner: Codex /root, sole writer. Baseline 32cec0f (coordination PR47),
branch feature/wave3-routes; /tmp/navzeroth-wave3-routes. Claimed before implementation.
Read STATUS, planning/export/atlas handoffs and plan sections 9,12,13.
Scope: recomputed selected travel and final stopping route, exact active dataset
projection, schematic same-space endpoint connections and explicit geometry gaps.
Checks/results and review are recorded below when performed. Live/native interaction
remain unverified. Next: integrated task reviews and candidate packaging.

## Actual verification

Seven planning/export/route integration tests passed, including selected+stopping
order, no-imagery gaps, same-space placement, different-floor refusal and changed
snapshot/revision/stale inputs. Scoped app Clippy and fmt passed; desktop typecheck,
lint, 79 unit tests and seven mocked Chromium workflows passed. The route flow
checks explicit projection gaps; late responses after input edits are discarded.
Initial imagery test failed because the new floor lacked a declared map space and
matching objective reference; corrected the fixture declaration, reran successfully.
No real imagery or road geometry is invented. Frame diagrams use exact dataset
projection and expose a schematic grid even when background imagery is unavailable.
Hosted current-head checks and read-only review are recorded in PR metadata.

Review follow-up: same-profile minute refresh previously cleared all manual planner
selections. A focused UI regression now verifies preservation during refresh;
results still invalidate, missing catalog IDs are pruned and profile switches reset
scoped controls. Typecheck/lint,80 unit tests and seven Chromium flows passed after
the fix. No new Rust changes. Read-only reviewer checks this small delta separately.
