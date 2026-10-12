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
    version: 3,
    name: "test: profile note",
    sql: "ALTER TABLE profiles ADD COLUMN note TEXT;",
};
const BROKEN: Migration = Migration {
    version: 3,
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
    assert_eq!(latest_version(), 2);
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
