mod common;
use azeroth_core::{sessions::SessionDocument, types::SessionId};
use azeroth_storage::{ImportSource, ProfileKey, SnapshotStore, SqliteStore};
fn setup() -> (SqliteStore, ProfileKey, SessionDocument) {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let envelope = common::Doc::base().decode();
    let key = ProfileKey::of(&envelope);
    store
        .import_character_snapshot(&envelope, ImportSource::Fixture, None)
        .unwrap();
    let document=serde_json::from_value(serde_json::json!({"format":"anav-session-events","version":1,"fixture":key.fixture,"batch":{"character":key.character,"context":key.context,"events":[{"session":"s1","sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}},{"session":"s1","sequence":2,"elapsed_ms":5000,"kind":{"type":"activity","label":"sample","quality":"unclassified"}}]}})).unwrap();
    (store, key, document)
}
#[test]
fn duplicate_canonical_events_and_conflicting_identity_are_atomic() {
    let (mut store, key, mut doc) = setup();
    assert_eq!(store.import_sessions(&key, &doc).unwrap().inserted, 1);
    doc.batch.events.reverse();
    doc.batch.events.push(doc.batch.events[0].clone());
    assert_eq!(store.import_sessions(&key, &doc).unwrap().duplicates, 1);
    let before =
        serde_json::to_value(store.session(&key, &SessionId::new("s1").unwrap()).unwrap()).unwrap();
    doc.batch.events.truncate(2);
    doc.batch.events[0].elapsed_ms = azeroth_core::types::Millis::new(6000).unwrap();
    let mut fresh = doc.batch.events[1].clone();
    fresh.session = SessionId::new("fresh").unwrap();
    doc.batch.events.push(fresh);
    assert!(store.import_sessions(&key, &doc).is_err());
    assert_eq!(store.session_ids(&key).unwrap().len(), 1);
    assert_eq!(
        serde_json::to_value(store.session(&key, &SessionId::new("s1").unwrap()).unwrap()).unwrap(),
        before
    );
}
#[test]
fn exact_context_and_fixture_scope() {
    let (mut store, key, mut doc) = setup();
    doc.fixture = !doc.fixture;
    assert!(store.import_sessions(&key, &doc).is_err());
    doc.fixture = key.fixture;
    doc.batch.context.build.build = azeroth_core::types::BuildId::new("OTHER").unwrap();
    assert!(store.import_sessions(&key, &doc).is_err());
    assert!(store.session_ids(&key).unwrap().is_empty());
}
#[test]
fn capacity_rejects_without_pruning_then_explicit_delete() {
    let (mut store, key, mut doc) = setup();
    for i in 0..128 {
        for event in &mut doc.batch.events {
            event.session = SessionId::new(format!("session-{i:03}")).unwrap();
        }
        store.import_sessions(&key, &doc).unwrap();
    }
    assert_eq!(store.session_ids(&key).unwrap().len(), 128);
    assert_eq!(store.import_sessions(&key, &doc).unwrap().duplicates, 1);
    for event in &mut doc.batch.events {
        event.session = SessionId::new("overflow").unwrap();
    }
    assert!(store.import_sessions(&key, &doc).is_err());
    assert_eq!(store.session_ids(&key).unwrap().len(), 128);
    assert!(
        store
            .delete_session(&key, &SessionId::new("session-000").unwrap())
            .unwrap()
    );
    assert_eq!(store.import_sessions(&key, &doc).unwrap().inserted, 1);
    assert_eq!(store.session_ids(&key).unwrap().len(), 128);
}
#[test]
fn summaries_preserve_unclassified_zero_unknown_and_uncertain_clock() {
    let (mut store, key, doc) = setup();
    store.import_sessions(&key, &doc).unwrap();
    let detail = store
        .session(&key, &SessionId::new("s1").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(detail.analytics.timing.covered_ms.get(), 5000);
    assert_eq!(detail.analytics.timing.totals.unclassified_ms.get(), 5000);
    assert_eq!(detail.analytics.progression.observed_level_change, None);
    assert!(matches!(
        detail.analytics.utc,
        azeroth_core::sessions::SessionUtc::Uncertain { .. }
    ));
}
#[test]
fn populated_v4_upgrade_preserves_other_domains() {
    let db = common::TempDb::new("sessions-v4");
    let envelope = common::Doc::base().decode();
    let key = ProfileKey::of(&envelope);
    let content: azeroth_storage::content::ContentRevision = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    {
        let mut store = SqliteStore::open(&db.path).unwrap();
        store
            .import_character_snapshot(&envelope, ImportSource::Fixture, None)
            .unwrap();
        store
            .set_preference(
                &key,
                &azeroth_core::types::PreferenceId::new("keep-false").unwrap(),
                &azeroth_core::observation::Observation::known(
                    false,
                    envelope.payload.capture.clone(),
                )
                .unwrap(),
            )
            .unwrap();
        store.stage_content_revision(&content).unwrap();
        store.activate_content_revision(&content.key()).unwrap();
        store
            .save_party(&azeroth_storage::party::PartyProfile {
                id: azeroth_core::types::PartyId::new("kept").unwrap(),
                participants: vec![key.clone()],
            })
            .unwrap();
    }
    // Remove only the new empty table and migration marker to reconstruct the
    // exact previous schema with real populated domain records.
    let conn = common::raw(&db.path);
    conn.execute_batch("DROP TABLE session_calibration_annotations; DROP TABLE reminder_state; DROP TABLE sessions; DELETE FROM schema_migrations WHERE version>=5")
        .unwrap();
    let before = common::dump_state(&db.path);
    drop(conn);
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(
        store.schema_version().unwrap(),
        azeroth_storage::migrations::latest_version()
    );
    assert_eq!(common::dump_state(&db.path), before);
    assert_eq!(store.party_ids().unwrap()[0].as_str(), "kept");
    assert_eq!(
        store
            .preferences(&key)
            .unwrap()
            .values()
            .next()
            .unwrap()
            .value(),
        Some(&false)
    );
    assert_eq!(
        store
            .active_content_revision(&content.key().scope)
            .unwrap()
            .unwrap(),
        content
    );
    assert_eq!(store.current_snapshot(&key).unwrap().unwrap(), envelope);
}
#[test]
fn same_session_identity_is_isolated_between_complete_profile_keys() {
    let (mut store, key, doc) = setup();
    store.import_sessions(&key, &doc).unwrap();
    let other_envelope = common::Doc::base().build("OTHER").decode();
    store
        .import_character_snapshot(&other_envelope, ImportSource::Fixture, None)
        .unwrap();
    let other_key = ProfileKey::of(&other_envelope);
    assert!(store.session_ids(&other_key).unwrap().is_empty());
    let mut other_doc = doc.clone();
    other_doc.batch.context = other_key.context.clone();
    other_doc.batch.events[1].elapsed_ms = azeroth_core::types::Millis::new(1000).unwrap();
    store.import_sessions(&other_key, &other_doc).unwrap();
    let id = SessionId::new("s1").unwrap();
    assert_eq!(
        store
            .session(&key, &id)
            .unwrap()
            .unwrap()
            .analytics
            .timing
            .covered_ms
            .get(),
        5000
    );
    assert_eq!(
        store
            .session(&other_key, &id)
            .unwrap()
            .unwrap()
            .analytics
            .timing
            .covered_ms
            .get(),
        1000
    );
    store.delete_session(&other_key, &id).unwrap();
    assert_eq!(store.session_ids(&key).unwrap().len(), 1);
}
#[test]
fn direct_storage_rejects_fixture_observation_source_disagreement() {
    let (mut store, key, mut doc) = setup();
    doc.batch.events[1].kind=serde_json::from_value(serde_json::json!({"type":"level","level":{"status":"known","value":12,"source":"addon_snapshot"}})).unwrap();
    assert!(store.import_sessions(&key, &doc).is_err());
    assert!(store.session_ids(&key).unwrap().is_empty());
}
