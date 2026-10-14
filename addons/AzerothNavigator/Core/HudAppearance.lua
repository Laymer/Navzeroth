-- Pure, bounded local display preferences. No client API or readiness inference.
local _, ns = ...
local Appearance = {}
local anchors = {TOP=true, TOPLEFT=true, TOPRIGHT=true, LEFT=true, CENTER=true,
  RIGHT=true, BOTTOM=true, BOTTOMLEFT=true, BOTTOMRIGHT=true}
local fonts = {[100]=true, [125]=true, [150]=true, [175]=true, [200]=true}
local function integer(value, low, high)
  return type(value) == 'number' and value == value and value >= low and
    value <= high and value == math.floor(value)
end
function Appearance.position(anchor, relative, x, y)
  if not anchors[anchor] or not anchors[relative] or
    not integer(x, -4096, 4096) or not integer(y, -4096, 4096) then return nil end
  return {anchor=anchor, relative=relative, x=x, y=y}
end
function Appearance.defaults()
  return {hud_width=420, hud_height=220, hud_font_percent=100, hud_contrast='default',
    hud_position={anchor='TOP', relative='TOP', x=0, y=-120}}
end
function Appearance.validate(key, value)
  if key == 'hud_width' then return integer(value, 280, 960) end
  if key == 'hud_height' then return integer(value, 160, 720) end
  if key == 'hud_font_percent' then return fonts[value] == true end
  if key == 'hud_contrast' then return value == 'default' or value == 'high' end
  if key == 'hud_position' and type(value) == 'table' then
    local count = 0
    for name in pairs(value) do
      if name ~= 'anchor' and name ~= 'relative' and name ~= 'x' and name ~= 'y' then
        return false
      end
      count = count + 1
    end
    return count == 4 and Appearance.position(value.anchor, value.relative, value.x, value.y) ~= nil
  end
  return false
end
function Appearance.copy(current)
  local result = Appearance.defaults()
  for key in pairs(result) do
    if current and Appearance.validate(key, current[key]) then
      local value = current[key]
      if key == 'hud_position' then
        value = Appearance.position(value.anchor, value.relative, value.x, value.y)
      end
      result[key] = value
    end
  end
  return result
end
function Appearance.command(command, tail, current)
  if type(tail) ~= 'string' or #tail > 80 then return nil, 'hud_argument_invalid' end
  local result = Appearance.copy(current)
  if command == 'size' then
    local width, height = tail:match('^(%d+)%s+(%d+)$')
    width, height = tonumber(width), tonumber(height)
    if not Appearance.validate('hud_width', width) or
      not Appearance.validate('hud_height', height) then return nil, 'hud_size_invalid' end
    result.hud_width, result.hud_height = width, height
  elseif command == 'font' then
    local percent = tail:match('^(%d+)$')
    percent = tonumber(percent)
    if not Appearance.validate('hud_font_percent', percent) then return nil, 'hud_font_invalid' end
    result.hud_font_percent = percent
  elseif command == 'contrast' then
    if not Appearance.validate('hud_contrast', tail) then return nil, 'hud_contrast_invalid' end
    result.hud_contrast = tail
  elseif command == 'reset' and tail == '' then result = Appearance.defaults()
  else return nil, 'hud_argument_invalid' end
  return result
end
function Appearance.gate(build, combatGetter)
  -- Protect the complete observation/truth decision, including restricted values.
  local ok, allowed, code = pcall(function()
    if not ns.Capabilities.isCaptureTarget(build) then return false, 'hud_build_unrecorded' end
    if type(combatGetter) ~= 'function' then return false, 'hud_combat_unknown' end
    local combat = combatGetter()
    if type(combat) ~= 'boolean' then return false, 'hud_combat_unknown' end
    if combat then return false, 'hud_combat_deferred' end
    return true, 'hud_gate_allowed'
  end)
  if not ok then return false, 'hud_combat_unknown' end
  return allowed, code
end
ns.provide('HudAppearance', Appearance)
