//! Application DTOs and snapshot workflow. Call synchronous methods on a
//! blocking worker. The only watch is for an explicitly selected export file.
use crate::{AppStatus, CapabilityState, DataMode, atlas};
use azeroth_core::{
    contract::{AnyExchange, ContractKind, Envelope},
    entities::CharacterSnapshot,
    error::{AppError, ErrorCode, Result},
    observation::{ClockTrust, Freshness, FreshnessContext, freshness},
    types::{DataContext, DatasetNamespace, Millis, SnapshotId, UtcMillis},
};
use azeroth_ingest::{
    import::{ExportLocation, FileFormat, ImportOptions, ImportResult, import_file, import_text},
    watcher::{Cancellation, SelectedFileWatcher, WatchStatus},
};
use azeroth_storage::{
    HistoryReason, ImportOutcome, ImportSource, ProfileKey, SnapshotStore, SqliteStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    thread::JoinHandle,
};

#[derive(Clone, Debug, Serialize)]
pub struct ProfileDto {
    pub id: String,
    pub display_name: String,
    pub data_mode: DataMode,
    pub context: DataContext,
    pub fixture: bool,
    pub snapshot_count: u32,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ImportDisposition {
    Stored {
        previous_current: Option<SnapshotId>,
    },
    Duplicate,
    HistoryOnly {
        reason: HistoryDisposition,
    },
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryDisposition {
    Older,
    SamePosition,
    Incomparable,
}
#[derive(Clone, Debug, Serialize)]
pub struct ImportReceipt {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
    pub disposition: ImportDisposition,
    pub fixture: bool,
    pub namespace: DatasetNamespace,
    pub freshness: Freshness,
    pub data_mode: DataMode,
}
#[derive(Clone, Debug, Serialize)]
pub struct StoredSnapshot {
    pub envelope: Envelope<CharacterSnapshot>,
    pub profile_id: String,
    pub freshness: Freshness,
    pub data_mode: DataMode,
}
#[derive(Clone, Debug, Serialize)]
pub struct HistoryEntry {
    pub snapshot_id: SnapshotId,
    pub is_current: bool,
    pub freshness: Freshness,
    pub observed_at_utc_ms: Option<UtcMillis>,
    pub received_at_utc_ms: Option<UtcMillis>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FileInputFormat {
    Anav1Text,
    SavedVariables { global: String, key: String },
}
impl FileInputFormat {
    fn format(&self) -> FileFormat {
        match self {
            Self::Anav1Text => FileFormat::Anav1Text,
            Self::SavedVariables { global, key } => FileFormat::SavedVariables(ExportLocation {
                global: global.clone(),
                key: key.clone(),
            }),
        }
    }
    fn source(&self) -> ImportSource {
        match self {
            Self::Anav1Text => ImportSource::File,
            Self::SavedVariables { .. } => ImportSource::SavedVariables,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct WatchState {
    pub active: bool,
    pub status: Option<WatchStatus>,
    pub last_import: Option<ImportReceipt>,
    pub error: Option<AppError>,
}
struct RunningWatch {
    cancellation: Cancellation,
    thread: JoinHandle<()>,
}
struct State {
    store: SqliteStore,
    selected: Option<String>,
    watch: Option<RunningWatch>,
    watch_state: WatchState,
    watch_generation: u64,
}
/// Shared application service; SQLite access is serialized, and no filesystem
/// path or profile key is accepted as an arbitrary database query.
pub struct SnapshotApplication {
    state: Arc<Mutex<State>>,
    watch_control: Mutex<()>,
}
fn error(code: ErrorCode, message: &'static str) -> AppError {
    AppError::new(code, message, false)
}
fn invalid(message: &'static str) -> AppError {
    error(ErrorCode::InvalidInput, message)
}
fn context(now: Option<UtcMillis>) -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: now,
        clock_trust: if now.is_some() {
            ClockTrust::Trusted
        } else {
            ClockTrust::Uncertain
        },
        session: None,
        max_age_ms: Millis::new(1_800_000).expect("constant is valid"),
    }
}
fn options(now: Option<UtcMillis>) -> ImportOptions {
    ImportOptions {
        accepted_kinds: BTreeSet::from([ContractKind::CharacterSnapshot]),
        freshness: context(now),
    }
}
/// OS clock supplied only at the application boundary; missing time stays uncertain.
pub fn system_now() -> Option<UtcMillis> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .and_then(|n| UtcMillis::new(n).ok())
}
fn profile_id(key: &ProfileKey) -> Result<String> {
    let encoded =
        serde_json::to_vec(&(&key.character, &key.context, key.fixture)).map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Profile identity cannot be encoded",
            )
        })?;
    Ok(format!("stored-{:x}", Sha256::digest(encoded)))
}
fn resolve(store: &SqliteStore, id: &str) -> Result<ProfileKey> {
    if id.len() > 128 {
        return Err(invalid("No such profile"));
    }
    for profile in store.list_profiles()? {
        if profile_id(&profile.key)? == id {
            return Ok(profile.key);
        }
    }
    Err(invalid("No such profile"))
}
fn persist(
    state: &mut State,
    imported: ImportResult,
    source: ImportSource,
    now: Option<UtcMillis>,
) -> Result<ImportReceipt> {
    let AnyExchange::CharacterSnapshot(envelope) = imported.exchange else {
        return Err(invalid("Only character snapshots are stored"));
    };
    let key = ProfileKey::of(&envelope);
    let id = profile_id(&key)?;
    let outcome = state
        .store
        .import_character_snapshot(&envelope, source, now)?;
    let disposition = match outcome {
        ImportOutcome::Stored { previous_current } => {
            ImportDisposition::Stored { previous_current }
        }
        ImportOutcome::Duplicate => ImportDisposition::Duplicate,
        ImportOutcome::StoredHistoryOnly { reason } => ImportDisposition::HistoryOnly {
            reason: match reason {
                HistoryReason::Older => HistoryDisposition::Older,
                HistoryReason::SamePosition => HistoryDisposition::SamePosition,
                HistoryReason::Incomparable => HistoryDisposition::Incomparable,
            },
        },
    };
    if state.selected.is_none() {
        state.selected = Some(id.clone());
    }
    Ok(ImportReceipt {
        profile_id: id,
        snapshot_id: envelope.payload.id.clone(),
        disposition,
        fixture: envelope.fixture,
        namespace: key.context.namespace,
        freshness: freshness(&envelope.payload.capture, &context(now)),
        data_mode: DataMode::Stored,
    })
}
impl SnapshotApplication {
    pub fn open(path: &Path) -> Result<Self> {
        Self::with_store(SqliteStore::open(path)?)
    }
    pub fn in_memory() -> Result<Self> {
        Self::with_store(SqliteStore::open_in_memory()?)
    }
    fn with_store(store: SqliteStore) -> Result<Self> {
        let selected = store
            .list_profiles()?
            .first()
            .map(|p| profile_id(&p.key))
            .transpose()?;
        Ok(Self {
            watch_control: Mutex::new(()),
            state: Arc::new(Mutex::new(State {
                store,
                selected,
                watch: None,
                watch_state: WatchState::default(),
                watch_generation: 0,
            })),
        })
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>> {
        self.state.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Snapshot service is unavailable",
            )
        })
    }
    pub fn status(&self) -> Result<AppStatus> {
        let state = self.lock()?;
        Ok(AppStatus {
            version: env!("CARGO_PKG_VERSION"),
            data_mode: DataMode::Stored,
            active_profile: state.selected.clone(),
            capabilities: [
                ("addon_transport", CapabilityState::Unverified),
                ("live_position", CapabilityState::Unverified),
            ]
            .into(),
        })
    }
    pub fn profiles(&self) -> Result<Vec<ProfileDto>> {
        self.lock()?
            .store
            .list_profiles()?
            .into_iter()
            .map(|p| {
                Ok(ProfileDto {
                    id: profile_id(&p.key)?,
                    display_name: p.key.character.as_str().to_owned(),
                    data_mode: DataMode::Stored,
                    context: p.key.context,
                    fixture: p.key.fixture,
                    snapshot_count: p.snapshot_count,
                })
            })
            .collect()
    }
    pub fn select_profile(&self, id: &str) -> Result<()> {
        let mut state = self.lock()?;
        resolve(&state.store, id)?;
        state.selected = Some(id.to_owned());
        Ok(())
    }
    pub fn current(&self, id: &str, now: Option<UtcMillis>) -> Result<Option<StoredSnapshot>> {
        let state = self.lock()?;
        let key = resolve(&state.store, id)?;
        Ok(state
            .store
            .current_snapshot(&key)?
            .map(|envelope| StoredSnapshot {
                freshness: freshness(&envelope.payload.capture, &context(now)),
                envelope,
                profile_id: id.to_owned(),
                data_mode: DataMode::Stored,
            }))
    }
    pub fn history(
        &self,
        id: &str,
        limit: usize,
        now: Option<UtcMillis>,
    ) -> Result<Vec<HistoryEntry>> {
        let state = self.lock()?;
        let key = resolve(&state.store, id)?;
        Ok(state
            .store
            .snapshot_history(&key, limit)?
            .into_iter()
            .map(|s| HistoryEntry {
                snapshot_id: s.snapshot,
                is_current: s.is_current,
                freshness: freshness(&s.capture, &context(now)),
                observed_at_utc_ms: s.capture.observed_at_utc_ms,
                received_at_utc_ms: s.received_at_utc_ms,
            })
            .collect())
    }
    pub fn import_text(&self, text: &str, now: Option<UtcMillis>) -> Result<ImportReceipt> {
        self.import_result(
            import_text(text, &options(now)),
            ImportSource::Clipboard,
            now,
        )
    }
    pub fn import_file(
        &self,
        path: &Path,
        format: &FileInputFormat,
        now: Option<UtcMillis>,
    ) -> Result<ImportReceipt> {
        self.import_result(
            import_file(path, &format.format(), &options(now)),
            format.source(),
            now,
        )
    }
    fn import_result(
        &self,
        result: Result<ImportResult>,
        source: ImportSource,
        now: Option<UtcMillis>,
    ) -> Result<ImportReceipt> {
        let mut state = self.lock()?;
        match result {
            Ok(imported) => {
                let receipt = persist(&mut state, imported, source, now)?;
                state.selected = Some(receipt.profile_id.clone());
                Ok(receipt)
            }
            Err(err) => {
                state.store.record_failed_import(source, &err, now)?;
                Err(err)
            }
        }
    }
    pub fn markers(&self, now: Option<UtcMillis>) -> Result<Vec<atlas::SnapshotMarker>> {
        let id = self.lock()?.selected.clone();
        let Some(id) = id else {
            return Ok(vec![]);
        };
        let Some(snapshot) = self.current(&id, now)? else {
            return Ok(vec![]);
        };
        let mut marker = atlas::snapshot_marker_for(&snapshot.envelope, now)?;
        marker.data_mode = DataMode::Stored;
        marker.profile_id = id;
        Ok(vec![marker])
    }
    pub fn watch_status(&self) -> Result<WatchState> {
        Ok(self.lock()?.watch_state.clone())
    }
    pub fn stop_watch(&self) -> Result<WatchState> {
        let _control = self.watch_control.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Watcher controls unavailable",
            )
        })?;
        self.stop_watch_inner()
    }
    fn stop_watch_inner(&self) -> Result<WatchState> {
        let running = {
            let mut state = self.lock()?;
            state.watch_generation = state.watch_generation.wrapping_add(1);
            state.watch_state.active = false;
            state.watch_state.status = Some(WatchStatus::Cancelled);
            state.watch.take()
        };
        if let Some(running) = running {
            running.cancellation.cancel();
            running
                .thread
                .join()
                .map_err(|_| error(ErrorCode::ProviderUnavailable, "Watcher worker failed"))?;
        }
        self.watch_status()
    }
    pub fn start_watch(&self, path: &Path, format: &FileInputFormat) -> Result<WatchState> {
        let _control = self.watch_control.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Watcher controls unavailable",
            )
        })?;
        self.stop_watch_inner()?;
        let watcher = SelectedFileWatcher::new(path.to_owned(), format.format(), options(None));
        let source = format.source();
        let cancellation = Cancellation::default();
        let worker_cancel = cancellation.clone();
        let shared = self.state.clone();
        let mut state = self.lock()?;
        let generation = state.watch_generation;
        state.watch_state = WatchState {
            active: true,
            status: Some(WatchStatus::WaitingForStableFile),
            ..WatchState::default()
        };
        let thread = std::thread::Builder::new()
            .name("snapshot-file-watch".into())
            .spawn(move || {
                watcher.run(worker_cancel, |event| {
                    let Ok(mut state) = shared.lock() else {
                        return;
                    };
                    if state.watch_generation != generation {
                        return;
                    }
                    state.watch_state.status = Some(event.status);
                    if let Some(import) = event.import {
                        match persist(&mut state, import, source, system_now()) {
                            Ok(receipt) => {
                                state.watch_state.last_import = Some(receipt);
                                state.watch_state.error = None;
                            }
                            Err(err) => state.watch_state.error = Some(err),
                        }
                    }
                    if event.status == WatchStatus::Cancelled {
                        state.watch_state.active = false;
                    }
                });
            })
            .map_err(|_| {
                let err = error(
                    ErrorCode::ProviderUnavailable,
                    "Watcher worker could not start",
                );
                state.watch_state.active = false;
                state.watch_state.error = Some(err.clone());
                err
            })?;
        state.watch = Some(RunningWatch {
            cancellation,
            thread,
        });
        Ok(state.watch_state.clone())
    }
}
impl Drop for SnapshotApplication {
    fn drop(&mut self) {
        if let Ok(state) = self.state.lock()
            && let Some(watch) = &state.watch
        {
            watch.cancellation.cancel();
        }
    }
}
