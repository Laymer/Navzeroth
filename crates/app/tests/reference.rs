use azeroth_app::{reference::*, snapshots::SnapshotApplication};
use azeroth_core::{
    contract::Envelope,
    entities::CharacterSnapshot,
    observation::{ClockTrust, FreshnessContext},
    types::{Millis, UtcMillis},
};
use serde_json::{Value, json};
const NOW: i64 = 1791792000000;
fn clock() -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(NOW).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(1800000).unwrap(),
    }
}
fn snapshot() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap()
}
#[test]
fn imported_zero_unknown_restricted_and_distinct_field_age_remain_explicit() {
    let mut s = snapshot();
    s["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW);
    s["payload"]["level"]["observed_at_utc_ms"] = json!(NOW - 2000000);
    s["payload"]["level"]
        .as_object_mut()
        .unwrap()
        .remove("received_at_utc_ms");
    s["payload"]["location"] = json!({"status":"restricted"});
    s["x_secret"] = json!("private-secret-debug");
    let typed: Envelope<CharacterSnapshot> = serde_json::from_value(s).unwrap();
    let report = serde_json::to_value(build(Some(&typed), &clock()).unwrap()).unwrap();
    assert_eq!(report["snapshot"]["capture_freshness"]["age_ms"], 0);
    assert_eq!(report["snapshot"]["level_freshness"]["status"], "stale");
    assert_eq!(report["snapshot"]["capture_freshness"]["status"], "fresh");
    assert_eq!(report["snapshot"]["role"]["status"], "unknown");
    assert_eq!(report["snapshot"]["location"]["status"], "restricted");
    assert!(!report.to_string().contains("private-secret"));
    assert!(report.to_string().len() < MAX_REFERENCE_BYTES);
}
#[test]
fn actual_selected_snapshot_changes_and_missing_clock_remains_uncertain() {
    let app = SnapshotApplication::in_memory().unwrap();
    assert!(
        app.reference_view(Some(UtcMillis::new(NOW).unwrap()))
            .unwrap()
            .snapshot
            .is_none()
    );
    let first = app
        .import_text(
            &format!("ANAV1:{}", snapshot()),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap();
    let mut second = snapshot();
    second["payload"]["character"] = json!("second-synthetic-character");
    second["payload"]["id"] = json!("second-synthetic-snapshot");
    let second = app
        .import_text(
            &format!("ANAV1:{second}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap();
    app.select_profile(&second.profile_id).unwrap();
    assert_eq!(
        app.reference_view(None)
            .unwrap()
            .snapshot
            .unwrap()
            .character
            .as_str(),
        "second-synthetic-character"
    );
    app.select_profile(&first.profile_id).unwrap();
    let out = serde_json::to_value(app.reference_view(None).unwrap()).unwrap();
    assert_eq!(out["snapshot"]["character"], "demo-character-paladin");
    assert_eq!(out["snapshot"]["capture_freshness"]["status"], "uncertain");
}
#[test]
fn native_caller_policy_allows_main_and_exact_reference_read_only() {
    for command in [
        "get_reference_view",
        "set_reference_window",
        "get_reference_window_state",
        "import_snapshot_file",
        "unknown_command",
    ] {
        assert!(permits_command("main", command));
        assert_eq!(
            permits_command("reference", command),
            command == "get_reference_view"
        );
        assert!(!permits_command("arbitrary-webview", command));
    }
    assert!(permits_management("main"));
    assert!(!permits_management("reference"));
    assert!(!permits_management("arbitrary-webview"));
}
