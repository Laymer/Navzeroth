# Addon: the in-game window

Task G3 (0.9.2 program). The addon has one window that exposes every addon feature
through buttons and fields, with a tooltip on every control. It adds no capability:
each button calls the same guarded code as the matching slash command, and every slash
command keeps working. **Nothing here has been observed in a Forever client.** The
window, its templates, tooltips, minimap button and options page are built from
UNVERIFIED mainline-style assumptions and tested only against a synthetic mock
(`crates/lua-harness/tests/common/ui.rs`). The checks that need the real client are
listed in the LIVE-GUI section of the [acceptance runbook](V1_ACCEPTANCE_RUNBOOK.md).

## Opening it

| Entry point | How | If the client lacks it |
| --- | --- | --- |
| Slash command | `/anav` toggles the window; `/anav help` lists every command in chat | Always registered |
| Minimap button | Left-click toggles the window; drag it around the minimap rim (the angle is saved) | No button; status code `gui_minimap_unavailable` |
| Options page | Under the game's Options > AddOns: a short description and an **Open Azeroth Navigator** button | Skipped; status code `gui_options_unavailable` |
| Escape | The frame `AzerothNavigatorFrame` is in `UISpecialFrames`, so Escape closes it | Use the Close button |

The minimap button and options page are registered in their own `PLAYER_LOGIN` branch,
independent of `auto_probe`. Registration is silent: a missing API sets a status code
(`state.gui_minimap_code`, `state.gui_options_code`) and prints nothing, so the one-line
login summary is unchanged and an `auto_probe=false` login still registers both. No new
event is registered. No frame is created until the window is first opened, and at load
only the existing event frame exists.

The window never calls `EnableKeyboard` or `SetPropagateKeyboardInput`, never takes
focus when it opens (every edit box is created with auto-focus off), and does not use
`StaticPopupDialogs`, `HideUIPanel`, `SettingsPanel`, `ReloadUI`, timers or
`OnUpdate`. A source scan in `static_scan.rs` enforces this.

## Combat policy

Opening the window uses a **combat-only gate**: it refuses while the player is in combat
or when the combat state is unknown (no `InCombatLockdown`, a non-boolean answer, or an
error), records the reason in the message list, and creates nothing. It deliberately does
not use the HUD gate, which also refuses unadmitted builds; the window must open on any
build so it can explain the build state.

Every action button applies the same combat check on each click and calls nothing when it
refuses. Closing the window and switching tabs stay possible in combat; the remembered tab
is saved only out of combat. Slash behaviour is unchanged. Each action keeps its own build
gate (below), so a refused action reports its reason in the window.

## Build admission

The Overview shows the build and what it means:

| Label | Build | Meaning |
| --- | --- | --- |
| `release_target` | 1.60.1.70374 / 16001 | The 1.0 release target. Capture features are enabled; every live check is still UNVERIFIED. |
| `development_unverified` | 1.60.1.70425 / 16001 | Admitted for development capture only. UNVERIFIED: installation metadata, no addon-run evidence. Not the 1.0 release target. |
| `previous_recorded` | 1.60.1.70338 / 16001 | A previously recorded build. Capture is enabled. |
| not admitted | anything else | Probe, status and copy work; capture, plan, clock and session actions refuse. |

`Capabilities.admission(build)` returns these labels; `isCaptureTarget` is true exactly
when a label exists. Every export records the exact build, so 70425 data is never mistaken
for 70374 data. See [BUILD_CAPABILITIES.md](BUILD_CAPABILITIES.md).

## Tabs

Every control has a tooltip with what it does, its slash equivalent and its gate. States
are written in words (ON/OFF, `[Tab]` for the active tab, status sentences), never by
colour alone. Dynamic text from imported data has pipes doubled and control characters
replaced before it is shown.

