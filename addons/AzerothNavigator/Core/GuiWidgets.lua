-- Guarded widget factory for the in-game window. The client environment is passed in,
-- never read from a global. Every frame template is requested under pcall and falls
-- back to a plain frame with a color texture (as the copy view does), because which
-- templates Forever ships is UNVERIFIED. Styling names client fonts only and ships no
-- art. Tooltips use GameTooltip when it exists, otherwise an inline help line.
local _, ns = ...

local Widgets = {}
Widgets.__index = Widgets

local Widget = {}
Widget.__index = Widget

local function has(object, name)
  if not object then return false end
  local ok, method = pcall(function() return object[name] end)
  return ok and type(method) == 'function'
end

-- Calls object:name(...) when the method exists; returns pcall's results.
local function call(object, name, ...)
  if not has(object, name) then return false end
  return pcall(object[name], object, ...)
end

local FONT = 'GameFontNormal'
local FONT_LIGHT = 'GameFontHighlight'

function Widgets.new(env, text)
  return setmetatable({env = env, text = text}, Widgets)
end

-- Widget methods. A widget is {id, kind, frame, text, label, enabled, checked}.
function Widget.set(self, value)
  self.text = value
  if self.fs then call(self.fs, 'SetText', value) end
end

function Widget.setLabel(self, label)
  self.label = label
  if self.fs then
    call(self.fs, 'SetText', label)
  else
    call(self.frame, 'SetText', label)
  end
end

function Widget.setEnabled(self, enabled)
  self.enabled = enabled == true
  call(self.frame, self.enabled and 'Enable' or 'Disable')
end

function Widget.setShown(self, shown)
  call(self.frame, shown and 'Show' or 'Hide')
end

function Widget.setChecked(self, checked)
  self.checked = checked == true
  if self.templated then call(self.frame, 'SetChecked', self.checked) end
  if self.refreshLabel then self:refreshLabel() end
end

function Widget.getText(self)
  if not has(self.frame, 'GetText') then return self.text or '' end
  local ok, value = pcall(self.frame.GetText, self.frame)
  if ok and type(value) == 'string' then return value end
  return self.text or ''
end

function Widget.setText(self, value)
  self.text = value
  call(self.frame, 'SetText', value)
end

-- Frame creation: a template failure is not an error, a missing CreateFrame is.
function Widgets:create(kind, parent, template, name)
  local create = self.env.CreateFrame
  if type(create) ~= 'function' then error('gui_ui_unavailable', 0) end
  if template then
    local ok, frame = pcall(create, kind, name, parent, template)
    if ok and frame ~= nil then return frame, true end
  end
  return create(kind, name, parent), false
end

function Widgets.background(_, frame, r, g, b, a)
  local ok, texture = pcall(function() return frame:CreateTexture(nil, 'BACKGROUND') end)
  if not ok or not texture then return nil end
  call(texture, 'SetAllPoints', frame)
  call(texture, 'SetColorTexture', r, g, b, a)
  return texture
end

local BACKDROP = {
  bgFile = 'Interface\\DialogFrame\\UI-DialogBox-Background',
  edgeFile = 'Interface\\DialogFrame\\UI-DialogBox-Border',
  tile = true, tileSize = 32, edgeSize = 24,
  insets = {left = 6, right = 6, top = 6, bottom = 6},
}

-- A framed panel: the dialog backdrop when BackdropTemplate works, else a color texture.
function Widgets:panel(parent, width, height, name)
  local frame, templated = self:create('Frame', parent, 'BackdropTemplate', name)
  call(frame, 'SetSize', width, height)
  local backdrop = templated and has(frame, 'SetBackdrop') and
    call(frame, 'SetBackdrop', BACKDROP)
  if not backdrop then self:background(frame, 0.06, 0.07, 0.12, 0.96) end
  return {id = name, kind = 'panel', frame = frame}
end

-- A text line. Returns a widget whose set() redraws; a missing FontString degrades to
-- a widget that only remembers its text, so state is never lost.
function Widgets.fontString(_, parent, width, template)
  local widget = setmetatable({kind = 'text', text = ''}, Widget)
  local ok, fs = pcall(function()
    return parent:CreateFontString(nil, 'OVERLAY', template or FONT_LIGHT)
  end)
  if not ok or not fs then
    ok, fs = pcall(function() return parent:CreateFontString(nil, 'OVERLAY') end)
  end
  if ok and fs then
    widget.fs = fs
    call(fs, 'SetJustifyH', 'LEFT')
    call(fs, 'SetJustifyV', 'TOP')
    if width then call(fs, 'SetWidth', width) end
  end
  return widget
