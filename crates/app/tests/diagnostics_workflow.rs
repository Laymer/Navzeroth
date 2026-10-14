use azeroth_app::{diagnostics::DiagnosticOperation, recovery::DatabaseHost};
use azeroth_core::{
    error::{AppError, ErrorCode},
    types::{DatasetRevision, UtcMillis},
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
const CONTENT: &str = include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "nav-diagnostic-workflow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn report(host: &DatabaseHost, now: UtcMillis) -> Value {
    serde_json::to_value(host.diagnostic_report(Some(now)).unwrap()).unwrap()
}
#[test]
fn actual_foreground_attempts_include_duplicate_and_failure_without_history_or_secrets() {
    let temp = Temp::new();
    let host = DatabaseHost::open(&temp.0.join("store.sqlite3"));
    let now = UtcMillis::new(1791792000000).unwrap();
    assert_eq!(report(&host, now)["telemetry"]["import_attempts"], 0);
    for _ in 0..2 {
        host.observe(DiagnosticOperation::SnapshotImport, || {
            host.with_application(|app| app.import_text(&format!("ANAV1:{SNAPSHOT}"), Some(now)))
        })
        .unwrap();
    }
    host.observe(DiagnosticOperation::SnapshotImport, || {
        host.with_application(|app| app.import_text("private-secret-invalid", Some(now)))
    })
    .unwrap_err();
    host.with_application(|app| {
        let id = app.status()?.active_profile.unwrap();
        app.stage_content(CONTENT)?;
        app.activate_content(&id, &DatasetRevision::new("synthetic-planning-v1")?)?;
        Ok(())
    })
    .unwrap();
    let result = report(&host, now);
    assert_eq!(result["telemetry"]["import_attempts"], 3);
    assert_eq!(result["telemetry"]["import_succeeded"], 2);
    assert_eq!(result["telemetry"]["import_failed"], 1);
    assert_eq!(result["snapshot_freshness"]["status"], "fresh");
    assert_eq!(result["snapshot_freshness"]["age_ms"], 10000);
    assert_eq!(result["dataset"]["coverage_class"], "demo");
    assert!(
        result["capabilities"]
            .as_object()
            .unwrap()
            .values()
            .all(|v| v["availability"] == "unknown")
    );
    let encoded = result.to_string();
    for secret in [
        "private-secret",
        "demo-character",
        "demo-snapshot",
        "synthetic-planning-v1",
        "store.sqlite3",
        "profile_id",
    ] {
        assert!(!encoded.contains(secret), "{secret}");
    }
    drop(host);
    let reopened = DatabaseHost::open(&temp.0.join("store.sqlite3"));
    assert_eq!(report(&reopened, now)["telemetry"]["import_attempts"], 0);
}
#[test]
fn selection_changes_snapshot_and_content_together_and_preserves_field_age() {
    let temp = Temp::new();
    let host = DatabaseHost::open(&temp.0.join("store.sqlite3"));
    let now = UtcMillis::new(1791792000000).unwrap();
    let mut snapshot: Value = serde_json::from_str(SNAPSHOT).unwrap();
    snapshot["payload"]["level"]["observed_at_utc_ms"] = json!(1791789000000i64);
    snapshot["payload"]["level"]
        .as_object_mut()
        .unwrap()
        .remove("received_at_utc_ms");
    let first = host
        .with_application(|app| app.import_text(&format!("ANAV1:{snapshot}"), Some(now)))
        .unwrap()
        .profile_id;
    host.with_application(|app| {
        app.stage_content(CONTENT)?;
        app.activate_content(&first, &DatasetRevision::new("synthetic-planning-v1")?)?;
        Ok(())
    })
    .unwrap();
    let r = report(&host, now);
    assert_eq!(r["snapshot_freshness"]["status"], "fresh");
    assert_eq!(r["field_freshness"]["level"]["status"], "stale");
    snapshot["payload"]["character"] = json!("second-synthetic-character");
    snapshot["payload"]["id"] = json!("second-synthetic-snapshot");
    snapshot = serde_json::from_str(
        &snapshot
            .to_string()
            .replace("UNVERIFIED_DEMO", "SECOND_SYNTHETIC_BUILD"),
    )
    .unwrap();
    let second = host
        .with_application(|app| app.import_text(&format!("ANAV1:{snapshot}"), Some(now)))
        .unwrap()
        .profile_id;
    host.with_application(|app| app.select_profile(&second))
        .unwrap();
    assert!(report(&host, now)["dataset"].is_null());
    host.with_application(|app| app.select_profile(&first))
        .unwrap();
    assert!(!report(&host, now)["dataset"].is_null());
}
#[test]
fn explicit_export_recaptures_and_never_overwrites_and_recovery_keeps_unknown_metadata() {
    let temp = Temp::new();
    let db = temp.0.join("store.sqlite3");
    let host = DatabaseHost::open(&db);
    let now = UtcMillis::new(1791792000000).unwrap();
    let path = temp.0.join("report.json");
    host.observe::<()>(DiagnosticOperation::Planning, || {
        Err(AppError::new(
            ErrorCode::OperationCanceled,
            "private-secret",
            false,
        ))
    })
    .unwrap_err();
    host.export_diagnostics(&path, Some(now)).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["telemetry"]["events"][0]["error"], "OPERATION_CANCELED");
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("private-secret")
    );
    assert!(host.export_diagnostics(&path, Some(now)).is_err());
    assert_eq!(bytes, std::fs::read(&path).unwrap());
    let corrupt = temp.0.join("corrupt.sqlite3");
    std::fs::write(&corrupt, b"synthetic corruption").unwrap();
    let recovery = DatabaseHost::open(&corrupt);
    let v = report(&recovery, now);
    assert!(v["game"].is_null());
    assert!(v["snapshot_freshness"].is_null());
    assert_eq!(v["telemetry"]["import_attempts"], 0);
}
