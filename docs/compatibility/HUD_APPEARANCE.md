# Local HUD appearance

This experimental binding remains limited to recorded version1.60.1/build70338/
interface16001. Frame methods and an actual boolean combat query are required.
Synthetic harness checks establish engineering behavior; real-client UI, taint,
focus, action-bar and display acceptance remains unverified.

The same controls are on the HUD and Plan tabs of the in-game window (`/anav`, see
[ADDON_GUI.md](ADDON_GUI.md)); each button calls the matching action below, refuses in
combat or when the combat state is unknown, and shows the requested versus applied
values and the application code in words. The window adds no anchor control: it has no
slash equivalent, and dragging plus reset already cover position.

Use explicit commands:

- `/anavplan size 420 220`: width280–960, height160–720 logical UI units.
- `/anavplan font 150`: percent100,125,150,175 or200 of this HUD's captured local font.
- `/anavplan contrast high`: white text on an opaque black local texture, when
  required methods exist. `contrast default` restores the captured text color.
- `/anavplan reset`: default420×220/font100/default contrast/TOP position(0,-120).
- `/anavplan status`: refresh the evaluation and show requested preferences plus
  the last application result and observed movement persistence status.
- `/anavplan hide` or `show`: persisted visibility request.

Drag with the left mouse button when allowed. Position persists only after a
successful stop and a successful single-point observation relative to actual
UIParent, with allowlisted anchor/relative anchor and finite offsets within±4096.
Offsets round to whole logical UI units. Missing methods, failed reads, multiple
anchors, another parent, malformed or out-of-range observations retain the saved
position and report session-only movement. Model refreshes, font/size/contrast
changes and hide/show do not re-anchor unchanged position preferences, so the
session-only drag survives until reload or an explicit reset. No coordinates are
inferred. Reset explicitly applies the default anchor even when the saved tuple
already equals that default; its pending anchor request survives combat deferral
and a failed setter, and supersedes a pending drag persistence observation.

Known combat, unavailable/throwing combat queries and nonboolean results defer
HUD mutations, including creation, text, visibility and drag operations. One
latest request is retained. Supported combat-end event registration or another
explicit status/show/appearance command retries with a fresh gate and a fresh
plan evaluation. An unsupported event does not break startup events. A deferred
card is still labeled as a last evaluated addon observation; it can remain visible
until mutations are allowed. The addon does not claim that frozen text stays current.

`hud_applied` in diagnostic runtime state records the last fully successful
configuration, not a continuous native observation. A failed method can leave
partial UI changes; the application result stays unavailable/deferred and the
latest request remains pending. Requested preferences never establish plan readiness.

Appearance fields are additive in SavedVariables DB version1: `hud_width`,
`hud_height`, `hud_font_percent`, `hud_contrast`, and `hud_position` with only
`anchor`, `relative`, `x`, `y`. Old settings receive defaults; false booleans survive.
Existing schema validation still discards unknown/mistyped active settings. Future
DB versions remain untouched and use runtime-only defaults. Diagnostic/snapshot
exports and unrelated saved fields are preserved. Plans remain runtime-only.

Font scaling uses a captured per-HUD baseline and never compounds or mutates a
global font. No targeting, casting, gameplay movement, focus operation, protected
action template, external bridge or automatic import is added. The plan paste UI
and informational slash commands retain their existing behavior and bounds.
