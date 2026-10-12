-- Bounded JSON encoder. Pure: no client globals, no I/O, no dynamic code loading.
-- On any failure it returns nil plus a short error code and never a partial string.
-- Output is deterministic: object keys are sorted, numbers are integers only.
-- An empty table encodes as [] (the diagnostic export never needs an empty object).
local _, ns = ...

local MAX_DEPTH = 32 -- matches plan section 7.1 and the Rust MAX_ADDON_JSON_DEPTH
local MAX_NODES = 2048 -- values visited, containers included
local MAX_ELEMENTS = 256 -- entries in one array, keys in one object (objects stop at 64)
local MAX_KEYS = 64
local MAX_STRING_BYTES = 512
local MAX_OUTPUT_BYTES = 16384 -- well under the 32 KiB addon text bound of section 7.1
local MAX_SAFE_INTEGER = 9007199254740991

local Json = {
  limits = {
    max_depth = MAX_DEPTH,
    max_nodes = MAX_NODES,
    max_elements = MAX_ELEMENTS,
    max_keys = MAX_KEYS,
    max_string_bytes = MAX_STRING_BYTES,
    max_output_bytes = MAX_OUTPUT_BYTES,
  },
}

local ESCAPES = {
  ['"'] = '\\"',
  ["\\"] = "\\\\",
  ["\b"] = "\\b",
  ["\f"] = "\\f",
  ["\n"] = "\\n",
  ["\r"] = "\\r",
  ["\t"] = "\\t",
}

local function escapeChar(char)
  return ESCAPES[char] or string.format("\\u%04x", char:byte())
end

local function isContinuation(byte)
  return byte ~= nil and byte >= 0x80 and byte <= 0xBF
end

-- Strict UTF-8 validation: no overlong forms, surrogates or values above U+10FFFF.
local function isValidUtf8(text)
  local i, n = 1, #text
  while i <= n do
    local c = text:byte(i)
    if c < 0x80 then
      i = i + 1
    elseif c >= 0xC2 and c <= 0xDF then
      if not isContinuation(text:byte(i + 1)) then
        return false
      end
      i = i + 2
    elseif c >= 0xE0 and c <= 0xEF then
      local c1, c2 = text:byte(i + 1, i + 2)
      if not (isContinuation(c1) and isContinuation(c2)) then
        return false
      end
      if (c == 0xE0 and c1 < 0xA0) or (c == 0xED and c1 > 0x9F) then
        return false
      end
      i = i + 3
    elseif c >= 0xF0 and c <= 0xF4 then
      local c1, c2, c3 = text:byte(i + 1, i + 3)
      if not (isContinuation(c1) and isContinuation(c2) and isContinuation(c3)) then
        return false
      end
      if (c == 0xF0 and c1 < 0x90) or (c == 0xF4 and c1 > 0x8F) then
        return false
      end
      i = i + 4
    else
      return false
    end
  end
  return true
end

-- Returns the quoted JSON string, or nil plus an error code.
local function quote(text)
  if #text > MAX_STRING_BYTES then
    return nil, "json_string_too_long"
  end
  if not isValidUtf8(text) then
    return nil, "json_invalid_utf8"
  end
  return '"' .. (text:gsub('[%c"\\]', escapeChar)) .. '"'
end

local function formatNumber(value)
  if value ~= value then
    return nil, "json_number_nan"
  end
  if value == math.huge or value == -math.huge then
    return nil, "json_number_infinite"
  end
  if value ~= math.floor(value) then
    return nil, "json_number_not_integer"
  end
  if value > MAX_SAFE_INTEGER or value < -MAX_SAFE_INTEGER then
    return nil, "json_number_out_of_range"
  end
  if value == 0 then
    return "0"
  end
  return string.format("%.0f", value)
end

