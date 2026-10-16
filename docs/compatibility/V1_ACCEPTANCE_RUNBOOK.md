# v1 live-game and Windows acceptance runbook

Target: **Forever 1.60.1.70374**, Windows desktop, and every mandatory unit in the
[frozen coverage inventory](../../release/v1/coverage-inventory.json). The user
[reported interface 16001](FOREVER_70374_EVIDENCE.md); complete installed-candidate
identification is still required. This runbook records procedures, not passes.
Historical 70338 observations and 70205 reference data do not establish 70374 facts.

The [release evidence contract](../../release/v1/README.md) and
[acceptance manifest](../../release/v1/acceptance-evidence.json) own machine-readable
requirements. The 25 headings below are their fixed mandatory row IDs. Record
`pass`, `fail`, or `unverified`: missing hardware, content, implementation or
observations means unverified; an executed unmet expectation means fail. A
successful refusal is evidence for that refusal, not completion of the blocked
feature. Every mandatory subcheck and inventory-unit case must pass before its
aggregate row passes. Synthetic tests and hosted packaging remain separate.

## Prepare and retain evidence

Use a private evidence directory outside Git. Keep original exports and databases
private; publish only reviewed, sanitized extracts/screenshots without account
paths, character names or unrelated chat. Record sanitization and hashes of the
reviewable evidence files. Never execute SavedVariables as Lua or edit its build,
timestamps, sequence, completion flags, or fixture flag to make a check pass.

Each row needs an operator, UTC date/time, procedure/subcases, expected and actual
result, evidence paths and SHA-256, environment, installed app/addon versions,
`tested_source_commit`, production-tree identity, artifact hashes, frozen inventory
hash, exact content revision/hash and source pins. Use method `live_game` for
`LIVE-*` and `interactive_windows` for `WIN-*`; retain native desktop evidence
alongside game evidence for workflows crossing both environments. Record actual
client version/build/interface, locale, combat state, character pseudonyms, and
relevant before/after snapshot/plan/session IDs. A diagnostic's bundled addon
version does not prove which addon was installed.

On Windows, from the downloaded candidate directory, substitute its actual version:

```powershell
$candidateVersion = '1.0.0-rc.1' # example; use the candidate being tested
Get-Content -LiteralPath .\SOURCE_COMMIT.txt
Get-Content -LiteralPath .\SHA256SUMS.txt
Get-FileHash -Algorithm SHA256 -LiteralPath ".\AzerothNavigator-$candidateVersion-x64-setup.exe", ".\AzerothNavigator-addon-$candidateVersion.zip", '.\CANDIDATE_NOTES.md', '.\SOURCE_COMMIT.txt'
Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber, OSArchitecture
Get-CimInstance Win32_Processor | Select-Object Name, NumberOfLogicalProcessors
Get-CimInstance Win32_ComputerSystem | Select-Object TotalPhysicalMemory
$PSVersionTable.PSVersion
[DateTimeOffset]::UtcNow.ToString('o')
```

Compare every downloaded asset against the checksum/release manifest, including
content and required source/license assets once packaged. Record Windows edition,
build, CPU/RAM, storage, GPU/driver, WebView2 version, screen resolution/scaling,
monitor arrangement and game display mode. Record exact installed addon file hashes
against the verified ZIP and its TOC/Core version; a file name alone is insufficient.

Run `LIVE-BUILD` first, then capture/transfer/reload, planning and the end-to-end
duo session. Run independent Windows checks as soon as a candidate is available.
For each failure retain evidence, create a focused fix, and rerun affected rows
on its newly packaged revision. Do not carry results across changed production
trees/content without the evidence contract's identity and retest rules.

At the runbook's inspected baseline `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`,
capture/clock/position/plan/session/HUD gates allowed only 70338. PRs117/118 add
copyable diagnostic/snapshot exports and guarded experimental 70374 admission.
Their portable checks do not establish live results. PlanControls still declares
completion mappings and current dependency checks unavailable; the manual recorder
samples level and unclassified spans. These remain implementation/data blockers,
not instructions to weaken acceptance criteria. Reinspect limitations after each
relevant implementation PR and bind observations to the installed candidate.

## LIVE-BUILD — identify the installed client and addon

