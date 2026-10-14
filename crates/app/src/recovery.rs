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
fn serial() -> String {
    format!(
        "{}-{}",
        std::process::id(),
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
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseStatus {
    pub state: &'static str,
    pub error: Option<AppError>,
    pub schema_version: u32,
    pub restore_limit_bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct BackupPreview {
    pub candidate_id: String,
    pub inspection: RecoveryInspection,
}
#[derive(Clone, Debug, Serialize)]
pub struct RestoreReceipt {
    pub original_files_retained: bool,
    pub recovery_directory: String,
}
struct Candidate {
    id: String,
    path: PathBuf,
    hash: String,
}
struct Hosted {
    checking: bool,
    app: Option<SnapshotApplication>,
    error: Option<AppError>,
}
pub struct DatabaseHost {
    path: PathBuf,
    gate: RwLock<Hosted>,
    candidate: Mutex<Option<Candidate>>,
}
impl DatabaseHost {
    pub fn start(path: &Path) -> Arc<Self> {
        let host = Arc::new(Self {
            path: path.to_owned(),
            gate: RwLock::new(Hosted {
                checking: true,
                app: None,
                error: None,
            }),
            candidate: Mutex::new(None),
        });
        let worker = host.clone();
        let spawned = std::thread::Builder::new()
            .name("database-startup-check".into())
            .spawn(move || {
                let opened = Self::open(&worker.path);
                if let (Ok(mut live), Ok(mut checked)) = (worker.gate.write(), opened.gate.write())
                {
                    live.app = checked.app.take();
                    live.error = checked.error.take();
                    live.checking = false;
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
    pub fn open(path: &Path) -> Self {
        let opened = (|| {
            if path.exists() {
                let probe = path.with_file_name(format!("recovery-check-{}.sqlite3", serial()));
                let checked = reconstruct_backup(path, &probe);
                let _ = fs::remove_file(&probe);
                checked?;
            }
            SnapshotApplication::open(path)
        })();
        let hosted = match opened {
            Ok(app) => Hosted {
                checking: false,
                app: Some(app),
                error: None,
            },
            Err(error) => Hosted {
                checking: false,
                app: None,
                error: Some(error),
            },
        };
        Self {
            path: path.to_owned(),
            gate: RwLock::new(hosted),
            candidate: Mutex::new(None),
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
            SnapshotApplication::open(&self.path)
        })();
        match install {
            Ok(app) => {
                hosted.app = Some(app);
                hosted.error = None;
                let retained = !moved.is_empty();
                slot.take();
                Ok(RestoreReceipt {
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
                hosted.app = if rollback_ok && was_ready {
                    SnapshotApplication::open(&self.path).ok()
                } else {
                    None
                };
                hosted.error = if hosted.app.is_none() {
                    Some(fail("Restore failed; preserved originals require recovery"))
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
    fn corrupt_startup_can_inspect_and_restore_without_touching_original_until_explicit_action() {
        let dir = Directory::new();
        let database = dir.path("live.sqlite3");
        let backup = dir.path("good.sqlite3");
        seed(&backup);
        fs::write(&database, b"corrupt original").unwrap();
        let host = DatabaseHost::open(&database);
        assert_eq!(host.status().unwrap().state, "recovery_required");
        assert!(host.with_application(|app| app.status()).is_err());
        let preview = host.inspect(&backup).unwrap();
        assert_eq!(fs::read(&database).unwrap(), b"corrupt original");
        let receipt = host.restore(&preview.candidate_id).unwrap();
        assert_eq!(host.status().unwrap().state, "ready");
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
}