end

function Widgets.place(_, widget, parent, x, y)
  local frame = widget.frame or widget.fs
  call(frame, 'ClearAllPoints')
  call(frame, 'SetPoint', 'TOPLEFT', parent, 'TOPLEFT', x, -y)
end

function Widgets:label(parent, x, y, width, template)
  local widget = self:fontString(parent, width, template)
  self:place(widget, parent, x, y)
  return widget
end

local function showInline(factory, message)
  if factory.inline_help then factory.inline_help(message) end
end

-- Tooltip: title, body, slash equivalent and gate from the text table, by widget id.
function Widgets:tip(widget, id)
  local title, lines = self.text.tooltip(id)
  if not title then return false end
  widget.tooltip_id = id
  local env = self.env
  local entered = call(widget.frame, 'SetScript', 'OnEnter', function(owner)
    local tooltip = env.GameTooltip
    if has(tooltip, 'SetOwner') and has(tooltip, 'SetText') and has(tooltip, 'AddLine') and
      has(tooltip, 'Show') then
      local ok = pcall(function()
        tooltip:SetOwner(owner, 'ANCHOR_RIGHT')
        tooltip:SetText(title, 1, 1, 1)
        for _, line in ipairs(lines) do tooltip:AddLine(line, nil, nil, nil, true) end
        tooltip:Show()
      end)
      if ok then return end
    end
    showInline(self, title .. ': ' .. lines[1] .. ' ' .. lines[2])
  end)
  call(widget.frame, 'SetScript', 'OnLeave', function()
    local tooltip = env.GameTooltip
    if has(tooltip, 'Hide') then pcall(tooltip.Hide, tooltip) end
    showInline(self, '')
  end)
  return entered
end

function Widgets:button(parent, id, width, height, onClick)
  local frame, templated = self:create('Button', parent, 'UIPanelButtonTemplate')
  call(frame, 'SetSize', width, height)
  local widget = setmetatable({id = id, kind = 'button', frame = frame, enabled = true,
    label = '', templated = templated}, Widget)
  if not templated then
    self:background(frame, 0.2, 0.2, 0.32, 0.95)
    widget.fs = self:fontString(frame, width - 8, FONT)
    call(widget.fs.fs, 'SetPoint', 'CENTER', frame, 'CENTER', 0, 0)
    call(widget.fs.fs, 'SetJustifyH', 'CENTER')
    widget.fs = widget.fs.fs
  end
  call(frame, 'SetScript', 'OnClick', function()
    if widget.enabled and onClick then onClick(widget) end
  end)
  self:tip(widget, id)
  return widget
end

-- A checkbox with its own text line so the state is always readable as ON or OFF.
function Widgets:checkbox(parent, id, onToggle)
  local frame, templated = self:create('CheckButton', parent, 'UICheckButtonTemplate')
  call(frame, 'SetSize', 26, 26)
  local widget = setmetatable({id = id, kind = 'checkbox', frame = frame, enabled = true,
    checked = false, templated = templated, caption = ''}, Widget)
  if not templated then self:background(frame, 0.2, 0.2, 0.32, 0.95) end
  widget.line = self:fontString(parent, 520, FONT)
  function widget.refreshLabel(box)
    local mark = box.checked and 'ON' or 'OFF'
    box.line:set(box.caption .. ': ' .. mark)
  end
  call(frame, 'SetScript', 'OnClick', function()
    if not widget.enabled then return end
    local checked = not widget.checked
    if templated and has(frame, 'GetChecked') then
      local ok, value = pcall(frame.GetChecked, frame)
      if ok and type(value) == 'boolean' then checked = value end
    end
    widget:setChecked(checked)
    if onToggle then onToggle(widget, checked) end
  end)
  self:tip(widget, id)
  return widget
end

function Widget.setCaption(self, caption)
  self.caption = caption
  self:refreshLabel()
end

