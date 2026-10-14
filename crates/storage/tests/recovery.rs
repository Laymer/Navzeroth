mod common;
use azeroth_storage::{
    ImportSource, ProfileKey, SnapshotStore, SqliteStore,
    migrations::{MIGRATIONS, migrate_with},
    recovery::reconstruct_backup,
};
use common::{Doc, TempDb, raw};
#[test]
fn all_supported_historical_schemas_reconstruct_populated_profiles_without_source_writes() {
    for version in 1..=MIGRATIONS.len() {
        let source = TempDb::new(&format!("recover-v{version}"));
        let restored = TempDb::new(&format!("restored-v{version}"));
        let mut conn = raw(&source.path);
        migrate_with(&mut conn, &MIGRATIONS[..version]).unwrap();
        conn.execute("INSERT INTO profiles(character_id,edition,build,interface,namespace,is_fixture) VALUES('kept','wow_forever','SYNTHETIC','SYNTHETIC','demo',1)",[]).unwrap();
        drop(conn);
        let before = std::fs::read(&source.path).unwrap();
        let report = reconstruct_backup(&source.path, &restored.path).unwrap();
        assert_eq!(report.source_schema_version, version as u32);
        assert_eq!(
            report.restored_schema_version,
            azeroth_storage::migrations::latest_version()
        );
        let store = SqliteStore::open(&restored.path).unwrap();
        assert_eq!(
            store.list_profiles().unwrap()[0].key.character.as_str(),
            "kept"
        );
        assert_eq!(std::fs::read(&source.path).unwrap(), before);
    }
}
#[test]
fn current_snapshots_preferences_and_reminder_false_values_survive() {
    let source = TempDb::new("recovery-state");
    let target = TempDb::new("recovery-target");
    let envelope = Doc::base().decode();
    let key = ProfileKey::of(&envelope);
    let mut store = SqliteStore::open(&source.path).unwrap();
    store
        .import_character_snapshot(&envelope, ImportSource::Fixture, None)
        .unwrap();
    let preference =
        azeroth_core::observation::Observation::known(false, envelope.payload.capture.clone())
            .unwrap();
    store
        .set_preference(
            &key,
            &azeroth_core::types::PreferenceId::new("manual").unwrap(),
            &preference,
        )
        .unwrap();
    let state = azeroth_storage::reminders::StoredReminderState {
        enabled: false,
        ..Default::default()
    };
    store.save_reminder_state(&key, &state).unwrap();
    drop(store);
    reconstruct_backup(&source.path, &target.path).unwrap();
    let store = SqliteStore::open(&target.path).unwrap();
    assert_eq!(store.current_snapshot(&key).unwrap().unwrap(), envelope);
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
    assert!(!store.reminder_state(&key).unwrap().enabled);
}
#[test]
fn malicious_schema_future_history_and_corrupt_data_refuse_without_destination() {
    for (name, sql) in [
        ("sqliteXtable", "CREATE TABLE sqliteXevil(x TEXT)"),
        (
            "sqliteXtrigger",
            "CREATE TRIGGER sqliteXevil AFTER INSERT ON profiles BEGIN SELECT load_extension('malicious'); END",
        ),
        (
            "sqliteXview",
            "CREATE VIEW sqliteXevil AS SELECT load_extension('malicious')",
        ),
        (
            "trigger",
            "CREATE TRIGGER malicious AFTER INSERT ON profiles BEGIN SELECT load_extension('malicious'); END",
        ),
        (
            "view",
            "CREATE VIEW malicious AS SELECT load_extension('malicious')",
        ),
        (
            "future",
            "INSERT INTO schema_migrations(version,name) SELECT max(version)+1,'future' FROM schema_migrations",
        ),
        (
            "history",
            "UPDATE schema_migrations SET name='false' WHERE version=1",
        ),
        (
            "hash",
            "UPDATE snapshots SET content_sha256=printf('%064d',0)",
        ),
        (
            "derived",
            "UPDATE snapshots SET producer_version='different'",
        ),
        ("observation", "DELETE FROM snapshot_observations"),
        ("context", "UPDATE profiles SET build='other'"),
        ("document", "UPDATE snapshots SET document='{}'"),
        (
            "audit-code",
            "UPDATE imports SET status='failed',error_code='INVALID_NEW_CODE'",
        ),
        (
            "audit-message",
            "UPDATE imports SET message=printf('%2048d',0)",
        ),
    ] {
        let source = TempDb::new(name);
        let target = TempDb::new(&format!("refused-{name}"));
        let mut store = SqliteStore::open(&source.path).unwrap();
        store
            .import_character_snapshot(&Doc::base().decode(), ImportSource::Fixture, None)
            .unwrap();
        drop(store);
        raw(&source.path).execute_batch(sql).unwrap();
        let before = std::fs::read(&source.path).unwrap();
        assert!(
            reconstruct_backup(&source.path, &target.path).is_err(),
            "{name}"
        );
        assert!(!target.path.exists(), "{name}");
        assert_eq!(std::fs::read(&source.path).unwrap(), before);
    }
}
#[test]
fn source_with_committed_wal_pages_is_read_consistently() {
    let source = TempDb::new("recovery-wal");
    let target = TempDb::new("recovery-wal-target");
    let mut store = SqliteStore::open(&source.path).unwrap();
    let writer = raw(&source.path);
    writer.execute_batch("PRAGMA journal_mode=WAL").unwrap();
    let envelope = Doc::base().decode();
    store
        .import_character_snapshot(&envelope, ImportSource::Fixture, None)
        .unwrap();
    reconstruct_backup(&source.path, &target.path).unwrap();
    assert!(
        SqliteStore::open(&target.path)
            .unwrap()
            .current_snapshot(&ProfileKey::of(&envelope))
            .unwrap()
            .is_some()
    );
}
#[test]
fn oversized_and_non_sqlite_files_refuse_without_target() {
    let source = TempDb::new("large-backup");
    let target = TempDb::new("large-target");
    let file = std::fs::File::create(&source.path).unwrap();
    file.set_len(azeroth_storage::recovery::MAX_BACKUP_BYTES + 1)
        .unwrap();
    assert!(reconstruct_backup(&source.path, &target.path).is_err());
    assert!(!target.path.exists());
    std::fs::write(&source.path, b"not SQLite").unwrap();
    assert!(reconstruct_backup(&source.path, &target.path).is_err());
    assert!(!target.path.exists());
}
#[test]
fn content_sessions_annotations_and_party_proposals_survive_and_tampered_metadata_refuses() {
    use azeroth_core::{
        calibration::{CalibrationConditions, SessionCalibrationAnnotation},
        sessions::SessionDocument,
        types::{Millis, PartyId, SessionId, Sha256Hex},
    };
    use azeroth_storage::{
        content::ContentRevision,
        party::PartyProfile,
        party_proposals::{PartyProposal, ProposalGuard},
    };
    let source = TempDb::new("all-feature-backup");
    let target = TempDb::new("all-feature-restored");
    let envelope = Doc::base().decode();
    let second = Doc::base()
        .character("two")
        .snapshot_id("two-snapshot")
        .decode();
    let key = ProfileKey::of(&envelope);
    let secondkey = ProfileKey::of(&second);
    let mut store = SqliteStore::open(&source.path).unwrap();
    store
        .import_character_snapshot(&envelope, ImportSource::Fixture, None)
        .unwrap();
    store
        .import_character_snapshot(&second, ImportSource::Fixture, None)
        .unwrap();
    let revision: ContentRevision = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    store.stage_content_revision(&revision).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    let doc:SessionDocument=serde_json::from_value(serde_json::json!({"format":"anav-session-events","version":1,"fixture":true,"batch":{"character":key.character,"context":key.context,"events":[{"session":"s1","sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}},{"session":"s1","sequence":2,"elapsed_ms":5000,"kind":{"type":"activity","label":"sample","quality":"unclassified"}}]}})).unwrap();
    store.import_sessions(&key, &doc).unwrap();
    let annotation = SessionCalibrationAnnotation {
        conditions: CalibrationConditions {
            route_stage: "manual".into(),
            level: 22,
            role: "manual".into(),
            party_size: 2,
        },
        complete: false,
        completion_quest: "quest".into(),
    };
    store
        .set_session_calibration(&key, &SessionId::new("s1").unwrap(), &annotation)
        .unwrap();
    let party = PartyId::new("duo").unwrap();
    store
        .save_party(&PartyProfile {
            id: party.clone(),
            participants: vec![key.clone(), secondkey],
        })
        .unwrap();
    let proposal = PartyProposal {
        schema_version: 1,
        party_id: party.clone(),
        guard: ProposalGuard {
            proposal_id: Sha256Hex::new("a".repeat(64)).unwrap(),
            revision: 1,
        },
        proposer_profile_id: "one".into(),
        request: "{}".into(),
        result_hash: Sha256Hex::new("b".repeat(64)).unwrap(),
        selected_objectives: vec![azeroth_core::types::ObjectiveId::new("selected").unwrap()],
        expected_total_ms: Millis::new(0).unwrap(),
        context: key.context.clone(),
        fixture: true,
        created_at_utc_ms: None,
        acceptances: Default::default(),
    };
    store.save_party_proposal(&proposal, None).unwrap();
    drop(store);
    reconstruct_backup(&source.path, &target.path).unwrap();
    let restored = SqliteStore::open(&target.path).unwrap();
    assert_eq!(
        restored
            .session_calibration(&key, &SessionId::new("s1").unwrap())
            .unwrap(),
        Some(annotation)
    );
    assert!(
        restored
            .session(&key, &SessionId::new("s1").unwrap())
            .unwrap()
            .is_some()
    );
    assert_eq!(
        restored.party_proposal(&party).unwrap().unwrap().guard,
        proposal.guard
    );
    assert_eq!(
        restored
            .active_content_revision(&revision.key().scope)
            .unwrap()
            .unwrap()
            .envelope,
        revision.envelope
    );
    drop(restored);
    for (name, sql) in [
        (
            "content-pointer",
            "UPDATE active_datasets SET build='other'",
        ),
        ("source-id", "UPDATE source_records SET source_id='another'"),
        (
            "session-hash",
            "UPDATE sessions SET content_sha256=printf('%064d',0)",
        ),
        (
            "session-summary",
            "UPDATE sessions SET summary_document='{}'",
        ),
        (
            "annotation",
            "UPDATE session_calibration_annotations SET document='{}'",
        ),
        (
            "party-pointer",
            "UPDATE party_participants SET profile_key='{}' WHERE position=0",
        ),
    ] {
        let corrupted = TempDb::new(name);
        std::fs::copy(&source.path, &corrupted.path).unwrap();
        let conn = raw(&corrupted.path);
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        if name == "source-id" {
            let trigger: String = conn
                .query_row(
                    "SELECT sql FROM sqlite_schema WHERE name='source_records_no_update'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            conn.execute_batch("DROP TRIGGER source_records_no_update")
                .unwrap();
            conn.execute_batch(sql).unwrap();
            conn.execute_batch(&trigger).unwrap();
        } else {
            conn.execute_batch(sql).unwrap();
        }
        drop(conn);
        let refused = TempDb::new(&format!("refused-{name}"));
        assert!(
            reconstruct_backup(&corrupted.path, &refused.path).is_err(),
            "{name}"
        );
        assert!(!refused.path.exists());
    }
}
