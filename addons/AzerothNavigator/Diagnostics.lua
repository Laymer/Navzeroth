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

  local function say(message)
    if type(env.print) == "function" then
      pcall(env.print, "AzerothNavigator: " .. message)
    end
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

  local function onSlash(message)
    local command = ""
    if type(message) == "string" then
      command = message:lower():match("^%s*(%S*)") or ""
    end
    if command == "probe" then
      run()
    elseif command == "status" then
      if state.report then
        say(summarize(state.report, state.persist, state.last_code))
      else
        say("no probe has run yet")
      end
    else
      say(Version.addon .. " usage: " .. SLASH_COMMAND .. " probe | status")
    end
  end

  state.onEvent = onEvent
  state.onSlash = onSlash
  state.run = run

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
  end

  return state
end

Diagnostics.state = Diagnostics.attach(_G)

ns.provide("Diagnostics", Diagnostics)
