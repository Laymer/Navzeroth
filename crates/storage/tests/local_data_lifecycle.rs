use azeroth_core::{
    local_data::InventoryDisposition,
    local_data::PerformancePreset,
    local_data_catalog::{ExtractionSummary, TableInventory},
    local_data_lifecycle::*,
};
use azeroth_storage::local_data::{LocalDataIndex, SavedSelection};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "nav-lifecycle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn db(&self) -> LocalDataIndex {
        LocalDataIndex::open(&self.0.join("index.sqlite3")).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn checkpoint() -> Checkpoint {
    Checkpoint {
        format_version: 1,
        binding: CheckpointBinding {
            format_version: 1,
            source_sha256: "a".repeat(64),
            extractor_sha256: "b".repeat(64),
            selection_sha256: "c".repeat(64),
            preset: PerformancePreset::Small,
            total: 2,
        },
        tables: vec![],
        gaps: vec!["unknown_hotfix".into()],
    }
}
fn row(table: &str) -> TableInventory {
    TableInventory {
        table: table.into(),
        locale: Some("enUS".into()),
        file_data_id: Some(1),
        disposition: InventoryDisposition::NotLocallyPresent,
        reason: "not_locally_present".into(),
        retained_sha256: None,
        decoded_sha256: None,
        encoded_bytes: None,
        records: None,
        layout_hash: None,
    }
}
fn summary() -> ExtractionSummary {
    ExtractionSummary {
        format_version: 1,
        catalog_sha256: "d".repeat(64),
        build: 70450,
        tables_processed: 2,
        tables_decoded: 0,
        preset: PerformancePreset::Small,
        actual_workers: 1,
        gameplay_projection_available: false,
        activated: false,
    }
}
#[test]
fn selections_and_checkpoints_survive_reopen_and_reject_changed_bindings() {
    let dir = Temp::new();
    let mut db = dir.db();
    let mut selection = SavedSelection {
        selection_id: "installation-1".into(),
        installation: dir.0.clone(),
        product: "wow_classic_beta".into(),
        preset: PerformancePreset::Small,
    };
    db.save_selection(&selection).unwrap();
    selection.preset = PerformancePreset::Large;
    db.save_selection(&selection).unwrap();
    let mut cp = checkpoint();
    db.begin("job-1", "private-selected-path", &cp, false)
        .unwrap();
    cp.tables.push(row("Map"));
    db.checkpoint("job-1", &cp).unwrap();
    drop(db);
    let mut db = dir.db();
    assert_eq!(
        db.selection("installation-1").unwrap().preset,
        PerformancePreset::Large
    );
    assert_eq!(db.status("job-1").unwrap().completed, 1);
    assert!(
        !serde_json::to_string(&db.jobs().unwrap())
            .unwrap()
            .contains("private-selected-path")
    );
    for change in 0..6 {
        let mut altered = checkpoint();
        match change {
            0 => altered.binding.source_sha256 = "f".repeat(64),
            1 => altered.binding.extractor_sha256 = "f".repeat(64),
            2 => altered.binding.selection_sha256 = "f".repeat(64),
            3 => altered.binding.preset = PerformancePreset::Max,
            4 => altered.binding.total = 1,
            _ => altered.format_version = 2,
        };
        assert!(db.begin("job-1", "other", &altered, true).is_err());
        assert_eq!(db.status("job-1").unwrap().completed, 1);
    }
    let resumed = db.begin("job-1", "other", &checkpoint(), true).unwrap();
    assert_eq!(resumed.tables.len(), 1);
    assert_eq!(resumed.gaps, cp.gaps);
    assert!(db.begin("job-1", "other", &checkpoint(), false).is_err());
}
#[test]
fn cancellation_checkpoint_and_terminal_states_are_transactional() {
    let dir = Temp::new();
    let mut db = dir.db();
    let mut cp = checkpoint();
    db.begin("job", "request", &cp, false).unwrap();
    assert!(db.forget("job").is_err());
    cp.tables.push(row("Map"));
    db.checkpoint("job", &cp).unwrap();
    let mut substituted = cp.clone();
    substituted.tables[0].table = "Item".into();
    assert!(db.checkpoint("job", &substituted).is_err());
    let mut rebound = cp.clone();
    rebound.binding.source_sha256 = "e".repeat(64);
    assert!(db.checkpoint("job", &rebound).is_err());
    assert!(db.checkpoint("job", &checkpoint()).is_err());
    assert_eq!(
        db.finish("job", JobState::Cancelled, None).unwrap().state,
        JobState::Cancelled
    );
    cp.tables.push(row("Item"));
    assert!(db.checkpoint("job", &cp).is_err());
    assert_eq!(
        db.finish("job", JobState::Complete, Some(summary()))
            .unwrap()
            .state,
        JobState::Cancelled
    );
    db.prepare("job", "request", true).unwrap();
    db.begin("job", "request", &checkpoint(), true).unwrap();
    assert!(
        db.finish("job", JobState::Complete, Some(summary()))
            .is_err()
    );
    db.checkpoint("job", &cp).unwrap();
    assert_eq!(
        db.finish("job", JobState::Complete, Some(summary()))
            .unwrap()
            .state,
        JobState::Complete
    );
    assert!(db.begin("job", "request", &checkpoint(), true).is_err());
    assert_eq!(
        db.finish("job", JobState::Cancelled, None).unwrap().state,
        JobState::Complete
    );
    db.forget("job").unwrap();
    assert!(db.status("job").is_err());
}
#[test]
fn database_failures_leave_prior_checkpoint_and_activation_usable() {
    let dir = Temp::new();
    let mut db = dir.db();
    let mut cp = checkpoint();
    db.begin("job", "request", &cp, false).unwrap();
    let sql = rusqlite::Connection::open(dir.0.join("index.sqlite3")).unwrap();
    sql.execute_batch("CREATE TRIGGER refuse_job BEFORE UPDATE ON jobs BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    cp.tables.push(row("Map"));
    assert!(db.checkpoint("job", &cp).is_err());
    assert_eq!(db.status("job").unwrap().completed, 0);
    sql.execute_batch("DROP TRIGGER refuse_job").unwrap();
    db.checkpoint("job", &cp).unwrap();
    let scope = "a".repeat(64);
    let first = "b".repeat(64);
    let second = "c".repeat(64);
    db.activate(&scope, &first, 0, false).unwrap();
    sql.execute_batch("CREATE TRIGGER refuse_activation BEFORE INSERT ON activations BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    assert!(db.activate(&scope, &second, 1, false).is_err());
    assert_eq!(db.active(&scope).unwrap().catalog_sha256, Some(first));
    assert_eq!(db.active(&scope).unwrap().generation, 1);
}
#[test]
fn activation_is_scoped_guarded_and_rollback_requires_actual_history() {
    let dir = Temp::new();
    let mut db = dir.db();
    let scope = "a".repeat(64);
    let other = "f".repeat(64);
    let first = "b".repeat(64);
    let second = "c".repeat(64);
    assert_eq!(db.active(&scope).unwrap().generation, 0);
    assert!(db.activate(&scope, &first, 0, true).is_err());
    assert!(
        !db.activate(&scope, &first, 0, false)
            .unwrap()
            .verified_for_planning
    );
    assert!(db.activate(&scope, &second, 0, false).is_err());
    db.activate(&scope, &second, 1, false).unwrap();
    assert!(db.activate(&other, &first, 0, true).is_err());
    assert!(db.activate(&scope, &"d".repeat(64), 2, true).is_err());
    assert_eq!(db.activate(&scope, &first, 2, true).unwrap().generation, 3);
    assert_eq!(
        db.active(&scope).unwrap().catalog_sha256,
        Some(first.clone())
    );
    assert!(db.activate(&scope, &first, u64::MAX, false).is_err());
    assert_eq!(db.active(&other).unwrap().catalog_sha256, None);
}
#[test]
fn capacities_and_unknown_versions_refuse_without_erasing_records() {
    let dir = Temp::new();
    let mut db = dir.db();
    for i in 0..MAX_JOBS {
        db.begin(&format!("job-{i}"), "request", &checkpoint(), false)
            .unwrap();
    }
    assert!(db.begin("extra", "request", &checkpoint(), false).is_err());
    assert_eq!(db.jobs().unwrap().len(), MAX_JOBS);
    drop(db);
    let sql = rusqlite::Connection::open(dir.0.join("index.sqlite3")).unwrap();
    sql.pragma_update(None, "user_version", 3).unwrap();
    drop(sql);
    assert!(LocalDataIndex::open(&dir.0.join("index.sqlite3")).is_err());
    let sql = rusqlite::Connection::open(dir.0.join("index.sqlite3")).unwrap();
    assert_eq!(
        sql.query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        MAX_JOBS as i64
    );
    assert_eq!(
        sql.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
}
#[test]
fn foreign_database_is_never_migrated_or_relabelled() {
    let dir = Temp::new();
    let file = dir.0.join("other.sqlite3");
    let conn = rusqlite::Connection::open(&file).unwrap();
    conn.execute_batch(
        "CREATE TABLE personal(value TEXT); INSERT INTO personal VALUES('retained');",
    )
    .unwrap();
    drop(conn);
    assert!(LocalDataIndex::open(&file).is_err());
    let conn = rusqlite::Connection::open(&file).unwrap();
    assert_eq!(
        conn.query_row("SELECT value FROM personal", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "retained"
    );
}

#[test]
fn corrupt_checkpoint_versions_and_transaction_refusal_fail_closed() {
    let dir = Temp::new();
    let mut db = dir.db();
    db.begin("job", "request", &checkpoint(), false).unwrap();
    let sql = rusqlite::Connection::open(dir.0.join("index.sqlite3")).unwrap();
    let mut cp = checkpoint();
    cp.format_version = 99;
    sql.execute(
        "UPDATE jobs SET checkpoint=?1",
        [serde_json::to_string(&cp).unwrap()],
    )
    .unwrap();
    assert!(db.begin("job", "request", &checkpoint(), true).is_err());
    sql.execute(
        "UPDATE jobs SET checkpoint=?1",
        [serde_json::to_string(&checkpoint()).unwrap()],
    )
    .unwrap();
    sql.execute_batch("CREATE TRIGGER quota_failure BEFORE UPDATE ON jobs BEGIN SELECT RAISE(ABORT,'synthetic quota'); END;").unwrap();
    cp = checkpoint();
    cp.tables.push(row("Map"));
    assert!(db.checkpoint("job", &cp).is_err());
    assert_eq!(db.status("job").unwrap().completed, 0);
}

#[test]
fn preparation_is_durable_cancellable_and_never_reuses_unbound_rows() {
    let dir = Temp::new();
    let mut db = dir.db();
    let preparing = db.prepare("preparing", "request", false).unwrap();
    assert_eq!(preparing.state, JobState::Preparing);
    assert_eq!(preparing.total, 0);
    assert!(db.forget("preparing").is_err());
    drop(db);
    let mut db = dir.db();
    assert_eq!(db.status("preparing").unwrap().state, JobState::Preparing);
    db.finish("preparing", JobState::Cancelled, None).unwrap();
    assert!(
        db.begin("preparing", "request", &checkpoint(), false)
            .is_err()
    );
    db.prepare("preparing", "request", true).unwrap();
    let bound = db
        .begin("preparing", "request", &checkpoint(), true)
        .unwrap();
    assert!(bound.tables.is_empty());
    assert_eq!(db.status("preparing").unwrap().state, JobState::Running);
    let mut cp = checkpoint();
    cp.tables.push(row("Map"));
    db.checkpoint("preparing", &cp).unwrap();
    db.finish("preparing", JobState::Interrupted, None).unwrap();
    db.prepare("preparing", "request", true).unwrap();
    db.finish("preparing", JobState::Cancelled, None).unwrap();
    assert!(
        db.begin("preparing", "request", &checkpoint(), true)
            .is_err()
    );
    assert_eq!(db.status("preparing").unwrap().completed, 1);
}
#[test]
fn version_one_migration_preserves_catalog_activation_and_isolates_delivery_history() {
    let dir = Temp::new();
    let path = dir.0.join("index.sqlite3");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE selections(id TEXT PRIMARY KEY, document TEXT NOT NULL); CREATE TABLE jobs(id TEXT PRIMARY KEY, request TEXT NOT NULL, checkpoint TEXT NOT NULL, status TEXT NOT NULL); CREATE TABLE activations(scope TEXT NOT NULL,generation INTEGER NOT NULL,catalog TEXT NOT NULL,PRIMARY KEY(scope,generation)); PRAGMA application_id=1313621041;PRAGMA user_version=1;").unwrap();
    let scope = "a".repeat(64);
    let first = "b".repeat(64);
    let second = "c".repeat(64);
    conn.execute(
        "INSERT INTO activations VALUES(?1,1,?2)",
        rusqlite::params![scope, first],
    )
    .unwrap();
    drop(conn);
    let mut db = LocalDataIndex::open(&path).unwrap();
    assert_eq!(
        db.active(&scope).unwrap().catalog_sha256,
        Some(first.clone())
    );
    assert_eq!(db.delivery_active(&scope).unwrap().generation, 0);
    assert!(db.activate_delivery(&scope, &first, 0, true).is_err());
    let active = db.activate_delivery(&scope, &first, 0, false).unwrap();
    assert_eq!(active.generation, 1);
    assert!(!active.verified_for_planning);
    assert!(db.activate_delivery(&scope, &second, 0, false).is_err());
    assert_eq!(
        db.delivery_active(&scope).unwrap().delivery_sha256,
        Some(first.clone())
    );
    db.activate_delivery(&scope, &second, 1, false).unwrap();
    db.activate_delivery(&scope, &first, 2, true).unwrap();
    assert_eq!(db.delivery_active(&scope).unwrap().generation, 3);
    assert_eq!(db.active(&scope).unwrap().generation, 1);
    drop(db);
    assert_eq!(
        dir.db().delivery_active(&scope).unwrap().delivery_sha256,
        Some(first)
    );
}
