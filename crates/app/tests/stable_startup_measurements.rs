//! Explicit current-process measurements and labelled manual workload contracts.
use azeroth_app::{
    recovery::{DatabaseHost, StartupMeasurements},
    snapshots::SnapshotApplication,
};
use azeroth_core::{
    contract::{Envelope, decode_envelope},
    entities::CharacterSnapshot,
    observation::StateSource,
    types::UtcMillis,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static COUNTER: AtomicU64 = AtomicU64::new(0);
struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "anav-startup-measurement-{}-{}.sqlite3",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn settled(host: &DatabaseHost) -> azeroth_app::recovery::DatabaseStatus {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let status = host.status().unwrap();
        if status.state != "checking" {
            return status;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn successful_explicit_measurement_is_visible_and_failure_or_legacy_startup_has_none() {
    let database = Database::new();
    let host = DatabaseHost::start_initialized_measured(&database.0, |_| {
        Ok(StartupMeasurements {
            load_ms: 12,
            activation_ms: 0,
        })
    });
    let status = settled(&host);
    assert_eq!(status.state, "ready");
    assert_eq!(
        status.startup_measurements,
        Some(StartupMeasurements {
            load_ms: 12,
            activation_ms: 0
        })
    );
    drop(host);
    let host = DatabaseHost::start_initialized_measured(&database.0, |_| {
        Err(azeroth_core::error::AppError::new(
            azeroth_core::error::ErrorCode::InvalidInput,
            "Synthetic measurement failure",
            false,
        ))
    });
    let status = settled(&host);
    assert_eq!(status.state, "recovery_required");
    assert!(status.startup_measurements.is_none());
    drop(host);
    let host = DatabaseHost::start(&database.0);
    let status = settled(&host);
    assert_eq!(status.state, "ready");
    assert!(status.startup_measurements.is_none());
    assert!(
        !serde_json::to_string(&status)
            .unwrap()
            .contains("startup_measurements")
    );
}
#[test]
fn engineering_snapshot_is_manual_source_compatible_and_cannot_claim_fixture_or_live_capture() {
    let raw = include_str!("support/stable_engineering_snapshot.json");
    let envelope: Envelope<CharacterSnapshot> = decode_envelope(raw).unwrap();
    assert!(!envelope.fixture);
    assert_eq!(envelope.provenance.source, StateSource::ManualEntry);
    assert_eq!(envelope.payload.capture.source, StateSource::ManualEntry);
    let app = SnapshotApplication::in_memory().unwrap();
    let receipt = app
        .import_text(&format!("ANAV1:{raw}"), Some(UtcMillis::new(1000).unwrap()))
        .unwrap();
    let profile = app
        .profiles()
        .unwrap()
        .into_iter()
        .find(|p| p.id == receipt.profile_id)
        .unwrap();
    assert!(!profile.fixture);
    assert_eq!(profile.display_name, "synthetic-engineering-workload");
    assert!(
        envelope
            .payload
            .quests
            .iter()
            .any(|(_, q)| q.accepted.value().is_none()),
        "Unasserted eligibility/progress stays unknown"
    );
}
#[test]
fn expensive_private_initialization_keeps_status_responsive_and_never_exposes_partial_ready_state()
{
    use std::sync::{Arc, Mutex, mpsc};
    let database = Database::new();
    let (begun_tx, begun_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = Arc::new(Mutex::new(release_rx));
    let host = DatabaseHost::start_initialized_measured(&database.0, move |_| {
        begun_tx.send(()).unwrap();
        release.lock().unwrap().recv().unwrap();
        Ok(StartupMeasurements {
            load_ms: 100,
            activation_ms: 1,
        })
    });
    begun_rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    let copy = host.clone();
    let (status_tx, status_rx) = mpsc::channel();
    let observer = std::thread::spawn(move || {
        status_tx
            .send((
                copy.status().unwrap(),
                copy.with_application(|app| app.profiles()).is_err(),
            ))
            .unwrap();
    });
    let status = status_rx.recv_timeout(std::time::Duration::from_secs(2));
    release_tx.send(()).unwrap();
    observer.join().unwrap();
    let (status, refused) =
        status.expect("Status must remain readable while bundle initialization is pending");
    assert_eq!(status.state, "checking");
    assert!(status.startup_measurements.is_none());
    assert!(refused);
    assert_eq!(settled(&host).state, "ready");
}
