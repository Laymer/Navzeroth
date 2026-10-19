# Addon: install, capture and remove

The current release is the **0.9.2 developer preview**, with copyable exports and
guarded 70374 support. Use its exact artifact/source identity when testing it. The AddOns entry is
still named **Azeroth Navigator Diagnostics**, but the addon now includes manual
snapshots, plan import and HUD controls, and manual session recording. These are
experimental client bindings. It runs inside the permitted addon environment;
desktop exchange uses explicit files or pasted text.

Current source enables the exact experimental tuples `1.60.1.70338`,
`1.60.1.70374`, `1.60.1.70425` and `1.60.1.70450`, each with interface `16001`. The
first two were enabled in 0.9.1; **70425 and 70450 are admitted for development capture
only and are UNVERIFIED**: the owner's installation metadata reports them (70450's interface 16001 is
owner-reported; see
[BUILD_CAPABILITIES.md](BUILD_CAPABILITIES.md#client-160170425-development-admission)
and [the 70450 section](BUILD_CAPABILITIES.md#client-160170450-development-admission)),
and no addon run on either is recorded. Under [ADR 0006](../decisions/0006-v1-program.md), the stable 1.0 target is the
first verified stable live build after launch. Historical 70374 observations do
not satisfy future candidate-bound LIVE acceptance. Limited user-reported
diagnostic retrieval, level capture and clock-age results are recorded in
[BUILD_CAPABILITIES.md](BUILD_CAPABILITIES.md). Full quest/equipment, restriction,
HUD, plan, session and reload acceptance remains unverified.

The previous development target was **Forever 1.60.1.70374**. On 2026-10-14 the user reported
`GetBuildInfo` version `1.60.1`, build `70374`, date `Oct 12 2026`, interface `16001`,
on Windows 11 out of combat, with `/anavdiag` reporting addon `0.8.1` and probe/status
showing no errors. The installed archive hash/source commit was not inspected;
this is a narrow build/version/diagnostic report, not full adapter acceptance.

The candidate's capture, clock, position, plan, recording and HUD adapters share
the exact experimental build guard. Other tuples remain unsupported. On 70374,
retain the diagnostic build/interface and exact installed source record; guarded
admission does not establish readable fields or live acceptance. Do not relabel
a 70338 export or edit a guard to claim 70374 evidence.

## Install

1. Close the game client.
2. Download `AzerothNavigator-addon-0.9.2.zip` and `SHA256SUMS.txt` from the
   [0.9.2 release](https://github.com/Laymer/Navzeroth/releases/tag/v0.9.2). Verify
   the ZIP's SHA-256. Extract its `AzerothNavigator` folder into the directory the
   client actually uses for addons, so that this file exists:

   ```text
   <product folder>/Interface/AddOns/AzerothNavigator/AzerothNavigator.toc
   ```

   From source, copy the entire `addons/AzerothNavigator` directory, including its
   `LICENSE`. Avoid an extra enclosing ZIP or repository directory. The exact
   Forever product path is not established by the conventional layout above.
3. Start the client and enable **Azeroth Navigator Diagnostics** in the AddOns list.
   Record the installed addon version and exact source commit from the release's
   `SOURCE_COMMIT.txt` when gathering acceptance evidence.

The Windows application's **Azeroth Navigator - WoW addon** Start menu entry
provides an alternative installer. Choose the existing AddOns folder or the product
folder containing `Interface\AddOns`. The helper checks its source, refuses a
different addon with the same folder name, and verifies copied bytes before replacing
its own folder. It attempts to detect processes running from the conventional game
directory; still close the client yourself. See
[Windows addon installation](WINDOWS_INSTALL.md#install-the-addon).

For the guided 70374 development check, the independently verified
[candidate package](https://github.com/Laymer/Navzeroth/actions/runs/38627880967/artifacts/11925586142)
has source `004058f863c3fe11472ddaa09d106c67e4f2c86a` and still displays `0.8.1`.
It differs from the published ZIP with the same versioned filename. Its exact
bytes and limitations are in the [candidate verification record](../releases/candidates/70374-004058f.json).
Extract the outer Actions artifact, then its inner addon ZIP using the same
installation steps. The packaged notes describe published 0.8.1; this paragraph
and the verification record describe the additional candidate changes. No stable
or complete-content acceptance is implied.

The TOC declares interface `16001`, matching both the earlier 70338 diagnostic and
the user's 70374 report. A matching interface does not establish addon compatibility.
If the client marks the addon out of date, record that result and the actual build
before changing anything; do not hide it with compatibility tools or client edits.

## The in-game window

`/anav` opens a window with every addon feature behind buttons and fields, each with a
tooltip. The same window opens from a minimap button and from Options > AddOns when the
client offers them. It calls the same guarded code as the slash commands, which all keep
working. See [the in-game window](ADDON_GUI.md) for the tabs, the combat policy, the
fallbacks and the controls that are deliberately absent. Its behaviour on the real client
is UNVERIFIED.

## Record the client first

On login, the default `auto_probe` setting runs one bounded capability probe.
Run `/anavdiag probe` to repeat it and `/anavdiag status` for the last summary.
The probe checks API-name presence and calls `GetBuildInfo`; API presence does not
prove readable values, permitted use or behavior in restricted contexts.

Run `/reload` or log out to let the client persist SavedVariables. One retrieval
is recorded for 70338; general write timing and the new target's behavior still need
evidence. The conventional file is:

```text
<product folder>/WTF/Account/<account>/SavedVariables/AzerothNavigator.lua
```

Use the actual client file, not a guessed account path. Copy only sanitized results
into the [capability observations template](capability-observations.template.json),
with installed revision, client build/interface, locale, conditions and observed
outcome. Do not commit the raw file or personal account path. Stored keys and their
formats are described in [ADDON_SAVEDVARIABLES.md](ADDON_SAVEDVARIABLES.md).

## Manual snapshots on an enabled experimental build

Use a unique anonymous identifier for each character, containing 1–80 letters,
digits, underscores or hyphens. Keep it stable across captures:

```text
/anavdiag snapshot anonymous-one
```

The current adapter attempts bounded level, quest-log and equipment-identity reads.
Unreadable or unsupported fields remain distinct; missing item metadata or quest
completion mappings are not invented. `/anavdiag position` reports readability
diagnostics only: snapshot location stays unknown until map, floor, instance and
coordinate meaning are verified.

After a successful capture, reload/logout. In the desktop **Imports** screen,
select that exact SavedVariables file, choose **SavedVariables export string**, and
use global `AzerothNavigatorDB` / key `snapshot_export`. Import once, or explicitly
watch that selected file for later manually requested captures. The parser treats
the file as bounded data and never executes its Lua. `diagnostic_export` is a
separate diagnostic report and cannot be imported as a character snapshot.

The snapshot sequence is a capture-order counter, not elapsed gameplay time. Do
not reset it to make a capture appear newer. A fresh timestamp does not supply
missing quests, equipment metadata, geography or a compatible playable catalog.

Current source also provides `/anavdiag copy` for an existing diagnostic report
and `/anavdiag copy snapshot` for an existing ANAV1 capture. Open out of combat,
choose **Select text**, press **Ctrl+C**, and paste a snapshot into **Snapshot
export text** / **Import text**. Copying does not capture again or change freshness.
These commands are included in the 0.9.2 ZIP; see
[copy behavior and acceptance limits](ADDON_COPY_EXPORT.md).

### Capture age

Run `/anavdiag clock` and compare its reported UTC epoch seconds with a trusted PC
clock. For example, in Windows PowerShell:

```powershell
[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
```

Allow for switching windows. Only after the values agree, run
`/anavdiag clock trust` within 120 seconds, then capture. Confirmation lasts only
for the current addon runtime; reload clears it. `/anavdiag clock clear` revokes it.
Invalid/missing values, reversals, slow samples or inconsistent clocks leave timing
uncertain. These are engineering guards; restricted-context and target-build
clock behavior still require acceptance evidence.

## Plans, HUD and sessions

On the recorded build, copy a compatible desktop-exported `ANAV1:` plan and use
`/anavplan paste`, then explicitly import it. The importer is bounded to 32 KiB.
`/anavplan status`, `show`, `hide` and `clear` inspect or control the runtime plan.
Invalid input retains the previous plan. Plans clear on reload; retained snapshots
are labelled cached and keep their original capture time. Unknown/stale state or
missing completion mappings cannot establish advancement. The addon does not
generate missing routes or perform game actions.

Use [HUD appearance](HUD_APPEARANCE.md) for size, font, contrast and movement
commands. UI updates defer when combat state is restricted or unreadable; a visible
card may therefore describe the last evaluation. Native UI/taint/focus acceptance
remains unverified.

For explicit session recording, use `/anavsession start anonymous-one`, followed
by `sample`, `pause`, `resume`, `stop`, `export` or `status`. Stop and export before
reload, then import `AzerothNavigatorDB / session_export` in **Sessions** for the
same previously imported character. Recording never samples in the background or
resumes after reload. Measured intervals, gaps and unknown activity remain distinct.
See [session transport](SESSION_TRANSPORT.md) for bounds and retention behavior.

## Update and remove

Close the game, then replace the addon with the same version as the desktop using
the helper or a verified ZIP. Preserve SavedVariables when updating. After a new
addon revision, record that revision and rerun any affected acceptance checks.

To remove the addon, close the client and remove only
`Interface\AddOns\AzerothNavigator`. SavedVariables may remain for later use.
Deleting the addon's `AzerothNavigator.lua` and `AzerothNavigator.lua.bak` from a
chosen account's SavedVariables folder is a separate, explicit removal of settings
and retained exports.

The Windows helper's `-Action Uninstall` removes its own addon and leaves
SavedVariables. Adding `-RemoveSavedVariables` removes those two filenames from
**every account folder** under that selected game's conventional `WTF\Account`;
preview with `-WhatIf` before choosing that wider deletion. The desktop uninstaller
can offer addon removal but does not request SavedVariables deletion.

## Checks that do not need a client

`cargo test --locked -p azeroth-lua-harness` runs synthetic Lua 5.1 checks for
load order, bounded data, capture/plan/session behavior, HUD guards and privacy.
CI also runs Lua static checks. These do not establish real-client behavior; see
[Lua runtime differences](LUA_RUNTIME_DIFFERENCES.md). The
[v1 release process](../releases/V1_RELEASE_PROCESS.md) requires actual target-build
evidence before stable publication.


The 0.9.2 preview includes `/anavsession start-quests <anonymous-profile-id>`
for explicit quest-turn-in recording; ordinary `start` stays manual-only. Use a
distinct anonymous profile per character. Stop/export/reload manually. The recorder
returns to idle after reload, and milestone timing stays unclassified. See the
[development evidence](FOREVER_70374_EVIDENCE.md); it does not replace final acceptance.