-- A single-line field. onChange(widget, text) fires for every edit.
function Widgets:editbox(parent, id, width, height, options)
  options = options or {}
  local frame, templated = self:create('EditBox', parent, 'InputBoxTemplate')
  call(frame, 'SetSize', width, height)
  local widget = setmetatable({id = id, kind = 'editbox', frame = frame, enabled = true,
    templated = templated, text = ''}, Widget)
  if not templated then self:background(frame, 0, 0, 0, 0.7) end
  call(frame, 'SetAutoFocus', false)
  call(frame, 'SetFontObject', 'ChatFontNormal')
  if options.multiline then call(frame, 'SetMultiLine', true) end
  if options.max then call(frame, 'SetMaxLetters', options.max) end
  call(frame, 'SetScript', 'OnEscapePressed', function(box) call(box, 'ClearFocus') end)
  if not options.multiline then
    call(frame, 'SetScript', 'OnEnterPressed', function(box) call(box, 'ClearFocus') end)
  end
  call(frame, 'SetScript', 'OnTextChanged', function()
    widget.text = widget:getText()
    if options.onChange then options.onChange(widget, widget.text) end
  end)
  self:tip(widget, id)
  return widget
end

-- A multi-line field inside a scroll frame, for pasted text up to options.max letters.
function Widgets:scrollEdit(parent, id, width, height, options)
  local scroll = self:create('ScrollFrame', parent, 'UIPanelScrollFrameTemplate')
  call(scroll, 'SetSize', width, height)
  options = options or {}
  options.multiline = true
  local widget = self:editbox(scroll, id, width - 28, height, options)
  call(scroll, 'SetScrollChild', widget.frame)
  widget.scroll = scroll
  return widget
end

-- A numeric field kept as text so an invalid entry stays visible. value() is the whole
-- number when valid, nil otherwise; nothing is clamped silently.
function Widgets:numberField(parent, id, width, low, high, onChange)
  local widget = self:editbox(parent, id, width, 24, {max = 5, onChange = onChange})
  widget.low, widget.high = low, high
  function widget.value(field)
    local text = field:getText()
    if not text:match('^%d+$') then return nil end
    local number = tonumber(text)
    if number < field.low or number > field.high then return nil end
    return number
  end
  return widget
end

-- A button that steps through values; label shows the current one in words.
function Widgets:cycleButton(parent, id, width, values, onChange)
  local widget
  widget = self:button(parent, id, width, 26, function()
    widget.index = widget.index % #values + 1
    widget:show()
    if onChange then onChange(widget, values[widget.index]) end
  end)
  widget.values, widget.index, widget.caption = values, 1, ''
  function widget.show(cycle)
    cycle:setLabel(cycle.caption .. ': ' .. tostring(values[cycle.index]) .. '%')
  end
  function widget.select(cycle, value)
    for index, candidate in ipairs(values) do
      if candidate == value then cycle.index = index end
    end
    cycle:show()
  end
  return widget
end

-- Read-only wrapped text inside a scroll frame (reason inspector, long lists).
function Widgets:scrollText(parent, id, width, height)
  local widget = setmetatable({id = id, kind = 'scrolltext', text = ''}, Widget)
  local scroll = self:create('ScrollFrame', parent, 'UIPanelScrollFrameTemplate')
  call(scroll, 'SetSize', width, height)
  local content = self:create('Frame', scroll)
  call(content, 'SetSize', width - 28, height)
  call(scroll, 'SetScrollChild', content)
  local line = self:fontString(content, width - 28, FONT_LIGHT)
  call(line.fs, 'SetPoint', 'TOPLEFT', content, 'TOPLEFT', 0, 0)
  widget.frame, widget.fs, widget.content, widget.height = scroll, line.fs, content, height
  function widget.set(area, value)
    area.text = value
    call(area.fs, 'SetText', value)
    local ok, tall = call(area.fs, 'GetStringHeight')
    if ok and type(tall) == 'number' then
      call(area.content, 'SetHeight', math.max(area.height, tall + 4))
    end
  end
  return widget
end

