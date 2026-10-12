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
fn injected_hud_keeps_unknown_blocked_ready_complete_and_expired_distinct() {
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
      assert(hud:refresh().status == 'complete')
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
      assert(renders == 10)
    "#,
        )
        .unwrap();
}
