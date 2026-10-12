use azeroth_core::{
    observation::{ClockTrust, FreshnessContext},
    types::Millis,
};
use azeroth_ingest::{
    import::{FileFormat, ImportOptions},
    watcher::*,
};
use std::{path::PathBuf, time::Duration};

struct File(PathBuf);
impl File {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "anav-watch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )))
    }
    fn write(&self, text: &str) {
        std::fs::write(&self.0, text).unwrap();
    }
    fn watcher(&self) -> SelectedFileWatcher {
        SelectedFileWatcher::new(
            self.0.clone(),
            FileFormat::Anav1Text,
            ImportOptions::snapshots(FreshnessContext {
                now_utc_ms: None,
                clock_trust: ClockTrust::Uncertain,
                session: None,
                max_age_ms: Millis::new(1000).unwrap(),
            }),
        )
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn valid() -> &'static str {
    include_str!("../../../fixtures/transport/anav1-character-padded.txt")
}
fn poll(w: &mut SelectedFileWatcher, ms: u64, c: &Cancellation) -> Option<WatchEvent> {
    w.poll(Duration::from_millis(ms), c)
}

#[test]
fn stable_file_imports_once_and_preserves_fixture_and_uncertain_age() {
    let file = File::new();
    file.write(valid());
    let mut w = file.watcher();
    let c = Cancellation::default();
    assert_eq!(
        poll(&mut w, 0, &c).unwrap().status,
        WatchStatus::WaitingForStableFile
    );
    assert!(poll(&mut w, 500, &c).is_none());
    let event = poll(&mut w, 1000, &c).unwrap();
    assert_eq!(event.status, WatchStatus::Imported);
    let result = event.import.unwrap();
    assert!(result.is_fixture_or_demo());
    assert!(result.snapshots.values().all(|s| matches!(
        s.freshness,
        azeroth_core::observation::Freshness::Uncertain { .. }
    )));
    assert!(poll(&mut w, 1500, &c).is_none());
}

#[test]
fn partial_writes_retry_with_a_strict_bound_then_recover_on_change() {
    let file = File::new();
    file.write("ANAV1:{");
    let mut w = file.watcher();
    let c = Cancellation::default();
    poll(&mut w, 0, &c);
    assert_eq!(
        poll(&mut w, 1000, &c).unwrap().status,
        WatchStatus::RetryPending
    );
    assert!(poll(&mut w, 1500, &c).is_none());
    assert_eq!(
        poll(&mut w, 2000, &c).unwrap().status,
        WatchStatus::RetryExhausted
    );
    assert!(poll(&mut w, 2500, &c).is_none());
    file.write(valid());
    assert_eq!(
        poll(&mut w, 3000, &c).unwrap().status,
        WatchStatus::WaitingForStableFile
    );
    assert_eq!(
        poll(&mut w, 4000, &c).unwrap().status,
        WatchStatus::Imported
    );
}

#[test]
fn changed_size_restarts_debounce_deletion_and_replacement_recover() {
    let file = File::new();
    file.write("ANAV1:");
    let mut w = file.watcher();
    let c = Cancellation::default();
    poll(&mut w, 0, &c);
    file.write(valid());
    poll(&mut w, 500, &c);
    assert!(poll(&mut w, 1000, &c).is_none());
    assert_eq!(
        poll(&mut w, 1500, &c).unwrap().status,
        WatchStatus::Imported
    );
    let replacement = File::new();
    replacement.write(valid());
    std::fs::remove_file(&file.0).unwrap();
    assert_eq!(poll(&mut w, 2000, &c).unwrap().status, WatchStatus::Missing);
    assert!(poll(&mut w, 2500, &c).is_none());
    std::fs::rename(&replacement.0, &file.0).unwrap();
    assert_eq!(
        poll(&mut w, 3000, &c).unwrap().status,
        WatchStatus::WaitingForStableFile
    );
    assert_eq!(
        poll(&mut w, 4000, &c).unwrap().status,
        WatchStatus::Imported
    );
}

#[test]
fn cancellation_prevents_import_and_wakes_runner() {
    let file = File::new();
    file.write(valid());
    let mut w = file.watcher();
    let c = Cancellation::default();
    poll(&mut w, 0, &c);
    c.cancel();
    assert_eq!(
        poll(&mut w, 1000, &c).unwrap().status,
        WatchStatus::Cancelled
    );
    assert!(poll(&mut w, 1500, &c).is_none());
    let c = Cancellation::default();
    let stop = c.clone();
    let thread = std::thread::spawn(move || {
        file.watcher().run(c, |event| {
            if event.status == WatchStatus::WaitingForStableFile {
                stop.cancel();
            }
        })
    });
    thread.join().unwrap();
}

#[cfg(unix)]
#[test]
fn symlink_is_not_followed() {
    let target = File::new();
    target.write(valid());
    let link = File::new();
    std::os::unix::fs::symlink(&target.0, &link.0).unwrap();
    assert_eq!(
        poll(&mut link.watcher(), 0, &Cancellation::default())
            .unwrap()
            .status,
        WatchStatus::InvalidFile
    );
}

#[test]
fn modification_time_change_restarts_stability_even_with_same_size() {
    let file = File::new();
    file.write(valid());
    let mut w = file.watcher();
    let c = Cancellation::default();
    poll(&mut w, 0, &c);
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(&file.0)
        .unwrap();
    handle
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(100)),
        )
        .unwrap();
    poll(&mut w, 500, &c);
    assert!(poll(&mut w, 1000, &c).is_none());
    assert_eq!(
        poll(&mut w, 1500, &c).unwrap().status,
        WatchStatus::Imported
    );
}

#[cfg(unix)]
#[test]
fn atomic_replacement_is_detected_without_a_missing_poll() {
    let file = File::new();
    file.write(valid());
    let mut w = file.watcher();
    let c = Cancellation::default();
    poll(&mut w, 0, &c);
    poll(&mut w, 1000, &c);
    let replacement = File::new();
    replacement.write(valid());
    std::fs::rename(&replacement.0, &file.0).unwrap();
    assert_eq!(
        poll(&mut w, 1500, &c).unwrap().status,
        WatchStatus::WaitingForStableFile
    );
    assert_eq!(
        poll(&mut w, 2500, &c).unwrap().status,
        WatchStatus::Imported
    );
}
