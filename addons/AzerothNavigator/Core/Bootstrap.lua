-- Azeroth Navigator diagnostic addon, M0.3. Every client API and event name used by
-- this addon is an UNVERIFIED mainline-style assumption (plan section 3.2).
-- Bootstrap only prepares the shared addon namespace; it touches no client global.
local addonName, ns = ...

if ns.bootstrapped then
  return
end

ns.bootstrapped = true
ns.addonName = addonName
ns.modules = {}

-- Register a table of functions under a name so later files can read it as ns.<name>.
function ns.provide(name, exports)
  ns.modules[#ns.modules + 1] = name
  ns[name] = exports
  return exports
end
