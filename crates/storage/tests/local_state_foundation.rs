//! Migration/recovery boundaries for the shared local state schema.
mod common;
use azeroth_storage::{
    SqliteStore,
    content::{ContentRevision, StageOutcome},
    recovery::reconstruct_backup,
};
use common::{TempDb, raw};
use serde_json::{Value, json};
type SeededDisposition = (String, String, i64, Option<i64>, Option<i64>);
fn content(name: &str) -> ContentRevision {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    v["envelope"]["payload"]["revision"] = json!(name);
    serde_json::from_value(v).unwrap()
}
#[test]
fn populated_v8_backfills_only_recorded_activations_and_preserves_immutable_bytes() {
    let db = TempDb::new("local-state-v8");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let a = content("a");
    let b = content("b");
    let c = content("c");
    for x in [&a, &b, &c] {
        store.stage_content_revision(x).unwrap();
    }
    store.activate_content_revision(&a.key()).unwrap();
    store.activate_content_revision(&b.key()).unwrap();
    drop(store);
    let conn = raw(&db.path);
    let before: Vec<String> = conn
        .prepare("SELECT document FROM dataset_revisions ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    conn.execute_batch("ALTER TABLE dataset_revisions DROP COLUMN content_version; DROP TABLE local_submissions; DROP TABLE current_plan; DROP TABLE app_settings; DROP TABLE content_dispositions; DELETE FROM schema_migrations WHERE version>=9").unwrap();
    drop(conn);
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(
        store
            .active_content_revision(&b.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        b.key()
    );
    let conn = raw(&db.path);
    let after: Vec<String> = conn
        .prepare("SELECT document FROM dataset_revisions ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(before, after);
    let rows:Vec<SeededDisposition>=conn.prepare("SELECT r.revision,d.state,d.state_revision,d.last_activated_order,d.changed_at_utc_ms FROM dataset_revisions r JOIN content_dispositions d ON d.revision_id=r.id ORDER BY r.revision").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap().map(Result::unwrap).collect();
    assert_eq!(
        rows,
        vec![
            ("a".into(), "eligible".into(), 1, Some(1), None),
            ("b".into(), "eligible".into(), 1, Some(2), None),
            ("c".into(), "eligible".into(), 1, None, None)
        ]
    );
    azeroth_storage::recovery::validate(&store).unwrap();
}
#[test]
fn current_seed_restore_preserves_empty_tombstone_revision_and_source_bytes() {
    let db = TempDb::new("plan-seed-source");
    let dest = TempDb::new("plan-seed-destination");
    drop(SqliteStore::open(&db.path).unwrap());
    raw(&db.path)
        .execute("UPDATE current_plan SET revision=23", [])
        .unwrap();
    let before = std::fs::read(&db.path).unwrap();
    let report = reconstruct_backup(&db.path, &dest.path).unwrap();
    assert_eq!(report.rows["current_plan"], 1);
    let row: (i64, Option<String>) = raw(&dest.path)
        .query_row(
            "SELECT revision,kind FROM current_plan WHERE slot=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(row, (23, None));
    assert_eq!(std::fs::read(&db.path).unwrap(), before);
}
#[test]
fn invalid_populated_feature_documents_fail_closed_without_source_changes() {
    for (label, sql) in [
        ("settings", "INSERT INTO app_settings VALUES (1,1,1,'{}')"),
        (
            "plan",
            "UPDATE current_plan SET kind='solo',request_json='{}',guard_json='{}',summary_json='{}'",
        ),
    ] {
        let db = TempDb::new(label);
        let dest = TempDb::new(&format!("refused-{label}"));
        drop(SqliteStore::open(&db.path).unwrap());
        raw(&db.path).execute_batch(sql).unwrap();
        let before = std::fs::read(&db.path).unwrap();
        assert!(reconstruct_backup(&db.path, &dest.path).is_err());
        assert!(!dest.path.exists());
        assert_eq!(std::fs::read(&db.path).unwrap(), before);
    }
}
#[test]
fn blocked_missing_or_exhausted_disposition_cannot_become_active_or_reset_on_restage() {
    let db = TempDb::new("eligibility-boundary");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let a = content("eligible");
    let b = content("blocked");
    store.stage_content_revision(&a).unwrap();
    store.stage_content_revision(&b).unwrap();
    store.activate_content_revision(&a.key()).unwrap();
    store.activate_content_revision(&b.key()).unwrap();
    let conn = raw(&db.path);
    conn.execute("UPDATE content_dispositions SET state='quarantined',state_revision=7,reason_code='invalid_content' WHERE revision_id=(SELECT id FROM dataset_revisions WHERE revision='blocked')",[]).unwrap();
    assert!(
        store
            .active_content_revision(&b.key().scope)
            .unwrap()
            .is_none()
    );
    assert!(store.activate_content_revision(&b.key()).is_err());
    assert_eq!(
        store.stage_content_revision(&b).unwrap(),
        StageOutcome::Duplicate
    );
    let guard:(String,i64)=conn.query_row("SELECT state,state_revision FROM content_dispositions WHERE revision_id=(SELECT id FROM dataset_revisions WHERE revision='blocked')",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(guard, ("quarantined".into(), 7));
    store
        .rollback_content_revision(&a.key().scope)
        .unwrap()
        .unwrap();
    let before: i64 = conn
        .query_row("SELECT revision_id FROM active_datasets", [], |r| r.get(0))
        .unwrap();
    assert!(store.rollback_content_revision(&a.key().scope).is_err());
    conn.execute("UPDATE content_dispositions SET last_activated_order=9007199254740991 WHERE revision_id=?1",[before]).unwrap();
    let c = content("new-candidate");
    store.stage_content_revision(&c).unwrap();
    assert!(store.activate_content_revision(&c.key()).is_err());
    assert_eq!(
        conn.query_row("SELECT revision_id FROM active_datasets", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        before
    );
    conn.execute(
        "DELETE FROM content_dispositions WHERE revision_id=?1",
        [before],
    )
    .unwrap();
    assert!(
        store
            .active_content_revision(&a.key().scope)
            .unwrap()
            .is_none()
    );
    assert!(store.activate_content_revision(&a.key()).is_err());
}
#[test]
fn sql_constraints_refuse_multibyte_overflow_partial_plans_and_noninteger_counters() {
    let db = TempDb::new("local-constraints");
    drop(SqliteStore::open(&db.path).unwrap());
    let conn = raw(&db.path);
    assert!(
        conn.execute(
            "INSERT INTO app_settings VALUES(1,1,1,?1)",
            ["é".repeat(32769)]
        )
        .is_err()
    );
    for sql in [
        "UPDATE current_plan SET revision=1.5",
        "UPDATE current_plan SET revision=9007199254740992",
        "UPDATE current_plan SET kind='solo'",
        "INSERT INTO app_settings VALUES(1,2,1,'{}')",
    ] {
        assert!(conn.execute_batch(sql).is_err(), "{sql}");
    }
    assert_eq!(
        conn.query_row("SELECT revision FROM current_plan", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn submission_constraints_preserve_scope_source_and_erased_dedup_tombstones() {
    let db = TempDb::new("submission-constraints");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let base = content("submission-base");
    store.stage_content_revision(&base).unwrap();
    let scope = base.key().scope;
    let conn = raw(&db.path);
    let rid: i64 = conn
        .query_row("SELECT id FROM dataset_revisions", [], |r| r.get(0))
        .unwrap();
    let fingerprint = "a".repeat(64);
    let insert = "INSERT INTO local_submissions(id,fingerprint,edition,build,interface,namespace,is_fixture,base_revision_id,source_id,state,state_revision,consent_version,document,decision_code,created_at_utc_ms) VALUES('local-probe',?1,?2,?3,?4,'demo',?5,?6,?7,'pending',1,1,'{}',?8,-1)";
    for (build, source, decision) in [
        ("OTHER", "demo-source-1", "pending_review"),
        (
            scope.context.build.build.as_str(),
            "missing-source",
            "pending_review",
        ),
        (
            scope.context.build.build.as_str(),
            "demo-source-1",
            "accepted_local_review",
        ),
    ] {
        assert!(
            conn.execute(
                insert,
                rusqlite::params![
                    fingerprint,
                    scope.context.build.edition.as_str(),
                    build,
                    scope.context.build.interface.as_str(),
                    scope.fixture,
                    rid,
                    source,
                    decision
                ]
            )
            .is_err()
        );
    }
    conn.execute(
        insert,
        rusqlite::params![
            fingerprint,
            scope.context.build.edition.as_str(),
            scope.context.build.build.as_str(),
            scope.context.build.interface.as_str(),
            scope.fixture,
            rid,
            "demo-source-1",
            "pending_review"
        ],
    )
    .unwrap();
    assert_eq!(
        conn.query_row("SELECT created_at_utc_ms FROM local_submissions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        -1
    );
    assert!(
        conn.execute(
            "UPDATE local_submissions SET state='withdrawn',decision_code='withdrawn_by_submitter'",
            []
        )
        .is_err()
    );
    conn.execute("UPDATE local_submissions SET state='withdrawn',decision_code='withdrawn_by_submitter',document=NULL,state_revision=2",[]).unwrap();
    assert!(conn.execute("INSERT INTO local_submissions SELECT 'resurrection',fingerprint,edition,build,interface,namespace,is_fixture,base_revision_id,source_id,'pending',1,1,'{}','pending_review',created_at_utc_ms,changed_at_utc_ms FROM local_submissions",[]).is_err());
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM local_submissions WHERE document IS NULL AND state='withdrawn'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
