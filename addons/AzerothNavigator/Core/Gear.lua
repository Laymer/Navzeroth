-- Synthetic scoring-only foundation. No live equipment observations or client APIs.
local _, ns = ...
local D, V, Gear = ns.Decode, ns.Predicate.validation, {}
local SAFE = 9007199254740991
local function check(ok) if not ok then error('INVALID_INPUT', 0) end end
local function safe(n)
  check(type(n) == 'number' and n == math.floor(n) and math.abs(n) <= SAFE)
  return n
end
local function sum(a, b)
  if b > 0 then check(a <= SAFE - b) else check(a >= -SAFE - b) end
  return safe(a + b)
end
local function copy(value)
  if type(value) ~= 'table' then return value end
  local out = {}
  for k, v in pairs(value) do out[k] = copy(v) end
  return out
end
local function keys(value)
  local out = {}; for key in pairs(value) do out[#out + 1] = key end
  table.sort(out, V.byteLess); return out
end
local function keyLess(a, b)
  for _, name in ipairs({'edition', 'build', 'interface'}) do
    if a.context.build[name] ~= b.context.build[name] then
      return V.byteLess(a.context.build[name], b.context.build[name])
    end
  end
  if a.context.namespace ~= b.context.namespace then return a.context.namespace == 'demo' end
  if a.item_id ~= b.item_id then return V.byteLess(a.item_id, b.item_id) end
  return V.byteLess(a.variant, b.variant)
end
-- Decimal limbs keep every multiplication and carry exactly representable in doubles.
local function limbs(n)
  local out = {}
  repeat out[#out + 1] = n % 1000; n = math.floor(n / 1000) until n == 0
  return out
end
local function multiply(a, b)
  safe(a); safe(b)
  local sign = ((a < 0) ~= (b < 0)) and -1 or 1
  local x, y, product = limbs(math.abs(a)), limbs(math.abs(b)), {}
  for i = 1, #x + #y do product[i] = 0 end
  for i, left in ipairs(x) do
    for j, right in ipairs(y) do product[i + j - 1] = product[i + j - 1] + left * right end
  end
  for i = 1, #product - 1 do
    product[i + 1] = product[i + 1] + math.floor(product[i] / 1000)
    product[i] = product[i] % 1000
  end
  -- Dividing by scale 1000 drops the low limb. Round halves away from zero.
  local result = 0
  for i = #product, 2, -1 do
    check(result <= math.floor((SAFE - product[i]) / 1000))
    result = result * 1000 + product[i]
  end
  if product[1] >= 500 then result = sum(result, 1) end
  return sign * result
end
function Gear.fixedMul(a, b)
  local ok, value = pcall(multiply, a, b)
  if ok then return value end
  return nil, 'INVALID_INPUT'
end
function Gear.toFixed(value)
  local ok, result = pcall(function()
    check(type(value) == 'number' and value == value and math.abs(value) <= SAFE)
    local n = math.abs(value * 1000)
    check(n <= SAFE)
    local integer = math.floor(n)
    if n - integer >= 0.5 then integer = sum(integer, 1) end
    return value < 0 and -integer or integer
  end)
  if ok then return result end
  return nil, 'INVALID_INPUT'
end
local function idArray(x, limit)
  V.unique(x, V.text); check(#x <= limit)
end
local function syntheticObservation(o, validate)
  V.observation(o, validate)
  check(o.source ~= 'addon_snapshot' and o.source ~= 'manual_entry' and o.source ~= 'official_api')
end
local function validate(input)
  check(input.format == 'lua-gear-input-v1' and input.fixture == true)
  V.context(input.context)
  check(input.context.namespace == 'demo' and input.context.build.build == 'SYNTHETIC' and
    input.context.build.interface == 'SYNTHETIC')
  local p = input.profile
  V.id(p.role); check(p.version == 'gear-model-v1' and p.weight_source.type == 'synthetic_test')
  V.record(p.weights, function(_, owner, key) V.integer(owner, key) end)
  p.weight_source = {type='synthetic_test'}
  local attributes = keys(p.weights); check(#attributes > 0 and #attributes <= 64)
  idArray(p.known_unmodeled, 64)
  V.array(input.candidates); check(#input.candidates <= 64)
  local identities = {}
  for _, candidate in ipairs(input.candidates) do
    local key = candidate.key
    V.context(key.context); check(V.same(key.context, input.context))
    V.id(key.item_id); V.id(key.variant)
    candidate.key = {item_id=key.item_id, variant=key.variant, context={
      namespace=key.context.namespace, build={edition=key.context.build.edition,
        build=key.context.build.build, interface=key.context.build.interface}}}
    key = candidate.key
    local identity = key.item_id .. '\0' .. key.variant
    check(not identities[identity]); identities[identity] = true
    syntheticObservation(candidate.attributes_milli, function(a)
      V.record(a, function(_, owner, name) V.integer(owner, name) end); check(#keys(a) <= 128)
    end)
    syntheticObservation(candidate.effects, function(e)
      V.array(e); check(#e <= 64); for _, text in ipairs(e) do V.text(text) end
    end)
  end
  table.sort(input.candidates, function(a, b) return keyLess(a.key, b.key) end)
end
local function score(input, indices)
  V.array(indices); check(#indices <= 64)
  local p, candidates, seen = input.profile, {}, {}
  for _, index in ipairs(indices) do
    safe(index); check(index >= 1 and index <= #input.candidates and not seen[index])
    seen[index] = true; candidates[#candidates + 1] = input.candidates[index]
  end
  table.sort(candidates, function(a, b) return keyLess(a.key, b.key) end)
  local attributes, rows, reasons, effects, itemKeys = keys(p.weights), {}, {}, {}, {}
  local certainty = 'complete'
  for _, attribute in ipairs(attributes) do rows[attribute] = {value=0, contribution=0} end
  for _, candidate in ipairs(candidates) do
    local key, observation = candidate.key, candidate.attributes_milli
    itemKeys[#itemKeys + 1] = key
    if observation.status ~= 'known' then
      reasons[#reasons + 1] = {code='attributes_unavailable', item=key, status=observation.status}
      certainty = 'low'
    else
      for _, attribute in ipairs(keys(observation.value)) do
        local value, weight = observation.value[attribute], p.weights[attribute]
        if weight == nil then
          reasons[#reasons + 1] = {code='unweighted_attribute', item=key, attribute=attribute}
        else
          local row = rows[attribute]
          row.value = sum(row.value, value)
          row.contribution = sum(row.contribution, multiply(value, weight))
        end
      end
    end
    observation = candidate.effects
    if observation.status ~= 'known' then
      reasons[#reasons + 1] = {code='effects_unavailable', item=key, status=observation.status}
      effects[#effects + 1] = {code='unavailable', item=key, status=observation.status}
    elseif #observation.value > 0 then
      reasons[#reasons + 1] = {code='unmodeled_effects_listed', item=key, count=#observation.value}
      for _, text in ipairs(observation.value) do
        effects[#effects + 1] = {code='listed', item=key, text=text}
      end
    end
  end
  if certainty ~= 'low' and #reasons > 0 then certainty = 'reduced' end
  local contributions, total = {}, 0
  for _, attribute in ipairs(attributes) do
    local row = rows[attribute]; total = sum(total, row.contribution)
    contributions[#contributions + 1] = {attribute=attribute, value=row.value,
      weight=p.weights[attribute], contribution=row.contribution}
  end
  return {label='model score', model_version=p.version, role=p.role, weight_source=p.weight_source,
    contributions=contributions, total=total, certainty=certainty, certainty_reasons=reasons,
    unmodeled_effects=effects, profile_unmodeled=p.known_unmodeled}, itemKeys
end
local certaintyOrder = {complete=1, reduced=2, low=3}
local function compare(a, b)
  if a.score.total ~= b.score.total then return a.score.total > b.score.total end
  if a.score.certainty ~= b.score.certainty then
    return certaintyOrder[a.score.certainty] < certaintyOrder[b.score.certainty]
  end
  for i = 1, math.min(#a.keys, #b.keys) do
    if keyLess(a.keys[i], b.keys[i]) then return true end
    if keyLess(b.keys[i], a.keys[i]) then return false end
  end
  return #a.keys < #b.keys
end
function Gear.new(text)
  local input, code = D.decode(text)
  if not input then return nil, code end
  local ok = pcall(validate, input)
  if not ok then return nil, 'INVALID_INPUT' end
  local scorer = {}
  scorer.score = function(_, indices)
    local valid, result = pcall(score, input, indices)
    if not valid then return nil, 'INVALID_INPUT' end
    return copy(result)
  end
  -- Order caller-selected sets only; no legality/eligibility/search claim is made.
  scorer.rank = function(_, configurations)
    local valid, result = pcall(function()
      V.array(configurations); check(#configurations <= 128)
      local entries = {}
      for _, indices in ipairs(configurations) do
        local breakdown, identity = score(input, indices)
        for _, prior in ipairs(entries) do check(not V.deepEqual(prior.keys, identity)) end
        entries[#entries + 1] = {score=breakdown, keys=identity}
      end
      table.sort(entries, compare); return entries
    end)
    if not valid then return nil, 'INVALID_INPUT' end
    return copy(result)
  end
  return scorer
end
function Gear.definitivelyExceeds(a, b)
  return a.certainty == 'complete' and b.certainty == 'complete' and
    a.model_version == b.model_version and a.role == b.role and a.total > b.total
end
ns.provide('Gear', Gear)
