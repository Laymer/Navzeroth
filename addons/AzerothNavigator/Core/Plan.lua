-- Transactional loaded-plan state. Fixed production decoder limits; no client I/O.
local _, ns = ...
local D, P = ns.Decode, ns.Predicate
local V, Plan = P.validation, {}
local function check(ok) if not ok then error('plan_invalid', 0) end end
local function has(x) return x ~= nil and x ~= D.null end
local function ids(x) V.unique(x, V.id) end
local function duration(_, owner, key) V.integer(owner, key, 0) end
local function range(x)
  V.integer(x, 'min_ms', 0); V.integer(x, 'max_ms', 0); check(x.min_ms <= x.max_ms)
end
local function timing(expected, estimate)
  V.observation(expected, duration); V.observation(estimate, range)
  if expected.status == 'known' and estimate.status == 'known' then
    check(expected.value >= estimate.value.min_ms and expected.value <= estimate.value.max_ms)
  end
end
local function ref(x)
  V.id(x.id); V.id(x.character); V.context(x.context)
end
local function validate(envelope, expected)
  check(type(envelope) == 'table' and envelope.format == 'azeroth-navigator' and
    envelope.kind == 'route_plan')
  V.integer(envelope, 'schema_version', 1, 1)
  check(type(envelope.fixture) == 'boolean')
  check(envelope.namespace == 'reference' or (envelope.namespace == 'demo' and envelope.fixture))
  local context = {build={edition=envelope.game.edition, build=envelope.game.build,
    interface=envelope.game.interface}, namespace=envelope.namespace}
  V.context(context); V.id(envelope.game.locale)
  check(V.same(context, expected.context) and envelope.fixture == expected.fixture)
  V.nonemptyText(envelope.producer.version)
  check(({addon=true, desktop=true, cli=true, test_fixture=true})[envelope.producer.component])
  check(envelope.provenance.source ~= nil and envelope.provenance.clock_trust ~= nil)
  V.metadata(envelope.provenance)
  if has(envelope.producer.session_id) then V.id(envelope.producer.session_id) end
  if has(envelope.producer.sequence) then
    V.integer(envelope.producer, 'sequence', 0); check(has(envelope.producer.session_id))
  end
  check(envelope.producer.component ~= 'test_fixture' or envelope.fixture)
  if has(envelope.provenance.created_at_utc_ms) then
    V.integer(envelope.provenance, 'created_at_utc_ms')
  end
  check((envelope.provenance.source == 'synthetic_fixture') == envelope.fixture)
  local plan, deps = envelope.payload, envelope.dependencies
  V.integer(plan, 'schema_version', 1, 1); V.id(plan.id); V.context(plan.context)
  check(V.same(context, plan.context))
  V.integer(plan, 'generated_at_utc_ms'); V.integer(plan, 'time_budget_ms', 0)
  for _, k in ipairs({'dataset_revision', 'ruleset_version', 'score_model_version'}) do
    V.id(plan[k]); check(deps[k] == plan[k])
  end
  V.array(deps.snapshots); V.array(plan.inputs)
  check(#deps.snapshots == #plan.inputs)
  for i, input in ipairs(plan.inputs) do
    ref(input); ref(deps.snapshots[i])
    check(input.id == deps.snapshots[i].id and input.character == deps.snapshots[i].character and
      V.same(input.context, deps.snapshots[i].context))
  end
  V.unique(plan.inputs); check(#plan.inputs > 0)
  local participants, snapshots = {}, {}
  for _, input in ipairs(plan.inputs) do
    ref(input); check(V.same(context, input.context))
    check(not participants[input.character] and not snapshots[input.id])
    participants[input.character], snapshots[input.id] = true, true
  end
  V.text(plan.goal); ids(plan.exclusions); ids(plan.evidence)
  V.array(plan.manual_assumptions)
  for _, assumption in ipairs(plan.manual_assumptions) do V.text(assumption) end
  V.record(plan.capability_assumptions, function(status)
    check(({verified_available=true, verified_unavailable=true, restricted=true,
      failed=true, untested=true})[status])
    if envelope.fixture then
      check(status ~= 'verified_available' and status ~= 'verified_unavailable')
    end
  end)
  V.unique(plan.revalidation, function(trigger)
    check(({dataset_revision_changed=true, build_changed=true, snapshot_superseded=true,
      ruleset_changed=true})[trigger])
  end)
  if has(plan.expires_at_utc_ms) then
    V.integer(plan, 'expires_at_utc_ms'); check(plan.expires_at_utc_ms >= plan.generated_at_utc_ms)
  end
  V.unique(plan.steps); check(#plan.steps <= 100)
  local stepIds, evidence, exclusions = {}, {}, {}
  for _, id in ipairs(plan.evidence) do evidence[id] = true end
  for _, id in ipairs(plan.exclusions) do exclusions[id] = true end
  for _, step in ipairs(plan.steps) do
    V.id(step.id); check(not stepIds[step.id]); stepIds[step.id] = true
    V.id(step.objective); check(not exclusions[step.objective])
    ids(step.participants); check(#step.participants > 0)
    for _, id in ipairs(step.participants) do check(participants[id]) end
    ids(step.alternatives); check(#step.alternatives <= 8)
    for _, id in ipairs(step.alternatives) do check(id ~= step.objective and not exclusions[id]) end
    for _, k in ipairs({'prerequisite', 'invalidation'}) do
      V.observation(step[k], function(predicate)
        V.predicate(predicate, {depth=12, nodes=1024, steps=1024}, context)
      end)
    end
    V.array(step.required_fields)
    for _, field in ipairs(step.required_fields) do V.text(field) end
    V.text(step.explanation); ids(step.evidence)
    for _, id in ipairs(step.evidence) do check(evidence[id]) end
    timing(step.expected_ms, step.estimate)
  end
  V.array(plan.travel)
  local previous
  for _, segment in ipairs(plan.travel) do
    for _, k in ipairs({'from', 'to'}) do
      V.id(segment[k].id); V.map(segment[k].map); check(V.same(context, segment[k].map.context))
    end
    if previous then check(V.deepEqual(previous, segment.from)) end
    previous = segment.to; timing(segment.expected_ms, segment.range)
  end
  -- Validate source claims throughout the known observation-bearing payload.
  local function sources(value)
    if type(value) ~= 'table' or value == D.null then return end
    if value.status ~= nil and value.source ~= nil then
      local source = value.source
      if envelope.fixture then
        check(source ~= 'addon_snapshot' and source ~= 'manual_entry' and source ~= 'official_api')
      else check(source ~= 'synthetic_fixture') end
    end
    for _, child in pairs(value) do sources(child) end
  end
  sources(plan)
  local reserved = {format=true, kind=true, schema_version=true, fixture=true, producer=true,
    game=true, namespace=true, provenance=true, dependencies=true, payload=true}
  local extensionCount, extensionBytes = 0, 0
  local function extension(value)
    if type(value) == 'number' then check(math.abs(value) <= 9007199254740991)
    elseif type(value) == 'table' and value ~= D.null then
      for _, child in pairs(value) do extension(child) end
    end
  end
  for key, value in pairs(envelope) do
    if not reserved[key] then
      check(type(key) == 'string' and #key > 0 and #key <= 256)
      for _, cp in ipairs(D.codepoints(key)) do
        check(cp >= 32 and not (cp >= 127 and cp <= 159))
      end
      extensionCount = extensionCount + 1
      local bytes = D.bytes(envelope, key)
      extensionBytes = extensionBytes + bytes
      check(extensionCount <= 64 and bytes <= 16384 and extensionBytes <= 65536)
      extension(value)
    end
  end
  return plan
end
function Plan.new(expected)
  V.context(expected.context); check(type(expected.fixture) == 'boolean')
  -- Copy context so callers cannot change the acceptance boundary after creation.
  local boundary = {fixture=expected.fixture, context={namespace=expected.context.namespace,
    build={edition=expected.context.build.edition, build=expected.context.build.build,
      interface=expected.context.build.interface}}}
  local text, revision = nil, 0
  local state = {}
  state.importText = function(_, candidate)
    local envelope, code = D.decode(candidate)
    if not envelope then return nil, code end
    local ok, result = pcall(validate, envelope, boundary)
    if not ok then return nil, 'plan_invalid' end
    if candidate ~= text then revision = revision + 1 end
    text = candidate
    return {id=result.id, fixture=envelope.fixture, steps=#result.steps}
  end
  -- Manual ANAV1 framing. Raw-JSON callers keep importText; replacement stays atomic.
  state.importTransport = function(_, candidate)
    if type(candidate) ~= 'string' or #candidate > 32768 then return nil, 'transport_bytes' end
    local document = candidate:match('^[ \t\r\n]*ANAV1:(.*)$')
    if not document or document:sub(1, 1) ~= '{' then return nil, 'transport_invalid' end
    document = document:gsub('[ \t\r\n]+$', '')
    return state:importText(document)
  end
  -- Every query returns an independent decoded copy; no external mutable plan alias.
  state.get = function(_)
    if not text then return nil end
    return D.decode(text)
  end
  state.revision = function(_) return revision end
  state.clear = function(_)
    if text then revision = revision + 1 end
    text = nil
  end
  return state
end
ns.provide("Plan", Plan)
