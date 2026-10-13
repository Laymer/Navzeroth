mod common;
use azeroth_storage::{
    ImportSource, ProfileKey, SnapshotStore, SqliteStore,
    migrations::{MIGRATIONS, migrate_with},
};
#[test]
fn preferences_roundtrip_and_exact_profile_isolation() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let a = common::Doc::base().character("first").decode();
    let b = common::Doc::base().character("second").decode();
    for doc in [&a, &b] {
        store
            .import_character_snapshot(doc, ImportSource::Fixture, None)
            .unwrap();
    }
    let key = ProfileKey::of(&a);
    let mut state = store.reminder_state(&key).unwrap();
    state.enabled = false;
    store.save_reminder_state(&key, &state).unwrap();
    assert!(!store.reminder_state(&key).unwrap().enabled);
    assert!(store.reminder_state(&ProfileKey::of(&b)).unwrap().enabled);
    state.version = 2;
    assert!(store.save_reminder_state(&key, &state).is_err());
    assert!(!store.reminder_state(&key).unwrap().enabled);
}
#[test]
fn v5_upgrade_preserves_populated_session_history() {
    let db = common::TempDb::new("reminder-v5");
    let mut conn = common::raw(&db.path);
    migrate_with(&mut conn, &MIGRATIONS[..5]).unwrap();
    conn.execute("INSERT INTO profiles(character_id,edition,build,interface,namespace,is_fixture) VALUES ('kept','wow_forever','SYNTHETIC','SYNTHETIC','demo',1)",[]).unwrap();
    conn.execute("INSERT INTO sessions(profile_id,session_id,content_sha256,document,summary_document) VALUES (1,'kept',?1,'event-document','summary-document')",["a".repeat(64)]).unwrap();
    drop(conn);
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(
        store.schema_version().unwrap(),
        azeroth_storage::migrations::latest_version()
    );
    let conn = common::raw(&db.path);
    assert_eq!(
        conn.query_row(
            "SELECT document FROM sessions WHERE session_id='kept'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "event-document"
    );
    assert_eq!(common::count(&db.path, "SELECT COUNT(*) FROM profiles"), 1);
}