1. Record the selected Forever product/edition, addon ZIP/source identity, installed
   TOC/Core versions and file hashes, locale and out-of-combat conditions. Enable
   **Azeroth Navigator Diagnostics** in that client's AddOns list.
2. Run `/dump GetBuildInfo()`, `/anavdiag probe`, `/anavdiag status`. Retain the
   full returned build tuple and diagnostic results. Compare version `1.60.1`,
   build `70374`, interface `16001`; API presence is not proof of readable values.
3. Confirm no load/error loop. Preserve `diagnostic_export` after one user-triggered
   `/reload` or logout. Confirm the real selected SavedVariables path privately.
   An unexpected tuple stops target acceptance and requires a new compatibility
   decision; record the mismatch instead of substituting another build.

## LIVE-CAPTURE — truthful fields and capture clocks

1. Run `/anavdiag clock`; compare its UTC epoch seconds with
   `[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()` on a Windows clock whose sync
   conditions are recorded. Only when they agree, run `/anavdiag clock trust`
   within 120 seconds. Run `/anavdiag snapshot duo-paladin` (and `duo-mage` on the
   other character). Reuse each anonymous ID for that character only.
2. Compare the imported level, accepted quests, readable completion flags and
   equipment IDs with the visible game UI. Include an observed false value,
   available zero value, unknown, unsupported and restricted cases where the
   permitted API exposes them. Missing cases remain unverified, never fabricated.
   Headers must not be quests; a missing quest must not imply turn-in; absent
   item links must not imply empty slots. Record item-loading/event transitions.
3. Run `/anavdiag position`. Record readable coordinates and their actual map,
   floor/instance interpretation only if verified; the baseline intentionally
   keeps snapshot location unknown. Inventory required talents/class/role and
   other missing fields explicitly instead of inferring them from level/items.
4. Repeat an explicit capture after a permitted field change; inspect per-field
   provenance/freshness. On an observed read failure, the prior complete export
   must remain intact. Do not induce restricted-value access to manufacture a case.

## LIVE-TRANSFER — text, SavedVariables and file watching

1. After a successful capture, trigger `/reload` or logout once. In **Snapshot
   imports**, set **Export file path** to the exact operator-selected file,
   **File format** to **SavedVariables export string**, **SavedVariables global**
   to `AzerothNavigatorDB`, and **Snapshot export key** to `snapshot_export`.
   Select **Import selected file**. Confirm profile/build/namespace, snapshot ID,
   age and values. `diagnostic_export` is not a character snapshot.
2. Import the same file again; counts/current state must not change. Choose
   **Watch selected file**, perform another ordinary capture and reload, confirm
   one new complete import, then **Stop watching**. The desktop must not change
   merely because the character moves or completes a quest without a saved import.
3. In a disposable local file copy, check truncated/malformed input followed by
   the valid saved bytes; failed reads preserve current state and watching recovers.
   Import an older actual capture after a newer one; it must not silently replace
   current state. Record explicit history/promotion behavior separately.
4. Also run `/anavdiag copy snapshot` out of combat after an explicit capture.
   Choose **Select text**, press **Ctrl+C**, and paste into **Snapshot export text**
   / **Import text**. Compare the exact stored/copy/imported bytes, ensure editing
   the copy view leaves stored data unchanged, and close/reopen it. Diagnostic
   `/anavdiag copy` output must not import as a snapshot. Do not strip Lua
   quotes/escapes by hand or execute the file. See [copy bounds](ADDON_COPY_EXPORT.md).

## LIVE-RELOAD — persistence without invented freshness

Record capture/clock/plan/session state before reload, after reload and after
logout/login. Reload must revoke runtime clock trust, clear the loaded plan and
leave the recorder idle. A cached snapshot/export keeps its original timestamp
and cached label; it is not a new observation. Reconfirm the clock and explicitly
capture before expecting a fresh import. Verify settings and prior successful
snapshot/session exports persist, failed later exports preserve them, and a new
recording has a new identity. Let a real capture age past the displayed freshness
window and confirm stale advice is withheld; do not edit timestamps.

## LIVE-RESTRICTIONS — combat and missing observations

