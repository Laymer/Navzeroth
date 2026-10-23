# Exact draft installed native UI smoke driver

Owner world_review; baseline9cc774e4f2656f26d544071d6bd174367ccca16a,
claimb4fe3292; frozen CODE86667b01493346f69a620526a639bbd3a1e185f1.
New Draft-Installer-Workload.mjs and draft-installer-ui.mjs plus dedicated tests;
no existing stable driver/helper, app/runtime, workflow, canonical data or keys edited.
Root owns orchestration/aggregation/download/provenance; desktop owns stable workload.

Actual Windows driver requires disposable hosted environment and installed manifest
<exe-directory>/bundled-content/manifest.json. Checks version/reference target,
exact two admitted pack roots, directory/file regularity and bounded reads plus pack
manifest pins; the root independently verifies downloaded installer and installed
resource closure before invoking. Source/installer identities are explicit CLIargs;
report binding alone does not authenticate download provenance. Actual native app
must report stored/ready/errornull/version. Existing per-executable debug policy
refuses overwrite; only its issued value is removed, with graceful shutdown then
bounded browser/process cleanup. Remaining20minute deadline and root parent20minute
bound are unchanged. No addon/socket/game bridge or real local invocation.

Fresh driver imports an explicitly manual engineering character through native IPC,
checks true bundled defaults, then uses actual Content UI to verify/stage both
INSTALLED world/authored packs. User-visible Activate all staged packs composes the
same native revision; UI inferred checkbox explicitly selects false, preserved for
restart. Actual Settings controls save125percent and selected profile. Planning
uses bounded snapshot-projected objective pagination, a source-backed quest stage
with supported known eligibility and declared manual assertions. Actual Planner
controls set same-node candidate/origin/stop, utility10/completion1000ms/service0,
then Plan session and Save on Today. Native current-plan summary verifies exact
profile/snapshot/revision/context/objective/nonempty steps and manual values. No
plan_session RPC bypass, intercepted/forged UI response, synthetic content pack or
invented source geometry. Same-node smoke does not claim travel route coverage;
existing eight hosted travel checks remain separate.

Restart asserts exact installer/source state, stored profile/settings, false inferred
choice/default marker and pinned summary before UI navigation. Content/Settings/
Today render those persisted values. Historical persisted advice is allowed only
as historical, with unchanged summary and explicit native state/reason; never
promoted current. Relevant content/settings mismatches refuse. Fresh assumes a
truly fresh isolated APPDATA, as coordinator's dedicated draft job supplies.

CLI: --mode fresh|restart --exe --manifest --version --state --out --source-commit
--installer-sha256 [--port 9335]. Report schema1/phase draft, pid/version/mode,
source/installer IDs, human_performed false, completed/passed, ui_interaction_verified,
checks and screenshots. Fresh checks first_start/bundled_activation/source_backed_
planning true, restart_persistence false; restart first_start/restart_persistence
true, other two false. Coordinator aggregates fresh first three and restart final
check, requires both reports passed/completed and independently binds exact assets.

Five fresh PNGs: fresh-defaults, fresh-activated-content, fresh-settings,
fresh-planning-result, fresh-today. Three restart PNGs: restart-content,
restart-settings, restart-today. Each under <out>.screenshots/<fixed-name>.png,
<=8MiB, PNG signature checked, relativepath/length/SHA256 recorded. Failure attempts
one additional bounded screenshot without masking original error. Root enumerates
recursive owned evidence excluding state; normal total2raw+2progress+8PNGs=12.
Progress fixed phase labels carry no IPC arguments/results or imported data.

Actual local checks (engineering mocks, NOT native acceptance):
- node --test scripts/windows/draft-installer-ui.test.mjs:7PASS,0fail/skip.
- Native driver and helper node --check:PASS.
- pnpm --dir apps/desktop typecheck; lint:PASS.
- Actual React Content/Planner/Settings/Today dev-browser tests:3PASS,4.5s.
  Includes real paged controls, no direct plan_session/stage_content calls, wrong
  native pinned-profile refusal and empty UI-plan refusal. Mock content/native
  service is clearly labelled dev-only, never accepted as shipped-content evidence.
- git diff --check:PASS. No production per-file90 or Windows result is claimed.

Browser command used existing pinned dependencies and external config
/tmp/navzeroth-draft-ui-playwright.config.mjs with exact project cwd/Vite server,
retries0. First config omitted cwd and Corepack selected wrong pnpm before tests;
corrected cwd used project version, no package-manager guard bypass. First actual
browser attempt found a select label lookup mismatch; corrected to exact accessible
combobox role, retaining strict fieldset/objective identity. Initial fixture lint
non-null assertions replaced with explicit validation. Logs /tmp/navzeroth-draft-ui-
{unit,browser,typecheck,lint}.log. No real native application, installer, network
adapter or gameplay ran; actual hosted exact-draft outcome remains unverified.

Next independent frozen-source review and routine draft PR; integrate with separately
reviewed schema5 and root PS/workflow transport, renew exact source/review/receipts,
then actual hosted gates and independent downloaded-draft UI execution. No tag,
publication or canonical observation promotion by this task.