| Tab | Controls | Shows |
| --- | --- | --- |
| Overview | Run probe, Show/Hide HUD, Refresh | Build, admission and its meaning, plan status, recorder phase, clock trust, the last five messages |
| Plan | 32 KiB paste box, Import, Clear plan, Show/Hide HUD, Refresh status | Receipt (id and steps), import status, plan status, and a read-only reason inspector (prerequisite and invalidation reasons, rejected alternatives, revalidation flags) |
| HUD | Width and height fields (280-960, 160-720) with Apply size, text size cycle (100-200), High contrast, Reset HUD | Requested versus applied size, the HUD application code and the movement code. There is no anchor control: it has no slash equivalent, and dragging the HUD plus Reset already cover it |
| Capture | Profile id field, Capture snapshot, Position diagnostic, Copy snapshot export, Copy diagnostic export | Bytes, sequence, quest and equipment diagnostics, last error, copy status |
| Session | Profile id field, Start, Start with quests, Sample, Pause, Resume, Stop, Export, Copy session export | Phase, events (of 128), warnings, quest event availability |
| Diagnostics | Run probe, Check clock, Trust clock, Clear trust | Clock state, the probe summary and the 16 capability rows |
| Settings | Probe at login, Chat summary, Minimap button | Version and edition (UNVERIFIED), every slash command, what is deliberately not in the window, the non-affiliation line |

Behaviours worth knowing:

- **Profile ids** are validated live with the rule `/anavdiag snapshot` and
  `/anavsession start` use (1-80 of `A-Z a-z 0-9 _ -`); the same function
  (`Gui.validProfile`) backs both. Buttons that need a valid id stay disabled until it is.
- **Trust clock** is enabled only while a clock check is pending, at most 120 seconds
  old and coherent. It sits next to the server UTC value and the compare-then-confirm
  instruction: compare the value with trusted UTC on your PC first. Trust resets on reload.
- **Start with quests** is disabled, with the reason shown, when the client does not
  offer the `QUEST_TURNED_IN` event.
- **Plan paste** keeps the text after a refused import and clears it after a successful
  one. Oversize text (more than 32768 bytes) is refused, never truncated.
- **HUD fields** follow the saved request only when it changes, so typing is never replaced.
- Messages (what `say` printed, plus window notices) are kept in a ring of 20 so feedback
  does not depend on chat.

## Deliberately not in the window

Reasons live in `Core/GuiText.lua` (`not_exposed`) and are shown on the Settings tab.

- **Gear scoring** runs on synthetic fixtures only and is not bound to the client.
- **Party sync** is deferred; the addon sends no messages.
- **Reminders and the secondary card** have no addon data source in this release.
- **Manual step advance**: there is none, and steps never complete by themselves.
- **Persistence across reload**: plans are runtime only.
- **ReloadUI**: the addon never reloads the interface; type `/reload` yourself.

## Fallbacks

Every `CreateFrame` that names a template runs under `pcall`. If a template is missing the
widget falls back to a plain frame with a colour texture, as the copy view always has. The
window panel uses `BackdropTemplate` and the dialog textures when they exist and a solid
colour texture otherwise. Tooltips use `GameTooltip`; without it a one-line inline help
appears in the window footer. A page that fails to build leaves the other pages working
and sets `gui_page_unavailable`.

## Code layout

`Core/GuiText.lua` (every label and tooltip, keyed by id), `Core/GuiWidgets.lua` (the
guarded widget factory, minimap button and options page), `Core/Gui.lua` (the pure view
model `Gui.model`/`Gui.inspect` and the controller `Gui.new(env, state, actions)`).
`Diagnostics.lua` exposes `state.actions` (probe, status, copy, position, clock, snapshot,
plan, session, `setSetting`, note, gui). `Gui.new` returns `toggle`, `open`, `close`,
`refresh` and `select`. Logic stays in Core because Lua 5.1 limits a closure to 60
upvalues.

## Tests

`crates/lua-harness/tests/common/ui.rs` is the shared mock (parent tree, script dispatch,
FontStrings, a recording `GameTooltip`, `Minimap`, `UISpecialFrames`, the options APIs,
templates that raise when unknown, `InCombatLockdown` as true, false, returning nothing or raising).
`gui.rs` drives every button, walks the tree for tooltips, and covers fallbacks, combat,
admission, the minimap angle, missing APIs and the login output. A passing run is
synthetic engineering evidence only.
