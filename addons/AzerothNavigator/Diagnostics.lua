-- Diagnostics glue: the only file that binds to the client environment. It receives
-- the environment table (the client's _G, or a stub in tests) and only reads fields
-- from it. Event names, CreateFrame, SlashCmdList and print are UNVERIFIED
-- mainline-style assumptions for Forever (plan section 3.2). The addon never forces
-- a UI reload, reads no chat, and has no background process.
local addonName, ns = ...

local Version = ns.Version
local Capabilities = ns.Capabilities
local Export = ns.Export
local Settings = ns.Settings

local SLASH_KEY = "ANAVDIAG"
local SLASH_COMMAND = "/anavdiag"
local EVENTS = { "ADDON_LOADED", "PLAYER_LOGIN" }

local Diagnostics = {}

local function summarize(report, stored, code)
  local counts = {}
  for _, record in ipairs(report.capabilities) do
    counts[record.detail_code] = (counts[record.detail_code] or 0) + 1
  end
  local names = {}
  for name in pairs(counts) do
    names[#names + 1] = name
  end
  table.sort(names)
  local parts = {}
  for _, name in ipairs(names) do
    parts[#parts + 1] = name .. "=" .. counts[name]
  end
  local line = #report.capabilities .. " capabilities (" .. table.concat(parts, ", ") .. ")"
  if stored then
    return line .. "; export stored, written to disk at logout or reload"
  end
  return line .. "; export not stored (" .. tostring(code) .. ")"
end

-- Wires the addon to env. Returns a state table that tests inspect and drive.
function Diagnostics.attach(env)
  local state = {
    env = env,
    db = nil,
    settings = Settings.defaults(),
    persist = false,
    report = nil,
    last_code = nil,
  }

  local clock = ns.Clock.new(function()
    if type(env.GetServerTime) == 'function' then return env.GetServerTime() end
  end, function()
    if type(env.GetTime) == 'function' then return env.GetTime() end
  end)

  local function say(message)
    if type(env.print) == "function" then
      pcall(env.print, "AzerothNavigator: " .. message)
    end
  end

  local function renderPlan(model)
    state.hud_model = model
    if not state.settings.hud_visible then
      if state.hud_frame then pcall(function() state.hud_frame:Hide() end) end
      return
    end
    if type(env.CreateFrame) ~= 'function' then return end
    local ok = pcall(function()
      if not state.hud_frame then
        local frame = env.CreateFrame('Frame', nil, env.UIParent)
        frame:SetSize(420, 220)
        frame:SetPoint('TOP', 0, -120)
        frame:SetMovable(true)
        frame:EnableMouse(true)
        frame:RegisterForDrag('LeftButton')
        frame:SetScript('OnDragStart', function(self) self:StartMoving() end)
        frame:SetScript('OnDragStop', function(self) self:StopMovingOrSizing() end)
        local text = frame:CreateFontString(nil, 'OVERLAY', 'GameFontNormal')
        text:SetAllPoints(frame)
        text:SetJustifyH('LEFT')
        state.hud_frame, state.hud_text = frame, text
      end
      -- Imported text is display data: escape client texture/hyperlink markup.
      state.hud_text:SetText(state.plan_controls:describe(model):gsub('|', '||'))
      state.hud_frame:Show()
    end)
    if not ok then state.plan_ui_code = 'hud_ui_unavailable' end
  end

  local function ensurePlanControls()
    if state.plan_controls then return state.plan_controls end
    local build = Capabilities.probe(env).build
    if build.version ~= '1.60.1' or build.build ~= '70338' or build.interface ~= '16001' then
      return nil, 'plan_build_unrecorded'
    end
    local context = {namespace='reference', build={edition=Version.edition,
      build=build.version .. '.' .. build.build, interface=build.interface}}
    if not state.last_snapshot and state.persist and state.db.snapshot_export then
      local cached, cachedCode = ns.Snapshot.restoreTransport(state.db.snapshot_export,
        {context=context, fixture=false})
      if cached then state.last_snapshot, state.snapshot_cached = cached, true
      else state.cached_snapshot_code = cachedCode end
    end
    local plans = ns.Plan.new({context=context, fixture=false})
    state.plan_controls = ns.PlanControls.new(plans, function()
      if not state.last_snapshot then return {} end
      local now = clock:capture()
      return {snapshot=state.last_snapshot, fixture=false,
        cached_snapshot=state.snapshot_cached == true,
        freshness={clock_trust=now.clock_trust, now_utc_ms=now.observed_at_utc_ms,
          max_age_ms=1800000}, stale_policy='reject'}
    end, renderPlan)
    return state.plan_controls
  end

  local function planImport(text)
    local controls, code = ensurePlanControls()
    if not controls then say('plan unavailable: ' .. code); return end
    local receipt, importCode = controls:import(text)
    if not receipt then say('plan import failed (' .. importCode .. '); prior plan retained')
    else
      say('plan imported; runtime only, reload clears it')
      say(controls:describe():gsub("|", "||"))
    end
    return receipt, importCode
  end

  local function pastePlan()
    if type(env.CreateFrame) ~= 'function' then say('plan paste UI unavailable'); return end
    local ok = pcall(function()
      if not state.paste_frame then
        local frame = env.CreateFrame('Frame', nil, env.UIParent)
        frame:SetSize(520, 300)
        frame:SetPoint('CENTER')
        local title = frame:CreateFontString(nil, 'OVERLAY', 'GameFontNormal')
        title:SetPoint('TOP', 0, -10)
        title:SetText('Paste ANAV1 plan (32 KiB). Import explicitly; reload clears plan.')
        local input = env.CreateFrame('EditBox', nil, frame)
        input:SetSize(480, 210)
        input:SetPoint('TOP', 0, -35)
        input:SetMultiLine(true)
        input:SetAutoFocus(false)
        input:SetMaxLetters(32769) -- one extra byte makes overflow a visible rejection
        input:SetFontObject('ChatFontNormal')
        input:SetScript('OnEscapePressed', function() frame:Hide() end)
        local import = env.CreateFrame('Button', nil, frame, 'UIPanelButtonTemplate')
        import:SetSize(120, 24); import:SetPoint('BOTTOMLEFT', 20, 12); import:SetText('Import')
        import:SetScript('OnClick', function()
          local read, text = pcall(function() return input:GetText() end)
          if read then planImport(text) else say('plan paste unreadable; prior plan retained') end
        end)
        local close = env.CreateFrame('Button', nil, frame, 'UIPanelButtonTemplate')
        close:SetSize(120, 24); close:SetPoint('BOTTOMRIGHT', -20, 12); close:SetText('Close')
        close:SetScript('OnClick', function() frame:Hide() end)
        state.paste_frame, state.paste_input = frame, input
      end
      state.paste_frame:Show()
    end)
    if not ok then say('plan paste UI unavailable (experimental client binding)') end
  end

  local function planSlash(message)
    local command, tail = '', ''
    if type(message) == 'string' then
      command, tail = message:match('^%s*(%S+)%s*(.*)$')
      command = command and command:lower() or ''
    end
    if command == 'paste' then pastePlan(); return end
    if command == 'import' then return planImport(tail) end
    local controls, code = ensurePlanControls()
    if not controls then say('plan unavailable: ' .. code); return end
    if command == 'clear' then controls:clear(); say('loaded plan cleared')
    elseif command == 'status' then say(controls:describe(controls:status()):gsub("|", "||"))
    elseif command == 'show' or command == 'hide' then
      state.settings.hud_visible = command == 'show'
      if state.persist then state.db.settings.hud_visible = state.settings.hud_visible end
      controls:refresh()
    else say('/anavplan paste | import ANAV1:... | status | show | hide | clear') end
  end

  local function run()
    local report = Capabilities.probe(env)
    local stored, code = false, "export_not_persisted"
    if state.persist then
      stored, code = Export.store(state.db, report)
    end
    state.report = report
    state.last_code = code
    if state.settings.chat_summary then
      say(summarize(report, stored, code))
    end
    return report, stored, code
  end

  local function bindSavedVariables()
    local db = env[Version.saved_variables]
    if db == nil then
      db = {}
      env[Version.saved_variables] = db
    end
    local settings, code = Settings.apply(db)
    if settings then
      state.db = db
      state.settings = settings
      state.persist = true
    else
      state.last_code = code
    end
  end

  local function handleEvent(_, event, loadedAddon)
    if event == "ADDON_LOADED" and loadedAddon == addonName then
      bindSavedVariables()
    elseif event == "PLAYER_LOGIN" and state.settings.auto_probe then
      run()
    end
  end

  local function onEvent(...)
    if not pcall(handleEvent, ...) then
      state.last_code = "handler_failed"
      if not state.error_reported then
        state.error_reported = true
        say("internal error (handler_failed); diagnostics paused for this event")
      end
    end
  end

  local function snapshot(profile)
    if not state.persist then say('snapshot failed: saved variables unavailable'); return end
    if type(profile) ~= 'string' or #profile < 1 or #profile > 80 or
      not profile:match('^[A-Za-z0-9_-]+$') then
      say('usage: /anavdiag snapshot <anonymous-profile-id> (1-80 letters, digits, _ or -)')
      return
    end
    local report = Capabilities.probe(env)
    local build = report.build
    -- Only the user-reported tuple is enabled for this experimental manual slice.
    if build.version ~= '1.60.1' or build.build ~= '70338' or build.interface ~= '16001' then
      say('snapshot failed: build has no recorded capture evidence'); return
    end
    local sequence = state.db.snapshot_sequence
    if sequence == nil then sequence = 0 end
    if type(sequence) ~= 'number' or sequence ~= math.floor(sequence) or
      sequence < 0 or sequence >= 1000000000 then
      say('snapshot failed: invalid capture sequence'); return
    end
    sequence = sequence + 1
    local stream = 'snapshot-stream-' .. profile
    local metadata, clockCode = clock:capture()
    metadata.source, metadata.session_id, metadata.sequence = 'addon_snapshot', stream, sequence
    local getter
    if type(env.UnitLevel) == 'function' then
      getter = function() return env.UnitLevel('player') end
    end
    local level, code = ns.Snapshot.readLevel(getter, metadata)
    local context = {namespace='reference', build={edition=Version.edition,
      build=build.version .. '.' .. build.build, interface=build.interface}}
    local questApi = type(env.C_QuestLog) == 'table' and env.C_QuestLog or {}
    local quests, questDiagnostics = ns.Snapshot.readQuests(questApi.GetNumQuestLogEntries,
      questApi.GetInfo, context, metadata)
    if not quests then say('snapshot failed: ' .. questDiagnostics); return end
    local inventoryGetter
    if type(env.GetInventoryItemLink) == 'function' then
      inventoryGetter = function(slot) return env.GetInventoryItemLink('player', slot) end
    end
    local equipment, equipmentDiagnostics = ns.Snapshot.readEquipment(inventoryGetter,
      context, metadata)
    local spec = {id='snapshot-' .. profile .. '-' .. string.format('%.0f', sequence),
      character=profile, context=context, fixture=false, locale='unknown',
      capture=metadata, source_revision='addon-observations-v1',
      capabilities={level={context=context, status=level.status == 'known' and
        'verified_available' or level.status == 'unsupported' and
        'verified_unavailable' or 'untested'},
        quests={context=context, status='verified_available'},
        equipment={context=context, status='verified_available'}}}
    local captured, errorCode = ns.Snapshot.capture(spec, {level=function() return level end,
      quests=function() return quests end, equipment=function() return equipment end})
    if not captured then say('snapshot failed: ' .. errorCode); return end
    local text, storeCode = ns.Snapshot.store(state.db, captured,
      {fixture=false, locale=spec.locale})
    if not text then say('snapshot failed: ' .. storeCode); return end
    state.db.snapshot_sequence = sequence
    state.db.snapshot_quest_diagnostics = questDiagnostics
    state.db.snapshot_equipment_diagnostics = equipmentDiagnostics
    state.last_snapshot = captured
    state.snapshot_cached = false
    if state.plan_controls then state.plan_controls:refresh() end
    say('snapshot stored (' .. #text .. ' bytes). /reload or logout, then import ' ..
      'AzerothNavigatorDB / snapshot_export from the selected SavedVariables file.')
    if code then say('level remains unavailable (' .. code .. ')') end
    if clockCode or metadata.clock_trust ~= 'trusted' then
      say('capture clock uncertain; use /anavdiag clock to inspect it before confirming trust')
    end
  end

  local function clockCommand(action)
    local build = Capabilities.probe(env).build
    if build.version ~= '1.60.1' or build.build ~= '70338' or build.interface ~= '16001' then
      say('clock unavailable: build has no recorded capture evidence'); return
    end
    if action == 'clear' then clock:clear(); say('clock trust cleared'); return end
    if action and action ~= 'trust' then say('usage: /anavdiag clock [trust|clear]'); return end
    local result, code
    if action == 'trust' then result, code = clock:confirm()
    else result, code = clock:probe() end
    if not result then say('clock remains uncertain (' .. code .. ')'); return end
    say('UTC epoch seconds: ' .. string.format('%.0f', result.utc_seconds) ..
      '; clock ' .. result.clock_trust)
    if action ~= 'trust' then
      say('Compare with trusted UTC on your PC. Only if it agrees, run ' ..
        '/anavdiag clock trust within 120 seconds. Trust resets on reload.')
    end
  end

  local function onSlash(message)
    local command = ""
    if type(message) == "string" then
      command = message:lower():match("^%s*(%S*)") or ""
    end
    if command == "probe" then
      run()
    elseif command == 'position' then
      local ok = pcall(function()
        local build = Capabilities.probe(env).build
        if build.version ~= '1.60.1' or build.build ~= '70338' or
          build.interface ~= '16001' then
          say('position unavailable: build has no recorded capture evidence'); return
        end
        local mapApi = type(env.C_Map) == 'table' and env.C_Map or {}
        local getMap, getPoint
        if type(mapApi.GetBestMapForUnit) == 'function' then
          getMap = function() return mapApi.GetBestMapForUnit('player') end
        end
        if type(mapApi.GetPlayerMapPosition) == 'function' then
          getPoint = function(map) return mapApi.GetPlayerMapPosition(map, 'player') end
        end
        local result = ns.Snapshot.positionDiagnostics(getMap, getPoint)
        state.position_diagnostics = result
        if state.persist then state.db.position_diagnostics = result end
        say('position diagnostic: ' .. result.code ..
          '; snapshot location unknown (map/floor/instance/coordinate meaning unverified)')
      end)
      if not ok then say('position diagnostic failed (position_internal)') end
    elseif command == "clock" then
      local action = type(message) == 'string' and message:match('^%s*%S+%s+(%S+)%s*$')
      if type(message) == 'string' and
        not message:match('^%s*%S+%s*$') and not action then
        say('usage: /anavdiag clock [trust|clear]'); return
      end
      if action then action = action:lower() end
      if not pcall(clockCommand, action) then
        clock:clear(); say('clock remains uncertain (clock_internal)')
      end
    elseif command == "snapshot" then
      local profile = type(message) == 'string' and message:match('^%s*%S+%s+(%S+)%s*$')
      if not pcall(snapshot, profile) then say('snapshot failed: capture_internal') end
    elseif command == "status" then
      if state.report then
        say(summarize(state.report, state.persist, state.last_code))
      else
        say("no probe has run yet")
      end
    else
      say(Version.addon .. " usage: " .. SLASH_COMMAND ..
        " probe | status | position | clock [trust|clear] | snapshot <anonymous-profile-id>")
    end
  end

  local recorder = ns.Session.new(function()
    if type(env.GetTime) == 'function' then return env.GetTime() end
  end, function() return clock:capture() end)
  state.session_recorder = recorder
  local function sessionSlash(message)
    local command, profile = '', nil
    if type(message) == 'string' then
      command, profile = message:match('^%s*(%S+)%s*(.-)%s*$')
      command = command and command:lower() or ''
    end
    if command == 'status' then
      local status = recorder:status()
      say('session ' .. status.phase .. '; events=' .. status.events ..
        '; reason=' .. (status.reason or 'none') ..
        '; clock consistency=' ..
        (status.clock_warning or 'no detected inconsistency; trust not implied') ..
        '; observation=' .. (status.observation_warning or 'none'))
      return
    end
    if command == 'export' then
      if not state.persist then
        say('session export failed: saved variables unavailable'); return
      end
      local text, code = recorder:export(state.db)
      if not text then say('session export failed (' .. code .. '); prior export retained')
      else say('session export stored (' .. #text ..
        ' bytes); reload/logout, then import AzerothNavigatorDB / session_export') end
      return
    end
    if not ({start=true, sample=true, pause=true, resume=true, stop=true})[command] then
      say('/anavsession start <anonymous-profile-id> | sample | pause | resume | stop | ' ..
        'export | status')
      return
    end
    local build = Capabilities.probe(env).build
    if build.version ~= '1.60.1' or build.build ~= '70338' or build.interface ~= '16001' then
      say('session unavailable: build has no recorded capture evidence'); return
    end
    local result, code
    if command == 'start' then
      if not state.persist then say('session start failed: saved variables unavailable'); return end
      if type(profile) ~= 'string' or #profile < 1 or #profile > 80 or
        not profile:match('^[A-Za-z0-9_-]+$') then
        say('usage: /anavsession start <anonymous-profile-id> (1-80 letters, digits, _ or -)')
        return
      end
      local sequence = state.db.session_recording_sequence
      if sequence == nil then sequence = 0 end
      if type(sequence) ~= 'number' or sequence ~= math.floor(sequence) or
        sequence < 0 or sequence >= 1000000000 then
        say('session start failed: invalid recording sequence'); return
      end
      sequence = sequence + 1
      result, code = recorder:start({session='recording-' .. profile .. '-' .. sequence,
        character=profile, fixture=false, context={namespace='reference',
          build={edition=Version.edition, build=build.version .. '.' .. build.build,
            interface=build.interface}}})
      if result then state.db.session_recording_sequence = sequence end
    elseif command == 'sample' then
      result, code = recorder:sample(function(metadata)
        local getter
        if type(env.UnitLevel) == 'function' then
          getter = function() return env.UnitLevel('player') end
        end
        return ns.Snapshot.readLevel(getter, metadata)
      end)
    else result, code = recorder[command](recorder) end
    local status = recorder:status()
    if result then say('session ' .. status.phase .. '; manual ' .. command .. ' recorded')
    else say('session action failed (' .. (code or 'session_internal') ..
      '); state=' .. status.phase .. '; reason=' .. (status.reason or 'none')) end
    if status.clock_warning then say('session clock warning: ' .. status.clock_warning) end
    if status.observation_warning then
      say('session observation warning: ' .. status.observation_warning)
    end
  end

  state.onEvent = onEvent
  state.onSlash = onSlash
  state.run = run
  state.onPlanSlash = function(message)
    if not pcall(planSlash, message) then say('plan action failed (plan_internal)') end
  end
  state.onSessionSlash = function(message)
    if not pcall(sessionSlash, message) then say('session action failed (session_internal)') end
  end

  if type(env.CreateFrame) == "function" then
    local created, frame = pcall(env.CreateFrame, "Frame")
    if created and frame ~= nil then
      local wired = pcall(function()
        for _, event in ipairs(EVENTS) do
          frame:RegisterEvent(event)
        end
        frame:SetScript("OnEvent", onEvent)
      end)
      state.frame = wired and frame or nil
    end
  end

  if type(env.SlashCmdList) == "table" then
    env.SlashCmdList[SLASH_KEY] = onSlash
    env["SLASH_" .. SLASH_KEY .. "1"] = SLASH_COMMAND
    env.SlashCmdList.ANAVPLAN = state.onPlanSlash
    env.SLASH_ANAVPLAN1 = '/anavplan'
    env.SlashCmdList.ANAVSESSION = state.onSessionSlash
    env.SLASH_ANAVSESSION1 = '/anavsession'
  end

  return state
end

Diagnostics.state = Diagnostics.attach(_G)

ns.provide("Diagnostics", Diagnostics)
