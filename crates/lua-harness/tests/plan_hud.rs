//! Synthetic loaded-plan/HUD workflow; no client frames or events are modeled.
mod common;

use azeroth_lua_harness::Sandbox;
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(common::path(&format!("fixtures/contracts/v1/{name}"))).unwrap(),
    )
    .unwrap()
}

fn setup() -> Sandbox {
    let sandbox = common::strict_core();
    sandbox
        .ns()
        .set(
            "plan_text",
            serde_json::to_string(&fixture("route-plan.json")).unwrap(),
        )
        .unwrap();
    sandbox
        .run::<()>(
            "setup",
            r#"
      local envelope = assert(ns.Decode.decode(ns.plan_text))
      ns.plans = ns.Plan.new({context=envelope.payload.context, fixture=true})
      assert(ns.plans:importText(ns.plan_text))
    "#,
        )
        .unwrap();
    sandbox
}

fn import(sandbox: &Sandbox, value: &Value) -> bool {
    sandbox
        .ns()
        .set("candidate", serde_json::to_string(value).unwrap())
        .unwrap();
    sandbox
        .run("import", "return ns.plans:importText(ns.candidate) ~= nil")
        .unwrap()
}

#[test]
fn route_plan_contract_fixtures_validate_and_failed_import_preserves_plan() {
    let sandbox = setup();
    let manifest = fixture("manifest.json");
    let mut count = 0;
    for entry in manifest["fixtures"].as_array().unwrap() {
        if entry["kind"] != "route_plan" {
            continue;
        }
        let name = entry["file"].as_str().unwrap();
        // Keep exact input bytes so duplicate-key cases remain duplicates.
        sandbox
            .ns()
            .set(
                "candidate",
                std::fs::read_to_string(common::path(&format!("fixtures/contracts/v1/{name}")))
                    .unwrap(),
            )
            .unwrap();
        let accepted: bool = sandbox
            .run("import", "return ns.plans:importText(ns.candidate) ~= nil")
            .unwrap();
        assert_eq!(accepted, entry["expect"] == "valid", "{name}");
        assert!(
            sandbox
                .run::<bool>(
                    "retained",
                    "return ns.plans:get().payload.id == 'demo-plan-001'"
                )
                .unwrap()
        );
        count += 1;
    }
    assert!(count >= 10);
}

#[test]
fn context_fixture_expiry_and_alias_boundaries_are_enforced() {
    let sandbox = setup();
    let mut doc = fixture("route-plan.json");
    doc["fixture"] = json!(false);
    assert!(!import(&sandbox, &doc));
    doc = fixture("route-plan.json");
    doc["game"]["interface"] = json!("different");
    assert!(!import(&sandbox, &doc));
    doc = fixture("route-plan.json");
    doc["payload"]["expires_at_utc_ms"] = json!(0);
    assert!(!import(&sandbox, &doc));
    doc = fixture("route-plan.json");
    doc["payload"]["steps"][0]["estimate"]["value"]["min_ms"] = json!(120001);
    assert!(!import(&sandbox, &doc));
    doc = fixture("route-plan.json");
    doc["payload"]["steps"][0]["participants"] = json!(["foreign-character"]);
    assert!(!import(&sandbox, &doc));
    doc = fixture("route-plan.json");
    doc["payload"]["steps"][0]["prerequisite"]["source"] = json!("addon_snapshot");
    assert!(!import(&sandbox, &doc));
    sandbox
        .run::<()>(
            "alias",
            r#"
      local copy = ns.plans:get(); copy.payload.steps[1].id = 'tampered'
      assert(ns.plans:get().payload.steps[1].id == 'demo-step-1')
      local envelope = ns.plans:get()
      local boundary = {context=envelope.payload.context, fixture=true}
      local other = ns.Plan.new(boundary)
      boundary.context.build.build = 'tampered'
      assert(other:importText(ns.plan_text))
      ns.plans:clear(); assert(ns.plans:get() == nil)
    "#,
        )
        .unwrap();
}

