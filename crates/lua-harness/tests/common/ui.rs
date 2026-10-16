//! Shared widget mock for the in-game window tests. It is a synthetic stand-in for
//! the client UI: it models a parent tree, script dispatch, FontStrings, textures, a
//! recording `GameTooltip`, `Minimap`, `UISpecialFrames`, the options APIs, templates
//! that raise when unknown and a controllable `InCombatLockdown`. Nothing here says
//! anything about the real Forever client; the live-GUI runbook rows stay unverified.
#![allow(dead_code)]

use super::stub_client;
use azeroth_lua_harness::Sandbox;

/// Lua source of the mock. It installs the global `ui` table (controls and helpers),
/// replaces `CreateFrame` and defines the client globals the window looks for.
pub const UI_MOCK: &str = r#"
ui = {
  ns = ns, combat = false, mono = 100, wall = 1791792000, quest_event = false,
  frames = {}, moves = 0, categories = {}, registered = {}, legacy = {},
  fail = {}, backdrop_works = true,
}
ui.templates = {UIPanelButtonTemplate = true, UICheckButtonTemplate = true,
  InputBoxTemplate = true, UIPanelScrollFrameTemplate = true, BackdropTemplate = true,
  UIPanelCloseButton = true}
ui.fonts = {GameFontNormal = true, GameFontHighlight = true, ChatFontNormal = true}

local COMMON, BY_KIND, REGION = {}, {Frame = {}, Button = {}, CheckButton = {},
  EditBox = {}, ScrollFrame = {}}, {}

local function visible(f)
  while f do
    if not f.shown then return false end
    f = f.parent
  end
  return true
end
ui.visible = visible

function ui.fire(f, name, ...)
  local handler = f.scripts[name]
  if handler then return handler(f, ...) end
end

function COMMON.SetSize(f, w, h) f.w, f.h = w, h end
function COMMON.SetWidth(f, w) f.w = w end
function COMMON.SetHeight(f, h) f.h = h end
function COMMON.GetWidth(f) return f.w or 0 end
function COMMON.GetHeight(f) return f.h or 0 end
function COMMON.SetPoint(f, point, rel, relPoint, x, y)
  if type(rel) == 'number' then rel, relPoint, x, y = nil, nil, rel, relPoint end
  f.points[#f.points + 1] = {point = point, rel = rel, relPoint = relPoint or point,
    x = x or 0, y = y or 0}
end
function COMMON.ClearAllPoints(f) f.points = {} end
function COMMON.GetNumPoints(f) return #f.points end
function COMMON.GetPoint(f, index)
  local p = f.points[index or 1]
  if not p then return nil end
  return p.point, p.rel or f.parent, p.relPoint, p.x, p.y
