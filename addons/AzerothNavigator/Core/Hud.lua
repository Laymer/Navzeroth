-- Pure minimal HUD model. Observation and rendering callbacks are injected.
-- No frame, client events or equipment/quest APIs are bound by this foundation.
local _, ns = ...
local D, P, Hud = ns.Decode, ns.Predicate, {}
local V = P.validation
local function gate(observation, context, notApplicable)
  if observation.status ~= 'known' then
    return {truth=observation.status == 'not_applicable' and notApplicable or 'unknown',
      observation_status=observation.status, reasons={}, warnings={}, steps=0}
  end
  return P.evaluate(observation.value, {snapshot=context.snapshot, party=context.party,
    freshness=context.freshness, stale_policy='reject', limits=context.limits})
end
local function applies(step, character)
  for _, id in ipairs(step.participants) do
    if id == character then return true end
  end
  return false
end
local function completion(step, context)
  -- Completion is positive objective evidence, never the inverse of readiness.
  return P.evaluate({op='objective_state', objective=step.objective, completed=true},
    {snapshot=context.snapshot, party=context.party, freshness=context.freshness,
      stale_policy='reject', limits=context.limits})
end
local function advice(step, context)
  local invalidation = gate(step.invalidation, context, 'false')
  local prerequisite = gate(step.prerequisite, context, 'true')
  local status
  if invalidation.error or prerequisite.error then status = 'evaluation_error'
  elseif invalidation.truth == 'true' then status = 'invalidated'
  elseif invalidation.truth == 'unknown' or prerequisite.truth == 'unknown' then
    status = 'uncertain'
  elseif prerequisite.truth == 'true' then status = 'ready'
  else status = 'blocked' end
  return {status=status, step_id=step.id, objective=step.objective,
    explanation=step.explanation, alternatives=step.alternatives,
    prerequisite=prerequisite, invalidation=invalidation}
end
local function dependenciesMatch(plan, current)
  if current == nil then return true end
  if type(current) ~= 'table' then return false end
  for _, key in ipairs({'dataset_revision', 'ruleset_version', 'score_model_version'}) do
    if current[key] ~= plan[key] then return false end
  end
  if current.snapshots ~= nil and not V.deepEqual(current.snapshots, plan.inputs) then
    return false
  end
  return true
end
function Hud.new(plans, observe, render)
  assert(type(observe) == 'function' and type(render) == 'function')
  local hud, revision, progress = {}, nil, {}
  hud.refresh = function(_)
    local envelope = plans:get()
    local model = {status='no_plan', fixture=false}
    if envelope then
      local plan = envelope.payload
      model = {status='waiting', fixture=envelope.fixture, plan_id=plan.id, goal=plan.goal}
      local ok, context = pcall(observe)
      model.dependency_inputs_supplied = ok and type(context) == 'table' and
        context.dependencies ~= nil
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
      elseif not dependenciesMatch(plan, context.dependencies) then
        model.status = 'dependencies_changed'
      else
        local currentRevision = plans:revision()
        if currentRevision ~= revision then progress, revision = {}, currentRevision end
        local character = context.snapshot.character
        local participant = false
        for _, input in ipairs(plan.inputs) do
          if input.character == context.snapshot.character then participant = true end
        end
        local localProgress = progress[character] or {completed={}, selected={}}
        if participant then progress[character] = localProgress end
        model.status = participant and 'no_steps' or 'participant_missing'
        local byObjective, applicableCount = {}, 0
        for _, step in ipairs(plan.steps) do
          if applies(step, character) then
            applicableCount = applicableCount + 1
            -- Ambiguous duplicate objectives cannot act as alternative definitions.
            if byObjective[step.objective] ~= nil then byObjective[step.objective] = false
            else byObjective[step.objective] = step end
            local done = completion(step, context)
            if not done.error and done.truth == 'true' then
              localProgress.completed[step.id] = true
            end
          end
        end
        local anchor
        for _, step in ipairs(plan.steps) do
          if applies(step, character) and not localProgress.completed[step.id] then
            anchor = step; break
          end
        end
        if participant and applicableCount > 0 and not anchor then model.status = 'complete' end
        if anchor then
          local chosen, rejected = anchor, {}
          local primary = advice(anchor, context)
          -- Keep an eligible selected alternative until completion/readiness changes.
          local retained = localProgress.selected[anchor.id]
          local candidates = {}
          if retained then candidates[#candidates + 1] = retained end
          for _, objective in ipairs(anchor.alternatives) do
            if objective ~= retained then candidates[#candidates + 1] = objective end
          end
          if retained or primary.status ~= 'ready' then
            for _, objective in ipairs(candidates) do
              local alternative = byObjective[objective]
              local reason
              if not alternative then reason = 'readiness_not_declared'
              elseif localProgress.completed[alternative.id] then reason = 'completed'
              else
                local done = completion(alternative, context)
                local candidate = advice(alternative, context)
                if done.error then reason = 'evaluation_error'
                elseif done.truth ~= 'false' then reason = 'completion_uncertain'
                elseif candidate.status ~= 'ready' then reason = candidate.status
                else chosen = alternative; break end
              end
              rejected[#rejected + 1] = {objective=objective, reason=reason}
            end
          end
          localProgress.selected[anchor.id] = chosen ~= anchor and chosen.objective or nil
          local selected = advice(chosen, context)
          for key, value in pairs(selected) do model[key] = value end
          model.completion = completion(chosen, context)
          model.anchor_step_id = anchor.id
          model.selected_alternative = chosen ~= anchor
          -- v1 travel is global; an alternate objective cannot rewrite its route.
          model.travel_requires_revalidation = chosen ~= anchor
          model.rejected_alternatives = rejected
          -- Completing an alternative never proves that the primary was completed.
        end
      end
    end
    model.requires_revalidation = model.travel_requires_revalidation == true or
      model.status == 'invalidated' or
      model.status == 'dependencies_changed' or model.status == 'expired' or
      model.status == 'context_mismatch' or model.status == 'fixture_mismatch'
    render(model)
    return model
  end
  return hud
end
ns.provide("Hud", Hud)