#[test]
fn injected_hud_keeps_unknown_blocked_ready_invalidated_and_expired_distinct() {
    let sandbox = setup();
    let corpus: Value = serde_json::from_str(
        &std::fs::read_to_string(common::path("fixtures/predicates/v1/operators.json")).unwrap(),
    )
    .unwrap();
    let mut snapshot = corpus["snapshots"]["populated"].clone();
    let context = fixture("route-plan.json")["payload"]["context"].clone();
    fn replace_context(value: &mut Value, context: &Value) {
        match value {
            Value::Object(map) => {
                if map.contains_key("context") {
                    map.insert("context".into(), context.clone());
                }
                for v in map.values_mut() {
                    replace_context(v, context);
                }
            }
            Value::Array(items) => {
                for v in items {
                    replace_context(v, context);
                }
            }
            _ => {}
        }
    }
    replace_context(&mut snapshot, &context);
    snapshot["character"] = json!("demo-character-paladin");
    snapshot["quests"]["demo-quest-1"] = snapshot["quests"]["q1"].clone();
    snapshot["quests"]["demo-quest-1"]["quest"] = json!("demo-quest-1");
    sandbox
        .ns()
        .set("snapshot_text", serde_json::to_string(&snapshot).unwrap())
        .unwrap();
    sandbox
        .run::<()>(
            "hud",
            r#"
      local snapshot = assert(ns.Decode.decode(ns.snapshot_text))
      local context = {snapshot=snapshot, fixture=true, stale_policy='reject',
        freshness={clock_trust='trusted', now_utc_ms=150, max_age_ms=100}}
      local rendered, renders = nil, 0
      local hud = ns.Hud.new(ns.plans, function() return context end, function(model)
        rendered = model; renders = renders + 1
      end)
      -- Unknown invalidation prevents a definite ready recommendation.
      assert(hud:refresh().status == 'uncertain')
      assert(rendered.fixture and rendered.step_id == 'demo-step-1')
      snapshot.quests['demo-quest-1'].turned_in = {status='known', value=false,
        clock_trust='trusted', observed_at_utc_ms=100}
      assert(hud:refresh().status == 'ready')
      snapshot.level.value = 10
      assert(hud:refresh().status == 'blocked')
      snapshot.level = {status='restricted'}
      assert(hud:refresh().status == 'uncertain')
      snapshot.quests['demo-quest-1'].turned_in.value = true
      assert(hud:refresh().status == 'invalidated')
      assert(rendered.step_id == 'demo-step-1')
      -- NA is no prerequisite/no invalidation. It does not mean unknown.
      local noGates = ns.plans:get()
      noGates.payload.steps[1].prerequisite = {status='not_applicable'}
      noGates.payload.steps[1].invalidation = {status='not_applicable'}
      local original = ns.plans.get
      ns.plans.get = function() return noGates end
      assert(hud:refresh().status == 'ready')
      assert(rendered.prerequisite.observation_status == 'not_applicable')
      for _, status in ipairs({'unknown', 'unsupported', 'restricted'}) do
        noGates.payload.steps[1].prerequisite = {status=status}
        assert(hud:refresh().status == 'uncertain')
        assert(rendered.prerequisite.observation_status == status)
      end
      noGates.payload.steps[1].prerequisite = {status='not_applicable'}
      for _, status in ipairs({'unknown', 'unsupported', 'restricted'}) do
        noGates.payload.steps[1].invalidation = {status=status}
        assert(hud:refresh().status == 'uncertain')
        assert(rendered.invalidation.observation_status == status)
      end
      ns.plans.get = original
      context.freshness.now_utc_ms = 1791878400000
      assert(hud:refresh().status == 'expired')
      context.freshness.clock_trust = 'uncertain'
      assert(hud:refresh().status == 'clock_uncertain')
      context.fixture = false
      assert(hud:refresh().status == 'fixture_mismatch')
      context.fixture = true
      snapshot.context.build.build = 'different'
      assert(hud:refresh().status == 'observations_invalid')
      ns.plans:clear()
      assert(hud:refresh().status == 'no_plan')
      assert(renders == 17)
    "#,
        )
        .unwrap();
}

