-- Every label, tooltip and state sentence of the in-game window, keyed by id, so a
-- later localization pass touches one file (plan section 7.4). Pure data and small
-- formatters: no client global, no frame. Text is ASCII until localization exists.
local _, ns = ...

local Text = {}

Text.title = 'Azeroth Navigator'
Text.not_affiliated = 'Not affiliated with or endorsed by Blizzard Entertainment.'

-- Tab ids avoid words the source scan rejects as identifiers or string literals.
Text.tabs = {
  {id = 'overview', label = 'Overview'},
  {id = 'plan', label = 'Plan'},
  {id = 'hud', label = 'HUD'},
  {id = 'capture', label = 'Capture'},
  {id = 'session', label = 'Session'},
  {id = 'diagnostics', label = 'Diagnostics'},
  {id = 'settings', label = 'Settings'},
}
Text.tab_ids = {}
for _, tab in ipairs(Text.tabs) do Text.tab_ids[#Text.tab_ids + 1] = tab.id end

function Text.isTab(id)
  for _, known in ipairs(Text.tab_ids) do
    if known == id then return true end
  end
  return false
end

local COMBAT = 'Refused during combat or while combat state is unknown.'
local TARGET = 'Needs an admitted build. ' .. COMBAT
local STORE = 'Needs an admitted build and loaded SavedVariables. ' .. COMBAT
local ANY = 'No build gate. ' .. COMBAT
local TYPING = 'Typing is always allowed; the action buttons apply the gates.'
local WINDOW = 'none (window only)'

-- widgets[id] = {label, tooltip_title, tooltip_body, slash_equivalent, gate_hint}
local function entry(label, title, body, slash, gate)
  return {label = label, tooltip_title = title, tooltip_body = body,
    slash_equivalent = slash, gate_hint = gate}
end

local W = {}
Text.widgets = W

for _, tab in ipairs(Text.tabs) do
  W['tab_' .. tab.id] = entry(tab.label, tab.label .. ' page',
    'Shows the ' .. tab.label .. ' page of this window. The choice is remembered.',
    WINDOW, 'Always available; the choice is saved only outside combat.')
end

W.close = entry('Close', 'Close window',
  'Hides this window. Escape closes it too. Nothing is captured or changed.',
  '/anav', 'Always available.')
W.refresh = entry('Refresh', 'Refresh this page',
  'Re-reads the addon state and redraws this page. It captures nothing new.',
  '/anavdiag status', ANY)
W.minimap = entry('Azeroth Navigator', 'Azeroth Navigator',
  'Click to open or close the window. Drag along the minimap rim to move this button.',
  '/anav', 'Opening is refused during combat or while combat state is unknown.')
W.options_open = entry('Open Azeroth Navigator', 'Open the window',
  'Opens the Azeroth Navigator window with every feature of the addon.',
  '/anav', 'Opening is refused during combat or while combat state is unknown.')

W.probe = entry('Run probe', 'Run capability probe',
  'Checks which client functions exist and stores a diagnostic export. It reads no '
  .. 'player data and verifies nothing.', '/anavdiag probe',
  'No build gate; the export needs loaded SavedVariables. ' .. COMBAT)
W.hud_toggle = entry('Show HUD', 'Show or hide the HUD',
  'Shows or hides the plan HUD card. The preference is saved. The card itself may '
  .. 'defer; the HUD page says why.', '/anavplan show | hide', TARGET)

W.plan_paste = entry('Plan text', 'Plan text box',
  'Paste an ANAV1 plan exported by the desktop app (up to 32 KiB). Nothing is '
  .. 'imported until you press Import.', '/anavplan import ANAV1:...', TYPING)
W.plan_import = entry('Import', 'Import the pasted plan',
  'Validates the pasted plan and replaces the loaded plan only if it is valid. '
  .. 'Plans are runtime only: a reload clears them.', '/anavplan import <text>', TARGET)
W.plan_clear = entry('Clear plan', 'Clear the loaded plan',
  'Removes the loaded plan from this session. The HUD returns to no plan.',
  '/anavplan clear', TARGET)
W.plan_hud_toggle = entry('Show HUD', 'Show or hide the HUD',
  'Shows or hides the plan HUD card. The preference is saved.',
  '/anavplan show | hide', TARGET)
W.plan_status = entry('Refresh status', 'Re-evaluate the loaded plan',
  'Evaluates the loaded plan against the last capture and updates the reasons below. '
  .. 'It captures nothing new.', '/anavplan status', TARGET)

W.hud_width = entry('Width', 'HUD width',
  'Card width in pixels, 280 to 960. Press Apply size to use it.',
  '/anavplan size <W> <H>', TARGET)
W.hud_height = entry('Height', 'HUD height',
  'Card height in pixels, 160 to 720. Press Apply size to use it.',
  '/anavplan size <W> <H>', TARGET)
W.hud_size_apply = entry('Apply size', 'Apply HUD size',
  'Saves the width and height and asks the HUD to use them. The HUD may defer.',
  '/anavplan size <W> <H>', TARGET)
W.hud_font = entry('Font', 'HUD text size',
  'Cycles the HUD text size through 100, 125, 150, 175 and 200 percent.',
  '/anavplan font <percent>', TARGET)
W.hud_contrast = entry('High contrast', 'High contrast HUD',
  'Uses a solid black card with white text. Off uses the normal card.',
  '/anavplan contrast default | high', TARGET)
W.hud_reset = entry('Reset HUD', 'Reset the HUD',
  'Restores the default size, text size, contrast and position.',
  '/anavplan reset', TARGET)

W.capture_profile = entry('Profile id', 'Anonymous profile id',
  'A label for your captures: 1 to 80 letters, digits, underscore or hyphen. Do not '
  .. 'use a character name.', '/anavdiag snapshot <id>', TYPING)
W.capture_snapshot = entry('Capture snapshot', 'Capture a snapshot',
  'Reads level, quests and equipment links once and stores a snapshot export in '
  .. 'SavedVariables. Unreadable values stay unknown.', '/anavdiag snapshot <id>', STORE)
W.capture_position = entry('Position diagnostic', 'Position diagnostic',
  'Records whether the map position functions answer. The snapshot location stays '
  .. 'unknown.', '/anavdiag position', TARGET)
W.capture_copy_snapshot = entry('Copy snapshot export', 'Show the stored snapshot',
  'Shows the stored snapshot text in a selectable window. Press Ctrl+C yourself; '
  .. 'nothing is captured or sent.', '/anavdiag copy snapshot',
  'Needs a stored snapshot. ' .. COMBAT)
W.capture_copy_diagnostic = entry('Copy diagnostic export', 'Show the stored diagnostic',
  'Shows the stored diagnostic text in a selectable window. Press Ctrl+C yourself.',
  '/anavdiag copy', 'Needs a stored diagnostic. ' .. COMBAT)

W.session_profile = entry('Profile id', 'Anonymous profile id',
  'A label for this recording: 1 to 80 letters, digits, underscore or hyphen. Do not '
  .. 'use a character name.', '/anavsession start <id>', TYPING)
W.session_start = entry('Start', 'Start a recording',
  'Starts a manual recording. Nothing is sampled until you press Sample.',
  '/anavsession start <id>', STORE)
W.session_start_quests = entry('Start with quests', 'Start with quest turn-ins',
  'Starts a recording that also notes quest turn-in events while it runs. Unavailable '
  .. 'when the client does not offer the event.', '/anavsession start-quests <id>', STORE)
W.session_sample = entry('Sample', 'Record a sample',
  'Records one level reading and one unclassified activity marker.',
  '/anavsession sample', TARGET)
W.session_pause = entry('Pause', 'Pause the recording',
  'Pauses the running recording.', '/anavsession pause', TARGET)
W.session_resume = entry('Resume', 'Resume the recording',
  'Resumes a paused recording.', '/anavsession resume', TARGET)
W.session_stop = entry('Stop', 'Stop the recording',
  'Stops the recording. A stopped recording can be exported.',
  '/anavsession stop', TARGET)
W.session_export = entry('Export', 'Store the recording',
  'Stores the stopped recording in SavedVariables. /reload or log out to write it to '
  .. 'disk.', '/anavsession export', 'Needs a stopped recording and loaded SavedVariables. '
  .. COMBAT)
W.session_copy = entry('Copy session export', 'Show the stored session',
  'Shows the stored session text in a selectable window. Press Ctrl+C yourself.',
  '/anavdiag copy session', 'Needs a stored session export. ' .. COMBAT)

W.diag_probe = entry('Run probe', 'Run capability probe',
  'Checks which client functions exist and stores a diagnostic export.',
  '/anavdiag probe', 'No build gate; the export needs loaded SavedVariables. ' .. COMBAT)
W.diag_clock_check = entry('Check clock', 'Check the clock',
  'Reads the server UTC time and starts a confirmation. It does not trust the clock '
  .. 'by itself.', '/anavdiag clock', TARGET)
W.diag_clock_trust = entry('Trust clock', 'Trust the clock',
  'Compare the UTC value shown with trusted UTC on your PC first. Only if they agree, '
  .. 'press this within 120 seconds of the check. Trust resets on reload.',
  '/anavdiag clock trust', TARGET)
W.diag_clock_clear = entry('Clear trust', 'Clear clock trust',
  'Forgets the clock check and any trust decision.', '/anavdiag clock clear', TARGET)

W.set_auto_probe = entry('Probe at login', 'Probe at login',
  'Runs the capability probe once when you log in.', WINDOW, ANY)
W.set_chat_summary = entry('Chat summary', 'Chat summary',
  'Prints the one-line probe summary in chat. Off keeps the summary in this window.',
  WINDOW, ANY)
W.set_minimap_button = entry('Minimap button', 'Minimap button',
  'Shows the Azeroth Navigator button on the minimap rim.', WINDOW, ANY)

Text.admission = {
  release_target = 'Release target build. Capture features are enabled; every live '
    .. 'check is still UNVERIFIED.',
  development_unverified = 'Development build, UNVERIFIED. Admitted for capture only: '
    .. 'installation metadata, no addon-run evidence. Not the 1.0 release target.',
  previous_recorded = 'Previously recorded build. Capture is enabled; it is not the '
    .. 'release target.',
  none = 'Build not admitted. Probe, status and copy work; capture, plan, clock and '
    .. 'session actions refuse.',
}

Text.not_exposed = {
  {id = 'gear_scoring', reason = 'Gear scoring runs on synthetic fixtures only; it is '
    .. 'not bound to the client.'},
  {id = 'party_sync', reason = 'Party sync is deferred; the addon sends no messages.'},
  {id = 'reminders', reason = 'Reminders and the secondary card have no addon data '
    .. 'source in this release.'},
  {id = 'manual_advance', reason = 'There is no manual step advance; steps never '
    .. 'complete by themselves.'},
  {id = 'persistence', reason = 'Plans are runtime only; a reload clears them.'},
  {id = 'reload', reason = 'The addon never reloads the interface; type /reload '
    .. 'yourself when an export must reach the disk.'},
}

Text.options_intro = 'Azeroth Navigator Diagnostics. Open the window with /anav or the '
  .. 'minimap button. Every control has a tooltip that says what it does. '
  .. Text.not_affiliated

Text.help_lines = {
  '/anav [help]: open or close this window; help lists the commands.',
  '/anavdiag probe | status | copy [snapshot|session] | position',
  '/anavdiag clock [trust|clear] | snapshot <profile-id>',
  '/anavplan paste | import ANAV1:... | status | show | hide | clear',
  '/anavplan size W H | font 100/125/150/175/200 | contrast default/high | reset',
  '/anavsession start|start-quests <profile-id> | sample | pause | resume | stop',
  '/anavsession export | status',
}

local CODES = {
  hud_build_unrecorded = 'build not admitted',
  hud_combat_unknown = 'combat state unknown',
  hud_combat_deferred = 'deferred until combat ends',
  hud_ui_unavailable = 'client frame functions missing',
  hud_text_unavailable = 'client text functions missing',
  hud_font_unavailable = 'client font read refused',
  hud_contrast_unavailable = 'client contrast functions missing',
  hud_movement_unavailable = 'client movement functions missing',
  hud_hidden = 'hidden by preference',
  hud_applied = 'applied',
  hud_movement_session_only = 'position kept for this session only',
  hud_movement_persisted = 'position saved',
  hud_movement_not_observed = 'not moved yet',
  copy_combat_unknown = 'combat state unknown',
  copy_combat_deferred = 'deferred until combat ends',
  copy_ui_unavailable = 'client text window unavailable',
  copy_shown = 'shown',
  copy_export_missing = 'no stored export yet',
  copy_export_invalid = 'stored export is not valid',
  copy_export_too_large = 'stored export is too large',
  copy_db_unavailable = 'SavedVariables not loaded',
  copy_kind_invalid = 'unknown export kind',
  plan_build_unrecorded = 'build not admitted',
  plan_imported = 'imported',
  transport_bytes = 'text is larger than 32 KiB; prior plan kept',
  transport_invalid = 'text must start with ANAV1:; prior plan kept',
  plan_invalid = 'plan failed validation; prior plan kept',
  hud_size_invalid = 'size refused',
  gui_combat_deferred = 'refused: you are in combat',
  gui_combat_unknown = 'refused: combat state unknown',
  gui_ui_unavailable = 'window functions missing',
  gui_minimap_registered = 'registered',
  gui_minimap_unavailable = 'not available on this client',
  gui_minimap_hidden = 'hidden by setting',
  gui_options_registered = 'registered',
  gui_options_unavailable = 'not available on this client',
  no_plan = 'no plan loaded',
  waiting = 'waiting',
  observations_invalid = 'observations invalid',
  context_mismatch = 'context mismatch',
  fixture_mismatch = 'fixture mismatch',
  clock_uncertain = 'clock uncertain',
  expired = 'expired',
  dependencies_changed = 'dependencies changed',
  evaluation_error = 'evaluation error',
  participant_missing = 'participant missing',
  no_steps = 'no steps',
  complete = 'complete',
  ready = 'ready',
  blocked = 'blocked',
  uncertain = 'uncertain',
  invalidated = 'invalidated',
  snapshot_superseded = 'snapshot superseded',
}

-- A code in words; an unknown code is shown as itself, never hidden.
function Text.explain(code)
  if code == nil then return 'none' end
  return CODES[code] or tostring(code)
end

-- Returns the tooltip title and an array of lines for a widget id.
function Text.tooltip(id)
  local spec = W[id]
  if not spec then return nil end
  return spec.tooltip_title, {spec.tooltip_body, 'Slash command: ' .. spec.slash_equivalent,
    'Gate: ' .. spec.gate_hint}
end

ns.provide('GuiText', Text)