With a valid loaded plan, record ordinary noncombat, combat, combat exit and reload
conditions. Exercise informational status/HUD changes during an ordinary
user-operated encounter. Unsupported/restricted/unreadable inputs must remain
distinct and dependent advice must be withheld. Known combat or unavailable
combat query must defer HUD mutation; after a permitted exit, reevaluate current
observations before applying the latest request. Check error/taint reports and
action-bar, targeting, camera and movement input for interference. Record actual
errors and unsupported methods without bypasses. Unknown/throwing API cases that
cannot be observed naturally remain synthetic coverage, not live passes.

## LIVE-HUD — usable appearance and bounded updates

Out of combat run `/anavplan show`, `size 420 220`, `font 150`, `contrast high`,
`status`, `hide`, `show`, and `reset` (each preceded by `/anavplan`). Check visible
text, requested versus applied status and capture-age label. Repeat font selection
without compounded scaling; check default contrast and supported size/font bounds.
Drag with the left mouse button; refresh/status, resize and hide/show must not
snap a session-only move to old coordinates. Reload tests only actually reported
persisted placement. Repeat the latest-request/reset sequence across combat
deferral. Check readability, focus, action bars, error loops and representative
event bursts; do not count an accepted frame-method request as observed rendering.

## LIVE-PLAN — create and transfer a compatible real plan

Activate only a validated, distributable 70374 revision in **Content** using
**Content revision JSON**, **Validate and stage content**, then **Activate staged
revision**. In **Single-character planner**, choose **Planning profile**, **Origin**,
**Stopping node**, **Versioned goal identifier**, **Budget (ms)**, and documented
objective/node/utility/time estimates. Select **Plan session**, inspect actual
reasons and **Inspect route**, then **Export plan** / **Copy plan**.

Record snapshot, dataset, plan ID, expiry and byte count (at most 32 KiB). In game
run `/anavplan paste`, paste the ANAV1 text, click **Import**, then run
`/anavplan status`. Check next action and uncertainty/dependency/revalidation
messages. Invalid, oversized, mismatched-build and expired test replacements must
not become trusted actionable plans; malformed replacement must retain the valid
plan. Check `/anavplan hide`, `show`, `clear`; reload clears the runtime plan.
Keep negative test payloads separately labeled and never present them as captures.

## LIVE-ADVANCE — actual observation-driven completion

Choose an inventory-backed quest stage with a verified completion mapping and
record its prerequisites, relevant permitted observations and loaded step ID.
Complete that stage through ordinary play; collect before/after game state and
HUD/plan results. Only the intended participant's observed completion may advance
the intended step. Unknown, stale, missing mappings and invalidation must not
complete it. Repeat on a subsequent step and after a changed snapshot requiring
revalidation. The current `/anavplan status` explicitly says mappings are
unavailable: it cannot pass this row. There is no manual `/anavplan next` command.
Implement and document the actual supported binding before attempting this check.

## LIVE-DEVIATION — honest guidance outside the loaded route

During that same session, deliberately take an ordinary playable detour or leave
a planned prerequisite unmet. Record which permitted observation detects the
deviation, what coverage is unavailable, any verified preloaded alternative and
its readiness, and the return/replan path. The addon must neither invent a world
route nor silently complete/skip steps. If position/stage observation cannot
detect the chosen deviation, retain that limitation and choose a supported case
only when it exists. A missing detection path cannot pass by assuming arrival.

## LIVE-DUO — different progress, agreement and recipient plans

1. Import fresh trusted real paladin and mage snapshots under distinct IDs. In
   **Party**, **New party**, choose both, name it, and **Save membership**. Inspect
   each member's prerequisite/completed/turned-in state and capture age. Include
   an actual difference in chain progress and a documented catch-up or blocked case.
2. Under **Manual joint session planning**, set **Goal profile / utility unit**,
   **Shared origin**, **Shared stopping point**, budget and per-member utilities
   with their sources/assumptions; **Calculate joint estimate**. Every chosen
   objective must be eligible for each member. One completion cannot satisfy
   another member's prerequisite. Inspect unavailable directed travel connections.
3. Add a justified gear or preparation opportunity using `LIVE-GEAR` or
   `LIVE-DUNGEON`; retain its cost, eligibility and route explanation. Select
   **Proposing participant**, **Propose this estimate**, then record each actual
   participant's local acceptance. Local declarations do not establish remote consent.
