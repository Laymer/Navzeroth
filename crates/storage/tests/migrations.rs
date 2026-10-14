//! Migration behavior: fresh, reopened, upgraded, refused and failed runs.
mod common;

use azeroth_core::error::ErrorCode;
use azeroth_storage::{
    SqliteStore,
    migrations::{MIGRATIONS, Migration, current_version, latest_version, migrate, migrate_with},
};
use common::{TempDb, count, raw};
use rusqlite::Connection;

const ADD_NOTE: Migration = Migration {
    version: MIGRATIONS.len() as u32 + 1,
    name: "test: profile note",
    sql: "ALTER TABLE profiles ADD COLUMN note TEXT;",
};
const BROKEN: Migration = Migration {
    version: MIGRATIONS.len() as u32 + 1,
    name: "test: fails partway",
    sql: "CREATE TABLE half_done (x INTEGER);
          INSERT INTO profiles (character_id) VALUES ('violates not null');",
};
fn with_next(next: Migration) -> Vec<Migration> {
    let mut list = MIGRATIONS.to_vec();
    list.push(next);
    list
}
fn seed_profile(path: &std::path::Path) {
    raw(path)
        .execute(
            "INSERT INTO profiles (character_id, edition, build, interface, namespace, is_fixture)
             VALUES ('kept', 'wow_forever', 'SYNTHETIC', 'SYNTHETIC', 'demo', 1)",
            [],
        )
        .unwrap();
}

#[test]
fn fresh_in_memory_database_migrates_to_latest() {
    let store = SqliteStore::open_in_memory().unwrap();
    assert_eq!(store.schema_version().unwrap(), latest_version());
    assert_eq!(latest_version(), 9);
}

#[test]
fn version_two_upgrade_preserves_snapshots_preferences_and_hashes() {
    use azeroth_storage::{ImportSource, ProfileKey, SnapshotStore};
    let db = TempDb::new("party-v2-upgrade");
    let mut conn = raw(&db.path);
    migrate_with(&mut conn, &MIGRATIONS[..2]).unwrap();
    // Populate the supported old schema using an exact validated document.
    let envelope = common::Doc::base().character("kept").decode();
    let document = serde_json::to_string(&envelope).unwrap();
    let hash = azeroth_storage::snapshot_content_hash(&envelope).unwrap();
    seed_profile(&db.path);
    conn.execute(
        "UPDATE profiles SET build = 'UNVERIFIED_DEMO', interface = 'UNVERIFIED_DEMO'",
        [],
    )
    .unwrap();
    conn.execute("INSERT INTO snapshots (profile_id, snapshot_id, content_sha256, producer_component, producer_version, game_locale, source_revision, capture_source, capture_clock_trust, document) VALUES (1, ?1, ?2, 'cli', 'test', 'enUS', 'test', 'synthetic_fixture', 'trusted', ?3)", rusqlite::params![envelope.payload.id.as_str(), hash.as_str(), document]).unwrap();
    conn.execute("UPDATE profiles SET current_snapshot_id = 1", [])
        .unwrap();
    let preference =
        azeroth_core::observation::Observation::known(false, envelope.payload.capture.clone())
            .unwrap();
    conn.execute("INSERT INTO profile_preferences (profile_id, preference_id, status, value, document) VALUES (1, 'manual', 'known', 0, ?1)", [serde_json::to_string(&preference).unwrap()]).unwrap();
    let before = common::dump_state(&db.path);
    drop(conn);
    let mut store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(common::dump_state(&db.path), before);
    assert!(store.party_ids().unwrap().is_empty());
    // The migrated database still accepts normal idempotent imports.
    assert_eq!(
        store
            .import_character_snapshot(&envelope, ImportSource::Fixture, None)
            .unwrap(),
        azeroth_storage::ImportOutcome::Duplicate
    );
    assert!(
        store
            .current_snapshot(&ProfileKey::of(&envelope))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        store
            .preferences(&ProfileKey::of(&envelope))
            .unwrap()
            .values()
            .next()
            .unwrap()
            .value(),
        Some(&false)
    );
}