-- Tabs as buttons; the active tab is marked with brackets, never by color alone.
function Widgets:tabBar(parent, tabs, onSelect)
  local bar = {buttons = {}, active = nil}
  for index, tab in ipairs(tabs) do
    local button = self:button(parent, 'tab_' .. tab.id, 92, 24, function()
      onSelect(tab.id)
    end)
    button:setLabel(tab.label)
    call(button.frame, 'SetPoint', 'TOPLEFT', parent, 'TOPLEFT', 16 + (index - 1) * 94, -40)
    bar.buttons[tab.id] = button
  end
  function bar.setActive(id)
    bar.active = id
    for _, tab in ipairs(tabs) do
      bar.buttons[tab.id]:setLabel(tab.id == id and ('[' .. tab.label .. ']') or tab.label)
    end
  end
  return bar
end

local RIM = 80

-- The minimap button: unnamed, left-click toggles, dragging moves it around the rim and
-- stores the angle. Without a Minimap object nothing is created.
function Widgets:minimapButton(options)
  local minimap = self.env.Minimap
  if not minimap then return nil, 'gui_minimap_unavailable' end
  local frame = self:create('Button', minimap)
  call(frame, 'SetSize', 31, 31)
  call(frame, 'SetFrameStrata', 'MEDIUM')
  call(frame, 'SetFrameLevel', 8)
  self:background(frame, 0.79, 0.64, 0.16, 1)
  local widget = setmetatable({id = 'minimap', kind = 'button', frame = frame,
    enabled = true, dragging = false}, Widget)
  widget.fs = self:fontString(frame, 31, FONT)
  call(widget.fs.fs, 'SetPoint', 'CENTER', frame, 'CENTER', 0, 0)
  widget.fs:set('AN')
  widget.fs = widget.fs.fs
  function widget.place(button, angle)
    local radians = math.rad(angle)
    call(button.frame, 'ClearAllPoints')
    call(button.frame, 'SetPoint', 'CENTER', minimap, 'CENTER',
      math.floor(math.cos(radians) * RIM + 0.5), math.floor(math.sin(radians) * RIM + 0.5))
  end
  call(frame, 'SetMovable', true)
  call(frame, 'RegisterForDrag', 'LeftButton')
  call(frame, 'SetScript', 'OnClick', function()
    if not widget.dragging and options.onClick then options.onClick() end
  end)
  call(frame, 'SetScript', 'OnDragStart', function()
    if options.canDrag and not options.canDrag() then return end
    widget.dragging = true
    call(frame, 'StartMoving')
  end)
  call(frame, 'SetScript', 'OnDragStop', function()
    if not widget.dragging then return end
    call(frame, 'StopMovingOrSizing')
    widget.dragging = false
    local ok, bx, by = call(frame, 'GetCenter')
    local mok, mx, my = call(minimap, 'GetCenter')
    if ok and mok and type(bx) == 'number' and type(by) == 'number' and
      type(mx) == 'number' and type(my) == 'number' then
      local angle = math.floor(math.deg(math.atan2(by - my, bx - mx)) + 0.5) % 360
      if options.onAngle then options.onAngle(angle) end
      widget:place(angle)
    elseif options.angle then
      widget:place(options.angle())
    end
  end)
  self:tip(widget, 'minimap')
  widget:place(options.angle and options.angle() or 225)
  return widget
end

-- A page under the game's AddOns options with a button that opens the window.
function Widgets:optionsPanel(description, onOpen)
  local api = self.env.Settings
  local canvas = has(api, 'RegisterCanvasLayoutCategory') and
    has(api, 'RegisterAddOnCategory')
  local legacy = type(self.env.InterfaceOptions_AddCategory) == 'function'
  if not canvas and not legacy then return nil, 'gui_options_unavailable' end
  local frame = self:create('Frame', self.env.UIParent)
  frame.name = self.text.title
  local intro = self:label(frame, 16, 16, 560, FONT_LIGHT)
  intro:set(description)
  local open = self:button(frame, 'options_open', 200, 26, function(widget)
    if onOpen then onOpen(widget) end
  end)
  open:setLabel(self.text.widgets.options_open.label)
  self:place(open, frame, 16, 110)
  if canvas then
    local category = api.RegisterCanvasLayoutCategory(frame, frame.name)
    api.RegisterAddOnCategory(category)
  else
    self.env.InterfaceOptions_AddCategory(frame)
  end
  return {frame = frame, open = open}
end

ns.provide('GuiWidgets', Widgets)
