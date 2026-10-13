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
}

local Settings = { SCHEMA = SCHEMA, DB_VERSION = Version.db_version }

function Settings.defaults()
  local result = {}
  for key, spec in pairs(SCHEMA) do
    result[key] = spec.default
  end
  return result
end

-- Returns a fresh active-settings table built from saved (any value) and the defaults.
function Settings.merge(saved)
  local result = Settings.defaults()
  if type(saved) ~= "table" then
    return result
  end
  for key, spec in pairs(SCHEMA) do
    local value = saved[key]
    if type(value) == spec.type then
      result[key] = value
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