#[test]
fn reopening_a_migrated_file_is_a_no_op() {
    let db = TempDb::new("reopen");
    drop(SqliteStore::open(&db.path).unwrap());
    seed_profile(&db.path);
    let rows = count(&db.path, "SELECT COUNT(*) FROM schema_migrations");
    assert_eq!(rows, i64::from(latest_version()));
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(store.schema_version().unwrap(), latest_version());
    assert_eq!(
        count(&db.path, "SELECT COUNT(*) FROM schema_migrations"),
        rows
    );
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM profiles"), 1);
}

#[test]
fn empty_version_zero_database_migrates() {
    let db = TempDb::new("v0");
    drop(raw(&db.path)); // an empty file is a version 0 database
    let mut conn = Connection::open(&db.path).unwrap();
    assert_eq!(current_version(&conn).unwrap(), 0);
    migrate(&mut conn).unwrap();
    assert_eq!(current_version(&conn).unwrap(), latest_version());
}

#[test]
fn populated_previous_version_migrates_and_keeps_rows() {
    let db = TempDb::new("upgrade");
    drop(SqliteStore::open(&db.path).unwrap());
    seed_profile(&db.path);
    let mut conn = Connection::open(&db.path).unwrap();
    migrate_with(&mut conn, &with_next(ADD_NOTE)).unwrap();
    assert_eq!(current_version(&conn).unwrap(), latest_version() + 1);
    let kept: String = conn
        .query_row(
            "SELECT character_id FROM profiles WHERE note IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kept, "kept");
    // Running again is a no-op.
    migrate_with(&mut conn, &with_next(ADD_NOTE)).unwrap();
    assert_eq!(
        count(&db.path, "SELECT COUNT(*) FROM schema_migrations"),
        i64::from(latest_version() + 1)
    );
}

#[test]
fn newer_database_is_refused_and_left_unmodified() {
    let db = TempDb::new("newer");
    drop(SqliteStore::open(&db.path).unwrap());
    seed_profile(&db.path);
    let mut conn = Connection::open(&db.path).unwrap();
    migrate_with(&mut conn, &with_next(ADD_NOTE)).unwrap();
    drop(conn);
    let before = std::fs::read(&db.path).unwrap();

    let error = SqliteStore::open(&db.path)
        .err()
        .expect("a newer schema must be refused");
    assert_eq!(error.code, ErrorCode::StorageError);
    assert!(!error.retryable);
    assert_eq!(
        error.to_string(),
        "Database schema is newer than this application"
    );
    assert_eq!(
        std::fs::read(&db.path).unwrap(),
        before,
        "file must be byte-identical"
    );
    assert_eq!(
        count(&db.path, "SELECT COUNT(*) FROM schema_migrations"),
        i64::from(latest_version() + 1)
    );
}

#[test]
fn failed_migration_rolls_back_and_keeps_prior_version_and_data() {
    let db = TempDb::new("failed");
    drop(SqliteStore::open(&db.path).unwrap());
    seed_profile(&db.path);
    let mut conn = Connection::open(&db.path).unwrap();
    let error = migrate_with(&mut conn, &with_next(BROKEN)).unwrap_err();
    assert_eq!(error.code, ErrorCode::StorageError);
    assert_eq!(current_version(&conn).unwrap(), latest_version());
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM profiles"), 1);
    assert_eq!(
        count(
            &db.path,
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'half_done'"
        ),
        0,
        "statements before the failure must be rolled back"
    );
    // The prior database is still fully usable by this build.
    drop(conn);
    assert_eq!(
        SqliteStore::open(&db.path)
            .unwrap()
            .schema_version()
            .unwrap(),
        latest_version()
    );
}

#[test]
fn failed_first_migration_leaves_an_empty_database() {
    let mut conn = Connection::open_in_memory().unwrap();
    let bad = [Migration {
        version: 1,
        name: "test: bad first step",
        sql: "CREATE TABLE a (x INTEGER); THIS IS NOT SQL;",
    }];
    assert!(migrate_with(&mut conn, &bad).is_err());
    assert_eq!(current_version(&conn).unwrap(), 0);
    let tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);
}

#[test]
fn non_contiguous_migration_lists_are_rejected() {
    let mut conn = Connection::open_in_memory().unwrap();
    let skipped = [Migration {
        version: 2,
        name: "skips one",
        sql: "SELECT 1;",
    }];
    assert_eq!(
        migrate_with(&mut conn, &skipped).unwrap_err().code,
        ErrorCode::StorageError
    );
    assert_eq!(current_version(&conn).unwrap(), 0);
}

