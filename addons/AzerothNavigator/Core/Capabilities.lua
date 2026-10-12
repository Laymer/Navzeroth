-- Capability probe. Pure: the client environment is passed in as a table argument,
-- so no client global is read at file scope.
--
-- The probe checks that named functions exist and calls exactly one harmless getter
-- (GetBuildInfo) inside pcall. It never calls unit, chat, combat-log, map-position or
-- item APIs, reads no player name, realm or GUID, and installs no shim for a missing
-- API (plan sections 2.5 and 3.2). Every API name is an UNVERIFIED mainline-style
-- assumption for Forever.
--
-- Status vocabulary matches docs/compatibility/BUILD_CAPABILITIES.md. The probe emits
-- only "untested" and "failed". "verified available", "verified unavailable" and
-- "restricted" need an operator's judgement of conditions (M0.4) and are never
-- written by code.
local _, ns = ...

local STATUS_UNTESTED = "untested"
local STATUS_FAILED = "failed"
local MAX_DETAIL_BYTES = 96
local MAX_TOKEN_BYTES = 32

-- Sorted by id. api_names are existence checks only; manual entries carry a fixed
-- detail_code because no API name is known or no code can observe the capability.
local CATALOGUE = {
  { id = "addon_party_messages", apis = {
    "C_ChatInfo.RegisterAddonMessagePrefix",
    "C_ChatInfo.SendAddonMessage",
  } },
  { id = "build_identification", apis = { "GetBuildInfo" }, call = "GetBuildInfo" },
  { id = "character_level", apis = { "UnitLevel" } },
  { id = "combat_log_availability", apis = { "CombatLogGetCurrentEventInfo" } },
  { id = "equipment_item_links", apis = { "GetInventoryItemLink" } },
  { id = "event_frame_support", apis = { "CreateFrame" } },
  { id = "flight_path_unlocks", apis = { "C_TaxiMap.GetAllTaxiNodes" } },
  { id = "item_data", apis = { "C_Item.GetItemInfo" } },
  { id = "map_position", apis = {
    "C_Map.GetBestMapForUnit",
    "C_Map.GetPlayerMapPosition",
  } },
  { id = "map_zone_rectangle", apis = { "C_Map.GetMapRectOnMap" } },
  { id = "money_bags_durability", apis = {
    "C_Container.GetContainerNumFreeSlots",
    "GetInventoryItemDurability",
    "GetMoney",
  } },
  { id = "quest_completion_history", apis = { "C_QuestLog.IsQuestFlaggedCompleted" } },
  { id = "quest_log", apis = {
    "C_QuestLog.GetInfo",
    "C_QuestLog.GetNumQuestLogEntries",
  } },
  { id = "saved_variables_persistence", apis = {}, manual = "needs_relog_verification" },
  { id = "talents", apis = {}, manual = "manual_check_required" },
  { id = "text_import_capacity", apis = {}, manual = "manual_check_required" },
}

local Capabilities = { catalogue = CATALOGUE }

-- Best-effort redaction of an error message for the export. It collapses anything
-- path-like or hyphenated (Name-Realm, GUID-like tokens), quoted text and every byte
-- outside a small ASCII set, then truncates. It cannot recognise a bare name, so
-- callers must only pass messages from the calls this probe makes.
local function excerpt(message)
  if type(message) ~= "string" then
    return "<" .. type(message) .. ">"
  end
  local text = message
  text = text:gsub("%S*[\\/]%S*", "<path>")
  text = text:gsub("%S*%-%S*", "<id>")
  text = text:gsub("'[^']*'", "'?'")
  text = text:gsub('"[^"]*"', '"?"')
  text = text:gsub("[^%w %.:_%(%)<>'\"]", "?")
  text = text:gsub("%?%?+", "?")
  return text:sub(1, MAX_DETAIL_BYTES)
end

local function cleanToken(value)
  if type(value) == "string" and #value >= 1 and #value <= MAX_TOKEN_BYTES
    and value:match("^[%w%._%-]+$") then
    return value
  end
  return nil
end

local function cleanInterface(value)
  if type(value) == "number" and value == math.floor(value) and value >= 1 and value <= 999999 then
    return string.format("%d", value)
  end
  return nil
end

local function resolve(env, path)
  local current = env
  for segment in path:gmatch("[^.]+") do
    if type(current) ~= "table" then
      return nil
    end
    current = current[segment]
  end
  return current
end

-- Returns "present", "missing" or "error" (indexing raised), plus the function or message.
local function lookup(env, path)
  local ok, value = pcall(resolve, env, path)
  if not ok then
    return "error", value
  end
  if type(value) == "function" then
    return "present", value
  end
  return "missing"
end

local function newBuild()
  local Version = ns.Version
  return {
    edition = Version.edition,
    edition_basis = Version.edition_basis,
    version = "unknown",
    build = "unknown",
    interface = "unknown",
  }
end

-- Calls GetBuildInfo once inside pcall and records only the client's build strings.
local function readBuild(getter, build, record)
  local ok, version, buildNumber, _, interface = pcall(getter)
  if not ok then
    record.status = STATUS_FAILED
    record.detail_code = "call_failed"
    record.detail = excerpt(version)
    return
  end
  local cleanVersion = cleanToken(version)
  local cleanBuild = cleanToken(buildNumber)
  local cleanTarget = cleanInterface(interface)
  build.version = cleanVersion or "unknown"
  build.build = cleanBuild or "unknown"
  build.interface = cleanTarget or "unknown"
  if cleanVersion and cleanBuild and cleanTarget then
    record.detail_code = "call_returned_values"
  else
    record.detail_code = "call_unexpected_return"
  end
end

local function evaluate(env, entry, build)
  local record = { id = entry.id, status = STATUS_UNTESTED, api_names = {}, missing = {} }
  for index, name in ipairs(entry.apis) do
    record.api_names[index] = name
  end
  if #entry.apis == 0 then
    record.detail_code = entry.manual
    return record
  end

  local getter
  local lookupError
  for _, name in ipairs(entry.apis) do
    local state, value = lookup(env, name)
    if state == "missing" then
      record.missing[#record.missing + 1] = name
    elseif state == "error" then
      lookupError = lookupError or value
    elseif name == entry.call then
      getter = value
    end
  end

  if lookupError ~= nil then
    record.status = STATUS_FAILED
    record.detail_code = "lookup_failed"
    record.detail = excerpt(lookupError)
  elseif #record.missing == #entry.apis then
    record.detail_code = "api_missing"
  elseif #record.missing > 0 then
    record.detail_code = "api_partial"
  elseif getter then
    readBuild(getter, build, record)
  else
    record.detail_code = "api_present"
  end
  return record
end

-- Probes env (a table standing in for the client globals). Always returns a report;
-- a missing or failing API only affects its own capability.
function Capabilities.probe(env)
  if type(env) ~= "table" then
    env = {}
  end
  local report = { build = newBuild(), capabilities = {} }
  for index, entry in ipairs(CATALOGUE) do
    report.capabilities[index] = evaluate(env, entry, report.build)
  end
  return report
end

Capabilities.excerpt = excerpt

ns.provide("Capabilities", Capabilities)
