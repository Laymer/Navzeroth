use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{
    error::ErrorCode,
    types::{DatasetRevision, UtcMillis},
};
use serde_json::{Value, json};
const BUNDLE: &str = include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
fn setup() -> (SnapshotApplication, Value, UtcMillis) {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let now = UtcMillis::new(
        snapshot["payload"]["capture"]["observed_at_utc_ms"]
            .as_i64()
            .unwrap()
            + 100,
    )
    .unwrap();
    let id = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    app.stage_content(BUNDLE).unwrap();
    app.activate_content(&id, &DatasetRevision::new("synthetic-planning-v1").unwrap())
        .unwrap();
    let catalog = app.planning_catalog(&id).unwrap();
    let request = json!({"profile_id":id,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
    (app, request, now)
}
#[test]
fn planning_is_deterministic_bounded_and_includes_stopping_costs() {
    let (app, mut request, now) = setup();
    let result = app.plan_session(&request.to_string(), Some(now)).unwrap();
    assert_eq!(result.result.selected.len(), 1);
    assert_eq!(result.result.expected_total_ms.get(), 61000);
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        serde_json::to_value(app.plan_session(&request.to_string(), Some(now)).unwrap()).unwrap()
    );
    request["budget_ms"] = json!(60000);
    assert!(
        app.plan_session(&request.to_string(), Some(now))
            .unwrap()
            .result
            .selected
            .is_empty()
    );
    request["budget_ms"] = json!(100000);
    request["excluded"] = json!(["synthetic-service"]);
    assert!(
        app.plan_session(&request.to_string(), Some(now))
            .unwrap()
            .result
            .selected
            .is_empty()
    );
    request["excluded"] = json!([]);
    request["candidates"] = json!([request["candidates"][0], request["candidates"][0]]);
    assert!(app.plan_session(&request.to_string(), Some(now)).is_err());
}
#[test]
fn changed_stale_or_invalid_inputs_are_rejected() {
    let (app, request, now) = setup();
    for (field, value) in [
        ("snapshot_id", json!("replaced")),
        ("revision", json!("replaced")),
        ("origin", json!("absent")),
        ("goal_profile", json!("")),
        ("budget_ms", json!(-1)),
        ("budget_ms", json!(1.5)),
        ("excluded", json!(["absent"])),
    ] {
        let mut bad = request.clone();
        bad[field] = value;
        assert!(
            app.plan_session(&bad.to_string(), Some(now)).is_err(),
            "{field}"
        );
    }
    assert_eq!(
        app.plan_session(&request.to_string(), None)
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    assert!(
        app.plan_session(
            &request.to_string(),
            Some(UtcMillis::new(now.get() + 1800001).unwrap())
        )
        .is_err()
    );
    assert!(
        app.plan_session(&" ".repeat(1024 * 1024 + 1), Some(now))
            .is_err()
    );
    let mut changed: Value = serde_json::from_str(BUNDLE).unwrap();
    changed["envelope"]["payload"]["revision"] = json!("new-revision");
    app.stage_content(&changed.to_string()).unwrap();
    app.activate_content(
        request["profile_id"].as_str().unwrap(),
        &DatasetRevision::new("new-revision").unwrap(),
    )
    .unwrap();
    assert!(app.plan_session(&request.to_string(), Some(now)).is_err());
}

#[test]
fn export_recomputes_with_stable_identity_and_snapshot_bounded_expiry() {
    let (app, mut request, now) = setup();
    let first = app.export_plan(&request.to_string(), Some(now)).unwrap();
    assert_eq!(first.byte_count, first.text.len());
    assert!(first.text.starts_with("ANAV1:{"));
    assert!(first.byte_count <= 32768);
    let again = app.export_plan(&request.to_string(), Some(now)).unwrap();
    assert_eq!(first.text, again.text);
    let later = app
        .export_plan(
            &request.to_string(),
            Some(UtcMillis::new(now.get() + 1000).unwrap()),
        )
        .unwrap();
    assert_eq!(first.plan_id, later.plan_id);
    assert_eq!(first.expires_at_utc_ms, later.expires_at_utc_ms);
    assert_eq!(first.expires_at_utc_ms.get(), now.get() - 100 + 1800000);
    assert_ne!(first.text, later.text);
    let value: Value = serde_json::from_str(&first.text[6..]).unwrap();
    assert_eq!(value["payload"]["travel"].as_array().unwrap().len(), 2);
    assert_eq!(
        value["dependencies"]["snapshots"],
        value["payload"]["inputs"]
    );
    assert_eq!(
        value["payload"]["steps"][0]["invalidation"]["value"]["op"],
        "not"
    );
    request["excluded"] = json!(["synthetic-service"]);
    assert!(app.export_plan(&request.to_string(), Some(now)).is_err());
    request["excluded"] = json!([]);
    request["snapshot_id"] = json!("replaced");
    assert!(app.export_plan(&request.to_string(), Some(now)).is_err());
    assert!(app.export_plan(&request.to_string(), None).is_err());
}

#[test]
fn export_enforces_compiled_predicate_depth_and_size_without_truncation() {
    for depth in [9, 10] {
        let (app, mut request, now) = setup();
        let mut content: Value = serde_json::from_str(BUNDLE).unwrap();
        content["envelope"]["payload"]["revision"] = json!("deep");
        let mut predicate = json!({"op":"level_at_least","level":1});
        for _ in 0..depth {
            predicate = json!({"op":"all","predicates":[predicate]});
        }
        content["envelope"]["payload"]["objectives"][0]["requirements"] =
            json!({"status":"known","value":predicate});
        app.stage_content(&content.to_string()).unwrap();
        app.activate_content(
            request["profile_id"].as_str().unwrap(),
            &DatasetRevision::new("deep").unwrap(),
        )
        .unwrap();
        request["revision"] = json!("deep");
        assert_eq!(
            app.export_plan(&request.to_string(), Some(now)).is_ok(),
            depth == 9
        );
    }
    let (app, mut request, now) = setup();
    let mut content: Value = serde_json::from_str(BUNDLE).unwrap();
    content["envelope"]["payload"]["revision"] = json!("large");
    content["envelope"]["payload"]["objectives"][0]["requirements"] = json!({"status":"known","value":{"op":"all","predicates":vec![json!({"op":"not","predicate":{"op":"any","predicates":[]}});270]}});
    app.stage_content(&content.to_string()).unwrap();
    app.activate_content(
        request["profile_id"].as_str().unwrap(),
        &DatasetRevision::new("large").unwrap(),
    )
    .unwrap();
    request["revision"] = json!("large");
    let err = app
        .export_plan(&request.to_string(), Some(now))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::ImportTooLarge, "{err:?}");
}

#[test]
fn quest_stage_planning_and_export_preserve_unknown_false_and_completed_states() {
    for status in [
        "pending",
        "completed",
        "unknown",
        "restricted",
        "unaccepted",
        "joint",
    ] {
        let app = SnapshotApplication::in_memory().unwrap();
        let mut snapshot: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v1/character-snapshot.json"
        ))
        .unwrap();
        let now = UtcMillis::new(1791791990100).unwrap();
        let observed = |value| json!({"status":"known","value":value,"observed_at_utc_ms":1791791990000i64,"clock_trust":"trusted","source":"synthetic_fixture"});
        let quest = &mut snapshot["payload"]["quests"]["demo-quest-1"];
        quest["accepted"] = observed(status != "unaccepted");
        quest["completed"] = observed(false);
        quest["turned_in"] = observed(false);
        quest["objectives"]["demo-objective-1"] = match status {
            "unknown" | "restricted" => json!({"status":status}),
            _ => observed(status == "completed"),
        };
        snapshot["payload"]["objectives"]["demo-objective-1"] =
            snapshot["payload"]["quests"]["demo-quest-1"]["objectives"]["demo-objective-1"].clone();
        let profile = app
            .import_text(&format!("ANAV1:{snapshot}"), Some(now))
            .unwrap()
            .profile_id;
        let mut bundle: Value = serde_json::from_str(BUNDLE).unwrap();
        let original: Value = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        bundle["envelope"]["payload"]["quests"] = original["envelope"]["payload"]["quests"].clone();
        bundle["envelope"]["payload"]["quests"][0]["rewards"] = json!({"status":"unknown"});
        bundle["envelope"]["payload"]["objectives"][0]["targets"] =
            json!([{"kind":"quest_stage","quest":"demo-quest-1","stage":"demo-objective-1"}]);
        if status == "joint" {
            bundle["envelope"]["payload"]["objectives"][0]["participants"] =
                json!(["demo-character-paladin", "other"]);
        }
        app.stage_content(&bundle.to_string()).unwrap();
        app.activate_content(
            &profile,
            &DatasetRevision::new("synthetic-planning-v1").unwrap(),
        )
        .unwrap();
        let request = json!({"profile_id":profile,"snapshot_id":snapshot["payload"]["id"],"revision":"synthetic-planning-v1","goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
        let report = app.plan_session(&request.to_string(), Some(now)).unwrap();
        assert_eq!(
            report.result.selected.len(),
            usize::from(status == "pending"),
            "{status}"
        );
        assert_eq!(
            app.export_plan(&request.to_string(), Some(now)).is_ok(),
            status == "pending",
            "{status}"
        );
        if status != "pending" {
            assert_eq!(report.filtered.len(), 1);
        }
    }
}
