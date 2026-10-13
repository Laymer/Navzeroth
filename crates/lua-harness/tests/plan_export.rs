//! Executes actual application-produced ANAV1 bytes in the production Lua loader.
mod common;
use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};
fn exported_at_depth(depth: usize) -> String {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let value: Value = serde_json::from_str(snapshot).unwrap();
    let now = UtcMillis::new(
        value["payload"]["capture"]["observed_at_utc_ms"]
            .as_i64()
            .unwrap()
            + 100,
    )
    .unwrap();
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let mut predicate = json!({"op":"level_at_least","level":20});
    for _ in 0..depth {
        predicate = json!({"op":"all","predicates":[predicate]});
    }
    content["envelope"]["payload"]["objectives"][0]["requirements"] =
        json!({"status":"known","value":predicate});
    app.stage_content(&content.to_string()).unwrap();
    app.activate_content(
        &profile,
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    let request = json!({"profile_id":profile,"snapshot_id":value["payload"]["id"],"revision":"synthetic-planning-v1","goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
    app.export_plan(&request.to_string(), Some(now))
        .unwrap()
        .text
}
#[test]
fn rust_export_loads_and_failed_replacements_keep_the_loaded_plan() {
    let text = exported_at_depth(0);
    let sandbox = common::strict_core();
    sandbox.ns().set("exported", text.clone()).unwrap();
    sandbox
        .ns()
        .set(
            "snapshot_text",
            include_str!("../../../fixtures/contracts/v1/character-snapshot.json"),
        )
        .unwrap();
    sandbox
        .run::<()>(
            "load",
            r#"
        local envelope = assert(ns.Decode.decode(ns.exported:sub(7)))
        ns.plans = ns.Plan.new({context=envelope.payload.context, fixture=true})
        assert(ns.plans:importTransport(ns.exported))
        ns.original = ns.plans:get().payload.id
        assert(#ns.plans:get().payload.travel == 2)
        assert(ns.plans:importText(ns.exported:sub(7)))
        local snapshot = assert(ns.Decode.decode(ns.snapshot_text)).payload
        local context = {snapshot=snapshot, fixture=true, stale_policy='reject',
          freshness={clock_trust='trusted', now_utc_ms=1791791990100, max_age_ms=1800000}}
        local step = ns.plans:get().payload.steps[1]
        assert(ns.Predicate.evaluate(step.prerequisite.value, context).truth == 'true')
        assert(ns.Predicate.evaluate(step.invalidation.value, context).truth == 'false')
        snapshot.level.value = 1
        assert(ns.Predicate.evaluate(step.prerequisite.value, context).truth == 'false')
        assert(ns.Predicate.evaluate(step.invalidation.value, context).truth == 'true')
        assert(ns.plans:get().payload.steps[1].id == step.id)
        snapshot.level = {status='restricted'}
        assert(ns.Predicate.evaluate(step.prerequisite.value, context).truth == 'unknown')
        assert(ns.Predicate.evaluate(step.invalidation.value, context).truth == 'unknown')
        assert(ns.plans:importTransport(' \t\r\n' .. ns.exported .. '\r\n'))
    "#,
        )
        .unwrap();
    for invalid in [
        text[6..].to_owned(),
        format!("anav1:{}", &text[6..]),
        format!("\u{feff}{text}"),
        format!("ANAV1: {}", &text[6..]),
        format!("{text}{text}"),
        format!("{text}\u{000b}"),
        " ".repeat(32769),
    ] {
        sandbox.ns().set("invalid", invalid).unwrap();
        assert!(sandbox.run::<bool>("reject", "return ns.plans:importTransport(ns.invalid) == nil and ns.plans:get().payload.id == ns.original").unwrap());
    }
    for path in ["fixture", "dependency", "context"] {
        let mut value: Value = serde_json::from_str(&text[6..]).unwrap();
        match path {
            "fixture" => value["fixture"] = json!(false),
            "dependency" => value["dependencies"]["dataset_revision"] = json!("changed"),
            _ => value["game"]["build"] = json!("different"),
        }
        sandbox
            .ns()
            .set("invalid", format!("ANAV1:{value}"))
            .unwrap();
        assert!(sandbox.run::<bool>("reject mismatch", "return ns.plans:importTransport(ns.invalid) == nil and ns.plans:get().payload.id == ns.original").unwrap());
    }
    // Total transport size includes whitespace and prefix, with an exact boundary.
    let exact = format!("{}{}", text, " ".repeat(32768 - text.len()));
    sandbox.ns().set("exact", exact.clone()).unwrap();
    assert!(
        sandbox
            .run::<bool>(
                "boundary",
                "return ns.plans:importTransport(ns.exact) ~= nil"
            )
            .unwrap()
    );
    sandbox.ns().set("exact", format!("{exact} ")).unwrap();
    assert!(
        sandbox
            .run::<bool>(
                "boundary",
                "return ns.plans:importTransport(ns.exact) == nil"
            )
            .unwrap()
    );
}

#[test]
fn maximum_compiled_predicate_depth_is_consumed_by_lua() {
    let sandbox = common::strict_core();
    sandbox.ns().set("text", exported_at_depth(9)).unwrap();
    assert!(sandbox.run::<bool>("compiled depth", "local e = assert(ns.Decode.decode(ns.text:sub(7))); local p = ns.Plan.new({context=e.payload.context,fixture=true}); return p:importTransport(ns.text) ~= nil").unwrap());
}

#[test]
fn rust_export_advances_only_on_explicit_objective_completion() {
    let sandbox = common::strict_core();
    sandbox.ns().set("exported", exported_at_depth(0)).unwrap();
    sandbox
        .ns()
        .set(
            "snapshot_text",
            include_str!("../../../fixtures/contracts/v1/character-snapshot.json"),
        )
        .unwrap();
    sandbox
        .run::<()>(
            "exported_progression",
            r#"
      local envelope = assert(ns.Decode.decode(ns.exported:sub(7)))
      local plans = ns.Plan.new({context=envelope.payload.context, fixture=true})
      assert(plans:importTransport(ns.exported))
      local snapshot = assert(ns.Decode.decode(ns.snapshot_text)).payload
      local context = {snapshot=snapshot, fixture=true,
        freshness={clock_trust='trusted', now_utc_ms=1791791990100, max_age_ms=1800000}}
      local hud = ns.Hud.new(plans, function() return context end, function() end)
      local step = plans:get().payload.steps[1]
      assert(hud:refresh().step_id == step.id)
      snapshot.level.value = 1
      assert(hud:refresh().status == 'invalidated')
      assert(hud:refresh().step_id == step.id)
      snapshot.objectives[step.objective] = {status='known', value=true,
        clock_trust='trusted', observed_at_utc_ms=1791791990100}
      assert(hud:refresh().status == 'complete')
    "#,
        )
        .unwrap();
}

#[test]
fn explicit_plan_controller_uses_rust_bytes_and_invalidates_replaced_snapshot() {
    let sandbox = common::strict_core();
    sandbox.ns().set("text", exported_at_depth(0)).unwrap();
    sandbox
        .ns()
        .set(
            "snapshot_text",
            include_str!("../../../fixtures/contracts/v1/character-snapshot.json"),
        )
        .unwrap();
    sandbox
        .run::<()>(
            "controls",
            r#"
      local envelope=assert(ns.Decode.decode(ns.text:sub(7)))
      local snapshot=assert(ns.Decode.decode(ns.snapshot_text)).payload
      local plans=ns.Plan.new({context=snapshot.context,fixture=true})
      local context={snapshot=snapshot,fixture=true,freshness={clock_trust='trusted',
        now_utc_ms=1791791990100,max_age_ms=1800000},stale_policy='reject'}
      local last
      local controls=ns.PlanControls.new(plans,function() return context end,function(m) last=m end)
      assert(controls:import(ns.text))
      assert(last.plan_id==envelope.payload.id and last.requires_revalidation)
      assert(last.dependency_check_available==false and last.objective_mapping_available==false)
      assert(controls:describe():find('Dependency checks unavailable',1,true))
      assert(last.status~='complete')
      local prior=plans:get().payload.id
      assert(not controls:import('ANAV1:{'))
      assert(plans:get().payload.id==prior)
      snapshot.id='new-snapshot'
      assert(controls:refresh().status=='snapshot_superseded')
      assert(last.requires_revalidation and not last.step_id)
      snapshot.id=envelope.payload.inputs[1].id
      context.freshness.clock_trust='uncertain'
      assert(controls:refresh().status~='ready')
      context.freshness.clock_trust='trusted'
      context.freshness.now_utc_ms=9007199254740991
      assert(controls:refresh().status=='expired')
      controls:clear()
      assert(last.status=='no_plan' and plans:get()==nil)
    "#,
        )
        .unwrap();
}
