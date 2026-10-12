-- Pure minimal HUD model. Observation and rendering callbacks are injected.
-- No frame, client events or equipment/quest APIs are bound by this foundation.
local _, ns = ...
local D, P, Hud = ns.Decode, ns.Predicate, {}
local V = P.validation
local function gate(observation, context)
  if observation.status ~= 'known' then
    return {truth='unknown', reasons={}, warnings={}, steps=0}
  end
  return P.evaluate(observation.value, context)
end
function Hud.new(plans, observe, render)
  assert(type(observe) == 'function' and type(render) == 'function')
  local hud = {}
  hud.refresh = function(_)
    local envelope = plans:get()
    local model = {status='no_plan', fixture=false}
    if envelope then
      local plan = envelope.payload
      model = {status='waiting', fixture=envelope.fixture, plan_id=plan.id, goal=plan.goal}
      local ok, context = pcall(observe)
      if not ok or type(context) ~= 'table' or
        not P.validateInputs({op='all', predicates={}}, context.snapshot, context.party) then
        model.status = 'observations_invalid'
      elseif not V.same(plan.context, context.snapshot.context) then
        model.status = 'context_mismatch'
      elseif context.fixture ~= envelope.fixture then model.status = 'fixture_mismatch'
      elseif not pcall(V.freshness, context.freshness) then model.status = 'clock_uncertain'
      elseif plan.expires_at_utc_ms ~= nil and plan.expires_at_utc_ms ~= D.null and
        (context.freshness.clock_trust ~= 'trusted' or
          type(context.freshness.now_utc_ms) ~= 'number') then
        model.status = 'clock_uncertain'
      elseif plan.expires_at_utc_ms ~= nil and plan.expires_at_utc_ms ~= D.null and
        context.freshness.now_utc_ms >= plan.expires_at_utc_ms then model.status = 'expired'
      else
        local participant = false
        for _, input in ipairs(plan.inputs) do
          if input.character == context.snapshot.character then participant = true end
        end
        model.status = participant and 'complete' or 'participant_missing'
        for _, step in ipairs(plan.steps) do
          local applicable = false
          for _, id in ipairs(step.participants) do
            if id == context.snapshot.character then applicable = true end
          end
          if applicable then
            local invalidation = gate(step.invalidation, context)
            if invalidation.error then model.status = 'evaluation_error'; break end
            if invalidation.truth ~= 'true' then
              local prerequisite = gate(step.prerequisite, context)
              model.step_id, model.objective = step.id, step.objective
              model.explanation, model.alternatives = step.explanation, step.alternatives
              model.prerequisite, model.invalidation = prerequisite, invalidation
              if prerequisite.error then model.status = 'evaluation_error'
              elseif invalidation.truth == 'unknown' or prerequisite.truth == 'unknown' then
                model.status = 'uncertain'
              elseif prerequisite.truth == 'true' then model.status = 'ready'
              else model.status = 'blocked' end
              break
            end
          end
        end
      end
    end
    render(model)
    return model
  end
  return hud
end
ns.provide("Hud", Hud)
