-- In-game window: a pure view model (Gui.model, Gui.inspect) and a controller (Gui.new).
-- The controller creates no frame until it is opened, never re-parses slash commands
-- (every button calls the actions table), and refuses every action while combat is
-- active or unknown. Live behavior of the templates, tooltips and minimap on the
-- Forever client is UNVERIFIED.
local _, ns = ...

local Gui = {}
local Text = ns.GuiText
local Capabilities = ns.Capabilities

local WIDTH, HEIGHT = 700, 560
local PAGE_X, PAGE_Y, PAGE_W, PAGE_H = 24, 84, 652, 420
local FONTS = {100, 125, 150, 175, 200}
local MESSAGE_LINES = 5

-- Dynamic text is bounded and its pipe characters are doubled so imported ids can never
-- start a client escape sequence. Control characters become spaces.
local function esc(value)
  local text
  if type(value) == 'string' then text = value
  elseif type(value) == 'number' or type(value) == 'boolean' then text = tostring(value)
  else text = '<' .. type(value) .. '>' end
  text = text:sub(1, 120):gsub('[\1-\31]', ' ')
  return (text:gsub('|', '||'))
end
Gui.escape = esc

-- Same rule as /anavdiag snapshot and /anavsession start: 1-80 of A-Z a-z 0-9 _ -.
function Gui.validProfile(profile)
  return type(profile) == 'string' and #profile >= 1 and #profile <= 80 and
    profile:match('^[A-Za-z0-9_-]+$') ~= nil
end

-- True when no combat is active. A missing, failing or non-boolean answer is unknown
-- and refuses, the conservative choice of plan section 11.1.
function Gui.combatGate(env)
  local ok, combat = pcall(function()
    if type(env.InCombatLockdown) ~= 'function' then return nil end
    return env.InCombatLockdown()
  end)
  if not ok or type(combat) ~= 'boolean' then return false, 'gui_combat_unknown' end
  if combat then return false, 'gui_combat_deferred' end
  return true
end

