-- Bounded v1 character exports. Pure: normalized providers and clocks are injected.
local _, ns = ...
local D, V, Json = ns.Decode, ns.Predicate.validation, ns.Json
local Snapshot = {MAX_BYTES=32768}
local SAFE = 9007199254740991
local function check(ok) if not ok then error('snapshot_invalid', 0) end end
local fields = {'level', 'role', 'location', 'preferences', 'equipment', 'quests', 'objectives'}
local scalar = {level=true, role=true, location=true}
local function quote(value)
  local encoded, code = Json.encode(value)
  if not encoded then error(code, 0) end
  return encoded
end
-- CharacterSnapshot v1 has objects/maps, no arrays. Empty maps must encode as {}.
local function document(value, limit)
  local parts, bytes, nodes, active = {}, 0, 0, {}
  local function emit(text)
    bytes = bytes + #text
    if bytes > limit then error('snapshot_bytes', 0) end
    parts[#parts + 1] = text
  end
  local function encode(x, depth)
    nodes = nodes + 1
    if nodes > 8192 then error('snapshot_nodes', 0) end
    if x == D.null then emit('null')
    elseif type(x) == 'string' then emit(quote(x))
    elseif type(x) == 'boolean' then emit(x and 'true' or 'false')
    elseif type(x) == 'number' then
      check(x == x and math.abs(x) <= SAFE)
      local text = x == math.floor(x) and string.format('%.0f', x) or string.format('%.17g', x)
      emit(text)
    elseif type(x) == 'table' then
      if depth >= 32 then error('snapshot_depth', 0) end
      if active[x] then error('snapshot_cycle', 0) end
      local kind = D.kind(x)
      check(kind ~= 'array' and (getmetatable(x) == nil or kind == 'object'))
      active[x] = true
      local keys = {}
      for key in pairs(x) do
        check(type(key) == 'string')
        keys[#keys + 1] = key
        if #keys > 1024 then error('snapshot_entries', 0) end
      end
      table.sort(keys, V.byteLess)
      emit('{')
      for i, key in ipairs(keys) do
        if i > 1 then emit(',') end
        emit(quote(key)); emit(':'); encode(rawget(x, key), depth + 1)
      end
      emit('}'); active[x] = nil
    else error('snapshot_type', 0) end
  end
  encode(value, 0)
  return table.concat(parts)
end
local function sources(value, fixture)
  if type(value) ~= 'table' or value == D.null then return end
  local source = value.source
  if source ~= nil then
    check(fixture and source ~= 'addon_snapshot' and source ~= 'manual_entry' and
      source ~= 'official_api' or not fixture and source ~= 'synthetic_fixture')
  end
  for _, child in pairs(value) do sources(child, fixture) end
end
function Snapshot.encode(snapshot, options)
  local ok, text = pcall(function()
    check(type(options) == 'table' and type(options.fixture) == 'boolean')
    local context = snapshot.context
    local envelope = {format='azeroth-navigator', schema_version=1,
      kind='character_snapshot', fixture=options.fixture, namespace=context.namespace,
      game={edition=context.build.edition, build=context.build.build,
        interface=context.build.interface, locale=options.locale},
      producer={component='addon', version=ns.Version.addon},
      provenance={source=snapshot.capture.source,
        clock_trust=snapshot.capture.clock_trust or 'uncertain',
        created_at_utc_ms=snapshot.capture.observed_at_utc_ms}, payload=snapshot}
    envelope.producer.session_id = snapshot.capture.session_id
    envelope.producer.sequence = snapshot.capture.sequence
    -- Serialize before deep validation: cycles/depth/bytes cannot evade the bounds.
    local result = document(envelope, Snapshot.MAX_BYTES - 6)
    V.snapshot(snapshot)
    local parsed = assert(D.decode(result))
    V.snapshot(parsed.payload); V.id(parsed.game.locale)
    check(parsed.game.edition == 'wow_forever')
    check(parsed.namespace ~= 'demo' or parsed.fixture)
    local expected = parsed.fixture and 'synthetic_fixture' or 'addon_snapshot'
    check(parsed.payload.capture.source == expected)
    sources(parsed.payload, parsed.fixture)
    return 'ANAV1:' .. result
  end)
  if not ok then
    local codes = {snapshot_invalid=true, snapshot_bytes=true, snapshot_nodes=true,
      snapshot_depth=true, snapshot_cycle=true, snapshot_entries=true, snapshot_type=true,
      json_string_too_long=true, json_invalid_utf8=true}
    return nil, codes[text] and text or 'snapshot_invalid'
  end
  return text
end
function Snapshot.capture(spec, providers)
  local ok, snapshot, issues = pcall(function()
    check(type(spec) == 'table' and type(providers) == 'table')
    V.context(spec.context); check(type(spec.fixture) == 'boolean')
    local result = {schema_version=1, id=spec.id, character=spec.character,
      context=spec.context, capture=spec.capture, source_revision=spec.source_revision}
    local failures = {}
    for _, field in ipairs(fields) do
      local capability = spec.capabilities and spec.capabilities[field]
      local matches = capability and V.same(capability.context, spec.context)
      local allowed = matches and
        (capability.status == 'verified_available' and not spec.fixture or
          capability.status == 'synthetic_fixture' and spec.fixture)
      if allowed and type(providers[field]) == 'function' then
        local read, value = pcall(providers[field])
        if read and type(value) == 'table' then result[field] = value
        else failures[field] = 'adapter_failed' end
      end
      if result[field] == nil then
        local status = matches and capability.status
        result[field] = scalar[field] and {status=status == 'restricted' and 'restricted' or
          status == 'verified_unavailable' and 'unsupported' or 'unknown'} or {}
      end
    end
    local encoded, code = Snapshot.encode(result, {fixture=spec.fixture, locale=spec.locale})
    if not encoded then error(code, 0) end
    -- Freeze the result before returning: provider/config aliases cannot change capture.
    return assert(D.decode(encoded:sub(7))).payload, failures
  end)
  if not ok then return nil, 'snapshot_invalid' end
  return snapshot, issues
end
-- Explicit readback of an earlier addon-owned export, never a new client read.
function Snapshot.restoreTransport(text, expected)
  local ok, snapshot = pcall(function()
    check(type(text) == 'string' and #text <= Snapshot.MAX_BYTES and
      text:sub(1, 6) == 'ANAV1:')
    local envelope = assert(D.decode(text:sub(7)))
    check(envelope.format == 'azeroth-navigator' and envelope.schema_version == 1 and
      envelope.kind == 'character_snapshot' and envelope.fixture == expected.fixture)
    V.integer(envelope, 'schema_version', 1, 1)
    local context = {namespace=envelope.namespace, build={edition=envelope.game.edition,
      build=envelope.game.build, interface=envelope.game.interface}}
    V.context(context); check(V.same(context, expected.context))
    check(envelope.producer.component == 'addon' and
      V.same(context, envelope.payload.context))
    V.nonemptyText(envelope.producer.version); V.metadata(envelope.provenance)
    check(envelope.provenance.source == envelope.payload.capture.source and
      envelope.provenance.clock_trust == (envelope.payload.capture.clock_trust or 'uncertain') and
      envelope.provenance.created_at_utc_ms == envelope.payload.capture.observed_at_utc_ms and
      envelope.producer.session_id == envelope.payload.capture.session_id and
      envelope.producer.sequence == envelope.payload.capture.sequence)
    local encoded = assert(Snapshot.encode(envelope.payload,
      {fixture=expected.fixture, locale=envelope.game.locale}))
    return assert(D.decode(encoded:sub(7))).payload
  end)
  if not ok then return nil, 'snapshot_cached_invalid' end
  return snapshot
end
-- This is a one-capture read, not a declaration of availability in all game contexts.
function Snapshot.readLevel(getter, metadata)
  if type(getter) ~= 'function' then return {status='unsupported'}, 'level_api_missing' end
  local ok, value = pcall(function()
    local level = getter()
    check(type(level) == 'number' and level == math.floor(level) and level >= 1 and
      level <= 65535)
    return level
  end)
  if not ok then return {status='unknown'}, 'level_read_failed' end
  local observation = {status='known', value=value}
  for key, item in pairs(metadata) do observation[key] = item end
  return observation
end
-- Explicit log reads only: no history inference or objective-index/stage mapping.
-- Any incomplete enumeration rejects the entire read. Diagnostics contain counts,
-- never quest titles or raw client errors.
function Snapshot.readQuests(getCount, getInfo, context, metadata)
  if type(getCount) ~= 'function' or type(getInfo) ~= 'function' then
    return {}, {quest_api_missing=1}
  end
  local failure = 'quest_read_failed'
  local ok, quests, diagnostics = pcall(function()
    V.context(context)
    local count = getCount()
    check(type(count) == 'number' and count == math.floor(count) and count >= 0)
    if count > 1024 then failure = 'quest_entries'; error(failure, 0) end
    local result, reasons = {}, {objective_mapping_unavailable=1, history_unobserved=1}
    local function reason(code) reasons[code] = (reasons[code] or 0) + 1 end
    local function observed(value)
      local observation = {status='known', value=value}
      for key, item in pairs(metadata) do observation[key] = item end
      return observation
    end
    for index=1,count do
      local info = getInfo(index)
      check(type(info) == 'table' and type(info.isHeader) == 'boolean')
      if info.isHeader then reason('quest_header_excluded')
      else
        local id = info.questID
        if type(id) ~= 'number' or id ~= math.floor(id) or id < 1 or id > 4294967295 then
          reason('quest_id_invalid')
        else
          id = string.format('%.0f', id)
          if result[id] then failure = 'quest_duplicate'; error(failure, 0) end
          local completed = {status='unknown'}
          if type(info.isComplete) == 'boolean' then completed = observed(info.isComplete)
          else reason('quest_completion_unreadable') end
          result[id] = {quest=id, context=context, accepted=observed(true),
            completed=completed, turned_in={status='unknown'}, objectives={}}
        end
      end
    end
    if getCount() ~= count then failure = 'quest_log_changed'; error(failure, 0) end
    return result, reasons
  end)
  if not ok then return nil, failure end
  return quests, diagnostics
end
function Snapshot.store(db, snapshot, options)
  if type(db) ~= 'table' or getmetatable(db) ~= nil or db.db_version ~= ns.Version.db_version then
    return nil, 'snapshot_db_version'
  end
  local text, code = Snapshot.encode(snapshot, options)
  if not text then db.snapshot_export_error = code; return nil, code end
  db.snapshot_export = text
  db.snapshot_export_error = nil
  return text
end
ns.provide('Snapshot', Snapshot)
