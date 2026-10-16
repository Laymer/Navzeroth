-- Settings merge. Pure: operates on a table passed in, never on a client global.
-- Active settings contain only the keys declared in SCHEMA, with matching types;
-- saved keys that are unknown or mistyped are dropped (the SavedVariables table never
-- grows from stale input). Settings never touch diagnostic_export.
local _, ns = ...

local Version = ns.Version

local SCHEMA = {
  auto_probe = { type = "boolean", default = true },
  hud_visible = { type = "boolean", default = true },
  chat_summary = { type = "boolean", default = true },
  -- In-game window (0.9.2). A 0.9.1 addon drops these keys when it merges the table.
  minimap_button = { type = "boolean", default = true },
  minimap_angle = { type = "number", default = 225, valid = function(value)
    return value == value and value >= 0 and value <= 360
  end },
  gui_last_tab = { type = "string", default = "overview", valid = function(value)
    return ns.GuiText.isTab(value)
  end },
}

for key, value in pairs(ns.HudAppearance.defaults()) do
  SCHEMA[key] = {type=type(value), default=value, appearance=true}
end

local Settings = { SCHEMA = SCHEMA, DB_VERSION = Version.db_version }

function Settings.defaults()
  local result = {}
  for key, spec in pairs(SCHEMA) do
    if spec.appearance then result[key] = ns.HudAppearance.defaults()[key]
    else result[key] = spec.default end
  end
  return result
end

-- True when value is acceptable for key (type and range or enum). Unknown keys are not.
function Settings.validate(key, value)
  local spec = SCHEMA[key]
  if not spec or type(value) ~= spec.type then return false end
  if spec.appearance then return ns.HudAppearance.validate(key, value) end
  return not spec.valid or spec.valid(value) == true
end

-- Returns a fresh active-settings table built from saved (any value) and the defaults.
function Settings.merge(saved)
  local result = Settings.defaults()
  if type(saved) ~= "table" then
    return result
  end
  for key, spec in pairs(SCHEMA) do
    local value = saved[key]
    if type(value) == spec.type and
      (not spec.appearance or ns.HudAppearance.validate(key, value)) and
      (not spec.valid or spec.valid(value)) then
      if key == 'hud_position' then
        result[key] = ns.HudAppearance.position(value.anchor, value.relative, value.x, value.y)
      else result[key] = value end
    end
  end
  return result
end

-- Applies the merge to db.settings and stamps db.db_version. Returns the active
-- settings, or nil plus an error code when db is unusable (db is then left untouched).
function Settings.apply(db)
  if type(db) ~= "table" then
    return nil, "settings_no_db"
  end
  local version = db.db_version
  if version ~= nil and version ~= Settings.DB_VERSION then
    return nil, "settings_db_version"
  end
  db.db_version = Settings.DB_VERSION
  db.settings = Settings.merge(db.settings)
  return db.settings
end

ns.provide("Settings", Settings)