end
function COMMON.SetAllPoints(f, rel) f.all_points = rel or true end
function COMMON.SetFrameStrata(f, value) f.strata = value end
function COMMON.SetFrameLevel(f, value) f.level = value end
function COMMON.EnableMouse(f, value) f.mouse = value end
function COMMON.SetMovable(f, value) f.movable = value end
function COMMON.SetClampedToScreen(f, value) f.clamped = value end
function COMMON.RegisterForDrag(f, ...) f.drag = {...} end
function COMMON.RegisterForClicks(f, ...) f.click_kinds = {...} end
function COMMON.SetScript(f, name, handler) f.scripts[name] = handler end
function COMMON.GetScript(f, name) return f.scripts[name] end
function COMMON.RegisterEvent(f, event) f.events[#f.events + 1] = event end
function COMMON.Show(f)
  if not f.shown then f.shown = true; ui.fire(f, 'OnShow') end
end
function COMMON.Hide(f)
  if f.shown then f.shown = false; ui.fire(f, 'OnHide') end
end
function COMMON.IsShown(f) return f.shown end
function COMMON.IsVisible(f) return visible(f) end
function COMMON.GetParent(f) return f.parent end
function COMMON.StartMoving(f) f.moving = true; ui.moves = ui.moves + 1 end
function COMMON.StopMovingOrSizing(f) f.moving = false end
function COMMON.GetCenter(f)
  if f.center then return f.center[1], f.center[2] end
  return nil
end
function COMMON.SetBackdrop(f, spec) f.backdrop = spec end

local function region(parent, kind)
  local r = setmetatable({kind = kind, parent = parent, points = {}, shown = true,
    text = ''}, {__index = REGION})
  parent.regions[#parent.regions + 1] = r
  return r
end
function REGION.SetText(r, value) r.text = value end
function REGION.GetText(r) return r.text end
function REGION.SetPoint(r, ...) COMMON.SetPoint(r, ...) end
function REGION.ClearAllPoints(r) r.points = {} end
function REGION.SetAllPoints(r, rel) r.all_points = rel or true end
function REGION.SetJustifyH(r, value) r.justify_h = value end
function REGION.SetJustifyV(r, value) r.justify_v = value end
function REGION.SetWidth(r, value) r.w = value end
function REGION.SetHeight(r, value) r.h = value end
function REGION.SetSize(r, w, h) r.w, r.h = w, h end
function REGION.GetStringHeight(r)
  local lines = 1
  for _ in tostring(r.text):gmatch('\n') do lines = lines + 1 end
  return lines * 12
end
function REGION.SetTextColor(r, ...) r.color = {...} end
function REGION.SetFontObject(r, value) r.font = value end
function REGION.SetColorTexture(r, ...) r.color = {...} end
function REGION.Show(r) r.shown = true end
function REGION.Hide(r) r.shown = false end
function REGION.IsShown(r) return r.shown end

function COMMON.CreateFontString(f, name, layer, template)
  if template and not ui.fonts[template] then
    error('Couldn\'t find inherited font "' .. tostring(template) .. '"', 2)
  end
  local r = region(f, 'FontString')
  r.name, r.layer, r.template = name, layer, template
  return r
end
function COMMON.CreateTexture(f, name, layer)
  local r = region(f, 'Texture')
  r.name, r.layer = name, layer
  return r
end

function BY_KIND.Button.SetText(f, value) f.text = value end
function BY_KIND.Button.GetText(f) return f.text end
function BY_KIND.Button.Enable(f) f.enabled = true end
function BY_KIND.Button.Disable(f) f.enabled = false end
function BY_KIND.Button.IsEnabled(f) return f.enabled end
for name, method in pairs(BY_KIND.Button) do BY_KIND.CheckButton[name] = method end
function BY_KIND.CheckButton.SetChecked(f, value) f.checked = value and true or false end
function BY_KIND.CheckButton.GetChecked(f) return f.checked == true end

function BY_KIND.EditBox.SetText(f, value)
  value = tostring(value)
  if f.max and #value > f.max then value = value:sub(1, f.max) end
  local changed = value ~= f.text
  f.text = value
  if changed then ui.fire(f, 'OnTextChanged', false) end
end
function BY_KIND.EditBox.GetText(f) return f.text end
function BY_KIND.EditBox.SetMultiLine(f, value) f.multiline = value end
function BY_KIND.EditBox.SetAutoFocus(f, value) f.auto_focus = value end
function BY_KIND.EditBox.SetMaxLetters(f, value) f.max = value end
function BY_KIND.EditBox.SetFontObject(f, value) f.font = value end
function BY_KIND.EditBox.SetFocus(f) f.focus = true end
function BY_KIND.EditBox.ClearFocus(f) f.focus = false end
function BY_KIND.EditBox.HasFocus(f) return f.focus == true end
function BY_KIND.EditBox.HighlightText(f) f.highlight = f.text end
function BY_KIND.EditBox.SetTextInsets(f, ...) f.insets = {...} end
function BY_KIND.EditBox.SetNumeric(f, value) f.numeric = value end

function BY_KIND.ScrollFrame.SetScrollChild(f, child) f.scroll_child = child end
function BY_KIND.ScrollFrame.GetScrollChild(f) return f.scroll_child end

local function lookup(f, key)
  local own = BY_KIND[f.kind][key]
  if own ~= nil then return own end
  if key == 'SetBackdrop' and not f.has_backdrop then return nil end
  return COMMON[key]
end

function CreateFrame(kind, name, parent, template)
  if not BY_KIND[kind] then error('Unknown frame type: ' .. tostring(kind), 2) end
  local used = {}
  if template then
    for item in tostring(template):gmatch('[^, ]+') do
      if not ui.templates[item] or ui.fail[item] then
        error('Couldn\'t find inherited node "' .. item .. '"', 2)
      end
      used[#used + 1] = item
    end
  end
  local f = setmetatable({kind = kind, name = name, parent = parent, children = {},
    regions = {}, scripts = {}, events = {}, points = {}, shown = true, enabled = true,
    text = '', templates = used}, {__index = lookup})
  for _, item in ipairs(used) do
    if item == 'BackdropTemplate' and ui.backdrop_works then f.has_backdrop = true end
  end
  if parent then parent.children[#parent.children + 1] = f end
  ui.frames[#ui.frames + 1] = f
  if name then _G[name] = f end
  return f
end

UIParent = CreateFrame('Frame', nil, nil)
ui.parent = UIParent
Minimap = CreateFrame('Frame', nil, UIParent)
Minimap.center = {500, 400}
UISpecialFrames = {}

GameTooltip = {shown = false, log = {}, lines = {}}
function GameTooltip:SetOwner(owner, anchor)
  self.owner, self.anchor, self.title, self.lines, self.shown = owner, anchor, nil, {}, false
end
function GameTooltip:SetText(text, r, g, b) self.title = text; self.color = {r, g, b} end
function GameTooltip:AddLine(text, r, g, b, wrap)
  self.lines[#self.lines + 1] = {text = text, wrap = wrap}
end
function GameTooltip:Show()
  self.shown = true
  local copy = {}
  for index, line in ipairs(self.lines) do copy[index] = line.text end
  self.log[#self.log + 1] = {owner = self.owner, title = self.title, lines = copy}
end
function GameTooltip:Hide() self.shown = false end
function GameTooltip:IsShown() return self.shown end
ui.tooltip = GameTooltip

function InCombatLockdown()
  local value = ui.combat
  if value == 'error' then error('private combat error') end
  if value == 'nil' then return nil end
  if value == 'number' then return 1 end
  return value
end

Settings = {
  RegisterCanvasLayoutCategory = function(panel, name)
    local category = {panel = panel, name = name}
    ui.categories[#ui.categories + 1] = category
    return category
  end,
  RegisterAddOnCategory = function(category)
    ui.registered[#ui.registered + 1] = category
  end,
}
function InterfaceOptions_AddCategory(panel) ui.legacy[#ui.legacy + 1] = panel end

function GetTime() return ui.mono end
function GetServerTime() return ui.wall end
function UnitLevel() return 12 end
C_EventUtils = {IsEventValid = function(name)
  return ui.quest_event == true and name == 'QUEST_TURNED_IN'
end}

function ui.state() return ui.ns.Diagnostics.state end
function ui.event(event, ...)
  local state = ui.state()
  state.frame.scripts.OnEvent(state.frame, event, ...)
end
function ui.login()
  ui.event('ADDON_LOADED', 'AzerothNavigator')
  ui.event('PLAYER_LOGIN')
end
function ui.click(f)
  assert(f.kind == 'Button' or f.kind == 'CheckButton', 'not clickable: ' .. f.kind)
  if not f.enabled or not visible(f) then return false end
  if f.kind == 'CheckButton' then f.checked = not f.checked end
  ui.fire(f, 'OnClick', 'LeftButton', false)
  return true
end
function ui.enter(f) ui.fire(f, 'OnEnter', true) end
function ui.leave(f) ui.fire(f, 'OnLeave', true) end
function ui.type(f, value)
  if f.max and #value > f.max then value = value:sub(1, f.max) end
  f.text = value
  ui.fire(f, 'OnTextChanged', true)
end
function ui.walk(root, visit)
  visit(root)
  for _, child in ipairs(root.children) do ui.walk(child, visit) end
end
function ui.interactive(root)
  local out = {}
  ui.walk(root, function(f)
    if f.kind == 'Button' or f.kind == 'CheckButton' or f.kind == 'EditBox' then
      out[#out + 1] = f
    end
  end)
  return out
end
-- All FontString texts in a subtree whose frames are visible, in creation order.
function ui.texts(root)
  local out = {}
  ui.walk(root, function(f)
    if visible(f) then
      for _, r in ipairs(f.regions) do
        if r.kind == 'FontString' and r.text ~= '' then out[#out + 1] = r.text end
      end
      if (f.kind == 'Button') and f.text ~= '' then out[#out + 1] = f.text end
    end
  end)
  return out
end
function ui.drag(f, x, y)
  ui.fire(f, 'OnDragStart')
  f.center = {x, y}
  ui.fire(f, 'OnDragStop')
end
function ui.count_templates(name)
  local n = 0
  for _, f in ipairs(ui.frames) do
    for _, item in ipairs(f.templates) do if item == name then n = n + 1 end end
  end
  return n
end
"#;

/// A stub client on the given 1.60.1 build with the widget mock installed. The addon is
/// not loaded yet, so a test can adjust `ui` first.
pub fn ui_client_unloaded(build: &str) -> Sandbox {
    let sandbox = stub_client("1.60.1", build, 16001);
    sandbox.run::<()>("ui-mock", UI_MOCK).expect("ui mock");
    sandbox
}

/// [`ui_client_unloaded`] with the addon loaded and ADDON_LOADED/PLAYER_LOGIN delivered.
pub fn ui_client(build: &str) -> Sandbox {
    let sandbox = ui_client_unloaded(build);
    sandbox.load_toc_files().expect("addon loads");
    sandbox.run::<()>("ui-login", "ui.login()").expect("login");
    sandbox
}
