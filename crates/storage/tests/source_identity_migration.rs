//! Existing registered identities must remain usable across schema9/10.
mod common;
use azeroth_storage::{
    SqliteStore,
    content::ContentRevision,
    migrations::{latest_version, migrate},
};
use common::{TempDb, raw};
use rusqlite::{Connection, params, types::Value as SqlValue};
use serde_json::{Value, json};
fn rows(conn: &Connection) -> Vec<Vec<SqlValue>> {
    let mut stmt = conn
        .prepare("SELECT * FROM local_submissions ORDER BY id")
        .unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
#[test]
fn widen_source_bytes_preserves_every_row_tombstone_counter_and_foreign_key() {
    let db = TempDb::new("source-identity-migration");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let mut extra = v["sources"][0].clone();
    extra["id"] = json!("x".repeat(256));
    v["sources"].as_array_mut().unwrap().push(extra);
    let revision: ContentRevision = serde_json::from_value(v).unwrap();
    store.stage_content_revision(&revision).unwrap();
    drop(store);
    let mut conn = raw(&db.path);
    // Reconstruct the exact trusted old table rather than rewriting its migration.
    conn.execute_batch(
        "DROP TABLE catalog_asset_layers; DROP TABLE catalog_asset_blobs; ALTER TABLE dataset_revisions DROP COLUMN content_version; DROP TABLE local_submissions; DELETE FROM schema_migrations WHERE version>=10",
    )
    .unwrap();
    let old = include_str!("../src/schema_v9.sql");
    conn.execute_batch(&old[old.find("CREATE TABLE local_submissions (").unwrap()..])
        .unwrap();
    let scope = revision.key().scope;
    let rid: i64 = conn
        .query_row("SELECT id FROM dataset_revisions", [], |r| r.get(0))
        .unwrap();
    let insert = "INSERT INTO local_submissions(id,fingerprint,edition,build,interface,namespace,is_fixture,base_revision_id,source_id,state,state_revision,consent_version,document,decision_code,created_at_utc_ms) VALUES(?1,?2,?3,?4,?5,'demo',?6,?7,?8,?9,?10,1,?11,?12,0)";
    for (id, fingerprint, state, counter, document, decision) in [
        (
            "pending",
            "a".repeat(64),
            "pending",
            7,
            Some("synthetic SQL sentinel"),
            "pending_review",
        ),
        (
            "withdrawn",
            "b".repeat(64),
            "withdrawn",
            11,
            None,
            "withdrawn_by_submitter",
        ),
    ] {
        conn.execute(
            insert,
            params![
                id,
                fingerprint,
                scope.context.build.edition.as_str(),
                scope.context.build.build.as_str(),
                scope.context.build.interface.as_str(),
                scope.fixture,
                rid,
                "demo-source-1",
                state,
                counter,
                document,
                decision
            ],
        )
        .unwrap();
    }
    let before = rows(&conn);
    assert!(
        conn.execute(
            insert,
            params![
                "long",
                "c".repeat(64),
                scope.context.build.edition.as_str(),
                scope.context.build.build.as_str(),
                scope.context.build.interface.as_str(),
                scope.fixture,
                rid,
                "x".repeat(256),
                "pending",
                1,
                "synthetic SQL sentinel",
                "pending_review"
            ]
        )
        .is_err()
    );
    migrate(&mut conn).unwrap();
    assert_eq!(latest_version(), 12);
    assert_eq!(rows(&conn), before);
    conn.execute(
        insert,
        params![
            "long",
            "c".repeat(64),
            scope.context.build.edition.as_str(),
            scope.context.build.build.as_str(),
            scope.context.build.interface.as_str(),
            scope.fixture,
            rid,
            "x".repeat(256),
            "pending",
            1,
            "synthetic SQL sentinel",
            "pending_review"
        ],
    )
    .unwrap();
    assert!(
        conn.execute(
            "UPDATE local_submissions SET source_id=?1 WHERE id='long'",
            ["x".repeat(257)]
        )
        .is_err()
    );
    assert!(
        conn.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
    assert_eq!(rows(&conn).len(), 3);
}