4. Select each **Export recipient**, **Export agreed plan for recipient**, and
   load that bounded plan into the matching character's addon. Run advancement
   and deviation checks. Refresh one member's real progress and verify the old
   proposal is invalidated, requiring a new estimate/agreement. Let one capture
   become stale; joint planning must identify/block it until a fresh import.

## LIVE-GEAR — explained models and acquisition opportunities

In **Gear comparisons**, select the real profile and inventory-approved role
model. Enter its documented **Role identifier**, integer-thousandth **Attribute
weights**, **Assumed class identifier**, **Class proficiency map** and known
main/off-hand relationships; record these as explicit inputs. Select verified
items/categories/acquisition sources and finite effort/cost assumptions, then
**Compare gear**. Inspect baseline, theoretical/equip-eligible/obtainable/practical
differences, level/handedness/unique restrictions, unknown inputs and explanations.

Under **Route-aware acquisition**, declare the same justified route and source
node, **Preview reserved route**, choose the insertion leg, and **Assess independent
route opportunities**. Verify rejoining/stopping cost and uncertainty; neither
group loot nor a probabilistic source may become a guaranteed upgrade. Check a
real observed item against the shipped record/model. Repeat the prescribed cases
for every declared class/role/level/acquisition unit; invented weights cannot pass.

## LIVE-DUNGEON — verified preparation and actual geography

In **Dungeon preparation**, choose **Preparation members** and the imported
character/local party. Supply the inventory-approved **Local preparation package
JSON**, **Manually declared shared origin node**, and **Inspect preparation gaps**.
Check access, individual prerequisite chains, declared roles, consumables,
equipment, entrance travel and rewards against source evidence and permitted
game observations. Record missing versus verified sections and actual map/floor/
entrance placement. Do not use **Load synthetic preparation reference** for
acceptance, or claim a drop guarantee from a reward reference. Repeat for every
mandatory dungeon/preparation unit, including the original Deadmines scenario.

## LIVE-REMINDERS — useful, suppressible advice

For verified rules and a matching real profile, use **Reminders** / **Enable
reminders for this profile**. Observe a justified repair/training/prerequisite/
upgrade trigger and inspect its reason, inputs and priority. Repeated unchanged
captures must not repeat acknowledged advice. Exercise **Mark … as read**,
**Hide …**, **Restore …**, disable/re-enable and expiry/refresh. **Add a bag-space
checklist** and **I checked bag space manually** are declarations, not observed
capacity. Unsupported rules must stay evidence-gated. Record actual addon delivery
where required by the inventory; desktop-only delivery cannot establish live HUD
rule integration. No reminder may perform a game action.

## LIVE-SESSION — bounded recording and truthful review

Before the duo play, run `/anavsession start duo-paladin` on the matching character,
then `sample`, `status`, `pause`, `resume`, `sample`, `stop`, `export` (each with
`/anavsession`). Record wall-clock milestones and pause/reload boundaries; there
must be no background sampling. Export after stop, then reload/logout to persist.
In **Sessions**, select **Session profile**, set **Selected session file path** /
**Session file format** to **SavedVariables: AzerothNavigatorDB / session_export**,
and **Import selected session file**. **Inspect …** and repeat the import.

Verify idempotence, progress, measured/inferred/unclassified intervals, paused
gaps and clock uncertainty independently; offline time is not productive play.
Try **Import session text** only with an intact supported ANAVSESSION1 export
surface. A conflicting-identity negative fixture must preserve history; test
**Delete …** only on a designated disposable session. Reload must start idle;
unreadable/reversed clocks and bounds stop recording visibly when observed.
The current level-only/unclassified recorder cannot prove route/quest timings.

## LIVE-NEXT-PLAN — complete the real duo scenario

Link the same session's two imports, prerequisite difference, shared route,
justified opportunity, recipient exports, actual advancement, deviation and
session review. Import both post-session snapshots and create the next plan from
their actual progress. Explain changes versus the first plan with input IDs and
sources. In **Conditional personal estimate comparison**, declare route/stage,
level/role/party/baseline source; save an association only if the retained
observations support the entire completed stage. **Compare retained sessions**
must expose exclusions, sample count and uncertainty. A single run or unclassified
recording must not invent measured categories or redefine universal timings.

