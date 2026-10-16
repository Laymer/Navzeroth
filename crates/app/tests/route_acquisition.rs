use azeroth_app::{route_acquisition::*, snapshots::SnapshotApplication};
use azeroth_core::{
    error::ErrorCode,
    types::{DatasetRevision, UtcMillis},
};
use serde_json::{Value, json};
const NOW: i64 = 1791791990100;
fn now() -> Option<UtcMillis> {
    Some(UtcMillis::new(NOW).unwrap())
}
fn known(v: Value) -> Value {
    json!({"status":"known","value":v,"source":"synthetic_fixture","observed_at_utc_ms":NOW-100,"clock_trust":"trusted"})
}
fn setup(change: impl FnOnce(&mut Value)) -> (SnapshotApplication, Value, Value) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    snapshot["payload"]["equipment"] = json!({"head":{"status":"known","value":{"state":"empty"},"source":"synthetic_fixture","clock_trust":"trusted","observed_at_utc_ms":NOW-100}});
    snapshot["payload"]["quests"]["demo-quest-1"]["accepted"] = known(json!(true));
    snapshot["payload"]["quests"]["demo-quest-1"]["completed"] = known(json!(false));
    snapshot["payload"]["quests"]["demo-quest-1"]["turned_in"] = known(json!(false));
    snapshot["payload"]["objectives"]["demo-objective-1"] = known(json!(false));
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), now())
        .unwrap()
        .profile_id;
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let context = snapshot["payload"]["context"].clone();
    let key = json!({"context":context,"item_id":"synthetic-upgrade","variant":"base"});
    content["envelope"]["payload"]["items"] = json!([{"key":key,"slots":["head"],"two_handed":{"status":"not_applicable"},"unique_group":{"status":"not_applicable"},"attributes":known(json!({"power":10})),"requirements":known(json!({"op":"all","predicates":[]})),"effects":known(json!([])),"evidence":[]}]);
    content["envelope"]["payload"]["acquisitions"] = json!([{"id":"synthetic-source","context":context,"kind":"other","items":[key],"conditions":{"status":"not_applicable"},"cost":known(json!("0")),"probability":known(json!(1)),"evidence":[]}]);
    let mut node = content["envelope"]["payload"]["nodes"][0].clone();
    node["id"] = json!("synthetic-source-node");
    content["envelope"]["payload"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(node);
    for (id, from, to, time) in [
        ("detour-out", "demo-node-1", "synthetic-source-node", 5000),
        (
            "detour-rejoin",
            "synthetic-source-node",
            "demo-node-2",
            30000,
        ),
    ] {
        let mut edge = content["envelope"]["payload"]["edges"][0].clone();
        edge["id"] = json!(id);
        edge["from"]["id"] = json!(from);
        edge["to"]["id"] = json!(to);
        edge["expected_ms"] = known(json!(time));
        edge["uncertainty"] = json!({"status":"unknown"});
        content["envelope"]["payload"]["edges"]
            .as_array_mut()
            .unwrap()
            .push(edge);
    }
    change(&mut content);
    let _: azeroth_storage::content::ContentRevision =
        serde_json::from_value(content.clone()).unwrap();
    app.stage_content(&content.to_string()).unwrap();
    app.activate_content(
        &profile,
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    let planning = json!({"profile_id":profile,"snapshot_id":snapshot["payload"]["id"],"revision":"synthetic-planning-v1","goal_profile":"manual","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
    let preview = app
        .preview_acquisition_route(&planning.to_string(), now())
        .unwrap();
    let input = json!({"schema_version":1,"planning":planning,"expected_route_fingerprint":preview.route_fingerprint,"opportunities":[{"source":"synthetic-source","target":key,"node":{"id":"synthetic-source-node","map":content["envelope"]["payload"]["nodes"][0]["position"]["map"]},"leg_index":0,"formation_ms":known(json!(2000)),"attempt_ms":known(json!(10000)),"available_copper":{"status":"unknown"},"max_attempts":3,"independent_player_attempts":true,"cost_per_attempt":false}],"gear":null});
    (app, input, content)
}
fn run(app: &SnapshotApplication, input: &Value) -> Value {
    serde_json::to_value(
        app.assess_route_acquisition(&input.to_string(), now())
            .unwrap(),
    )
    .unwrap()
}
/// Every golden that TypeScript consumes is compared here, not only `report.json`.
/// `UPDATE_ROUTE_ACQUISITION_GOLDENS` rewrites them; otherwise any drift fails.
fn check_golden(name: &str, value: &Value) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/desktop/route-acquisition");
    let path = dir.join(format!("{name}.json"));
    let text = format!("{}\n", serde_json::to_string_pretty(value).unwrap());
    if std::env::var_os("UPDATE_ROUTE_ACQUISITION_GOLDENS").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing golden {name}; run with UPDATE_ROUTE_ACQUISITION_GOLDENS=1")
    });
    assert_eq!(
        committed, text,
        "{name} drifted; rerun with UPDATE_ROUTE_ACQUISITION_GOLDENS=1 and review the diff"
    );
}
#[test]
fn detour_rejoins_selected_route_and_reserves_baseline_return_once() {
    let (app, input, _content) = setup(|_| {});
    let output = run(&app, &input);
    assert_eq!(
        output["baseline"]["planning_report"]["result"]["expected_total_ms"],
        61000
    );
    assert_eq!(output["remaining_budget_ms"], 39000);
    let r = &output["opportunities"][0];
    assert_eq!(r["detour"]["incremental_ms"], 5000);
    assert_eq!(r["assessment"]["effort"]["stopping_ms"]["value"], 0);
    assert_eq!(r["assessment"]["feasible_attempts"], 3);
    assert_eq!(r["assessment"]["budgeted_effort_ms"], 37000);
    assert_eq!(r["total_with_reservation_ms"], 98000);
    assert_eq!(output, run(&app, &input));
    {
        for (name, value) in [
            ("baseline", output["baseline"].clone()),
            ("report", output.clone()),
            ("input", input.clone()),
            (
                "stored-snapshot",
                serde_json::to_value(
                    app.current(input["planning"]["profile_id"].as_str().unwrap(), now())
                        .unwrap(),
                )
                .unwrap(),
            ),
            (
                "gear-catalog",
                serde_json::to_value(
                    app.gear_catalog(input["planning"]["profile_id"].as_str().unwrap())
                        .unwrap(),
                )
                .unwrap(),
            ),
            (
                "catalog",
                serde_json::to_value(
                    app.planning_catalog(input["planning"]["profile_id"].as_str().unwrap())
                        .unwrap(),
                )
                .unwrap(),
            ),
        ] {
            check_golden(name, &value);
        }
    }
    let committed: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/route-acquisition/report.json"
    ))
    .unwrap();
    assert_eq!(output, committed);
}
#[test]
fn zero_on_route_and_missing_binding_are_distinct() {
    let (app, mut input, _) = setup(|_| {});
    input["opportunities"][0]["node"]["id"] = json!("demo-node-2");
    let r = run(&app, &input);
    assert_eq!(r["opportunities"][0]["detour"]["incremental_ms"], 0);
    input["opportunities"][0]["node"] = Value::Null;
    let r = run(&app, &input);
    assert_eq!(
        r["opportunities"][0]["detour"]["reason"],
        "missing_manual_binding"
    );
    assert_eq!(
        r["opportunities"][0]["assessment"]["effort"]["incremental_travel_ms"]["status"],
        "unknown"
    );
    assert!(r["opportunities"][0]["assessment"]["feasible_attempts"].is_null());
}
#[test]
fn directed_rejoin_gap_and_restricted_route_conditions_do_not_become_zero() {
    for status in ["restricted", "unknown"] {
        let (app, input, _) = setup(|c| {
            c["envelope"]["payload"]["edges"][3]["requirements"] = json!({"status":status});
        });
        let r = run(&app, &input);
        assert_eq!(
            r["opportunities"][0]["detour"]["reason"],
            "rejoin_incomplete"
        );
        assert!(
            r["opportunities"][0]["detour"]["rejoin"]["exclusions"]
                .to_string()
                .contains(status)
        );
        assert!(r["opportunities"][0]["assessment"]["credited_reward"].is_null());
    }
}
#[test]
fn unknown_formation_zero_probability_and_unqualified_player_model_are_preserved() {
    let (app, mut input, _) = setup(|_| {});
    input["opportunities"][0]["formation_ms"] = json!({"status":"restricted"});
    let r = run(&app, &input);
    assert!(
        r["opportunities"][0]["assessment"]["reasons"]
            .to_string()
            .contains("restricted")
    );
    input["opportunities"][0]["formation_ms"] = known(json!(0));
    input["opportunities"][0]["independent_player_attempts"] = json!(false);
    let r = run(&app, &input);
    assert!(r["opportunities"][0]["assessment"]["success_probability"].is_null());
    assert!(
        r["opportunities"][0]["assessment"]["reasons"]
            .to_string()
            .contains("probability_model_unqualified")
    );
    let (app, input, _) =
        setup(|c| c["envelope"]["payload"]["acquisitions"][0]["probability"] = known(json!(0)));
    assert_eq!(
        run(&app, &input)["opportunities"][0]["assessment"]["success_probability"].as_f64(),
        Some(0.0)
    );
}
#[test]
fn exact_nodes_variants_schema_counts_and_current_route_are_guarded() {
    let (app, input, _) = setup(|_| {});
    for mutation in 0..7 {
        let mut bad = input.clone();
        match mutation {
            0 => bad["opportunities"][0]["node"]["id"] = json!("missing-node"),
            1 => bad["opportunities"][0]["target"]["variant"] = json!("other"),
            2 => bad["schema_version"] = json!(2),
            3 => bad["planning"]["candidates"][0]["utility"] = json!(11),
            4 => bad["opportunities"] = json!(vec![bad["opportunities"][0].clone(); 129]),
            5 => {
                bad["opportunities"][0]["node"]["map"]["context"]["build"]["build"] =
                    json!("foreign")
            }
            _ => bad["unknown_field"] = json!(true),
        };
        assert!(
            app.assess_route_acquisition(&bad.to_string(), now())
                .is_err()
        );
    }
    assert_eq!(
        app.assess_route_acquisition(
            &input.to_string(),
            Some(UtcMillis::new(NOW + 1800001).unwrap())
        )
        .unwrap_err()
        .code,
        ErrorCode::SnapshotOutdated
    );
    assert!(
        app.assess_route_acquisition(&"x".repeat(MAX_REQUEST_BYTES + 1), now())
            .is_err()
    );
}
#[test]
fn optional_gear_receives_only_authoritative_derived_effort() {
    let (app, mut input, _) = setup(|_| {});
    let key = input["opportunities"][0]["target"].clone();
    input["gear"] = json!({"profile_id":input["planning"]["profile_id"],"snapshot_id":input["planning"]["snapshot_id"],"revision":"synthetic-planning-v1","model":{"role":"manual","version":"gear-model-v1","weight_source":{"type":"synthetic_test"},"weights":{"power":1000},"known_unmodeled":[]},"layout":{"slots":["head"],"hands":null},"class":{"status":"unknown"},"proficiency":{"classes":{}},"candidates":[{"item":key,"category":{"status":"not_applicable"},"acquisition":null}]});
    input["opportunities"][0]["formation_ms"] = json!({"status":"known","value":2000});
    input["opportunities"][0]["attempt_ms"] = json!({"status":"known","value":10000});
    let r = run(&app, &input);
    {
        for (name, value) in [
            ("report-with-gear", r.clone()),
            ("gear-request", input["gear"].clone()),
        ] {
            check_golden(name, &value);
        }
    }
    assert_eq!(
        r["gear"]["request"]["candidates"][0]["acquisition"]["effort"]["incremental_travel_ms"]["value"],
        5000
    );
    assert_eq!(r["gear"]["practical"][0]["reserved_effort_ms"], 37000);
    input["gear"]["candidates"][0]["acquisition"] =
        r["gear"]["request"]["candidates"][0]["acquisition"].clone();
    assert!(
        app.assess_route_acquisition(&input.to_string(), now())
            .is_err()
    );
}
#[test]
fn selected_quest_rewards_prevent_duplicate_full_item_credit_and_missing_coverage_blocks_credit() {
    for unknown in [false, true] {
        let (app, mut input, _) = setup(|c| {
            let original: Value = serde_json::from_str(include_str!(
                "../../../fixtures/desktop/content/demo-revision.json"
            ))
            .unwrap();
            c["envelope"]["payload"]["quests"] = original["envelope"]["payload"]["quests"].clone();
            c["envelope"]["payload"]["quests"][0]["rewards"] = if unknown {
                json!({"status":"unknown"})
            } else {
                known(json!([c["envelope"]["payload"]["items"][0]["key"].clone()]))
            };
            c["envelope"]["payload"]["objectives"][0]["targets"] =
                json!([{"kind":"quest_stage","quest":"demo-quest-1","stage":"demo-objective-1"}]);
        });
        input["gear"] = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/route-acquisition/gear-request.json"
        ))
        .unwrap();
        let r = run(&app, &input);
        assert!(r["gear"]["practical"].as_array().unwrap().is_empty());
        assert_eq!(
            r["baseline"]["planning_report"]["result"]["selected"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let a = &r["opportunities"][0];
        assert!(a["assessment"]["credited_reward"].is_null());
        assert_eq!(a["reward_accounting_complete"], !unknown);
        if !unknown {
            assert!(
                a["assessment"]["reasons"]
                    .to_string()
                    .contains("already_counted")
            );
        }
    }
}
#[test]
fn unsafe_stateful_intermediate_detour_is_refused_and_short_budget_is_not_infinite_trials() {
    let (app, mut input, _) = setup(|_| {});
    input["planning"]["budget_ms"] = json!(62000);
    input["expected_route_fingerprint"] = json!(
        app.preview_acquisition_route(&input["planning"].to_string(), now())
            .unwrap()
            .route_fingerprint
    );
    let r = run(&app, &input);
    assert_eq!(r["opportunities"][0]["assessment"]["feasible_attempts"], 0);
    assert_eq!(r["opportunities"][0]["assessment"]["status"], "unavailable");
    let (app, input, _) = setup(|c| {
        let e = &mut c["envelope"]["payload"]["edges"][2];
        e["mode"] = json!("return");
        e["status"] = json!("estimated");
    });
    let r = run(&app, &input);
    assert_eq!(
        r["opportunities"][0]["detour"]["reason"],
        "outbound_incomplete"
    );
    assert!(
        r["opportunities"][0]["detour"]["outbound"]["exclusions"]
            .to_string()
            .contains("stateful_not_verified")
    );
}

