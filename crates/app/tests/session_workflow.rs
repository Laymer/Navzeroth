use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::SessionId;
use serde_json::{Value, json};
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
fn setup() -> (SnapshotApplication, String, String) {
    let app = SnapshotApplication::in_memory().unwrap();
    let receipt = app.import_text(&format!("ANAV1:{SNAPSHOT}"), None).unwrap();
    let snapshot: Value = serde_json::from_str(SNAPSHOT).unwrap();
    let session = format!(
        "ANAVSESSION1:{}",
        json!({"format":"anav-session-events","version":1,"fixture":true,"batch":{"character":snapshot["payload"]["character"],"context":snapshot["payload"]["context"],"events":[{"session":"manual-1","sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}}]}})
    );
    (app, receipt.profile_id, session)
}
#[test]
fn manual_import_history_detail_and_delete_preserve_snapshots() {
    let (app, id, text) = setup();
    assert_eq!(app.import_session_text(&id, &text).unwrap().inserted, 1);
    assert_eq!(app.import_session_text(&id, &text).unwrap().duplicates, 1);
    let rows = app.list_sessions(&id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].analytics.timing.covered_ms.get(), 0);
    let sid = SessionId::new("manual-1").unwrap();
    assert!(app.get_session_detail(&id, &sid).unwrap().fixture);
    assert!(app.delete_session(&id, &sid).unwrap());
    assert!(app.list_sessions(&id).unwrap().is_empty());
    assert!(app.current(&id, None).unwrap().is_some());
}
#[test]
fn invalid_import_or_selected_scope_never_changes_history() {
    let (app, id, text) = setup();
    assert!(app.import_session_text("unknown-profile", &text).is_err());
    assert!(
        app.import_session_text(&id, &text.replace("\"fixture\":true", "\"fixture\":false"))
            .is_err()
    );
    assert!(app.import_session_text(&id, "loadstring('x')").is_err());
    assert!(app.list_sessions(&id).unwrap().is_empty());
}