local function describeInput(input)
  if type(input) ~= 'table' then return esc(input) end
  local parts = {esc(input.input)}
  local field = input.field
  if type(field) == 'table' then
    for _, key in ipairs({'field', 'quest', 'objective', 'key', 'state'}) do
      if field[key] ~= nil then parts[#parts + 1] = esc(field[key]) end
    end
  end
  return table.concat(parts, ' ')
end

local function gateLines(label, gate, lines)
  if type(gate) ~= 'table' then
    lines[#lines + 1] = label .. ': not evaluated'
    return
  end
  local head = label .. ': ' .. esc(gate.truth or 'unknown')
  if gate.observation_status then
    head = head .. ' (observation ' .. esc(gate.observation_status) .. ')'
  end
  lines[#lines + 1] = head
  for index, item in ipairs(gate.reasons or {}) do
    if index > 8 then lines[#lines + 1] = '  ...more reasons omitted'; break end
    local why = type(item) == 'table' and item.reason or nil
    local reason = type(why) == 'table' and why.reason or why
    local status = type(why) == 'table' and why.status or nil
    lines[#lines + 1] = '  - ' .. describeInput(item.input) .. ': ' .. esc(reason) ..
      (status and (' / ' .. esc(status)) or '')
  end
end

-- The read-only reason inspector (plan section 12.4 item 4): why the HUD says what it
-- says, from the last HUD model. Pure; reads only the model.
function Gui.inspect(model)
  if type(model) ~= 'table' then return {'No plan has been evaluated in this session.'} end
  local lines = {'Status: ' .. Text.explain(model.status) ..
    (model.step_id and (' (step ' .. esc(model.step_id) .. ')') or '')}
  if model.objective then lines[#lines + 1] = 'Objective: ' .. esc(model.objective) end
  if model.status == 'no_plan' then
    lines[#lines + 1] = 'Import a plan on this page to see prerequisite reasons.'
  end
  if model.prerequisite or model.invalidation then
    gateLines('Prerequisite', model.prerequisite, lines)
    gateLines('Invalidation', model.invalidation, lines)
  end
  if type(model.rejected_alternatives) == 'table' and #model.rejected_alternatives > 0 then
    lines[#lines + 1] = 'Rejected alternatives:'
    for index, item in ipairs(model.rejected_alternatives) do
      if index > 8 then break end
      lines[#lines + 1] = '  - ' .. esc(item.objective) .. ': ' .. esc(item.reason)
    end
  end
  if model.selected_alternative then
    lines[#lines + 1] = 'An alternative objective is selected; travel needs revalidation.'
  end
  if model.requires_revalidation then lines[#lines + 1] = 'Revalidation required.' end
  if model.plan_id then
    lines[#lines + 1] = 'Objective completion mapping unavailable; steps never auto-complete.'
  end
  return lines
end

function Gui.summary(report)
  if type(report) ~= 'table' then return 'No probe has run in this session.' end
  local counts, names = {}, {}
  for _, record in ipairs(report.capabilities) do
    counts[record.detail_code] = (counts[record.detail_code] or 0) + 1
  end
  for name in pairs(counts) do names[#names + 1] = name end
  table.sort(names)
  local parts = {}
  for _, name in ipairs(names) do parts[#parts + 1] = name .. '=' .. counts[name] end
  return #report.capabilities .. ' capabilities (' .. table.concat(parts, ', ') .. ')'
end

local function clockInfo(state)
  local read = state.clock and state.clock.status
  local ok, status = false, nil
  if read then ok, status = pcall(read) end
  if not ok or type(status) ~= 'table' then
    return {line = 'Clock: status not available', status = nil}
  end
  local line
  if not status.available then line = 'Clock: unavailable (time functions unreadable)'
  elseif status.trusted then line = 'Clock: trusted (confirmed in this session)'
  elseif status.pending and status.confirmable then
    line = 'Clock: check pending, server UTC epoch seconds ' ..
      string.format('%.0f', status.pending_utc_seconds) ..
      '. Compare with trusted UTC on your PC; trust within 120 seconds.'
  elseif status.pending then
    line = 'Clock: uncertain; the pending check expired or changed. Check the clock again.'
  else line = 'Clock: uncertain (not confirmed)' end
  return {line = line, status = status}
end

local function storedBytes(db, key)
  if type(db) == 'table' and type(db[key]) == 'string' then return #db[key] end
  return nil
end

local function diagnosticText(value)
  if value == nil then return 'none' end
  if type(value) == 'string' then return esc(value) end
  local text = ns.Json.encode(value, 300)
  return text and esc(text) or 'present (not displayable)'
end

-- The complete read-only view: text blocks per tab plus the enable flags the controller
-- applies. inputs carries the profile text typed so far. It reads state and never writes.
function Gui.model(env, state, inputs)
  inputs = inputs or {}
  local model = {}
  local report
  local ok, probed = pcall(Capabilities.probe, env)
  if ok then report = probed end
  local build = report and report.build
  local admission = build and Capabilities.admission(build) or nil
  local settings = state.settings or {}
  local recorder = state.session_recorder and state.session_recorder:status() or {phase = 'idle'}
  local clock = clockInfo(state)
  local hud = state.hud_model

  local overview = {}
  overview[1] = 'Build: ' .. esc(build and build.version or 'unknown') .. ' / ' ..
    esc(build and build.build or 'unknown') .. ' / interface ' ..
    esc(build and build.interface or 'unknown')
  overview[2] = 'Admission: ' .. (admission or 'not admitted')
  overview[3] = Text.admission[admission or 'none']
  overview[4] = 'Edition: ' .. esc(ns.Version.edition) .. ' (assumed target, UNVERIFIED)'
  overview[5] = 'Plan: ' .. (hud and Text.explain(hud.status) or 'not evaluated yet')
  overview[6] = 'Recorder: ' .. esc(recorder.phase) .. ', events ' ..
    esc(recorder.events or 0) .. '/' .. ns.Session.MAX_EVENTS
  overview[7] = clock.line
  overview[8] = ''
  overview[9] = 'Latest messages:'
  local messages = state.messages or {}
  if #messages == 0 then overview[10] = '  (none yet)' end
  for index = math.max(1, #messages - MESSAGE_LINES + 1), #messages do
    overview[#overview + 1] = '  ' .. esc(messages[index])
  end
  model.overview = overview
  model.hud_visible = settings.hud_visible == true

  local plan = {}
  local receipt = state.plan_receipt
  if receipt then
    plan[1] = 'Receipt: ' .. esc(receipt.id) .. ', ' .. esc(receipt.steps) .. ' steps, ' ..
      'fixture ' .. esc(receipt.fixture)
  else plan[1] = 'Receipt: none (nothing imported in this session)' end
  plan[2] = 'Last import: ' .. Text.explain(state.plan_import_code)
  plan[3] = 'Plan status: ' .. (hud and Text.explain(hud.status) or 'not evaluated yet')
  plan[4] = 'Plans are runtime only; a reload clears them.'
  model.plan = plan
  model.inspector = Gui.inspect(hud)

  local requested = ns.HudAppearance.copy(settings)
  local applied = state.hud_applied
  model.hud = {
    'Requested: ' .. requested.hud_width .. ' x ' .. requested.hud_height .. ', text ' ..
      requested.hud_font_percent .. '%, contrast ' .. requested.hud_contrast,
    applied and ('Applied: ' .. applied.hud_width .. ' x ' .. applied.hud_height ..
      ', text ' .. applied.hud_font_percent .. '%, contrast ' .. applied.hud_contrast) or
      'Applied: not applied yet',
    'Application: ' .. Text.explain(state.plan_ui_code),
    'Movement: ' .. Text.explain(state.hud_movement_code or 'hud_movement_not_observed'),
    'HUD preference: ' .. (settings.hud_visible and 'shown' or 'hidden'),
  }
  model.requested = requested

  local db = state.db
  local snapshotBytes = storedBytes(db, 'snapshot_export')
  model.capture = {
    'Snapshot: ' .. (snapshotBytes and (snapshotBytes .. ' bytes stored') or 'none stored'),
    'Capture sequence: ' .. esc(db and db.snapshot_sequence or 0),
    'Quest diagnostics: ' .. diagnosticText(db and db.snapshot_quest_diagnostics),
    'Equipment diagnostics: ' .. diagnosticText(db and db.snapshot_equipment_diagnostics),
    'Last snapshot error: ' .. diagnosticText(db and db.snapshot_export_error),
    'Position diagnostic: ' .. (state.position_diagnostics and
      esc(state.position_diagnostics.code) or 'not run'),
    'Copy window: ' .. Text.explain(state.copy_ui_code),
    'Snapshot location stays unknown; map meaning is unverified.',
  }
  model.capture_valid = Gui.validProfile(inputs.capture_profile)
  model.session_valid = Gui.validProfile(inputs.session_profile)

  local phase = recorder.phase
  local session = {
    'Phase: ' .. esc(phase) .. ', events ' .. esc(recorder.events or 0) .. '/' ..
      ns.Session.MAX_EVENTS,
    'Reason: ' .. esc(recorder.reason or 'none'),
    'Clock warning: ' .. esc(recorder.clock_warning or 'none'),
    'Observation warning: ' .. esc(recorder.observation_warning or 'none'),
    state.quest_turn_in_event_available and 'Quest turn-in events: available' or
      'Quest turn-in events: unavailable (event support not established)',
    'Stored session export: ' .. (storedBytes(db, 'session_export') and
      (storedBytes(db, 'session_export') .. ' bytes') or 'none'),
  }
  model.session = session
  model.can = {
    session_start = model.session_valid and phase ~= 'recording' and phase ~= 'paused',
    session_start_quests = model.session_valid and phase ~= 'recording' and
      phase ~= 'paused' and state.quest_turn_in_event_available == true,
    session_sample = phase == 'recording',
    session_pause = phase == 'recording',
    session_resume = phase == 'paused',
    session_stop = phase == 'recording' or phase == 'paused',
    session_export = phase == 'stopped',
    diag_clock_trust = clock.status ~= nil and clock.status.confirmable == true,
    capture_snapshot = model.capture_valid,
  }
  -- Checking a stored export decodes it, so only the pages that show it pay for it.
  model.copy = {}
  local wantCopy = inputs.tab == nil or inputs.tab == 'capture' or inputs.tab == 'session'
  for _, kind in ipairs({'diagnostic', 'snapshot', 'session'}) do
    local selected, code = nil, 'copy_not_checked'
    if wantCopy then selected, code = ns.CopyExport.select(db, kind) end
    model.copy[kind] = {ok = selected ~= nil, code = code}
  end
  if not model.session_valid then
    session[#session + 1] = 'Profile id must be 1-80 letters, digits, _ or -.'
  end
  if not state.quest_turn_in_event_available then
    session[#session + 1] = 'Start with quests is disabled: the quest turn-in event is ' ..
      'not available on this client.'
  end

  local rows = {Gui.summary(report), ''}
  if report then
    for _, record in ipairs(report.capabilities) do
      rows[#rows + 1] = record.id .. ': ' .. record.detail_code .. ' (' .. record.status .. ')'
    end
  end
  model.diagnostics = {clock.line, 'Probe: ' .. Gui.summary(state.report),
    'Last export result: ' .. esc(state.last_code or 'none')}
  model.rows = rows

  local help = {'Addon ' .. ns.Version.addon .. ', edition ' .. ns.Version.edition ..
    ' (assumed, UNVERIFIED), TOC interface ' .. ns.Version.toc_interface .. ' (UNVERIFIED).',
    Text.not_affiliated, '', 'Slash commands:'}
  for _, line in ipairs(Text.help_lines) do help[#help + 1] = '  ' .. line end
  help[#help + 1] = ''
  help[#help + 1] = 'Deliberately not in this window:'
  for _, item in ipairs(Text.not_exposed) do help[#help + 1] = '  ' .. item.reason end
  help[#help + 1] = ''
  help[#help + 1] = 'Entry points: minimap ' .. Text.explain(state.gui_minimap_code) ..
    ', options page ' .. Text.explain(state.gui_options_code) .. '.'
  model.help = help
  model.settings = settings
  model.admission = admission
  return model
end

local function join(lines) return table.concat(lines, '\n') end

local function at(factory, widget, parent, x, y) factory:place(widget, parent, x, y) end

-- Builds one button from its text id, places it and registers it by id.
local function button(ctx, parent, id, x, y, width, onClick)
  local widget = ctx.f:button(parent, id, width, 26, onClick)
  widget:setLabel(Text.widgets[id].label)
  at(ctx.f, widget, parent, x, y)
  ctx.gui.widgets[id] = widget
  return widget
end

local function checkbox(ctx, parent, id, y, onToggle)
  local widget = ctx.f:checkbox(parent, id, onToggle)
  at(ctx.f, widget, parent, 0, y)
  ctx.f:place(widget.line, parent, 34, y + 5)
  widget:setCaption(Text.widgets[id].label)
  ctx.gui.widgets[id] = widget
  return widget
end

local function field(ctx, parent, id, x, y, width, onChange)
  local widget = ctx.f:editbox(parent, id, width, 24, {max = 81, onChange = onChange})
  at(ctx.f, widget, parent, x, y)
  ctx.gui.widgets[id] = widget
  return widget
end

local function captionAt(ctx, parent, x, y, width, text)
  local label = ctx.f:label(parent, x, y, width)
  label:set(text)
  return label
end

local function enable(widget, flag) if widget then widget:setEnabled(flag == true) end end

local function buildOverview(ctx, page)
  local info = ctx.f:label(page, 0, 0, PAGE_W)
  button(ctx, page, 'probe', 0, 280, 150, function()
    ctx.act('probe', function() return ctx.actions.probe() end)
  end)
  local toggle = button(ctx, page, 'hud_toggle', 160, 280, 150, function()
    ctx.act('hud_toggle', function() return ctx.toggleHud() end)
  end)
  button(ctx, page, 'refresh', 320, 280, 150, function() ctx.act('refresh', function() end) end)
  ctx.updaters.overview = function(model)
    info:set(join(model.overview))
    toggle:setLabel(model.hud_visible and 'Hide HUD' or 'Show HUD')
  end
end

local function buildPlan(ctx, page)
  captionAt(ctx, page, 0, 0, PAGE_W, 'Paste an ANAV1 plan (up to 32 KiB), then press Import.')
  local paste = ctx.f:scrollEdit(page, 'plan_paste', 620, 110, {max = 32769})
  at(ctx.f, {frame = paste.scroll}, page, 0, 22)
  ctx.gui.widgets.plan_paste = paste
  button(ctx, page, 'plan_import', 0, 142, 150, function()
    ctx.act('plan_import', function()
      local receipt = ctx.actions.plan.import(paste:getText())
      if receipt then paste:setText('') end
      return receipt
    end)
  end)
  button(ctx, page, 'plan_clear', 160, 142, 150, function()
    ctx.act('plan_clear', function() return ctx.actions.plan.clear() end)
  end)
  local toggle = button(ctx, page, 'plan_hud_toggle', 320, 142, 150, function()
    ctx.act('plan_hud_toggle', function() return ctx.toggleHud() end)
  end)
  button(ctx, page, 'plan_status', 480, 142, 150, function()
    ctx.act('plan_status', function() return ctx.actions.plan.status() end)
  end)
  local status = ctx.f:label(page, 0, 178, PAGE_W)
  captionAt(ctx, page, 0, 226, PAGE_W, 'Reason inspector (read only)')
  local inspector = ctx.f:scrollText(page, 'plan_inspector', 640, 160)
  at(ctx.f, inspector, page, 0, 246)
  ctx.gui.widgets.plan_inspector = inspector
  ctx.updaters.plan = function(model)
    status:set(join(model.plan))
    inspector:set(join(model.inspector))
    toggle:setLabel(model.hud_visible and 'Hide HUD' or 'Show HUD')
  end
end

local function buildHud(ctx, page)
  captionAt(ctx, page, 0, 6, 60, 'Width')
  local width = ctx.f:numberField(page, 'hud_width', 70, 280, 960, function()
    ctx.gui.refresh()
  end)
  at(ctx.f, width, page, 60, 2)
  captionAt(ctx, page, 150, 6, 60, 'Height')
  local height = ctx.f:numberField(page, 'hud_height', 70, 160, 720, function()
    ctx.gui.refresh()
  end)
  at(ctx.f, height, page, 210, 2)
  ctx.gui.widgets.hud_width, ctx.gui.widgets.hud_height = width, height
  local apply = button(ctx, page, 'hud_size_apply', 300, 0, 150, function()
    ctx.act('hud_size_apply', function()
      local w, h = width:value(), height:value()
      if not w or not h then
        ctx.actions.note('HUD size refused: width 280-960 and height 160-720, whole numbers')
        return nil, 'hud_size_invalid'
      end
      ctx.hudResync = true
      return ctx.actions.plan.size(w, h)
    end)
  end)
  local hint = ctx.f:label(page, 0, 34, PAGE_W)
  local font = ctx.f:cycleButton(page, 'hud_font', 200, FONTS, function(_, value)
    ctx.act('hud_font', function() return ctx.actions.plan.font(value) end)
  end)
  font.caption = Text.widgets.hud_font.label
  at(ctx.f, font, page, 0, 64)
  ctx.gui.widgets.hud_font = font
  local contrast = checkbox(ctx, page, 'hud_contrast', 100, function(_, checked)
    ctx.act('hud_contrast', function()
      return ctx.actions.plan.contrast(checked and 'high' or 'default')
    end)
  end)
  button(ctx, page, 'hud_reset', 0, 136, 150, function()
    ctx.act('hud_reset', function()
      ctx.hudResync = true
      return ctx.actions.plan.reset()
    end)
  end)
  local status = ctx.f:label(page, 0, 176, PAGE_W)
  local last
  ctx.updaters.hud = function(model)
    -- Fields follow the saved request when it changes or after Apply/Reset ran, so
    -- typing is never replaced but a successful Reset always clears invalid text.
    local request = model.requested
    if ctx.hudResync or not last or last.hud_width ~= request.hud_width or
      last.hud_height ~= request.hud_height then
      ctx.hudResync = false
      last = request
      width:setText(tostring(request.hud_width))
      height:setText(tostring(request.hud_height))
    end
    font:select(model.requested.hud_font_percent)
    contrast:setChecked(model.requested.hud_contrast == 'high')
    local note = {}
    note[1] = width:value() and 'Width ok.' or 'Width must be a whole number from 280 to 960.'
    note[2] = height:value() and 'Height ok.' or 'Height must be a whole number from 160 to 720.'
    hint:set(note[1] .. ' ' .. note[2])
    enable(apply, width:value() ~= nil and height:value() ~= nil)
    status:set(join(model.hud))
  end
end

local function buildCapture(ctx, page)
  captionAt(ctx, page, 0, 6, 90, 'Profile id')
  local profile = field(ctx, page, 'capture_profile', 90, 2, 260, function()
    ctx.gui.refresh()
  end)
  local hint = ctx.f:label(page, 360, 6, 290)
  local snapshot = button(ctx, page, 'capture_snapshot', 0, 40, 170, function()
    ctx.act('capture_snapshot', function() return ctx.actions.snapshot(profile:getText()) end)
  end)
  button(ctx, page, 'capture_position', 180, 40, 170, function()
    ctx.act('capture_position', function() return ctx.actions.position() end)
  end)
  local copySnapshot = button(ctx, page, 'capture_copy_snapshot', 0, 76, 190, function()
    ctx.act('capture_copy_snapshot', function() return ctx.actions.copy('snapshot') end)
  end)
  local copyDiagnostic = button(ctx, page, 'capture_copy_diagnostic', 200, 76, 190,
    function()
      ctx.act('capture_copy_diagnostic', function() return ctx.actions.copy('diagnostic') end)
    end)
  local status = ctx.f:label(page, 0, 120, PAGE_W)
  ctx.inputs.capture_profile = function() return profile:getText() end
  ctx.updaters.capture = function(model)
    hint:set(model.capture_valid and 'Profile id ok.' or
      'Use 1-80 letters, digits, _ or -.')
    enable(snapshot, model.capture_valid)
    enable(copySnapshot, model.copy.snapshot.ok)
    enable(copyDiagnostic, model.copy.diagnostic.ok)
    local lines = {}
    for index, line in ipairs(model.capture) do lines[index] = line end
    lines[#lines + 1] = 'Snapshot copy: ' .. Text.explain(model.copy.snapshot.code or
      'copy_shown')
    lines[#lines + 1] = 'Diagnostic copy: ' .. Text.explain(model.copy.diagnostic.code or
      'copy_shown')
    status:set(join(lines))
  end
end

local function buildSession(ctx, page)
  captionAt(ctx, page, 0, 6, 90, 'Profile id')
  local profile = field(ctx, page, 'session_profile', 90, 2, 260, function()
    ctx.gui.refresh()
  end)
  ctx.inputs.session_profile = function() return profile:getText() end
  local a = ctx.actions
  local function run(name, fn)
    return function() ctx.act(name, fn) end
  end
  local buttons = {
    session_start = button(ctx, page, 'session_start', 0, 40, 110,
      run('session_start', function() return a.session.start(profile:getText()) end)),
    session_start_quests = button(ctx, page, 'session_start_quests', 120, 40, 150,
      run('session_start_quests', function()
        return a.session.startQuests(profile:getText())
      end)),
    session_sample = button(ctx, page, 'session_sample', 280, 40, 110,
      run('session_sample', function() return a.session.sample() end)),
    session_pause = button(ctx, page, 'session_pause', 400, 40, 110,
      run('session_pause', function() return a.session.pause() end)),
    session_resume = button(ctx, page, 'session_resume', 520, 40, 110,
      run('session_resume', function() return a.session.resume() end)),
    session_stop = button(ctx, page, 'session_stop', 0, 76, 110,
      run('session_stop', function() return a.session.stop() end)),
    session_export = button(ctx, page, 'session_export', 120, 76, 110,
      run('session_export', function() return a.session.export() end)),
    session_copy = button(ctx, page, 'session_copy', 240, 76, 190,
      run('session_copy', function() return a.copy('session') end)),
  }
  local status = ctx.f:label(page, 0, 120, PAGE_W)
  ctx.updaters.session = function(model)
    for id, widget in pairs(buttons) do
      if id == 'session_copy' then enable(widget, model.copy.session.ok)
      else enable(widget, model.can[id]) end
    end
    status:set(join(model.session))
  end
end

local function buildDiagnostics(ctx, page)
  button(ctx, page, 'diag_probe', 0, 0, 130, function()
    ctx.act('diag_probe', function() return ctx.actions.probe() end)
  end)
  button(ctx, page, 'diag_clock_check', 140, 0, 130, function()
    ctx.act('diag_clock_check', function() return ctx.actions.clock.check() end)
  end)
  local trust = button(ctx, page, 'diag_clock_trust', 280, 0, 130, function()
    ctx.act('diag_clock_trust', function() return ctx.actions.clock.trust() end)
  end)
  button(ctx, page, 'diag_clock_clear', 420, 0, 130, function()
    ctx.act('diag_clock_clear', function() return ctx.actions.clock.clear() end)
  end)
  local status = ctx.f:label(page, 0, 36, PAGE_W)
  local rows = ctx.f:scrollText(page, 'diag_rows', 640, 290)
  at(ctx.f, rows, page, 0, 112)
  ctx.gui.widgets.diag_rows = rows
  ctx.updaters.diagnostics = function(model)
    enable(trust, model.can.diag_clock_trust)
    status:set(join(model.diagnostics))
    rows:set(join(model.rows))
  end
end

local function buildSettings(ctx, page)
  local function setting(id, key, y)
    local widget = checkbox(ctx, page, id, y, function(_, checked)
      ctx.act(id, function() return ctx.actions.setSetting(key, checked) end)
    end)
    return widget
  end
  local boxes = {
    auto_probe = setting('set_auto_probe', 'auto_probe', 0),
    chat_summary = setting('set_chat_summary', 'chat_summary', 34),
    minimap_button = setting('set_minimap_button', 'minimap_button', 68),
  }
  local help = ctx.f:scrollText(page, 'settings_help', 640, 290)
  at(ctx.f, help, page, 0, 112)
  ctx.gui.widgets.settings_help = help
  ctx.updaters.settings = function(model)
    for key, box in pairs(boxes) do box:setChecked(model.settings[key] == true) end
    help:set(join(model.help))
  end
end

local BUILDERS = {
  overview = buildOverview, plan = buildPlan, hud = buildHud, capture = buildCapture,
  session = buildSession, diagnostics = buildDiagnostics, settings = buildSettings,
}

-- Returns the controller: toggle, open, close, refresh and select. Nothing is created
-- here; the first open builds the frames under pcall.
function Gui.new(env, state, actions)
  local gui = {widgets = {}, pages = {}, tab = nil, frame = nil}
  local ctx = {env = env, state = state, actions = actions, gui = gui, updaters = {},
    inputs = {}}

  -- Runs one GUI-initiated action: refused (and recorded) in or near combat.
  function ctx.act(name, fn)
    local allowed, code = Gui.combatGate(env)
    if not allowed then
      state.gui_code = code
      actions.note(name .. ' not run: ' .. Text.explain(code))
      gui.refresh()
      return nil, code
    end
    local ok, first, second = pcall(fn)
    if not ok then
      actions.note(name .. ' failed (gui_action_internal)')
      first, second = nil, 'gui_action_internal'
    end
    gui.refresh()
    return first, second
  end

  function ctx.toggleHud()
    if state.settings and state.settings.hud_visible then return actions.plan.hide() end
    return actions.plan.show()
  end

  local function inputs()
    local values = {}
    for key, read in pairs(ctx.inputs) do values[key] = read() end
    values.tab = gui.tab
    return values
  end

  local function frameCall(frame, name, ...)
    if type(frame[name]) == 'function' then return pcall(frame[name], frame, ...) end
    return false
  end

  local function build()
    ctx.f = ns.GuiWidgets.new(env, Text)
    local panel = ctx.f:panel(env.UIParent, WIDTH, HEIGHT, 'AzerothNavigatorFrame')
    local frame = panel.frame
    frameCall(frame, 'SetPoint', 'CENTER')
    frameCall(frame, 'SetFrameStrata', 'DIALOG')
    frameCall(frame, 'EnableMouse', true)
    frameCall(frame, 'SetMovable', true)
    frameCall(frame, 'SetClampedToScreen', true)
    frameCall(frame, 'RegisterForDrag', 'LeftButton')
    frameCall(frame, 'SetScript', 'OnDragStart', function(self)
      frameCall(self, 'StartMoving')
    end)
    frameCall(frame, 'SetScript', 'OnDragStop', function(self)
      frameCall(self, 'StopMovingOrSizing')
    end)
    frameCall(frame, 'Hide')
    ctx.partial = frame
    gui.frame = frame
    local title = ctx.f:label(frame, 24, 14, 520, 'GameFontNormal')
    title:set(Text.title .. ' ' .. ns.Version.addon .. ' (client binding UNVERIFIED)')
    local close = ctx.f:button(frame, 'close', 80, 24, function() gui.close() end)
    close:setLabel(Text.widgets.close.label)
    ctx.f:place(close, frame, WIDTH - 104, 10)
    gui.widgets.close = close
    ctx.bar = ctx.f:tabBar(frame, Text.tabs, function(id) gui.select(id) end)
    for id, widget in pairs(ctx.bar.buttons) do gui.widgets['tab_' .. id] = widget end
    ctx.footer = ctx.f:label(frame, 24, HEIGHT - 66, PAGE_W)
    ctx.helpLine = ctx.f:label(frame, 24, HEIGHT - 42, PAGE_W)
    ctx.f.inline_help = function(message) ctx.helpLine:set(message) end
    for _, tab in ipairs(Text.tabs) do
      local pageFrame = ctx.f:create('Frame', frame)
      frameCall(pageFrame, 'SetSize', PAGE_W, PAGE_H)
      frameCall(pageFrame, 'SetPoint', 'TOPLEFT', frame, 'TOPLEFT', PAGE_X, -PAGE_Y)
      frameCall(pageFrame, 'Hide')
      gui.pages[tab.id] = pageFrame
      local built = pcall(BUILDERS[tab.id], ctx, pageFrame)
      if not built then
        state.gui_code = 'gui_page_unavailable'
        actions.note('page ' .. tab.id .. ' is unavailable (gui_page_unavailable)')
      end
    end
    local specials = env.UISpecialFrames
    if type(specials) == 'table' then
      local present = false
      for _, name in ipairs(specials) do
        if name == 'AzerothNavigatorFrame' then present = true end
      end
      if not present then table.insert(specials, 'AzerothNavigatorFrame') end
    end
    ctx.partial = nil
  end

  -- Releases whatever a failed build created, so a retry starts from nothing: hidden,
  -- detached from the parent and no longer reachable under the global name.
  local function discardPartial()
    local frame = ctx.partial or gui.frame
    ctx.partial = nil
    if frame then
      frameCall(frame, 'Hide')
      frameCall(frame, 'ClearAllPoints')
      frameCall(frame, 'SetParent', nil)
      if env.AzerothNavigatorFrame == frame then env.AzerothNavigatorFrame = nil end
      local specials = env.UISpecialFrames
      if type(specials) == 'table' then
        for index = #specials, 1, -1 do
          if specials[index] == 'AzerothNavigatorFrame' then table.remove(specials, index) end
        end
      end
    end
  end

  -- Every refresh rebuilds the model, and the model re-reads the clock status through
  -- clockInfo, a pure function of the clock state at that moment; nothing about the
  -- clock is cached between refreshes. The plan forbids timers and OnUpdate in the
  -- window, so the page catches up on the next refresh, tab change or open.
  gui.refresh = function()
    if not gui.frame or not gui.tab then return false end
    local ok = pcall(function()
      local model = Gui.model(env, state, inputs())
      gui.model = model
      local last = state.messages and state.messages[#state.messages]
      ctx.footer:set('Latest message: ' .. (last and esc(last) or 'none yet'))
      local update = ctx.updaters[gui.tab]
      if update then update(model) end
    end)
    if not ok then state.gui_code = 'gui_refresh_failed' end
    return ok
  end

  gui.select = function(id, remember)
    if not gui.frame or not Text.isTab(id) then return false end
    for tab, pageFrame in pairs(gui.pages) do
      frameCall(pageFrame, tab == id and 'Show' or 'Hide')
    end
    gui.tab = id
    ctx.bar.setActive(id)
    if remember ~= false and Gui.combatGate(env) and
      state.settings and state.settings.gui_last_tab ~= id then
      actions.setSetting('gui_last_tab', id)
    end
    gui.refresh()
    return true
  end

  gui.open = function()
    local allowed, code = Gui.combatGate(env)
    if not allowed then
      state.gui_code = code
      actions.note('window not opened: ' .. Text.explain(code))
      return false, code
    end
    if not gui.frame then
      local built = pcall(build)
      if not built or not gui.frame then
        discardPartial()
        gui.frame, gui.tab, gui.widgets, gui.pages = nil, nil, {}, {}
        ctx.updaters, ctx.inputs = {}, {}
        state.gui_code = 'gui_ui_unavailable'
        actions.note('window unavailable (gui_ui_unavailable)')
        return false, 'gui_ui_unavailable'
      end
    end
    local tab = state.settings and state.settings.gui_last_tab
    gui.select(gui.tab or (Text.isTab(tab) and tab or 'overview'), false)
    frameCall(gui.frame, 'Show')
    state.gui_code = 'gui_shown'
    return true
  end

  gui.close = function()
    if not gui.frame then return false end
    frameCall(gui.frame, 'Hide')
    state.gui_code = 'gui_hidden'
    return true
  end

  gui.toggle = function()
    local shown = false
    if gui.frame and type(gui.frame.IsShown) == 'function' then
      local ok, value = pcall(gui.frame.IsShown, gui.frame)
      shown = ok and value == true
    end
    if shown then return gui.close() end
    return gui.open()
  end

  gui.isShown = function()
    if not gui.frame then return false end
    local ok, value = pcall(gui.frame.IsShown, gui.frame)
    return ok and value == true
  end

  return gui
end

ns.provide('Gui', Gui)