#[test]
fn duplicate_opportunities_missing_sources_and_variant_identity_refuse_or_remain_independent() {
    let (app, mut input, _) = setup(|_| {});
    let duplicate = input["opportunities"][0].clone();
    input["opportunities"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(
        app.assess_route_acquisition(&input.to_string(), now())
            .is_err()
    );
    input["opportunities"].as_array_mut().unwrap().pop();
    input["opportunities"][0]["source"] = json!("not-a-registered-source");
    assert!(
        app.assess_route_acquisition(&input.to_string(), now())
            .is_err()
    );
}
#[test]
fn quest_reward_deduplication_uses_complete_variant_identity() {
    let (app, mut input, _) = setup(|c| {
        let original: Value = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        let key = c["envelope"]["payload"]["items"][0]["key"].clone();
        c["envelope"]["payload"]["quests"] = original["envelope"]["payload"]["quests"].clone();
        c["envelope"]["payload"]["quests"][0]["rewards"] = known(json!([key]));
        c["envelope"]["payload"]["objectives"][0]["targets"] =
            json!([{"kind":"quest_stage","quest":"demo-quest-1","stage":"demo-objective-1"}]);
        let mut variant = c["envelope"]["payload"]["items"][0].clone();
        variant["key"]["variant"] = json!("other");
        let variant_key = variant["key"].clone();
        c["envelope"]["payload"]["items"]
            .as_array_mut()
            .unwrap()
            .push(variant);
        c["envelope"]["payload"]["acquisitions"][0]["items"]
            .as_array_mut()
            .unwrap()
            .push(variant_key);
    });
    input["opportunities"][0]["target"]["variant"] = json!("other");
    let r = run(&app, &input);
    assert_eq!(
        r["opportunities"][0]["assessment"]["credited_reward"]["variant"],
        "other"
    );
    assert!(
        r["opportunities"][0]["total_with_reservation_ms"]
            .as_u64()
            .unwrap()
            <= input["planning"]["budget_ms"].as_u64().unwrap()
    );
}
#[test]
fn verified_reserved_action_is_never_used_outbound_but_can_remain_at_route_ending() {
    let (app, input, _) = setup(|c| {
        let e = &mut c["envelope"]["payload"]["edges"][2];
        e["mode"] = json!("return");
        e["status"] = json!("verified");
        e["evidence"] = json!(["demo-evidence-1"]);
        e["cooldown_ms"] = known(json!(0));
    });
    let r = run(&app, &input);
    assert_eq!(
        r["opportunities"][0]["detour"]["reason"],
        "unsafe_stateful_action"
    );
    assert!(r["opportunities"][0]["assessment"]["credited_reward"].is_null());
    let (app, mut input, _) = setup(|c| {
        let e = &mut c["envelope"]["payload"]["edges"][1];
        e["mode"] = json!("return");
        e["status"] = json!("verified");
        e["evidence"] = json!(["demo-evidence-1"]);
        e["cooldown_ms"] = known(json!(0));
    });
    input["opportunities"][0]["leg_index"] = json!(1);
    input["opportunities"][0]["node"]["id"] = json!("demo-node-2");
    let r = run(&app, &input);
    assert_eq!(r["opportunities"][0]["detour"]["status"], "complete");
    assert_eq!(r["opportunities"][0]["detour"]["incremental_ms"], 0);
}
