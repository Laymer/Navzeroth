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

#[test]
fn selected_text_and_inert_saved_variables_share_idempotent_session_history() {
    use azeroth_app::snapshots::FileInputFormat;
    let (app, id, text) = setup();
    let root = std::env::temp_dir().join(format!(
        "nav-session-files-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(root.clone());
    let plain = root.join("synthetic-session.txt");
    let lua = root.join("synthetic-session.lua");
    std::fs::write(&plain, &text).unwrap();
    let lua_source = format!(
        "SyntheticDB = {{ session_export = {} }}",
        serde_json::to_string(&text).unwrap()
    );
    std::fs::write(&lua, &lua_source).unwrap();
    let snapshot = app.current(&id, None).unwrap().unwrap();
    let first = app
        .import_session_file(&id, &plain, FileInputFormat::Anav1Text)
        .unwrap();
    assert_eq!((first.inserted, first.duplicates), (1, 0));
    let format = FileInputFormat::SavedVariables {
        global: "SyntheticDB".into(),
        key: "session_export".into(),
    };
    let again = app.import_session_file(&id, &lua, format.clone()).unwrap();
    assert_eq!((again.inserted, again.duplicates), (0, 1));
    assert_eq!(again.session_ids, first.session_ids);
    assert_eq!(app.list_sessions(&id).unwrap().len(), 1);
    assert_eq!(
        app.list_sessions(&id).unwrap()[0]
            .analytics
            .timing
            .covered_ms
            .get(),
        0
    );
    assert_eq!(
        app.current(&id, None).unwrap().unwrap().envelope.payload.id,
        snapshot.envelope.payload.id
    );
    assert_eq!(std::fs::read_to_string(&lua).unwrap(), lua_source);
    for (path, format) in [
        (
            lua.clone(),
            FileInputFormat::SavedVariables {
                global: "SyntheticDB".into(),
                key: "snapshot_export".into(),
            },
        ),
        (
            lua.clone(),
            FileInputFormat::SavedVariables {
                global: "OtherDB".into(),
                key: "session_export".into(),
            },
        ),
        (root.join("missing"), FileInputFormat::Anav1Text),
    ] {
        assert!(app.import_session_file(&id, &path, format).is_err());
        assert_eq!(app.list_sessions(&id).unwrap().len(), 1);
    }
    std::fs::write(&lua, "SyntheticDB = loadstring('must remain inert')()").unwrap();
    assert!(app.import_session_file(&id, &lua, format).is_err());
    assert_eq!(app.list_sessions(&id).unwrap().len(), 1);
    let sid = SessionId::new("absent-session").unwrap();
    assert!(app.get_session_detail(&id, &sid).is_err());
    assert!(!app.delete_session(&id, &sid).unwrap());
}