## WIN-INSTALL — actual user installation and addon selection

On the recorded native Windows machine, verify artifact hashes and run the setup
interactively as the ordinary user. Record installer pages, warnings, WebView2
state, destination, first launch/version and Start menu entries. An offline
first-install claim requires WebView2 already present or a verified offline
prerequisite package; the current installer downloads its bootstrapper if absent.

With the game closed, use **Azeroth Navigator - WoW addon** / **Install** and select
the real AddOns directory. The helper also supports these exact commands (replace
the example game path; `-WhatIf` previews):

```powershell
$addonHelper = "$env:LOCALAPPDATA\Azeroth Navigator\addon\Install-AzerothNavigatorAddon.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File $addonHelper -Action Install -AddOnsPath 'D:\Games\WoW Forever\Interface\AddOns' -WhatIf
powershell -NoProfile -ExecutionPolicy Bypass -File $addonHelper -Action Status
```

Confirm only `AzerothNavigator` was installed, its files match the verified ZIP,
and unrelated addons/settings are unchanged. In disposable test directories also
check invalid selection and an existing foreign same-name addon are refused.
While the actual game is running, the helper must refuse replacement.

## WIN-UPGRADE — migration and retained data

Populate the previous published version with designated test profiles, content,
party, sessions, settings and saved plan; create a consistent backup. Close the
game/app and run the candidate setup over it. Record both installer hashes/source
commits, addon-update choice and resulting installed versions. Check startup,
integrity, records, current/history pointers, selections and content after
migration. A declined addon update must remain visibly a different installed
version, not be reported as current. Check reinstall and downgrade refusal on
the disposable installation. A failed content validation/activation must retain
the prior usable revision; exercise **Restore previous revision** separately.

## WIN-UNINSTALL — removal and deliberate data retention

Use a disposable Windows user/test installation for destructive subcases. Record
private baseline hashes of unrelated addons/settings and test SavedVariables.
Uninstall through Windows **Apps** with **Delete the application data** unchecked;
check program/shortcuts removal and retained database. Exercise both addon removal
choices; only the selected addon's folder may change, SavedVariables remain.
Reinstall and verify retained data. Repeat with explicit app-data deletion and
verify it removes only app data. For helper SavedVariables removal, first use
`-Action Uninstall -RemoveSavedVariables -WhatIf`; perform actual removal only
against the disposable game tree and verify only its own `.lua`/`.lua.bak` files
are removed. That option covers every account under the selected game tree.

## WIN-RECOVERY — consistent backup, refusal and restore

Use **Database backup and recovery** / **Check database integrity**, a new **New
backup file path**, and **Create consistent backup** with populated current data.
Record counts/hash; an existing destination must be refused. Change a test record,
select the backup under **Backup file to restore**, and **Inspect backup**. Review
counts/schema, check **Replace all local database state with this inspected backup**,
then **Restore inspected backup**. Close other instances first.

Confirm whole-state restoration, retained originals/recovery directory, current
pointers, settings/content/session history, and stopped watching. In a disposable
installation exercise corrupt/future/incompatible backups and a failed replacement;
current data must survive and refusal stay visible. Check startup recovery after
a controlled interrupted restore using a disposable copy/VM. Preserve that
evidence separately; an ordinary successful restore does not verify interruption.
See [DATABASE_RECOVERY.md](DATABASE_RECOVERY.md) for bounds and recovery semantics.

## WIN-OFFLINE — installed local workflows

After installing permitted complete content and real imports, disconnect network
access through ordinary Windows controls and record that condition. Restart and
exercise Today, imports, Content, Atlas search/routes, Planner, Gear, Party,
Reminders, Dungeons, Sessions, settings, diagnostics and recovery with installed
data. Record content hashes and any missing assets. Missing API credentials must
not block local use or cause retry loops. Reference imagery fallback must work
where declared. No demo catalog or incompatible build data may fill a real gap.

## WIN-FOCUS — ordinary controls and scoped shortcut