-- Appends a piece to the output; returns an error code when the output cap is hit.
local function emit(state, piece)
  state.bytes = state.bytes + #piece
  if state.bytes > state.limit then
    return "json_output_too_large"
  end
  state.parts[#state.parts + 1] = piece
  return nil
end

local encodeValue

-- Classifies a table as an array of n elements, or an object with sorted string keys.
-- Returns kind ("array" or "object"), a count or key list, and an optional error code.
local function classify(tbl)
  local count, maxIndex, hasString = 0, 0, false
  for key in pairs(tbl) do
    count = count + 1
    if count > MAX_ELEMENTS then
      return nil, nil, "json_too_many_elements"
    end
    local keyType = type(key)
    if keyType == "string" then
      hasString = true
    elseif keyType == "number" and key >= 1 and key == math.floor(key) then
      if key > maxIndex then
        maxIndex = key
      end
    else
      return nil, nil, "json_key"
    end
  end
  if count == 0 then
    return "array", 0
  end
  if maxIndex > 0 then
    if hasString or maxIndex ~= count then
      return nil, nil, "json_key"
    end
    return "array", count
  end
  if count > MAX_KEYS then
    return nil, nil, "json_too_many_elements"
  end
  local keys = {}
  for key in pairs(tbl) do
    keys[#keys + 1] = key
  end
  table.sort(keys)
  return "object", keys
end

local function encodeArray(state, tbl, count, depth)
  local err = emit(state, "[")
  for index = 1, count do
    if err then
      return err
    end
    if index > 1 then
      err = emit(state, ",")
    end
    err = err or encodeValue(state, tbl[index], depth)
  end
  return err or emit(state, "]")
end

local function encodeObject(state, tbl, keys, depth)
  local err = emit(state, "{")
  for index, key in ipairs(keys) do
    if err then
      return err
    end
    local quotedKey, keyErr = quote(key)
    if not quotedKey then
      return keyErr
    end
    err = emit(state, (index > 1 and "," or "") .. quotedKey .. ":")
    err = err or encodeValue(state, tbl[key], depth)
  end
  return err or emit(state, "}")
end

local function encodeTable(state, tbl, depth)
  if depth > MAX_DEPTH then
    return "json_depth"
  end
  if getmetatable(tbl) ~= nil then
    return "json_type"
  end
  if state.active[tbl] then
    return "json_cycle"
  end
  local kind, detail, classifyErr = classify(tbl)
  if classifyErr then
    return classifyErr
  end
  state.active[tbl] = true
  local err
  if kind == "array" then
    err = encodeArray(state, tbl, detail, depth)
  else
    err = encodeObject(state, tbl, detail, depth)
  end
  state.active[tbl] = nil
  return err
end

function encodeValue(state, value, depth)
  state.nodes = state.nodes + 1
  if state.nodes > MAX_NODES then
    return "json_too_many_elements"
  end
  local valueType = type(value)
  if valueType == "string" then
    local quoted, err = quote(value)
    return err or emit(state, quoted)
  elseif valueType == "number" then
    local text, err = formatNumber(value)
    return err or emit(state, text)
  elseif valueType == "boolean" then
    return emit(state, value and "true" or "false")
  elseif valueType == "table" then
    return encodeTable(state, value, depth + 1)
  end
  -- nil, functions, userdata and threads are never serialized.
  return "json_type"
end

-- Encodes value. maxBytes may only lower the built-in output cap.
-- Returns the JSON string, or nil plus an error code.
function Json.encode(value, maxBytes)
  local limit = MAX_OUTPUT_BYTES
  if type(maxBytes) == "number" and maxBytes >= 0 and maxBytes < limit then
    limit = maxBytes
  end
  local state = { nodes = 0, bytes = 0, limit = limit, parts = {}, active = {} }
  local ok, err = pcall(encodeValue, state, value, 0)
  if not ok then
    return nil, "json_internal"
  end
  if err then
    return nil, err
  end
  return table.concat(state.parts)
end

ns.provide("Json", Json)