#[test]
fn foreign_keys_are_enforced_on_every_opened_connection() {
    let db = TempDb::new("fk");
    let file = SqliteStore::open(&db.path).unwrap();
    let memory = SqliteStore::open_in_memory().unwrap();
    let reopened = SqliteStore::open(&db.path).unwrap();
    for store in [&file, &memory, &reopened] {
        assert!(store.foreign_keys_enabled().unwrap());
    }
}

#[test]
fn populated_v3_upgrades_preserving_content_parties_and_snapshot_documents() {
    use azeroth_core::types::PartyId;
    use azeroth_storage::{
        ImportSource, ProfileKey, SnapshotStore, content::ContentRevision, party::PartyProfile,
    };
    let db = TempDb::new("promotion-v3-upgrade");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let envelope = common::Doc::base().decode();
    store
        .import_character_snapshot(&envelope, ImportSource::Fixture, None)
        .unwrap();
    let party = PartyProfile {
        id: PartyId::new("kept-party").unwrap(),
        participants: vec![ProfileKey::of(&envelope)],
    };
    store.save_party(&party).unwrap();
    let content: ContentRevision = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    store.stage_content_revision(&content).unwrap();
    drop(store);
    // Reconstruct the exact v3 schema; v4 only adds this empty audit table.
    raw(&db.path)
        .execute_batch(
            "DROP TABLE local_submissions; DROP TABLE current_plan; DROP TABLE app_settings; DROP TABLE content_dispositions; DROP TABLE party_plan_proposals; DROP TABLE session_calibration_annotations; DROP TABLE reminder_state; DROP TABLE sessions; DROP TABLE snapshot_promotions; DELETE FROM schema_migrations WHERE version >= 4;",
        )
        .unwrap();
    let before = common::dump_state(&db.path);
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(store.schema_version().unwrap(), latest_version());
    assert_eq!(common::dump_state(&db.path), before);
    assert_eq!(store.party(&party.id).unwrap().unwrap(), party);
    assert_eq!(
        count(&db.path, "SELECT COUNT(*) FROM snapshot_promotions"),
        0
    );
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM dataset_revisions"), 1);
}

#[test]
fn version_five_upgrade_preserves_sessions_and_adds_bounded_cascading_state() {
    let db = TempDb::new("wave4-v5-upgrade");
    let mut conn = raw(&db.path);
    migrate_with(&mut conn, &MIGRATIONS[..5]).unwrap();
    seed_profile(&db.path);
    // Raw sentinel bytes test migration preservation, not session validity.
    conn.execute("INSERT INTO sessions (profile_id,session_id,content_sha256,document,summary_document) VALUES (1,'kept',?1,'sentinel-events','sentinel-summary')", ["0".repeat(64)]).unwrap();
    migrate(&mut conn).unwrap();
    let documents: (String, String) = conn
        .query_row(
            "SELECT document,summary_document FROM sessions WHERE session_id='kept'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        documents,
        ("sentinel-events".into(), "sentinel-summary".into())
    );
    conn.execute("INSERT INTO reminder_state VALUES (1,'{}')", [])
        .unwrap();
    conn.execute(
        "INSERT INTO session_calibration_annotations VALUES (1,'kept','{}')",
        [],
    )
    .unwrap();
    assert!(
        conn.execute(
            "UPDATE reminder_state SET document=?1",
            ["x".repeat(1_048_577)]
        )
        .is_err()
    );
    assert!(
        conn.execute(
            "UPDATE session_calibration_annotations SET document=?1",
            ["x".repeat(2049)]
        )
        .is_err()
    );
    assert!(
        conn.execute(
            "INSERT INTO session_calibration_annotations VALUES (1,'missing','{}')",
            []
        )
        .is_err()
    );
    conn.execute("DELETE FROM sessions WHERE session_id='kept'", [])
        .unwrap();
    let annotations: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM session_calibration_annotations",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(annotations, 0);
    let reminders: i64 = conn
        .query_row("SELECT COUNT(*) FROM reminder_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(reminders, 1);
    conn.execute("DELETE FROM profiles WHERE id=1", []).unwrap();
    let reminders: i64 = conn
        .query_row("SELECT COUNT(*) FROM reminder_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(reminders, 0);
}
