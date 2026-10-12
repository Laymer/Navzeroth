//! Optional polling of one explicitly selected export file. No directories are scanned.
//! Successful normalization is delivered to the caller; storage owns deduplication
//! and snapshot ordering. Failures contain fixed codes and never file contents.
use crate::import::{FileFormat, ImportOptions, ImportResult, import_file, read_bounded};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant, SystemTime},
};

pub const POLL_INTERVAL: Duration = Duration::from_millis(500);
pub const STABLE_INTERVAL: Duration = Duration::from_millis(750);
pub const MAX_ATTEMPTS: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchStatus {
    WaitingForStableFile,
    Imported,
    Missing,
    Unreadable,
    InvalidFile,
    RetryPending,
    RetryExhausted,
    Cancelled,
}

pub struct WatchEvent {
    pub status: WatchStatus,
    pub import: Option<ImportResult>,
}
impl WatchEvent {
    fn status(status: WatchStatus) -> Self {
        Self {
            status,
            import: None,
        }
    }
}

/// Cloneable cancellation handle. Cancellation also wakes a sleeping runner.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<(Mutex<bool>, Condvar)>);
impl Cancellation {
    pub fn cancel(&self) {
        let (lock, wake) = &*self.0;
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = true;
        wake.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn wait(&self) {
        let (lock, wake) = &*self.0;
        let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        let _ = wake.wait_timeout_while(guard, POLL_INTERVAL, |cancelled| !*cancelled);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    size: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64),
}
fn stamp(path: &std::path::Path) -> Result<Stamp, WatchStatus> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            WatchStatus::Missing
        } else {
            WatchStatus::Unreadable
        }
    })?;
    if !meta.file_type().is_file() {
        return Err(WatchStatus::InvalidFile);
    }
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Ok(Stamp {
        size: meta.len(),
        modified: meta.modified().map_err(|_| WatchStatus::Unreadable)?,
        created: meta.created().ok(),
        #[cfg(unix)]
        identity: (meta.dev(), meta.ino()),
    })
}

/// Run on a blocking worker. `poll` is also available for caller-owned scheduling
/// and deterministic tests; its time argument must be monotonically increasing.
pub struct SelectedFileWatcher {
    path: PathBuf,
    format: FileFormat,
    options: ImportOptions,
    candidate: Option<(Stamp, Duration)>,
    attempts: u8,
    imported: bool,
    last_status: Option<WatchStatus>,
}
impl SelectedFileWatcher {
    pub fn new(path: PathBuf, format: FileFormat, options: ImportOptions) -> Self {
        Self {
            path,
            format,
            options,
            candidate: None,
            attempts: 0,
            imported: false,
            last_status: None,
        }
    }
    fn event(&mut self, status: WatchStatus) -> Option<WatchEvent> {
        if self.last_status == Some(status) {
            return None;
        }
        self.last_status = Some(status);
        Some(WatchEvent::status(status))
    }
    pub fn poll(&mut self, elapsed: Duration, cancellation: &Cancellation) -> Option<WatchEvent> {
        if cancellation.is_cancelled() {
            return self.event(WatchStatus::Cancelled);
        }
        let current = match stamp(&self.path) {
            Ok(value) => value,
            Err(status) => {
                self.candidate = None;
                self.attempts = 0;
                self.imported = false;
                return self.event(status);
            }
        };
        if self
            .candidate
            .as_ref()
            .is_none_or(|(previous, _)| *previous != current)
        {
            self.candidate = Some((current, elapsed));
            self.attempts = 0;
            self.imported = false;
            return self.event(WatchStatus::WaitingForStableFile);
        }
        let (before, since) = self.candidate.as_ref().expect("candidate established");
        if elapsed.saturating_sub(*since) < STABLE_INTERVAL
            || self.imported
            || self.attempts >= MAX_ATTEMPTS
        {
            return None;
        }
        let before = before.clone();
        self.attempts += 1;
        let limit = match &self.format {
            FileFormat::Anav1Text => crate::anav1::MAX_TEXT_BYTES,
            FileFormat::SavedVariables(_) => crate::savedvars::MAX_FILE_BYTES,
        };
        let first = read_bounded(&self.path, limit);
        let result = import_file(&self.path, &self.format, &self.options);
        let second = read_bounded(&self.path, limit);
        // A save or rename during reading invalidates even a parseable document.
        if stamp(&self.path).as_ref() != Ok(&before)
            || matches!((&first, &second), (Ok(a), Ok(b)) if a != b)
        {
            self.candidate = None;
            return self.event(WatchStatus::WaitingForStableFile);
        }
        if cancellation.is_cancelled() {
            return self.event(WatchStatus::Cancelled);
        }
        match result {
            Ok(import) if first.is_ok() && second.is_ok() => {
                self.imported = true;
                self.last_status = Some(WatchStatus::Imported);
                Some(WatchEvent {
                    status: WatchStatus::Imported,
                    import: Some(import),
                })
            }
            _ => self.event(if self.attempts >= MAX_ATTEMPTS {
                WatchStatus::RetryExhausted
            } else {
                WatchStatus::RetryPending
            }),
        }
    }
    /// Poll immediately, then every 500 ms until cancellation. The callback
    /// executes on this worker and should return promptly. No thread is spawned.
    pub fn run(mut self, cancellation: Cancellation, mut deliver: impl FnMut(WatchEvent)) {
        let started = Instant::now();
        loop {
            if let Some(event) = self.poll(started.elapsed(), &cancellation) {
                deliver(event);
            }
            if cancellation.is_cancelled() {
                if let Some(event) = self.event(WatchStatus::Cancelled) {
                    deliver(event);
                }
                break;
            }
            cancellation.wait();
        }
    }
}