fn replay() -> Sandbox {
    let sandbox = setup();
    let mut snapshot = fixture("character-snapshot.json")["payload"].clone();
    snapshot["objectives"] = json!({});
    sandbox
        .ns()
        .set("snapshot_text", snapshot.to_string())
        .unwrap();
    sandbox
        .run::<()>(
            "replay_setup",
            r#"
      local envelope = ns.plans:get()
      local first = envelope.payload.steps[1]
      first.prerequisite = {status='not_applicable'}
      first.invalidation = {status='not_applicable'}
      first.alternatives = {'alternative-a', 'alternative-b', 'undefined-alternative'}
      local function step(id, objective)
        return {id=id, objective=objective, participants=first.participants,
          alternatives={}, prerequisite={status='not_applicable'},
          invalidation={status='not_applicable'}, required_fields={},
          explanation='Synthetic replay', evidence={},
          expected_ms={status='not_applicable'}, estimate={status='not_applicable'}}
      end
      envelope.payload.steps = {first, step('step-a', 'alternative-a'),
        step('step-b', 'alternative-b')}
      ns.document = envelope
      -- The diagnostic encoder accepts plain tables, not decoder metadata.
      local function plain(value)
        if value == ns.Decode.null then return nil end
        if type(value) ~= 'table' then return value end
        local copy = {}
        for k, v in pairs(value) do copy[k] = plain(v) end
        return copy
      end
      ns.encode = function() return assert(ns.Json.encode(plain(ns.document), 16384)) end
      assert(ns.plans:importText(ns.encode()))
      ns.context = {snapshot=assert(ns.Decode.decode(ns.snapshot_text)), fixture=true,
        stale_policy='allow_stale', freshness={clock_trust='trusted',
          now_utc_ms=1791792000000, max_age_ms=60000}}
      ns.known = function(value, stamp)
        return {status='known', value=value, clock_trust='trusted',
          observed_at_utc_ms=stamp or 1791792000000}
      end
      ns.setObjective = function(objective, value)
        ns.context.snapshot.objectives[objective] = ns.known(value)
      end
      ns.hud = ns.Hud.new(ns.plans, function() return ns.context end, function() end)
      ns.primary = first.objective
    "#,
        )
        .unwrap();
    sandbox
}

#[test]
fn progression_requires_fresh_positive_completion_and_is_monotonic() {
    let sandbox = replay();
    sandbox
        .run::<()>(
            "completion_replay",
            r#"
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      for _, status in ipairs({'unknown', 'unsupported', 'restricted', 'not_applicable'}) do
        ns.context.snapshot.objectives[ns.primary] = {status=status}
        local model = ns.hud:refresh()
        assert(model.step_id == 'demo-step-1' and model.completion.truth == 'unknown')
      end
      ns.setObjective(ns.primary, false)
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      ns.context.snapshot.objectives[ns.primary] = ns.known(true, 1791791000000)
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      ns.context.snapshot.objectives[ns.primary] = {status='known', value=true,
        clock_trust='uncertain'}
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      ns.setObjective(ns.primary, true)
      assert(ns.hud:refresh().step_id == 'step-a')
      ns.setObjective(ns.primary, false)
      assert(ns.hud:refresh().step_id == 'step-a')
      ns.setObjective('alternative-a', true)
      ns.setObjective('alternative-b', true)
      assert(ns.hud:refresh().status == 'complete')
      local revision = ns.plans:revision()
      assert(ns.plans:importText(ns.encode()))
      assert(ns.plans:revision() == revision and ns.hud:refresh().status == 'complete')
      assert(not ns.plans:importText('{}'))
      assert(ns.hud:refresh().status == 'complete')
      ns.document.payload.id = 'replacement-plan'
      assert(ns.plans:importText(ns.encode()))
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      ns.plans:clear(); assert(ns.hud:refresh().status == 'no_plan')
      assert(ns.plans:importText(ns.encode()))
      assert(ns.hud:refresh().step_id == 'demo-step-1')
    "#,
        )
        .unwrap();
}

