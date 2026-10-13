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
