# Diagnostic addon: install, use and removal

Task M0.3. Status: IMPLEMENTED_UNVERIFIED. No Forever client was available when
this addon was written, so **every path, file name, slash-command behavior, event
name and API name below is an unverified assumption** until a user-operated client
check (M0.4, M0.5) records it in
[BUILD_CAPABILITIES.md](BUILD_CAPABILITIES.md) and
[capability-observations.template.json](capability-observations.template.json).

The addon is a development tool. It records build and capability results for the
maintainers. It does not navigate, plan, show a HUD, sync parties or talk to the
desktop application.

## What it does

- On `ADDON_LOADED` it prepares its own SavedVariables table `AzerothNavigatorDB`
  (layout: [ADDON_SAVEDVARIABLES.md](ADDON_SAVEDVARIABLES.md)).
- On `PLAYER_LOGIN`, when the `auto_probe` setting is on (default), it runs the
  capability probe once and stores one bounded JSON diagnostic export in that table.
- `/anavdiag probe` runs the probe again; `/anavdiag status` prints the last summary;
  any other argument prints the usage line.
- The probe checks that a fixed list of function names exists and calls exactly one
  getter, `GetBuildInfo`, inside `pcall`. It never calls unit, chat, combat-log,
  map-position or item functions, and reads no character name, realm, GUID or chat
  text. A missing API is reported, never replaced by a shim or workaround.

## Install (manual)

1. Close the game client.
2. Copy the folder `addons/AzerothNavigator` from this repository, as a whole, into the
   client's AddOns directory so that the result is
   `<product folder>/Interface/AddOns/AzerothNavigator/AzerothNavigator.toc`.
   - The AddOns directory of the Forever product folder is **unverified**. Use the
     directory the client itself uses for addons; do not guess a path or move other
     files to make it appear.
3. Start the client and open the AddOns list on the character screen. The entry is
   named "Azeroth Navigator Diagnostics".

### Install with the Windows helper

The Windows installer ships a helper that performs the steps above with checks: it
refuses a folder not named `AddOns`, a running game, an incomplete addon copy and a
different addon that uses the folder name `AzerothNavigator`, and it swaps the new copy
in only after verifying every file. Start menu > *Azeroth Navigator - WoW addon*, or
[`scripts/windows/Install-AzerothNavigatorAddon.ps1`](../../scripts/windows/Install-AzerothNavigatorAddon.ps1)
from a checkout. Details: [WINDOWS_INSTALL.md](WINDOWS_INSTALL.md#install-the-addon).
The AddOns location stays the user's choice; the helper does not verify it either.

### Out-of-date marking

The TOC declares `## Interface: 16001`. That number comes from community client
exports of the Forever beta (plan section 3.2), not from a client check, so it is
**unverified**. If the client marks the addon out of date, that is itself a result:
record the number the client reports and the `build.interface` value in the export,
and change the TOC only to a value measured that way. Do not install third-party
compatibility tools or edit client files to hide the warning. Early reports about
SavedVariables problems on the beta conflict with later reports of fixes, so whether
writing works at all is checked in M0.5, not assumed.

## Use

1. Log in with the addon enabled. In chat you should see one line starting with
   `AzerothNavigator:` that counts capabilities by result code.
2. Optionally run `/anavdiag probe` or `/anavdiag status`.
3. Log out or run `/reload` so the client writes SavedVariables to disk. Whether and
   when the client writes the file is **unverified**; the addon never forces a UI
   reload.
4. The file is expected at
   `<product folder>/WTF/Account/<account folder>/SavedVariables/AzerothNavigator.lua`
   (conventional layout, **unverified** for Forever). The account folder name can be
   personal data. Do not commit the raw file or its path; sanitize before sharing
   (AGENTS.md: never commit personal exports).
5. Copy results into a filled copy of the capability observations template with the
   conditions under which they were observed. A probe result of `untested` stays
   `untested` until an operator records conditions and evidence.

## Remove

1. Close the client.
2. Delete the folder `AzerothNavigator` from the AddOns directory.
3. Delete the addon's own SavedVariables file `AzerothNavigator.lua` (and a backup
   copy of the same name, if the client keeps one) from the account's
   `SavedVariables` directory.

The Windows helper does steps 2 and 3 with `-Action Uninstall -RemoveSavedVariables`
(only files named `AzerothNavigator.lua` and `AzerothNavigator.lua.bak`), and the
application's uninstaller offers step 2 for the folder the helper recorded.

Nothing else is touched: no other addon, no client file, no executable, no registry
entry, no network configuration. The addon starts no process, opens no socket, reads
no file and has no background task; it runs only inside the client's addon sandbox,
reacts to two events and one slash command, and holds a small table in memory.

## Checks that do not need a client

`cargo test -p azeroth-lua-harness` loads every TOC-listed file into a restricted
Lua 5.1 state with a stub client and checks load order, reload stability, bounds and
privacy rules. The CI job "Lua static checks" runs luacheck over the addon. Neither
says anything about real client behavior; see
[LUA_RUNTIME_DIFFERENCES.md](LUA_RUNTIME_DIFFERENCES.md).

## Experimental character capture (M2.1 branch; not in released v0.1.0)

After installing the updated addon from the M2.1 build, the user-operated command

    /anavdiag snapshot anonymous-one

attempts a bounded capture on client 1.60.1.70338 / interface 16001. Choose a unique
anonymous identifier for each character and keep it consistent across captures.
No character name, GUID, realm or account path is read. The command calls only the
existing build probe and one guarded UnitLevel('player') read; all other fields
remain unknown or absent. Errors do not expose raw client messages.

Run /reload or log out to save. In the updated desktop Imports screen, select the
exact AzerothNavigator.lua file, choose SavedVariables export string, and use
global AzerothNavigatorDB / key snapshot_export. These are editable defaults.
Import selected file, or Watch selected file for later manually requested captures.
diagnostic_export remains a separate diagnostic JSON report and is not importable
as a character snapshot.

UTC/session elapsed clocks are not observed: captures explicitly carry uncertain
clock trust. The logical capture stream uses the anonymous identifier and a
persisted monotonic sequence; it is not an elapsed gameplay session. Do not reset
the counter to make snapshots appear newer. Import conflicts preserve stored data.
Uncertain freshness can prevent planning; level display/import does not establish a
verified live gameplay workflow. Actual capture/import still needs the recorded
client check. No automatic capture or background observation was added.

## Capture timing in v0.2.0

Clock APIs remain unverified on the recorded client. To inspect their guarded output:

    /anavdiag clock

It prints UTC epoch seconds. On Windows, compare with this PowerShell expression:

    [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()

Allow for time passing while switching windows. Only if the reported UTC agrees
with your trusted PC clock, run /anavdiag clock trust within 120 seconds, then run
/anavdiag snapshot with your existing anonymous profile ID. Reload/logout and import
the updated file. This confirmation is explicit and applies only to the current
addon runtime; reload clears trust, so repeat the check before the next capture.

If the APIs are missing, values invalid, a sample takes more than two seconds,
clocks reverse, or UTC/monotonic disagreement exceeds five seconds, timing remains
or becomes uncertain. The thresholds are engineering guards, not verified game facts.
Use /anavdiag clock clear to revoke trust. No trusted state is restored from disk.
The monotonic clock checks consistency only; no elapsed gameplay time is exported.
Level capture remains available with uncertain timing when clock checks fail.

A timestamp alone never grants trust. The desktop continues rejecting future
timestamps and treating captures older than its freshness window as stale.
Fresh capture timing does not supply missing real quest content, gear, or map data.
