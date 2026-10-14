use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{error::ErrorCode, types::DatasetRevision};
use azeroth_storage::content_dispositions::DispositionState;
use serde_json::{Value, json};
fn app() -> (SnapshotApplication, String) {
    let app = SnapshotApplication::in_memory().unwrap();
    let raw = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let profile = app
        .import_text(&format!("ANAV1:{raw}"), None)
        .unwrap()
        .profile_id;
    app.stage_content(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    (app, profile)
}
fn request(app: &SnapshotApplication, p: &str, state: &str) -> Value {
    let current = app.content_governance(p).unwrap();
    json!({"schema_version":1,"profile_id":p,"mutation":{"guard":current.revisions[0].guard(),"state":state,"reason_code":"operator_review"}})
}
#[test]
fn explicit_profile_decision_clears_active_then_release_never_autoactivates() {
    let (app, p) = app();
    let initial = app.content_governance(&p).unwrap();
    let inspection = app.active_content(&p).unwrap();
    let profiles = app.profiles().unwrap();
    let q = request(&app, &p, "quarantined");
    let outcome = app
        .change_content_disposition(&q.to_string(), None)
        .unwrap();
    let quarantined = app.content_governance(&p).unwrap();
    assert!(outcome.outcome.active.is_none());
    assert!(app.active_content(&p).unwrap().is_none());
    assert!(
        app.activate_content(&p, &DatasetRevision::new("demo-r1").unwrap())
            .is_err()
    );
    assert!(
        app.change_content_disposition(&q.to_string(), None)
            .is_err()
    );
    let release = request(&app, &p, "eligible");
    let released_decision = app
        .change_content_disposition(&release.to_string(), None)
        .unwrap();
    let flow = json!({"profiles":profiles,"initial":initial,"inspection":inspection,"quarantine_request":q,"quarantine_decision":outcome,"quarantined":quarantined,"release_request":release,"release_decision":released_decision,"released":app.content_governance(&p).unwrap()});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/desktop/content-governance/synthetic-flow.json");
    if std::env::var_os("UPDATE_CONTENT_DISPOSITION_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&flow).unwrap() + "\n").unwrap();
    }
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(flow, expected);
    assert!(app.active_content(&p).unwrap().is_none());
    let current = app.content_governance(&p).unwrap();
    assert_eq!(current.revisions[0].state, DispositionState::Eligible);
    assert_eq!(current.revisions[0].state_revision, 3);
    app.activate_content(&p, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    assert!(app.active_content(&p).unwrap().is_some());
}
#[test]
fn malformed_future_or_cross_context_requests_leave_state_unchanged() {
    let (app, p) = app();
    let good = request(&app, &p, "quarantined");
    for field in ["schema", "scope", "extra", "reason", "counter"] {
        let mut bad = good.clone();
        match field {
            "schema" => bad["schema_version"] = json!(2),
            "scope" => {
                bad["mutation"]["guard"]["key"]["scope"]["context"]["build"]["build"] =
                    json!("foreign-synthetic")
            }
            "extra" => bad["unrelated"] = json!("private"),
            "reason" => bad["mutation"]["reason_code"] = json!("private_free_text"),
            "counter" => bad["mutation"]["guard"]["state_revision"] = json!(9007199254740992u64),
            _ => unreachable!(),
        };
        assert!(
            app.change_content_disposition(&bad.to_string(), None)
                .is_err(),
            "{field}"
        );
        assert!(app.active_content(&p).unwrap().is_some());
    }
    assert_eq!(
        app.change_content_disposition(&"x".repeat(8193), None)
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
}
