-- Bounded JSON data decoder. Imported text is never executable code.
local _, ns = ...
local Decode = {}
Decode.null = {}
local SAFE = 9007199254740991
local function utf8(cp)
  if cp < 128 then return string.char(cp) end
  if cp < 2048 then return string.char(192 + math.floor(cp / 64), 128 + cp % 64) end
  if cp < 65536 then
    return string.char(224 + math.floor(cp / 4096), 128 + math.floor(cp / 64) % 64, 128 + cp % 64)
  end
  return string.char(240 + math.floor(cp / 262144), 128 + math.floor(cp / 4096) % 64,
    128 + math.floor(cp / 64) % 64, 128 + cp % 64)
end
function Decode.codepoints(text)
  local out, i = {}, 1
  while i <= #text do
    local c, count, cp = text:byte(i), 1, text:byte(i)
    if c >= 194 and c <= 223 then count, cp = 2, c - 192
    elseif c >= 224 and c <= 239 then count, cp = 3, c - 224
    elseif c >= 240 and c <= 244 then count, cp = 4, c - 240
    elseif c >= 128 then return nil end
    for j = 1, count - 1 do
      local b = text:byte(i + j)
      if not b or b < 128 or b > 191 then return nil end
      cp = cp * 64 + b - 128
    end
    if (count == 2 and cp < 128) or (count == 3 and cp < 2048) or
      (count == 4 and cp < 65536) or (cp >= 55296 and cp <= 57343) or cp > 1114111 then
      return nil
    end
    out[#out + 1], i = cp, i + count
  end
  return out
end
function Decode.integer(owner, key, min, max)
  local n, mt = owner[key], getmetatable(owner)
  return type(n) == 'number' and n == math.floor(n) and n >= (min or -SAFE) and n <= (max or SAFE)
    and not (mt and mt.floats and mt.floats[key])
end
function Decode.bytes(owner, key)
  local mt = getmetatable(owner)
  return mt and mt.bytes and mt.bytes[key]
end
function Decode.kind(value)
  local mt = type(value) == 'table' and getmetatable(value)
  return mt and mt.json_kind
end
-- Overrides are for repository fixture runners. Production plan import uses defaults.
function Decode.decode(text, options)
  options = options or {}
  local limits = {bytes = options.bytes or 32768, depth = options.depth or 32,
    nodes = options.nodes or 8192, entries = options.entries or 1024,
    string = options.string or 4096}
  if type(text) ~= 'string' or #text > limits.bytes then return nil, 'json_bytes' end
  local i, nodes = 1, 0
  local function fail(code) error(code, 0) end
  local function space() while text:sub(i, i):match('[ \t\r\n]') do i = i + 1 end end
  local function hex()
    local h = text:sub(i, i + 3)
    if #h ~= 4 or not h:match('^%x%x%x%x$') then fail('json_escape') end
    i = i + 4
    return tonumber(h, 16)
  end
  local function quoted()
    if text:sub(i, i) ~= '"' then fail('json_string') end
    i = i + 1
    local parts, bytes = {}, 0
    while true do
      local c = text:sub(i, i)
      if c == '' then fail('json_string') end
      if c == '"' then i = i + 1; break end
      if c == '\\' then
        i = i + 1
        local e = text:sub(i, i); i = i + 1
        local escapes = {['"']='"', ['\\']='\\', ['/']='/', b='\b', f='\f', n='\n', r='\r', t='\t'}
        c = escapes[e]
        if e == 'u' then
          local cp = hex()
          if cp >= 55296 and cp <= 56319 then
            if text:sub(i, i + 1) ~= '\\u' then fail('json_surrogate') end
            i = i + 2
            local low = hex()
            if low < 56320 or low > 57343 then fail('json_surrogate') end
            cp = 65536 + (cp - 55296) * 1024 + low - 56320
          elseif cp >= 56320 and cp <= 57343 then fail('json_surrogate') end
          c = utf8(cp)
        elseif not c then fail('json_escape') end
      else
        if c:byte() < 32 then fail('json_control') end
        i = i + 1
      end
      bytes = bytes + #c
      if bytes > limits.string then fail('json_string_bytes') end
      parts[#parts + 1] = c
    end
    local result = table.concat(parts)
    if not Decode.codepoints(result) then fail('json_utf8') end
    return result
  end
  local value
  value = function(depth)
    nodes = nodes + 1
    if nodes > limits.nodes then fail('json_nodes') end
    space()
    local c = text:sub(i, i)
    if c == '{' or c == '[' then
      if depth > limits.depth then fail('json_depth') end
      local object, close = c == '{', c == '{' and '}' or ']'
      local mt = {json_kind = object and 'object' or 'array', floats = {}, bytes = {}}
      local out, seen, count = setmetatable({}, mt), {}, 0
      i = i + 1; space()
      if text:sub(i, i) == close then i = i + 1; return out end
      while true do
        count = count + 1
        if count > limits.entries then fail('json_entries') end
        local key = count
        if object then
          key = quoted(); space()
          if seen[key] then fail('json_duplicate_key') end
          seen[key] = true
          if text:sub(i, i) ~= ':' then fail('json_colon') end
          i = i + 1
        end
        space()
        local start = i
        local item, floating = value(depth + 1)
        out[key], mt.floats[key], mt.bytes[key] = item, floating, i - start
        space(); c = text:sub(i, i); i = i + 1
        if c == close then return out end
        if c ~= ',' then fail('json_separator') end
        space()
      end
    elseif c == '"' then return quoted()
    elseif text:sub(i, i + 3) == 'true' then i = i + 4; return true
    elseif text:sub(i, i + 4) == 'false' then i = i + 5; return false
    elseif text:sub(i, i + 3) == 'null' then i = i + 4; return Decode.null
    else
      local start = i
      if c == '-' then i = i + 1 end
      c = text:sub(i, i)
      if c == '0' then i = i + 1
      elseif c:match('[1-9]') then repeat i = i + 1 until not text:sub(i, i):match('%d')
      else fail('json_number') end
      local floating = false
      if text:sub(i, i) == '.' then
        floating = true; i = i + 1
        if not text:sub(i, i):match('%d') then fail('json_number') end
        repeat i = i + 1 until not text:sub(i, i):match('%d')
      end
      if text:sub(i, i):match('[eE]') then
        floating = true; i = i + 1
        if text:sub(i, i):match('[+-]') then i = i + 1 end
        if not text:sub(i, i):match('%d') then fail('json_number') end
        repeat i = i + 1 until not text:sub(i, i):match('%d')
      end
      local n = tonumber(text:sub(start, i - 1))
      if not n or n ~= n or n == math.huge or n == -math.huge then fail('json_number_range') end
      if not floating and math.abs(n) > SAFE then fail('json_integer_range') end
      return n, floating
    end
  end
  local ok, result = pcall(function()
    local parsed = value(1); space()
    if i <= #text then fail('json_trailing') end
    return parsed
  end)
  if ok then return result end
  return nil, result
end
ns.provide("Decode", Decode)