Check keyboard navigation and focus after startup, import/plan completion, errors,
minimize/restore and switching to the game. Nothing may unexpectedly capture game
input or shift focus. In **Ordinary reference window**, enable **Enable Ctrl+Shift+F8
in this main desktop session**. It should toggle only with the main app focused
outside editable fields; verify disabled, text-input, held-key and game-focused
cases. Restart must leave the shortcut disabled. Record native results, not just
the operation receipt or a mocked browser event.

## WIN-REFERENCE — interactive, passive and fallback views

Use **Show ordinary reference**, **Keep on top when shown**, and **Requested
reference interaction**. Test **Interactive ordinary window**, then focus the main
app and request **Pass through mouse; request nonfocusable** / **Apply requested
interaction**. Observe actual pointer hit-testing and focus on a harmless desktop
target, then ordinary game interaction. Keep the main app reachable to **Hide
ordinary reference**; passive mode ignores even window controls. Hiding must turn
topmost off. Query **Check reference window state** and distinguish observed
behavior from setter dispatch. Verify imported ages, saved-plan identity and
**Open normal desktop reference** fallback after an unavailable/failed control.

## WIN-DISPLAY — scale, monitors and supported game modes

Record a matrix of windowed and borderless game modes, 100/150/200% Windows scale,
single monitor and mixed-DPI dual monitors, using available recorded hardware.
Move/resize main/reference windows across monitors, minimize/restore, change
resolution and disconnect/reconnect a secondary display. Controls must remain
reachable and text readable with correct hit targets/focus. Check the addon HUD
separately on the same conditions. Exclusive fullscreen is currently unsupported
for the ordinary reference window: verify the normal desktop/second-monitor
fallback and explicit limitation, without claiming an overlay there. Missing
mandatory matrix hardware leaves this row unverified.

## WIN-ACCESSIBILITY — native navigation and readable states

In **Settings**, test **Text size** 100–200%, **Contrast** / **High contrast**,
**Motion** / **Reduced**, then **Save local settings** and restart. Check every
major panel, navigation menu, forms, long labels, Atlas and reference at narrow
window widths and Windows contrast themes. Complete key workflows using Tab,
Shift+Tab, arrows, Enter and Escape; inspect visible focus and status/error
announcements with the recorded Windows accessibility tool. Loading/empty/error/
stale/offline and unknown/unsupported/restricted/false/zero must be distinguishable
in text and retain usable controls. Record screen-reader/tool/version limitations.

## WIN-PERFORMANCE — declared full-size native workloads

Before measuring, freeze hardware/build mode, full inventory/content size,
profile/session counts, workload, timing method/resolution, sample counts and
thresholds. Include ordinary supported Windows hardware. Measure cold startup,
cached profile/item lookup, full-size import/search, dense Atlas pages, focused
and full-world planning, cancellation and repeated import/replan cycles. Preserve
raw timings and CPU/private-memory samples, not just averages. Initial plan
budgets are cached lookup usually below 100 ms and focused-region planning p95
below 500 ms; predeclare a measurable interpretation/sample count (for example,
100 warm trials, at least 95 below threshold) before collecting results.

Use native timing instrumentation or a trace with sufficient resolution; a
hand-timed click cannot prove sub-100-ms behavior. **Local diagnostics** / **Preview
diagnostic metadata** and **Create local diagnostic file** expose bounded current
process import timings only: they exclude watcher attempts/earlier processes and
are not a general benchmark. Full-world work must stay bounded/cancelable with
progress, navigation responsive, prior valid state retained, and no continuing
memory growth across repeated equal workloads. Profile representative addon event
bursts on the recorded client using only permitted profiling; retain this as
supporting `LIVE-HUD` evidence. Missing measurement stays unverified.

## LIVE-GUI — the in-game window (UNVERIFIED; supporting evidence, not a release gate row)

This section is **not** in `release/v1/acceptance-evidence.json` and no release gate
reads it. It lists what the synthetic widget mock in `crates/lua-harness/tests/common/ui.rs`
cannot establish about the window described in
[ADDON_GUI.md](ADDON_GUI.md). Until an operator runs it on the recorded client, every
item is **unverified**. The slash-command steps of LIVE-HUD and LIVE-PLAN stay valid and
are not replaced by the window. Record the exact build first: a result on 70425 is
development evidence for the `development_unverified` label only and never satisfies a
70374 row.

