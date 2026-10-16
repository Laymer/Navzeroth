-- Lua static checks for the diagnostic addon (plan section 5.1, M0.3).
-- Run from the repository root: luacheck addons/AzerothNavigator
-- The Lua runtime is 5.1 (WoW family). Which standard-library parts the Forever
-- client really exposes is UNVERIFIED; docs/compatibility/LUA_RUNTIME_DIFFERENCES.md.
std = "lua51"
max_line_length = 100

-- The addon reaches the client only through an injected environment table (the
-- client's _G), so no client function is read as a global and read_globals stays
-- empty on purpose. Add a name here only when addon code references it directly.
read_globals = {}

-- The SavedVariables table named in the TOC and the slash-command key are the only
-- globals the addon may define. Both are bound through the injected environment
-- table in Diagnostics.lua (unverified directive and slash-command mechanics).
globals = {
  "AzerothNavigatorDB",
  "SLASH_ANAV1",
  "SLASH_ANAVDIAG1",
  "SLASH_ANAVSESSION1",
  "SLASH_ANAVPLAN1",
}

-- Whole warning classes stay enabled; do not add ignore patterns to get a clean run.
