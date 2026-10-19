//! Explicit recovery host. A failed database startup still exposes recovery UI.
use crate::snapshots::SnapshotApplication;
use azeroth_core::error::{AppError, ErrorCode, Result};
use azeroth_storage::{
    SqliteStore,
    recovery::{RecoveryInspection, reconstruct_backup},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
static SERIAL: AtomicU64 = AtomicU64::new(1);
fn fail(message: &'static str) -> AppError {
    AppError::new(ErrorCode::StorageError, message, false)
}
fn pending_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".restore-pending");
    PathBuf::from(name)
}
fn pending_directory(path: &Path) -> Option<String> {
    let marker = pending_path(path);
    let info = fs::symlink_metadata(&marker).ok()?;
    if !info.file_type().is_file() || info.len() > 4096 {
        return None;
    }
    let file = fs::File::open(marker).ok()?;
    let mut text = String::new();
    file.take(4097).read_to_string(&mut text).ok()?;
    if text.len() > 4096 { None } else { Some(text) }
}
fn serial() -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )
}
fn digest(path: &Path) -> Result<String> {
    let info = fs::symlink_metadata(path).map_err(|_| fail("Recovery candidate cannot be read"))?;
    if !info.file_type().is_file() || info.len() > azeroth_storage::recovery::MAX_BACKUP_BYTES {
        return Err(fail(
            "Recovery candidate exceeds bounds or is not a regular file",
        ));
    }
    let mut file = fs::File::open(path).map_err(|_| fail("Recovery candidate cannot be read"))?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    let mut total = 0u64;
    loop {
        let n = file
            .read(&mut bytes)
            .map_err(|_| fail("Recovery candidate cannot be read"))?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > azeroth_storage::recovery::MAX_BACKUP_BYTES {
            return Err(fail("Recovery candidate exceeds bounds"));
        }
        hash.update(&bytes[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
/// Current-process explicit bundle startup timings. Never persisted or sent.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StartupMeasurements {
    pub load_ms: u64,
    pub activation_ms: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseStatus {
    pub preserved_recovery_directory: Option<String>,
    pub state: &'static str,
    pub error: Option<AppError>,
    pub schema_version: u32,
    pub restore_limit_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_measurements: Option<StartupMeasurements>,
}
#[derive(Clone, Debug, Serialize)]
pub struct BackupPreview {
    pub candidate_id: String,
    pub inspection: RecoveryInspection,
}
#[derive(Clone, Debug, Serialize)]
pub struct RestoreReceipt {
    pub pending_marker_cleanup_required: bool,
    pub original_files_retained: bool,
    pub recovery_directory: String,
}
type Initializer =
    Arc<dyn Fn(&SnapshotApplication) -> Result<Option<StartupMeasurements>> + Send + Sync>;
struct Candidate {
    id: String,
    path: PathBuf,
    hash: String,
}
struct Hosted {
    startup_measurements: Option<StartupMeasurements>,
    checking: bool,
    app: Option<SnapshotApplication>,
    error: Option<AppError>,
}
pub struct DatabaseHost {
    path: PathBuf,
    gate: RwLock<Hosted>,
    candidate: Mutex<Option<Candidate>>,
    planning_jobs: crate::planning_jobs::PlanningJobs,
    telemetry: Mutex<crate::diagnostics::DiagnosticTelemetry>,
    initializer: Option<Initializer>,
}
impl DatabaseHost {
    /// Observe only explicitly wrapped foreground attempts, never watcher/history data.
    /// The collector has no message/path input and never changes the operation result.
    pub fn observe<T>(
        &self,
        operation: crate::diagnostics::DiagnosticOperation,
        call: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let started = std::time::Instant::now();
        let result = call();
        let duration = u64::try_from(started.elapsed().as_millis())
            .ok()
            .filter(|ms| *ms <= 86_400_000);
        let mut telemetry = self
            .telemetry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _ = telemetry.record(operation, duration, result.as_ref().err().map(|e| e.code));
        result
    }
    /// Selection and data are captured consistently; capabilities remain unknown.
    /// Telemetry is a current-process foreground sample, independent of stored history.
    pub fn diagnostic_report(
        &self,
        now: Option<azeroth_core::types::UtcMillis>,
    ) -> Result<crate::diagnostics::DiagnosticReport> {
        let started = std::time::Instant::now();
        let hosted = self
            .gate
            .read()
            .map_err(|_| fail("Diagnostic capture unavailable"))?;
        let (snapshot, content) = match &hosted.app {
            Some(app) => app.diagnostic_inputs()?,
            None => (None, None),
        };
        let telemetry = self
            .telemetry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        drop(hosted);
        let clock = crate::snapshots::context(crate::planning_job_service::advanced_now(
            now,
            started.elapsed(),
        ));
        crate::diagnostics::build_with_context(
            snapshot.as_ref(),
            content
                .as_ref()
                .map(|c| c as &dyn crate::catalog::CatalogView),
            Some(&telemetry),
            Some(&clock),
            None,
        )
    }
    pub fn export_diagnostics(
        &self,
        path: &Path,
        now: Option<azeroth_core::types::UtcMillis>,
    ) -> Result<crate::diagnostics::DiagnosticReceipt> {
        crate::diagnostics::write_new(&self.diagnostic_report(now)?, path)
    }
    /// Cancellation/status never acquire the recovery lease or store mutex.
    pub fn planning_jobs(&self) -> &crate::planning_jobs::PlanningJobs {
        &self.planning_jobs
    }
    pub fn start(path: &Path) -> Arc<Self> {
        Self::start_with(path, None)
    }
    /// Initialization runs on the startup worker before application commands can
    /// observe ready state, and again after a compatible backup is restored.
    pub fn start_initialized(
        path: &Path,
        initialize: impl Fn(&SnapshotApplication) -> Result<()> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::start_with(
            path,
            Some(Arc::new(move |app| initialize(app).map(|()| None))),
        )
    }
    /// Report only successful, explicitly measured bundle initialization.
    pub fn start_initialized_measured(
        path: &Path,
        initialize: impl Fn(&SnapshotApplication) -> Result<StartupMeasurements> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::start_with(path, Some(Arc::new(move |app| initialize(app).map(Some))))
    }
    fn start_with(path: &Path, initializer: Option<Initializer>) -> Arc<Self> {
        let host = Arc::new(Self {
            path: path.to_owned(),
            gate: RwLock::new(Hosted {
                startup_measurements: None,
                checking: true,
                app: None,
                error: None,
            }),
            candidate: Mutex::new(None),
            planning_jobs: crate::planning_jobs::PlanningJobs::default(),
            telemetry: Mutex::new(crate::diagnostics::DiagnosticTelemetry::observed()),
            initializer,
        });
        let worker = host.clone();
        let spawned = std::thread::Builder::new()
            .name("database-startup-check".into())
            .spawn(move || {
                let opened = Self::open(&worker.path);
                if let Ok(mut checked) = opened.gate.write() {
                    let mut measurements = None;
                    if let (Some(app), Some(initialize)) = (&checked.app, &worker.initializer) {
                        match initialize(app) {
                            Ok(value) => measurements = value,
                            Err(error) => {
                                checked.app = None;
                                checked.error = Some(error);
                            }
                        }
                    }
                    // Keep the live host readable in checking state throughout
                    // expensive private verification and staging. Publish once.
                    if let Ok(mut live) = worker.gate.write() {
                        live.startup_measurements = measurements;
                        live.app = checked.app.take();
                        live.error = checked.error.take();
                        live.checking = false;
                    }
                }
            });
        if spawned.is_err()
            && let Ok(mut live) = host.gate.write()
        {
            live.error = Some(fail("Database initialization worker unavailable"));
            live.checking = false;
        }
        host
    }
    fn open_application(&self) -> Result<SnapshotApplication> {
        let app = SnapshotApplication::open(&self.path)?;
        if let Some(initialize) = &self.initializer {
            initialize(&app)?;
        }
        Ok(app)
    }
    pub fn open(path: &Path) -> Self {
        let opened = (|| {
            if pending_path(path).exists() {
                return Err(fail(
                    "Interrupted restore detected; inspect a retained compatible backup before recovery",
                ));
            }
            if path.exists() {
                let probe = path.with_file_name(format!("recovery-check-{}.sqlite3", serial()));
                let checked = reconstruct_backup(path, &probe);
                let _ = fs::remove_file(&probe);
                checked?;
            }
            let app = SnapshotApplication::open(path)?;
            for party in app.party_ids()? {
                app.party_plan_proposal(&party, None)?;
            }
            Ok::<SnapshotApplication, AppError>(app)
        })();
        let hosted = match opened {
            Ok(app) => Hosted {
                startup_measurements: None,
                checking: false,
                app: Some(app),
                error: None,
            },
            Err(error) => Hosted {
                startup_measurements: None,
                checking: false,
                app: None,
                error: Some(error),
            },
        };
        Self {
            path: path.to_owned(),
            gate: RwLock::new(hosted),
            candidate: Mutex::new(None),
            planning_jobs: crate::planning_jobs::PlanningJobs::default(),
            telemetry: Mutex::new(crate::diagnostics::DiagnosticTelemetry::observed()),
            initializer: None,
        }
    }
    /// The read lease covers the entire operation, including any detached work
    /// it waits for. Restore cannot close a store while calls are still running.
    pub fn with_application<T>(
        &self,
        call: impl FnOnce(&SnapshotApplication) -> Result<T>,
    ) -> Result<T> {
        let state = self
            .gate
            .read()
            .map_err(|_| fail("Database operation lease unavailable"))?;
        if state.checking {
            return Err(AppError::new(
                ErrorCode::ProviderUnavailable,
                "Database initialization is still checking",
                true,
            ));
        }
        call(
            state
                .app
                .as_ref()
                .ok_or_else(|| fail("Database recovery required; inspect a compatible backup"))?,
        )
    }
    pub fn status(&self) -> Result<DatabaseStatus> {
        let state = self
            .gate
            .read()
            .map_err(|_| fail("Database status unavailable"))?;
        Ok(DatabaseStatus {
            startup_measurements: state.startup_measurements.clone(),
            preserved_recovery_directory: pending_directory(&self.path),
            state: if state.checking {
                "checking"
            } else if state.app.is_some() {
                "ready"
            } else {
                "recovery_required"
            },
            error: state.error.clone(),
            schema_version: azeroth_storage::migrations::latest_version(),
            restore_limit_bytes: azeroth_storage::recovery::MAX_BACKUP_BYTES,
        })
    }
    pub fn backup(&self, destination: &Path) -> Result<()> {
        self.with_application(|app| app.with_store_mut(|store| store.backup_to(destination)))
    }
    pub fn check(&self) -> Result<()> {
        self.with_application(|app| {
            app.with_store_mut(|store| azeroth_storage::recovery::validate(store))
        })
    }
    pub fn inspect(&self, source: &Path) -> Result<BackupPreview> {
        let mut slot = self
            .candidate
            .lock()
            .map_err(|_| fail("Backup inspection unavailable"))?;
        if let Some(old) = slot.take() {
            let _ = fs::remove_file(old.path);
        }
        let id = serial();
        let staged = self
            .path
            .with_file_name(format!("recovery-candidate-{id}.sqlite3"));
        let inspection = reconstruct_backup(source, &staged)?;
        let application_check = (|| {
            let app = SnapshotApplication::open(&staged)?;
            for party in app.party_ids()? {
                app.party_plan_proposal(&party, None)?;
            }
            Ok::<(), AppError>(())
        })();
        if let Err(error) = application_check {
            let _ = fs::remove_file(&staged);
            return Err(error);
        }
        let hash = match digest(&staged) {
            Ok(hash) => hash,
            Err(e) => {
                let _ = fs::remove_file(&staged);
                return Err(e);
            }
        };
        let candidate_id = format!("{id}-{hash}");
        if let Some(old) = slot.take() {
            let _ = fs::remove_file(old.path);
        }
        *slot = Some(Candidate {
            id: candidate_id.clone(),
            path: staged,
            hash,
        });
        Ok(BackupPreview {
            candidate_id,
            inspection,
        })
    }
    pub fn restore(&self, id: &str) -> Result<RestoreReceipt> {
        self.restore_using(id, |from, to| fs::rename(from, to))
    }
    fn restore_using(
        &self,
        id: &str,
        install_candidate: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    ) -> Result<RestoreReceipt> {
        self.restore_with_files(id, install_candidate, |path| fs::remove_file(path))
    }
    fn restore_with_files(
        &self,
        id: &str,
        install_candidate: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
        cleanup_marker: impl FnOnce(&Path) -> std::io::Result<()>,
    ) -> Result<RestoreReceipt> {
        let mut hosted = self
            .gate
            .write()
            .map_err(|_| fail("Database restore lease unavailable"))?;
        let mut slot = self
            .candidate
            .lock()
            .map_err(|_| fail("Backup inspection unavailable"))?;
        if hosted.checking {
            return Err(fail(
                "Database initialization is still checking; retry when complete",
            ));
        }
        let candidate = slot
            .as_ref()
            .filter(|c| c.id == id)
            .ok_or_else(|| fail("Backup inspection changed; inspect again"))?;
        if digest(&candidate.path)? != candidate.hash {
            return Err(fail("Staged recovery candidate changed; inspect again"));
        }
        // Revalidate bytes at the destructive boundary. No mutable source path
        // is read after preview. The staged DB contains only trusted schema.
        let checked = SqliteStore::open(&candidate.path)?;
        azeroth_storage::recovery::validate(&checked)?;
        drop(checked);
        let was_ready = hosted.app.is_some();
        let recovery = self
            .path
            .with_file_name(format!("recovery-original-{}", serial()));
        let created = fs::create_dir(&recovery)
            .map_err(|_| fail("Original database preservation directory cannot be created"));
        created?;
        if let Some(app) = hosted.app.as_ref() {
            app.stop_watch()?;
            app.with_store_mut(|store| {
                store.backup_to(&recovery.join("consistent-before-restore.sqlite3"))
            })?;
        }
        // Stop/join before closing, with no store mutex held.
        use std::io::Write;
        let marker_path = pending_path(&self.path);
        let marker_was_present = marker_path.exists();
        if marker_was_present {
            let info = fs::symlink_metadata(&marker_path)
                .map_err(|_| fail("Existing recovery marker metadata unavailable"))?;
            if !info.file_type().is_file() || info.len() > 4096 {
                return Err(fail(
                    "Existing recovery marker must be a regular file within bounds",
                ));
            }
        }
        // Never overwrite an existing marker during a retry: its original
        // retained-directory identity must survive every pre-install failure.
        if !marker_was_present {
            let mut marker = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker_path)
                .map_err(|_| fail("Recovery transaction marker cannot be created"))?;
            let persisted = marker
                .write_all(recovery.to_string_lossy().as_bytes())
                .and_then(|_| marker.sync_all());
            drop(marker);
            if persisted.is_err() {
                if fs::remove_file(&marker_path).is_err() {
                    hosted.error = Some(fail(
                        "Recovery marker write failed and cleanup remains required",
                    ));
                }
                return Err(fail("Recovery transaction marker cannot be persisted"));
            }
        }
        drop(hosted.app.take());
        let original_paths = [
            self.path.clone(),
            PathBuf::from(format!("{}-wal", self.path.to_string_lossy())),
            PathBuf::from(format!("{}-shm", self.path.to_string_lossy())),
        ];
        let mut moved = Vec::new();
        let install = (|| {
            for path in &original_paths {
                if path.exists() {
                    let target = recovery.join(
                        path.file_name()
                            .ok_or_else(|| fail("Invalid database filename"))?,
                    );
                    fs::rename(path, &target)
                        .map_err(|_| fail("Original database files cannot be preserved"))?;
                    moved.push((path.clone(), target));
                }
            }
            install_candidate(&candidate.path, &self.path)
                .map_err(|_| fail("Validated recovery candidate cannot be installed"))?;
            self.open_application()
        })();
        match install {
            Ok(app) => {
                hosted.app = Some(app);
                let pending_marker_cleanup_required = cleanup_marker(&marker_path).is_err();
                hosted.error = if pending_marker_cleanup_required {
                    Some(fail(
                        "Database restored but recovery marker cleanup failed; next startup will require recovery",
                    ))
                } else {
                    None
                };
                let retained = !moved.is_empty();
                slot.take();
                Ok(RestoreReceipt {
                    pending_marker_cleanup_required,
                    original_files_retained: retained,
                    recovery_directory: recovery.to_string_lossy().into_owned(),
                })
            }
            Err(error) => {
                // If candidate was installed, move it back before restoring originals.
                let mut rollback_ok = true;
                if !candidate.path.exists() && self.path.exists() {
                    rollback_ok = fs::rename(&self.path, &candidate.path).is_ok();
                }
                if rollback_ok {
                    for (original, saved) in moved.iter().rev() {
                        if fs::rename(saved, original).is_err() {
                            rollback_ok = false;
                        }
                    }
                }
                let marker_cleanup_failed =
                    rollback_ok && !marker_was_present && cleanup_marker(&marker_path).is_err();
                hosted.app = if rollback_ok && was_ready {
                    self.open_application().ok()
                } else {
                    None
                };
                hosted.error = if hosted.app.is_none() {
                    Some(fail("Restore failed; preserved originals require recovery"))
                } else if marker_cleanup_failed {
                    Some(fail(
                        "Original database restored but recovery marker cleanup failed; next startup will require recovery",
                    ))
                } else {
                    None
                };
                Err(error)
            }
        }
    }
}
impl Drop for DatabaseHost {
    fn drop(&mut self) {
        if let Ok(slot) = self.candidate.get_mut()
            && let Some(candidate) = slot.take()
        {
            let _ = fs::remove_file(candidate.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("navzeroth-recovery-test-{}", serial()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn seed(path: &Path) {
        let app = SnapshotApplication::open(path).unwrap();
        app.import_text(&format!("ANAV1:{SNAPSHOT}"), None).unwrap();
    }
    #[test]
    fn bounded_empty_or_unreadable_marker_content_does_not_block_explicit_recovery() {
        for bytes in [vec![], vec![0xff, 0x00], b"truncated-directory".to_vec()] {
            let dir = Directory::new();
            let live = dir.path("missing.sqlite3");
            let backup = dir.path("good.sqlite3");
            seed(&backup);
            fs::write(pending_path(&live), &bytes).unwrap();
            let host = DatabaseHost::open(&live);
            assert_eq!(host.status().unwrap().state, "recovery_required");
            let preview = host.inspect(&backup).unwrap();
            assert!(
                host.restore_using(&preview.candidate_id, |_, _| Err(std::io::Error::other(
                    "install failure"
                )))
                .is_err()
            );
            assert_eq!(fs::read(pending_path(&live)).unwrap(), bytes);
            assert!(!live.exists());
            host.restore(&preview.candidate_id).unwrap();
            assert_eq!(host.status().unwrap().state, "ready");
            assert!(!pending_path(&live).exists());
        }
    }

    #[test]
    fn marker_cleanup_failure_is_visible_and_next_startup_remains_guarded() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        seed(&live);
        let host = DatabaseHost::open(&live);
        let preview = host.inspect(&live).unwrap();
        let receipt = host
            .restore_with_files(
                &preview.candidate_id,
                |from, to| fs::rename(from, to),
                |_| Err(std::io::Error::other("cleanup failed")),
            )
            .unwrap();
        assert!(receipt.pending_marker_cleanup_required);
        assert!(host.status().unwrap().error.is_some());
        assert_eq!(host.status().unwrap().state, "ready");
        drop(host);
        assert_eq!(
            DatabaseHost::open(&live).status().unwrap().state,
            "recovery_required"
        );
    }

    #[test]
    fn interrupted_restore_does_not_silently_create_empty_database() {
        let dir = Directory::new();
        let live = dir.path("missing.sqlite3");
        let backup = dir.path("good.sqlite3");
        seed(&backup);
        fs::write(pending_path(&live), "retained-original-directory").unwrap();
        let host = DatabaseHost::open(&live);
        assert_eq!(host.status().unwrap().state, "recovery_required");
        assert!(!live.exists());
        let preview = host.inspect(&backup).unwrap();
        assert!(
            host.restore_using(&preview.candidate_id, |_, _| Err(std::io::Error::other(
                "install failed"
            )))
            .is_err()
        );
        assert_eq!(
            pending_directory(&live).as_deref(),
            Some("retained-original-directory")
        );
        assert!(!live.exists());
        host.restore(&preview.candidate_id).unwrap();
        assert!(!pending_path(&live).exists());
        assert_eq!(host.status().unwrap().state, "ready");
    }
    fn wait_started(host: &DatabaseHost) -> DatabaseStatus {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
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
    fn initialization_finishes_before_ready_and_repeats_after_restore() {
        let dir = Directory::new();
        let live = dir.path("initialized.sqlite3");
        let backup = dir.path("backup.sqlite3");
        seed(&live);
        seed(&backup);
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = attempts.clone();
        let host = DatabaseHost::start_initialized(&live, move |app| {
            assert_eq!(app.profiles()?.len(), 1);
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert_eq!(wait_started(&host).state, "ready");
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        let preview = host.inspect(&backup).unwrap();
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "preview cannot reset preferences"
        );
        host.restore(&preview.candidate_id).unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn failed_initializer_never_publishes_ready_application_or_changes_profiles() {
        let dir = Directory::new();
        let live = dir.path("rejected.sqlite3");
        seed(&live);
        let host =
            DatabaseHost::start_initialized(&live, |_| Err(fail("Synthetic bundle rejection")));
        let status = wait_started(&host);
        assert_eq!(status.state, "recovery_required");
        assert!(host.with_application(|app| app.profiles()).is_err());
        assert_eq!(
            SnapshotApplication::open(&live)
                .unwrap()
                .profiles()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn asynchronous_startup_publishes_ready_service_without_dropping_it() {
        let dir = Directory::new();
        let live = dir.path("async.sqlite3");
        seed(&live);
        let host = DatabaseHost::start(&live);
        // Allow busy CI hosts to finish opening SQLite, and leave time between
        // status reads for the startup worker to acquire the publication lock.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let status = loop {
            let status = host.status().unwrap();
            if status.state != "checking" {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "asynchronous database startup did not finish within 60 seconds"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(status.state, "ready");
        assert_eq!(
            host.with_application(|app| app.profiles()).unwrap().len(),
            1
        );
    }

    #[test]
    fn corrupt_startup_can_inspect_and_restore_without_touching_original_until_explicit_action() {
        let dir = Directory::new();
        let database = dir.path("live.sqlite3");
        let backup = dir.path("good.sqlite3");
        seed(&backup);
        fs::write(&database, b"corrupt original").unwrap();
        fs::write(database.with_file_name("live.sqlite3-wal"), b"original WAL").unwrap();
        fs::write(database.with_file_name("live.sqlite3-shm"), b"original SHM").unwrap();
        let host = DatabaseHost::open(&database);
        assert_eq!(host.status().unwrap().state, "recovery_required");
        assert!(host.with_application(|app| app.status()).is_err());
        let preview = host.inspect(&backup).unwrap();
        assert_eq!(fs::read(&database).unwrap(), b"corrupt original");
        let receipt = host.restore(&preview.candidate_id).unwrap();
        assert_eq!(host.status().unwrap().state, "ready");
        assert_eq!(
            fs::read(Path::new(&receipt.recovery_directory).join("live.sqlite3-wal")).unwrap(),
            b"original WAL"
        );
        assert_eq!(
            fs::read(Path::new(&receipt.recovery_directory).join("live.sqlite3-shm")).unwrap(),
            b"original SHM"
        );
        assert_eq!(
            fs::read(Path::new(&receipt.recovery_directory).join("live.sqlite3")).unwrap(),
            b"corrupt original"
        );
        assert_eq!(
            host.with_application(|app| app.profiles()).unwrap().len(),
            1
        );
    }
    #[test]
    fn stale_preview_and_tampered_candidate_refuse_without_changing_healthy_database() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        seed(&live);
        let host = DatabaseHost::open(&live);
        let before = fs::read(&live).unwrap();
        let first = host.inspect(&live).unwrap();
        let second = host.inspect(&live).unwrap();
        assert!(host.restore(&first.candidate_id).is_err());
        let staged = host
            .candidate
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .path
            .clone();
        fs::write(&staged, b"tampered").unwrap();
        assert!(host.restore(&second.candidate_id).is_err());
        assert_eq!(fs::read(&live).unwrap(), before);
    }
    #[test]
    fn failed_candidate_install_rolls_back_files_and_preserves_ready_state() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        seed(&live);
        let host = DatabaseHost::open(&live);
        let before = fs::read(&live).unwrap();
        let preview = host.inspect(&live).unwrap();
        assert!(
            host.restore_using(&preview.candidate_id, |_, _| Err(std::io::Error::other(
                "injected rename failure"
            )))
            .is_err()
        );
        assert_eq!(fs::read(&live).unwrap(), before);
        assert_eq!(host.status().unwrap().state, "ready");
        assert_eq!(
            host.with_application(|app| app.profiles()).unwrap().len(),
            1
        );
        host.restore(&preview.candidate_id).unwrap();
        assert_eq!(host.status().unwrap().state, "ready");
    }
    #[test]
    fn explicit_restore_stops_watcher_and_backup_destination_is_never_overwritten() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        seed(&live);
        let host = DatabaseHost::open(&live);
        let input = dir.path("selected.txt");
        fs::write(&input, format!("ANAV1:{SNAPSHOT}")).unwrap();
        host.with_application(|app| {
            app.start_watch(&input, &crate::snapshots::FileInputFormat::Anav1Text)
        })
        .unwrap();
        let backup = dir.path("backup.sqlite3");
        host.backup(&backup).unwrap();
        let bytes = fs::read(&backup).unwrap();
        assert!(host.backup(&backup).is_err());
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        let preview = host.inspect(&backup).unwrap();
        host.restore(&preview.candidate_id).unwrap();
        assert!(
            !host
                .with_application(|app| app.watch_status())
                .unwrap()
                .active
        );
    }
    #[test]
    fn restore_waits_for_in_flight_application_lease() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        seed(&live);
        let host = Arc::new(DatabaseHost::open(&live));
        let preview = host.inspect(&live).unwrap();
        let (started_rx_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let read_host = host.clone();
        let reader = std::thread::spawn(move || {
            read_host
                .with_application(|_| {
                    started_rx_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap()
        });
        started_rx.recv().unwrap();
        let (completed_tx, completed_rx) = std::sync::mpsc::channel();
        let write_host = host.clone();
        let writer = std::thread::spawn(move || {
            write_host.restore(&preview.candidate_id).unwrap();
            completed_tx.send(()).unwrap();
        });
        assert!(
            completed_rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err()
        );
        release_tx.send(()).unwrap();
        reader.join().unwrap();
        completed_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        writer.join().unwrap();
    }
    #[test]
    fn oversized_staged_candidate_is_rejected_before_destructive_boundary() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        seed(&live);
        let host = DatabaseHost::open(&live);
        let preview = host.inspect(&live).unwrap();
        let staged = host
            .candidate
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .path
            .clone();
        fs::OpenOptions::new()
            .write(true)
            .open(staged)
            .unwrap()
            .set_len(azeroth_storage::recovery::MAX_BACKUP_BYTES + 1)
            .unwrap();
        assert!(host.restore(&preview.candidate_id).is_err());
        assert_eq!(host.status().unwrap().state, "ready");
    }
    #[test]
    fn pending_marker_directory_and_oversized_file_are_not_trusted_as_paths() {
        let dir = Directory::new();
        let live = dir.path("live.sqlite3");
        let marker = pending_path(&live);
        fs::create_dir(&marker).unwrap();
        assert!(pending_directory(&live).is_none());
        fs::remove_dir(&marker).unwrap();
        fs::write(&marker, vec![b'x'; 4097]).unwrap();
        assert!(pending_directory(&live).is_none());
        // Even an unreadable marker preserves the interrupted-restore refusal.
        let host = DatabaseHost::open(&live);
        assert_eq!(host.status().unwrap().state, "recovery_required");
        assert!(!live.exists());
    }
}