1. **Entry points.** Run `/anav`; click the minimap button if one appears; open
   Options > AddOns and use **Open Azeroth Navigator**. Record which of the three exist,
   the status codes from `/dump` if you can read them, and whether Escape closes the
   window and does not leave the game menu blocked.
2. **Templates.** Look at the window: panel, tabs, buttons, check boxes, edit boxes and
   the scrolling plan box. Record any control that falls back to a plain colour frame
   and any missing texture or unreadable text. Check every text is readable without colour.
3. **Tooltips.** Hover every control on every tab. Each must show a title, a body, a
   `Slash command:` line and a `Gate:` line, and hide on leave. Record controls without a
   tooltip and any tooltip that stays shown or is clipped.
4. **Combat.** Out of combat open the window, then enter combat. Every action button must
   refuse and say so in the footer; closing and switching tabs still work. `/anav` and the
   minimap button must refuse to open during combat. Record any error or taint message.
5. **Taint, focus and action bars.** Opening must not take keyboard focus: movement keys
   and action-bar keys keep working while the window is open and no edit box is focused.
   Record any "action blocked" message, changed keybinding, or spell/action-bar change
   during and after use.
6. **Minimap.** Drag the button around the rim, release, confirm the drag does not also
   open the window, `/reload` and confirm the angle persists; hide it with the Minimap button check box and confirm it returns on enable.
7. **Plan box.** Paste a real exported ANAV1 plan (up to 32 KiB) with Ctrl+V into the Plan
   tab, import it, scroll the reason inspector, and confirm an oversize paste is refused
   rather than truncated. Do not record private plan contents.
8. **Clock.** Use Check clock and Trust clock on the Diagnostics tab following the same
   compare-then-confirm rule as LIVE-CAPTURE, and confirm Trust clock is unavailable
   without a check and after 120 seconds.
9. **Settings.** Toggle each Settings check box, `/reload`, and confirm the values persist
   in `AzerothNavigatorDB.settings` and `Probe at login` still behaves as described.

## Expand the rows to every mandatory inventory unit

The original paladin/mage duo is a required integrated scenario, not a substitute
for full-content acceptance. Freeze this case matrix with the inventory hash;
every unit needs linked source/validation evidence and the applicable live/native
case IDs. Record absent/unknown units as blockers, never silently omit them.

| Inventory dimension | Required case coverage |
| --- | --- |
| Quests/objectives/prerequisites | Every record/reference validates; representative observed stages per declared region/level/faction, branches, exclusivity and divergence; completion mappings support advancement |
| Equipment/acquisition | Every reference/source validates; each declared class/role/level model and acquisition type has provenance plus eligibility/configuration/obtainability/practical cases |
| Geography/travel | Every map/floor/coordinate and directed edge validates; per supported travel mode check direction, access/unlocks, missing geometry, boundary and stopping route |
| Dungeons/preparation | Every mandatory dungeon has evidence-backed access, chains, entrance/travel, preparation and rewards, with individual and party assessments |
| Class/race/faction/roles/levels | Explicit support inventory and model provenance, no generic fallback weights claimed as verification; exercise each declared applicability boundary |
| Party sizes/workflows | Each declared size supported by validated membership/readiness/fairness/export cases; real duo end-to-end plus representative declared non-duo scenarios |
| Reminders | Every shipped rule has provenance/coverage and true/false/unknown, priority, dismissal, duplicate and stale-state cases |
| Sources/community/compatibility | Reproducible normalization, conflict/distribution gates, local submission/review/withdrawal and edition/build isolation; real-game evidence remains separate |
| Storage/search/planning scale | Actual complete-content counts/hashes match the tested workload; bounded parsing, transactional failure and native performance cover that size |

Official API access, addon party messaging, retrospective combat-log analysis and
hosted distribution may be explicitly deferred when their stated prerequisites
are absent. Record each reason in the release evidence; none substitutes for a
mandatory local workflow or the remaining content inventory. Keep all nine
required exact-candidate automated checks and independent review linked separately.
An all-green automated run cannot close any unperformed live/Windows row.
