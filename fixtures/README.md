# Fixtures

Committed fixtures are inputs to tests or goldens written by Rust tests. A golden
is never edited by hand. `scripts/check-golden-drift.sh` regenerates all of them
and fails when anything changes; CI runs it on every pull request.

## Golden writers

Each variable rewrites its files instead of comparing. Run the named test binary
with the variable set, review the diff, and commit it.

| Variable | Test binary | Files written |
|---|---|---|
| `UPDATE_DESKTOP_FIXTURES=1` | `azeroth-app` `atlas_commands`, `session_analytics_golden` | `fixtures/desktop/atlas/**`, `fixtures/desktop/sessions/analytics.synthetic.json` (only its mock methodology sentence is hand-written) |
| `UPDATE_GEAR_GOLDENS` | `azeroth-app` `gear_workflow` | `fixtures/desktop/gear/**` |
| `UPDATE_TODAY_GOLDEN` | `azeroth-app` `today` | `fixtures/desktop/today/**` |
| `UPDATE_REFERENCE_GOLDEN` | `azeroth-app` `reference` | `fixtures/desktop/reference/**` |
| `UPDATE_ROUTE_ACQUISITION_GOLDENS` | `azeroth-app` `route_acquisition` | every file in `fixtures/desktop/route-acquisition/` (all are compared on normal runs) |
| `UPDATE_CONTENT_DISPOSITION_GOLDENS` | `azeroth-app` `content_dispositions` | `fixtures/desktop/content-governance/**` |
| `NAVZEROTH_UPDATE_PREPARATION_FIXTURE` | `azeroth-app` `dungeon_preparation` | `fixtures/desktop/preparation/**` |
| `UPDATE_WORLD_BROWSER_FIXTURES=1` | `azeroth-app` `world_browser` | `fixtures/desktop/world/pages.json` (pages and origins over a synthetic world pack built in the test) |
| `UPDATE_LUA_GEAR_FIXTURES` | `azeroth-core` `gear_lua` | `fixtures/gear-lua/v1/**`, compared by the Lua harness `gear_parity` |

Determinism: all nine writers were run twice in succession on one checkout and
the second run changed no bytes, so all nine are in the drift step. A writer
that cannot reproduce itself must be left out and its reason logged here.
Not yet written by any test: the three addon goldens under `fixtures/addon/`
(owned by the addon GUI task).

## Reading goldens from TypeScript

Vitest and Playwright import these files directly. When a golden changes shape,
update the TypeScript types and tests in the same change.
