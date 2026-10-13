//! Application DTOs and snapshot workflow. Call synchronous methods on a
//! blocking worker. The only watch is for an explicitly selected export file.
use crate::{AppStatus, CapabilityState, DataMode, atlas};
use azeroth_core::{
    contract::{AnyExchange, ContractKind, Envelope},
    entities::CharacterSnapshot,
    error::{AppError, ErrorCode, Result},
    observation::{ClockTrust, Freshness, FreshnessContext, freshness},
    types::{DataContext, DatasetNamespace, Millis, PartyId, SnapshotId, UtcMillis},
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
pub struct StoredParty {
    pub id: PartyId,
    pub data_mode: DataMode,
    pub participants: Vec<PartyParticipant>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PartyParticipant {
    pub profile_id: String,
    pub character: azeroth_core::types::CharacterId,
    /// None means no imported current snapshot, including a deleted profile.
    pub snapshot: Option<StoredSnapshot>,
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
    /// Explicit local configuration; never imports or alters participant state.
    pub fn save_party(&self, id: &PartyId, profile_ids: &[String]) -> Result<()> {
        if profile_ids.is_empty() || profile_ids.len() > azeroth_storage::party::MAX_PARTICIPANTS {
            return Err(invalid("Party participant count is out of range"));
        }
        let mut state = self.lock()?;
        let participants = profile_ids
            .iter()
            .map(|id| resolve(&state.store, id))
            .collect::<Result<Vec<_>>>()?;
        state
            .store
            .save_party(&azeroth_storage::party::PartyProfile {
                id: id.clone(),
                participants,
            })
    }
    pub fn party_ids(&self) -> Result<Vec<PartyId>> {
        self.lock()?.store.party_ids()
    }
    pub(crate) fn with_profile_store_mut<T>(
        &self,
        id: &str,
        call: impl FnOnce(&mut SqliteStore, &ProfileKey) -> Result<T>,
    ) -> Result<T> {
        let mut state = self.lock()?;
        let key = resolve(&state.store, id)?;
        call(&mut state.store, &key)
    }
    pub fn planning_catalog(&self, profile_id: &str) -> Result<crate::planning::PlanningCatalog> {
        let state = self.lock()?;
        let key = resolve(&state.store, profile_id)?;
        let snapshot = state.store.current_snapshot(&key)?;
        let content =
            state
                .store
                .active_content_revision(&azeroth_storage::content::ContentScope {
                    context: key.context,
                    fixture: key.fixture,
                })?;
        Ok(crate::planning::PlanningCatalog {
            snapshot_id: snapshot.map(|s| s.payload.id.clone()),
            fixture: key.fixture,
            revision: content
                .as_ref()
                .map(|c| c.envelope.payload.revision.clone()),
            nodes: content
                .as_ref()
                .map(|c| c.envelope.payload.nodes.clone())
                .unwrap_or_default(),
            objectives: content
                .map(|c| c.envelope.payload.objectives.clone())
                .unwrap_or_default(),
        })
    }
    pub fn plan_session(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<crate::planning::PlanningReport> {
        self.with_planning(text, now, crate::planning::calculate)
    }
    pub fn export_plan(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<crate::plan_export::PlanExport> {
        self.with_planning(text, now, |request, snapshot, content, clock| {
            let report = crate::planning::calculate(request, snapshot, content, clock)?;
            crate::plan_export::compile(&report, snapshot, content, clock)
        })
    }
    fn with_planning<T>(
        &self,
        text: &str,
        now: Option<UtcMillis>,
        calculate: impl FnOnce(
            crate::planning::PlanningRequest,
            &Envelope<CharacterSnapshot>,
            &azeroth_storage::content::ContentRevision,
            &FreshnessContext,
        ) -> Result<T>,
    ) -> Result<T> {
        if text.len() > crate::planning::MAX_REQUEST_BYTES {
            return Err(error(
                ErrorCode::ImportTooLarge,
                "Planning request exceeds size limit",
            ));
        }
        let request: crate::planning::PlanningRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid planning request"))?;
        let state = self.lock()?;
        let key = resolve(&state.store, &request.profile_id)?;
        let snapshot = state
            .store
            .current_snapshot(&key)?
            .ok_or_else(|| invalid("Profile has no current snapshot"))?;
        if snapshot.payload.id != request.snapshot_id {
            return Err(invalid("Snapshot changed; refresh planning inputs"));
        }
        if !matches!(
            freshness(&snapshot.payload.capture, &context(now)),
            Freshness::Fresh { .. }
        ) {
            return Err(error(
                ErrorCode::SnapshotOutdated,
                "Planning requires a fresh capture with trusted age",
            ));
        }
        let content = state
            .store
            .active_content_revision(&azeroth_storage::content::ContentScope {
                context: key.context,
                fixture: key.fixture,
            })?
            .ok_or_else(|| {
                error(
                    ErrorCode::DatasetIncomplete,
                    "No active content for this profile",
                )
            })?;
        if content.envelope.payload.revision != request.revision {
            return Err(invalid("Active content changed; refresh planning inputs"));
        }
        calculate(request, &snapshot, &content, &context(now))
    }
    pub fn stage_content(&self, text: &str) -> Result<crate::content::ContentInspection> {
        if text.len() > azeroth_storage::content::MAX_CONTENT_REVISION_BYTES {
            return Err(error(
                ErrorCode::ImportTooLarge,
                "Content revision limit exceeded",
            ));
        }
        let content: azeroth_storage::content::ContentRevision =
            serde_json::from_str(text).map_err(|_| invalid("Invalid content revision JSON"))?;
        self.lock()?.store.stage_content_revision(&content)?;
        Ok(crate::content::inspect(content))
    }
    pub fn active_content(
        &self,
        profile_id: &str,
    ) -> Result<Option<crate::content::ContentInspection>> {
        let state = self.lock()?;
        let key = resolve(&state.store, profile_id)?;
        let scope = azeroth_storage::content::ContentScope {
            context: key.context,
            fixture: key.fixture,
        };
        Ok(state
            .store
            .active_content_revision(&scope)?
            .map(crate::content::inspect))
    }
    pub fn activate_content(
        &self,
        profile_id: &str,
        revision: &azeroth_core::types::DatasetRevision,
    ) -> Result<crate::content::ContentInspection> {
        let mut state = self.lock()?;
        let key = resolve(&state.store, profile_id)?;
        let key = azeroth_storage::content::ContentRevisionKey {
            scope: azeroth_storage::content::ContentScope {
                context: key.context,
                fixture: key.fixture,
            },
            revision: revision.clone(),
        };
        state.store.activate_content_revision(&key)?;
        let content = state
            .store
            .content_revision(&key)?
            .ok_or_else(|| invalid("Content revision is not stored"))?;
        Ok(crate::content::inspect(content))
    }
    pub fn rollback_content(&self, profile_id: &str) -> Result<crate::content::ContentRollback> {
        let mut state = self.lock()?;
        let key = resolve(&state.store, profile_id)?;
        let scope = azeroth_storage::content::ContentScope {
            context: key.context,
            fixture: key.fixture,
        };
        let changed = state
            .store
            .rollback_content_revision(&scope)?
            .is_some_and(|result| result.changed);
        Ok(crate::content::ContentRollback {
            changed,
            active: state
                .store
                .active_content_revision(&scope)?
                .map(crate::content::inspect),
        })
    }
    pub fn party_quests(&self, id: &PartyId) -> Result<crate::party::PartyQuestCatalog> {
        let state = self.lock()?;
        let party = state
            .store
            .party(id)?
            .ok_or_else(|| invalid("No such party"))?;
        let key = &party.participants[0];
        let scope = azeroth_storage::content::ContentScope {
            context: key.context.clone(),
            fixture: key.fixture,
        };
        let content = state.store.active_content_revision(&scope)?;
        Ok(crate::party::PartyQuestCatalog {
            context: key.context.clone(),
            fixture: key.fixture,
            revision: content
                .as_ref()
                .map(|c| c.envelope.payload.revision.clone()),
            quests: content
                .map(|c| {
                    c.envelope
                        .payload
                        .quests
                        .iter()
                        .map(|q| q.id.clone())
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
    pub fn party_eligibility(
        &self,
        id: &PartyId,
        revision: &azeroth_core::types::DatasetRevision,
        quest: &azeroth_core::types::QuestId,
        now: Option<UtcMillis>,
    ) -> Result<crate::party::PartyQuestReport> {
        use crate::party::*;
        use azeroth_core::{
            eligibility::{EligibilityInputs, PartyOutcome, evaluate_participant, evaluate_party},
            entities::{PartySnapshot, PartySnapshotData},
            evaluation::StalePolicy,
            predicate::RuleLimits,
        };
        let state = self.lock()?;
        let party = state
            .store
            .party(id)?
            .ok_or_else(|| invalid("No such party"))?;
        let key = &party.participants[0];
        let scope = azeroth_storage::content::ContentScope {
            context: key.context.clone(),
            fixture: key.fixture,
        };
        let content = state
            .store
            .active_content_revision(&scope)?
            .ok_or_else(|| {
                error(
                    ErrorCode::DatasetIncomplete,
                    "No active content for this party",
                )
            })?;
        if &content.envelope.payload.revision != revision {
            return Err(invalid("Active content changed; refresh the quest list"));
        }
        let definition = content
            .envelope
            .payload
            .quests
            .iter()
            .find(|q| &q.id == quest)
            .ok_or_else(|| invalid("Quest is not in the active content"))?;
        let snapshots = party
            .participants
            .iter()
            .map(|key| state.store.current_snapshot(key))
            .collect::<Result<Vec<_>>>()?;
        let available = snapshots
            .iter()
            .flatten()
            .map(|e| e.payload.clone())
            .collect::<Vec<_>>();
        let domain_party = if available.is_empty() {
            None
        } else {
            Some(PartySnapshot::try_new(PartySnapshotData {
                id: id.clone(),
                context: key.context.clone(),
                participants: available,
            })?)
        };
        let clock = context(now);
        let facts = std::collections::BTreeMap::new();
        let inputs = EligibilityInputs {
            definition,
            access: None,
            exclusive_groups: &[],
            facts: &facts,
            freshness: &clock,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        };
        let mut participants = Vec::new();
        let mut all_fresh = true;
        for (key, snapshot) in party.participants.iter().zip(&snapshots) {
            let capture_freshness = snapshot
                .as_ref()
                .map(|s| freshness(&s.payload.capture, &clock));
            all_fresh &= matches!(capture_freshness, Some(Freshness::Fresh { .. }));
            let result = snapshot
                .as_ref()
                .map(|s| evaluate_participant(&inputs, &s.payload, domain_party.as_ref()))
                .transpose()?;
            participants.push(ParticipantAssessment {
                character: key.character.clone(),
                snapshot_id: snapshot.as_ref().map(|s| s.payload.id.clone()),
                freshness: capture_freshness,
                assessment: result.map(assessment),
            });
        }
        let joint_status = if !all_fresh {
            JointStatus::MissingOrOldSnapshots
        } else {
            match evaluate_party(
                &inputs,
                domain_party.as_ref().expect("all fresh requires snapshots"),
            )?
            .outcome
            {
                PartyOutcome::JointlyEligible => JointStatus::JointlyEligible,
                PartyOutcome::EligibleForSome { .. } => JointStatus::EligibleForSome,
                PartyOutcome::AllBehind { .. } => JointStatus::AllBehind,
                PartyOutcome::Blocked { .. } => JointStatus::Blocked,
                PartyOutcome::Unknown { .. } => JointStatus::Unknown,
            }
        };
        Ok(PartyQuestReport {
            context: key.context.clone(),
            fixture: key.fixture,
            revision: revision.clone(),
            quest: quest.clone(),
            joint_status,
            participants,
        })
    }
    pub fn delete_party(&self, id: &PartyId) -> Result<bool> {
        self.lock()?.store.delete_party(id)
    }
    /// One consistent service view; freshness is computed separately per capture.
    pub fn party(&self, id: &PartyId, now: Option<UtcMillis>) -> Result<Option<StoredParty>> {
        let state = self.lock()?;
        let Some(party) = state.store.party(id)? else {
            return Ok(None);
        };
        let participants = party
            .participants
            .into_iter()
            .map(|key| {
                let id = profile_id(&key)?;
                let snapshot = state
                    .store
                    .current_snapshot(&key)?
                    .map(|envelope| StoredSnapshot {
                        freshness: freshness(&envelope.payload.capture, &context(now)),
                        envelope,
                        profile_id: id.clone(),
                        data_mode: DataMode::Stored,
                    });
                Ok(PartyParticipant {
                    profile_id: id,
                    character: key.character,
                    snapshot,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(StoredParty {
            id: party.id,
            data_mode: DataMode::Stored,
            participants,
        }))
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
    pub fn historical(
        &self,
        id: &str,
        selected: &SnapshotId,
        now: Option<UtcMillis>,
    ) -> Result<Option<StoredSnapshot>> {
        let state = self.lock()?;
        let key = resolve(&state.store, id)?;
        Ok(state
            .store
            .historical_snapshot(&key, selected)?
            .map(|envelope| StoredSnapshot {
                freshness: freshness(&envelope.payload.capture, &context(now)),
                envelope,
                profile_id: id.to_owned(),
                data_mode: DataMode::Stored,
            }))
    }
    pub fn promote(
        &self,
        id: &str,
        selected: &SnapshotId,
        expected: &SnapshotId,
        reason: &str,
        now: Option<UtcMillis>,
    ) -> Result<()> {
        let now = now.ok_or_else(|| invalid("Operation time unavailable"))?;
        let mut state = self.lock()?;
        let key = resolve(&state.store, id)?;
        state
            .store
            .promote_snapshot(&key, selected, expected, reason, now)
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
