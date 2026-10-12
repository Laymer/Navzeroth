-- Pure Rust-compatible, three-valued declarative evaluator.
local _, ns = ...
local D, P, V = ns.Decode, {}, {}
local SAFE = 9007199254740991
local function check(ok) if not ok then error('INVALID_INPUT', 0) end end
local function object(x) check(type(x) == 'table' and x ~= D.null and D.kind(x) ~= 'array') end
local function array(x)
  check(type(x) == 'table' and x ~= D.null and D.kind(x) ~= 'object')
  local n = 0
  for k in pairs(x) do check(type(k) == 'number' and k == math.floor(k) and k >= 1); n = n + 1 end
  check(n == #x and n <= 1024)
end
local function optional(x) return x ~= nil and x ~= D.null end
local function one(x, choices) check(choices[x] == true) end
local function finite(x) check(type(x) == 'number' and x == x and math.abs(x) <= SAFE) end
local function whitespace(cp)
  return cp == 32 or (cp >= 9 and cp <= 13) or cp == 133 or cp == 160 or cp == 5760 or
    (cp >= 8192 and cp <= 8202) or cp == 8232 or cp == 8233 or cp == 8239 or
    cp == 8287 or cp == 12288
end
function V.id(x)
  check(type(x) == 'string' and #x > 0 and #x <= 256)
  local cps = D.codepoints(x); check(cps ~= nil)
  check(not whitespace(cps[1]) and not whitespace(cps[#cps]))
  for _, cp in ipairs(cps) do check(cp >= 32 and not (cp >= 127 and cp <= 159)) end
end
function V.nonemptyText(x)
  V.text(x)
  local cps, content = D.codepoints(x), false
  for _, cp in ipairs(cps) do if not whitespace(cp) then content = true end end
  check(content)
end
function V.text(x)
  check(type(x) == 'string' and #x <= 4096)
  local cps = D.codepoints(x); check(cps ~= nil)
  for _, cp in ipairs(cps) do
    check((cp >= 32 or cp == 9 or cp == 10) and not (cp >= 127 and cp <= 159))
  end
end
function V.integer(x, k, min, max) check(D.integer(x, k, min, max)) end
function V.context(x)
  object(x); object(x.build)
  for _, k in ipairs({'edition', 'build', 'interface'}) do V.id(x.build[k]) end
  one(x.namespace, {demo=true, reference=true})
end
function V.same(a, b)
  return a.namespace == b.namespace and a.build.edition == b.build.edition and
    a.build.build == b.build.build and a.build.interface == b.build.interface
end
local function compatible(a, b) if not V.same(a, b) then error('BUILD_MISMATCH', 0) end end
function V.map(x)
  object(x); V.context(x.context); V.id(x.map_id); V.id(x.floor)
  if optional(x.instance) then V.id(x.instance) end
end
function V.position(x)
  object(x); V.map(x.map)
  for _, k in ipairs({'x', 'y'}) do finite(x[k]); check(x[k] >= 0 and x[k] <= 1) end
end
function V.metadata(x)
  object(x)
  if x.source ~= nil then one(x.source, {unrecorded=true, addon_snapshot=true, manual_entry=true,
    official_api=true, reference_data=true, synthetic_fixture=true}) end
  if x.clock_trust ~= nil then one(x.clock_trust, {trusted=true, uncertain=true}) end
  for _, k in ipairs({'observed_at_utc_ms', 'received_at_utc_ms'}) do
    if optional(x[k]) then V.integer(x, k) end
  end
  for _, k in ipairs({'session_elapsed_ms', 'sequence'}) do
    if optional(x[k]) then V.integer(x, k, 0); check(optional(x.session_id)) end
  end
  if optional(x.session_id) then V.id(x.session_id) end
end
function V.observation(x, validate)
  object(x); V.metadata(x)
  one(x.status, {known=true, unknown=true, unsupported=true, restricted=true, not_applicable=true})
  if x.status == 'known' then
    check(optional(x.value)); validate(x.value, x, 'value')
  else check(x.value == nil) end
end
local function bool(x) check(type(x) == 'boolean') end
local function level(_, owner, key) V.integer(owner, key, 1, 65535) end
function V.record(x, validate)
  object(x)
  local n = 0
  for k, value in pairs(x) do
    n = n + 1; check(n <= 1024); V.id(k); validate(value, x, k)
  end
end
function V.unique(x, validate)
  array(x); local seen = {}
  for _, item in ipairs(x) do
    if validate then validate(item) end
    check(not seen[item]); seen[item] = true
  end
end
function V.snapshot(x)
  object(x); V.integer(x, 'schema_version', 1, 1); V.id(x.id); V.id(x.character)
  V.context(x.context); V.metadata(x.capture); V.id(x.source_revision)
  V.observation(x.level, level); V.observation(x.role, V.id)
  V.observation(x.location, function(p) V.position(p); check(V.same(x.context, p.map.context)) end)
  V.record(x.preferences, function(o) V.observation(o, bool) end)
  V.record(x.objectives, function(o) V.observation(o, bool) end)
  V.record(x.equipment, function(o)
    V.observation(o, function(slot)
      object(slot); one(slot.state, {empty=true, item=true})
      if slot.state == 'item' then
        local item = slot.item; object(item); V.context(item.context)
        check(V.same(x.context, item.context)); V.id(item.item_id)
        V.observation(item.variant, V.id); V.observation(item.instance, V.id)
        V.observation(item.modifiers, function(m) V.record(m, finite) end)
      end
    end)
  end)
  V.record(x.quests, function(q, _, key)
    object(q); V.id(q.quest); check(key == q.quest); V.context(q.context)
    check(V.same(x.context, q.context))
    for _, k in ipairs({'accepted', 'completed', 'turned_in'}) do V.observation(q[k], bool) end
    V.record(q.objectives, function(o) V.observation(o, bool) end)
  end)
end
function V.party(x)
  object(x); V.id(x.id); V.context(x.context); array(x.participants); check(#x.participants > 0)
  local characters, ids = {}, {}
  for _, s in ipairs(x.participants) do
    V.snapshot(s); check(V.same(x.context, s.context))
    check(not characters[s.character] and not ids[s.id])
    characters[s.character], ids[s.id] = true, true
  end
end
local states = {accepted=true, completed=true, turned_in=true}
local function field(f)
  object(f); one(f.field, {level=true, role=true, preference=true, quest=true, objective=true})
  if f.field == 'preference' then V.id(f.key)
  elseif f.field == 'quest' then V.id(f.quest); one(f.state, states)
  elseif f.field == 'objective' then V.id(f.objective) end
end
local function scalar(s)
  object(s)
  one(s.type, {bool=true, integer=true, number=true, copper=true, text=true})
  if s.type == 'bool' then bool(s.value)
  elseif s.type == 'integer' then V.integer(s, 'value')
  elseif s.type == 'number' then finite(s.value)
  elseif s.type == 'text' then V.text(s.value)
  else
    check(type(s.value) == 'string' and s.value:match('^%d+$') and
      (#s.value == 1 or s.value:sub(1, 1) ~= '0') and #s.value <= 20)
    check(#s.value < 20 or s.value <= '18446744073709551615')
  end
end
function V.predicate(root, limits, context)
  limits = limits or {depth=32, nodes=1024, steps=1024}
  for _, k in ipairs({'depth', 'nodes', 'steps'}) do
    V.integer(limits, k, 1, k == 'depth' and 32 or 1024)
  end
  local stack, count = {{root, 1}}, 0
  while #stack > 0 do
    local entry = table.remove(stack); local n, depth = entry[1], entry[2]
    object(n); count = count + 1; check(depth <= limits.depth and count <= limits.nodes)
    local op = n.op
    if op == 'all' or op == 'any' then
      array(n.predicates); check(count + #stack + #n.predicates <= limits.nodes)
      for _, child in ipairs(n.predicates) do stack[#stack + 1] = {child, depth + 1} end
    elseif op == 'not' or op == 'participant_state' then
      if op == 'participant_state' then V.id(n.participant) end
      stack[#stack + 1] = {n.predicate, depth + 1}
    elseif op == 'known' then field(n.field)
    elseif op == 'equals' or op == 'greater_than_or_equal' then
      field(n.field); scalar(n.value)
      local kind = n.field.field
      check((kind == 'level' and n.value.type == 'integer') or
        (kind == 'role' and n.value.type == 'text') or
        ((kind == 'preference' or kind == 'quest' or kind == 'objective') and
          n.value.type == 'bool'))
      check(op ~= 'greater_than_or_equal' or kind == 'level')
    elseif op == 'quest_state' then V.id(n.quest); one(n.state, states); bool(n.value)
    elseif op == 'objective_state' then V.id(n.objective); bool(n.completed)
    elseif op == 'level_at_least' then level(nil, n, 'level')
    elseif op == 'equipment_matches' then
      V.id(n.slot); object(n.item); V.context(n.item.context)
      V.id(n.item.item_id); V.id(n.item.variant)
      V.record(n.modifiers, finite)
      if context then compatible(context, n.item.context) end
    elseif op == 'role_is' then V.id(n.role)
    elseif op == 'location_in_supported_area' then
      object(n.area); V.map(n.area.map)
      if optional(n.area.bounds) then
        local b = n.area.bounds; object(b)
        for _, k in ipairs({'min_x', 'min_y', 'max_x', 'max_y'}) do
          finite(b[k]); check(b[k] >= 0 and b[k] <= 1)
        end
        check(b.min_x <= b.max_x and b.min_y <= b.max_y)
      end
      if context then compatible(context, n.area.map.context) end
    elseif op == 'user_preference' then V.id(n.key); bool(n.value)
    else check(false) end
  end
end
function V.deepEqual(a, b)
  if type(a) ~= type(b) then return false end
  if type(a) ~= 'table' then return a == b end
  if a == D.null or b == D.null then return a == b end
  for k, v in pairs(a) do if not V.deepEqual(v, b[k]) then return false end end
  for k in pairs(b) do if a[k] == nil then return false end end
  return true
end
-- Serde drops unknown struct members and applies metadata/optional defaults.
local function project(x, keys)
  local out = {}
  for _, k in ipairs(keys) do if optional(x[k]) then out[k] = x[k] end end
  return out
end
local function normalizeContext(x)
  return {namespace=x.namespace, build=project(x.build, {'edition', 'build', 'interface'})}
end
local function normalizeMap(x)
  local out = project(x, {'map_id', 'floor', 'instance'})
  out.context = normalizeContext(x.context)
  return out
end
local function metadata(x)
  local out = project(x, {'observed_at_utc_ms', 'received_at_utc_ms',
    'session_id', 'session_elapsed_ms', 'sequence'})
  out.source, out.clock_trust = x.source or 'unrecorded', x.clock_trust or 'uncertain'
  return out
end
local function observation(x, normalize)
  local out = metadata(x); out.status = x.status
  if x.status == 'known' then out.value = normalize and normalize(x.value) or x.value end
  return out
end
function V.normalizedSnapshot(x)
  local out = project(x, {'schema_version', 'id', 'character', 'source_revision'})
  out.context, out.capture = normalizeContext(x.context), metadata(x.capture)
  out.level, out.role = observation(x.level), observation(x.role)
  out.location = observation(x.location, function(p)
    return {map=normalizeMap(p.map), x=p.x, y=p.y}
  end)
  for _, k in ipairs({'preferences', 'objectives'}) do
    out[k] = {}; for key, o in pairs(x[k]) do out[k][key] = observation(o) end
  end
  out.equipment = {}
  for key, o in pairs(x.equipment) do
    out.equipment[key] = observation(o, function(slot)
      local result = {state=slot.state}
      if slot.state == 'item' then
        local item = slot.item
        result.item = {context=normalizeContext(item.context), item_id=item.item_id,
          variant=observation(item.variant), instance=observation(item.instance),
          modifiers=observation(item.modifiers)}
      end
      return result
    end)
  end
  out.quests = {}
  for key, q in pairs(x.quests) do
    local progress = {quest=q.quest, context=normalizeContext(q.context), objectives={}}
    for _, k in ipairs({'accepted', 'completed', 'turned_in'}) do
      progress[k] = observation(q[k])
    end
    for k, o in pairs(q.objectives) do progress.objectives[k] = observation(o) end
    out.quests[key] = progress
  end
  return out
end
function V.byteLess(a, b)
  for i = 1, math.min(#a, #b) do
    local x, y = a:byte(i), b:byte(i)
    if x ~= y then return x < y end
  end
  return #a < #b
end
function V.freshness(f)
  object(f); one(f.clock_trust, {trusted=true, uncertain=true})
  if optional(f.now_utc_ms) then V.integer(f, 'now_utc_ms') end
  V.integer(f, 'max_age_ms', 0)
  if optional(f.session) then
    array(f.session); check(#f.session == 2); V.id(f.session[1]); V.integer(f.session, 2, 0)
  end
end
local function truth(b) return b and 'true' or 'false' end
local function freshness(o, f)
  local now, observed
  if optional(f.session) and optional(o.session_id) and optional(o.session_elapsed_ms) and
    f.session[1] == o.session_id then now, observed = f.session[2], o.session_elapsed_ms
  elseif f.clock_trust == 'trusted' and o.clock_trust == 'trusted' and
    optional(f.now_utc_ms) and optional(o.observed_at_utc_ms) then
    now, observed = f.now_utc_ms, o.observed_at_utc_ms
  else return 'clock_uncertain' end
  if now < observed then return 'clock_uncertain' end
  -- Compare before subtraction: a double cannot exactly represent all UTC deltas.
  if observed < 0 and now > SAFE + observed then return 'clock_uncertain' end
  local age = now - observed
  if age > SAFE then return 'clock_uncertain' end
  return age > f.max_age_ms and 'stale' or 'fresh'
end
function P.evaluate(predicate, context)
  local s, party = context.snapshot, context.party
  if party == D.null then party = nil end
  local limits = context.limits or {depth=32, nodes=1024, steps=1024}
  local result = {truth='unknown', steps=0, reasons={}, warnings={}}
  local function charge()
    check(result.steps < limits.steps); result.steps = result.steps + 1
  end
  local function issue(who, input, reason, status, warning)
    local why = {reason=reason}; if status then why.status = status end
    local list = warning and result.warnings or result.reasons
    list[#list + 1] = {participant=who, input=input, reason=why}
  end
  local function usable(o, who, input)
    charge()
    if o.status ~= 'known' then issue(who, input, 'unavailable', o.status); return nil end
    local state = freshness(o, context.freshness)
    if state == 'fresh' then return o.value end
    if state == 'stale' and context.stale_policy == 'allow_stale' then
      issue(who, input, 'stale', nil, true); return o.value
    end
    issue(who, input, state); return nil
  end
  local function getField(snapshot, f)
    charge()
    local input, o, kind = {input='field', field=f}, nil, f.field
    if kind == 'level' or kind == 'role' then o = snapshot[kind]
    elseif kind == 'preference' then o = snapshot.preferences[f.key]
    elseif kind == 'objective' then o = snapshot.objectives[f.objective]
    elseif kind == 'quest' and snapshot.quests[f.quest] then
      o = snapshot.quests[f.quest][f.state]
    end
    if not o then issue(snapshot.character, input, 'missing_record'); return nil end
    return usable(o, snapshot.character, input)
  end
  local function compare(snapshot, f, expected, gte)
    local actual = getField(snapshot, f)
    if actual == nil then return 'unknown' end
    if gte then return truth(actual >= expected) end
    return truth(actual == expected)
  end
  local node
  node = function(n, snapshot)
    charge()
    local op, who = n.op, snapshot.character
    if op == 'all' or op == 'any' then
      local answer = op == 'all' and 'true' or 'false'
      for _, child in ipairs(n.predicates) do
        local t = node(child, snapshot)
        if (op == 'all' and t == 'false') or (op == 'any' and t == 'true') then return t end
        if t == 'unknown' then answer = 'unknown' end
      end
      return answer
    elseif op == 'not' then
      local t = node(n.predicate, snapshot)
      return t == 'unknown' and t or (t == 'true' and 'false' or 'true')
    elseif op == 'known' then return truth(getField(snapshot, n.field) ~= nil)
    elseif op == 'equals' or op == 'greater_than_or_equal' then
      return compare(snapshot, n.field, n.value.value, op == 'greater_than_or_equal')
    elseif op == 'level_at_least' then return compare(snapshot, {field='level'}, n.level, true)
    elseif op == 'role_is' then return compare(snapshot, {field='role'}, n.role)
    elseif op == 'user_preference' then
      return compare(snapshot, {field='preference', key=n.key}, n.value)
    elseif op == 'objective_state' then
      return compare(snapshot, {field='objective', objective=n.objective}, n.completed)
    elseif op == 'quest_state' then
      return compare(snapshot, {field='quest', quest=n.quest, state=n.state}, n.value)
    elseif op == 'participant_state' then
      if n.participant == who then return node(n.predicate, snapshot) end
      if party then
        for _, participant in ipairs(party.participants) do
          charge()
          if participant.character == n.participant then return node(n.predicate, participant) end
        end
      end
      issue(n.participant, {input='participant'}, 'missing_record'); return 'unknown'
    elseif op == 'location_in_supported_area' then
      local point = usable(snapshot.location, who, {input='location'})
      if point == nil then return 'unknown' end
      compatible(n.area.map.context, point.map.context)
      if not V.deepEqual(normalizeMap(n.area.map), normalizeMap(point.map)) then return 'false' end
      local b = n.area.bounds
      return truth(not optional(b) or
        (point.x >= b.min_x and point.x <= b.max_x and point.y >= b.min_y and point.y <= b.max_y))
    elseif op == 'equipment_matches' then
      charge()
      local input, o = {input='equipment', slot=n.slot}, snapshot.equipment[n.slot]
      if not o then issue(who, input, 'missing_record'); return 'unknown' end
      local slot = usable(o, who, input)
      if slot == nil then return 'unknown' end
      if slot.state == 'empty' then return 'false' end
      local equipped = slot.item; compatible(equipped.context, n.item.context)
      if equipped.item_id ~= n.item.item_id then return 'false' end
      local variant = usable(equipped.variant, who, {input='variant', slot=n.slot})
      if variant ~= nil and variant ~= n.item.variant then return 'false' end
      local answer = variant == nil and 'unknown' or 'true'
      local keys = {}; for k in pairs(n.modifiers) do keys[#keys + 1] = k end
      table.sort(keys, V.byteLess)
      if #keys > 0 then
        local path = {input='modifiers', slot=n.slot}
        local actual = usable(equipped.modifiers, who, path)
        if actual == nil then return 'unknown' end
        for _, k in ipairs(keys) do
          charge()
          if actual[k] == nil then issue(who, path, 'missing_record'); answer = 'unknown'
          elseif actual[k] ~= n.modifiers[k] then return 'false' end
        end
      end
      return answer
    end
  end
  local ok, value = pcall(function()
    V.predicate(predicate, limits); V.snapshot(s); V.freshness(context.freshness)
    check(context.stale_policy == nil or context.stale_policy == 'reject' or
      context.stale_policy == 'allow_stale')
    if party then
      V.party(party)
      compatible(s.context, party.context)
      for _, participant in ipairs(party.participants) do
        if participant.character == s.character then
          check(V.deepEqual(V.normalizedSnapshot(participant), V.normalizedSnapshot(s)))
        end
      end
    end
    V.predicate(predicate, nil, s.context)
    return node(predicate, s)
  end)
  if not ok then return {error=value == 'BUILD_MISMATCH' and value or 'INVALID_INPUT'} end
  result.truth = value; return result
end
-- Validate all supplied observations, not just the short-circuited branches.
function P.validateInputs(predicate, snapshot, party)
  local ok = pcall(function()
    V.predicate(predicate); V.snapshot(snapshot)
    if optional(party) then V.party(party) end
  end)
  return ok
end
V.normalizedMap = normalizeMap
V.array = array
P.validation = V
P.freshness = freshness
ns.provide("Predicate", P)
