-- Runtime-only explicit plan controls. Client UI and observations are injected.
local _, ns = ...
local Controls = {}
function Controls.new(plans, observe, render)
  local state, last = {}, {status='no_plan'}
  local hud = ns.Hud.new(plans, observe, function(model) last = model end)
  state.refresh = function(_)
    local envelope = plans:get()
    local model, observed
    if envelope then
      local ok, context = pcall(observe)
      observed = ok and type(context) == 'table' and context or nil
      local snapshot = ok and type(context) == 'table' and context.snapshot
      if type(snapshot) == 'table' then
        for _, input in ipairs(envelope.payload.inputs) do
          if input.character == snapshot.character and input.id ~= snapshot.id then
            model = {status='snapshot_superseded', requires_revalidation=true}
          end
        end
      end
    end
    model = model or hud:refresh()
    if envelope then
      model.plan_id = envelope.payload.id
      model.expires_at_utc_ms = envelope.payload.expires_at_utc_ms
      model.dependency_check_available = model.dependency_inputs_supplied == true
      model.objective_mapping_available = false
      model.observation_clock_uncertain = not observed or not observed.freshness or
        observed.freshness.clock_trust ~= 'trusted' or not observed.snapshot or
        not observed.snapshot.capture or observed.snapshot.capture.clock_trust ~= 'trusted'
      -- The addon has no authoritative current dataset/ruleset identity in this wave.
      if not model.dependency_check_available then model.requires_revalidation = true end
    end
    last = model
    render(model)
    return model
  end
  state.import = function(_, text)
    local receipt, code = plans:importTransport(text)
    if not receipt then return nil, code end
    state:refresh()
    return receipt
  end
  state.clear = function(_) plans:clear(); return state:refresh() end
  state.status = function(_) return state:refresh() end
  state.describe = function(_, model)
    model = model or last
    local lines = {'Azeroth Navigator - ' .. model.status}
    if model.plan_id then lines[#lines + 1] = 'Plan: ' .. model.plan_id end
    if model.step_id then lines[#lines + 1] = 'Step: ' .. model.step_id end
    if model.explanation then lines[#lines + 1] = model.explanation:sub(1, 1200) end
    if model.plan_id then
      local expires = model.expires_at_utc_ms
      lines[#lines + 1] = expires ~= nil and expires ~= ns.Decode.null and
        ('Expiry UTC milliseconds: ' .. string.format('%.0f', expires)) or 'No declared expiry'
      lines[#lines + 1] = model.dependency_check_available and
        'Dependency checks supplied' or 'Dependency checks unavailable'
      lines[#lines + 1] = 'Objective completion mapping unavailable; steps do not auto-complete'
      if model.observation_clock_uncertain then
        lines[#lines + 1] = 'Observation age uncertain; clock confirmation/capture needed'
      end
      if model.status == 'uncertain' or model.status == 'clock_uncertain' or
        model.status == 'observations_invalid' then
        lines[#lines + 1] = 'Observations uncertain; capture explicitly and inspect missing inputs'
      end
      if model.requires_revalidation then lines[#lines + 1] = 'Revalidation required' end
    end
    return table.concat(lines, '\n')
  end
  return state
end
ns.provide('PlanControls', Controls)