#[test]
fn alternatives_stay_selected_until_change_and_never_complete_primary() {
    let sandbox = replay();
    sandbox
        .run::<()>(
            "alternative_replay",
            r#"
      ns.setObjective(ns.primary, false)
      ns.setObjective('alternative-a', false)
      ns.setObjective('alternative-b', false)
      ns.document.payload.steps[1].prerequisite = {status='known',
        value={op='level_at_least', level=100}}
      assert(ns.plans:importText(ns.encode()))
      local selected = ns.hud:refresh()
      assert(selected.status == 'ready' and selected.objective == 'alternative-a')
      assert(selected.selected_alternative and selected.anchor_step_id == 'demo-step-1')
      assert(selected.travel_requires_revalidation and selected.requires_revalidation)
      ns.context.snapshot.level = ns.known(100)
      assert(ns.hud:refresh().objective == 'alternative-a')
      ns.setObjective('alternative-a', true)
      assert(ns.hud:refresh().objective == 'alternative-b')
      ns.setObjective('alternative-b', true)
      local primary = ns.hud:refresh()
      assert(primary.step_id == 'demo-step-1' and not primary.selected_alternative)
      assert(primary.status == 'ready')
      assert(primary.rejected_alternatives[3].reason == 'readiness_not_declared')
      ns.document.payload.steps[1].invalidation = {status='known',
        value={op='level_at_least', level=100}}
      assert(ns.plans:importText(ns.encode()))
      primary = ns.hud:refresh()
      assert(primary.status == 'invalidated' and primary.requires_revalidation)
      assert(primary.step_id == 'demo-step-1')
      ns.context.snapshot.objectives['alternative-a'] = {status='restricted'}
      ns.context.snapshot.objectives['alternative-b'] = {status='unknown'}
      assert(ns.hud:refresh().status == 'invalidated')
    "#,
        )
        .unwrap();
}

#[test]
fn progression_is_participant_scoped_and_dependency_changes_suspend_it() {
    let sandbox = replay();
    sandbox
        .run::<()>(
            "participant_replay",
            r#"
      local other = 'second-character'
      local input = ns.document.payload.inputs[1]
      local newInput = {id='second-snapshot', character=other, context=input.context}
      ns.document.payload.inputs[2] = newInput
      ns.document.dependencies.snapshots[2] = newInput
      for _, step in ipairs(ns.document.payload.steps) do step.participants[2] = other end
      assert(ns.plans:importText(ns.encode()))
      ns.setObjective(ns.primary, true)
      assert(ns.hud:refresh().step_id == 'step-a')
      ns.context.snapshot.character = other
      ns.setObjective(ns.primary, false)
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      ns.context.dependencies = {dataset_revision='changed',
        ruleset_version=ns.document.payload.ruleset_version,
        score_model_version=ns.document.payload.score_model_version}
      ns.setObjective(ns.primary, true)
      local model = ns.hud:refresh()
      assert(model.status == 'dependencies_changed' and model.requires_revalidation)
      ns.context.dependencies.dataset_revision = ns.document.payload.dataset_revision
      ns.context.dependencies.snapshots = {}
      assert(ns.hud:refresh().status == 'dependencies_changed')
      ns.context.dependencies.snapshots = ns.document.payload.inputs
      ns.setObjective(ns.primary, false)
      assert(ns.hud:refresh().step_id == 'demo-step-1')
      ns.context.snapshot.character = input.character
      assert(ns.hud:refresh().step_id == 'step-a')
      ns.context.freshness.now_utc_ms = ns.document.payload.expires_at_utc_ms
      assert(ns.hud:refresh().status == 'expired')
    "#,
        )
        .unwrap();
}

#[test]
fn undeclared_ambiguous_and_unready_alternatives_are_not_recommended() {
    let sandbox = replay();
    sandbox
        .run::<()>(
            "alternative_gaps",
            r#"
      ns.document.payload.steps[1].prerequisite = {status='unknown'}
      ns.document.payload.steps[3].objective = 'alternative-a'
      assert(ns.plans:importText(ns.encode()))
      ns.setObjective('alternative-a', false)
      local model = ns.hud:refresh()
      assert(model.status == 'uncertain' and not model.selected_alternative)
      assert(model.rejected_alternatives[1].reason == 'readiness_not_declared')
      ns.document.payload.steps[3].objective = 'alternative-b'
      ns.document.payload.steps[2].invalidation = {status='known',
        value={op='level_at_least', level=1}}
      ns.document.payload.steps[3].prerequisite = {status='restricted'}
      ns.setObjective('alternative-b', false)
      assert(ns.plans:importText(ns.encode()))
      model = ns.hud:refresh()
      assert(model.rejected_alternatives[1].reason == 'invalidated')
      assert(model.rejected_alternatives[2].reason == 'uncertain')
      assert(model.status == 'uncertain' and not model.selected_alternative)
      ns.context.snapshot.objectives[ns.primary] = {status='known', value=0}
      assert(ns.hud:refresh().status == 'observations_invalid')
    "#,
        )
        .unwrap();
}
