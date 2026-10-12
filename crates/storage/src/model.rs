//! Plain result types and the store trait. Nothing here mentions SQLite, so
//! the application layer can depend on the trait alone.
use azeroth_core::{
    contract::Envelope,
    entities::{CharacterSnapshot, SnapshotRef},
    error::{AppError, Result},
    observation::{Observation, ObservationMetadata},
    types::{CharacterId, DataContext, PreferenceId, Sha256Hex, SnapshotId, UtcMillis},
};
use std::collections::BTreeMap;

/// Largest `limit` accepted by `snapshot_history`.
pub const MAX_HISTORY_LIMIT: usize = 256;
/// History kept per profile; the oldest non-current snapshots are pruned
/// (insertion order) beyond it, so the database never grows without bound.
pub const MAX_SNAPSHOTS_PER_PROFILE: usize = 256;
/// Audit rows kept in `imports` (newest by insertion).
pub const MAX_IMPORT_ROWS: usize = 1024;
/// Profiles the store accepts.
pub const MAX_PROFILES: usize = 1024;
/// Stored preferences per profile (the core 1024 collection bound).
pub const MAX_PREFERENCES_PER_PROFILE: usize = 1024;
/// Largest number of observation rows one snapshot may produce.
pub const MAX_OBSERVATION_ROWS: usize = 65_536;

/// Identifies one profile. Characters of different editions, builds,
/// interfaces or namespaces, and fixture versus non-fixture data, are separate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProfileKey {
    pub character: CharacterId,
    pub context: DataContext,
    pub fixture: bool,
}
impl ProfileKey {
    /// The profile an envelope belongs to.
    pub fn of(envelope: &Envelope<CharacterSnapshot>) -> Self {
        Self {
            character: envelope.payload.character.clone(),
            context: envelope.data_context(),
            fixture: envelope.fixture,
        }
    }
}

/// Where an import came from. Stored for the audit trail only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportSource {
    File,
    Clipboard,
    SavedVariables,
    Fixture,
}
impl ImportSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Clipboard => "clipboard",
            Self::SavedVariables => "saved_variables",
            Self::Fixture => "fixture",
        }
    }
}

/// Why a stored snapshot did not become current.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryReason {
    /// Provably older than the current snapshot.
    Older,
    /// Same position as the current snapshot but different content.
    SamePosition,
    /// No rule orders it against the current snapshot (untrusted clocks or
    /// different sessions); it is not assumed to be newer.
    Incomparable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportOutcome {
    /// Stored and now current. `previous_current` is the snapshot it replaced.
    Stored {
        previous_current: Option<SnapshotId>,
    },
    /// Stored in history; the current snapshot is unchanged.
    StoredHistoryOnly { reason: HistoryReason },
    /// Identical content was already stored; nothing changed.
    Duplicate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotSummary {
    pub snapshot: SnapshotId,
    pub content_sha256: Sha256Hex,
    pub is_current: bool,
    pub capture: ObservationMetadata,
    pub received_at_utc_ms: Option<UtcMillis>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSummary {
    pub key: ProfileKey,
    pub current: Option<SnapshotRef>,
    pub snapshot_count: u32,
}

/// Snapshot persistence as seen by the application. Signatures use only core
/// types and the plain types above. Implementations are synchronous.
pub trait SnapshotStore {
    /// Stores a validated snapshot idempotently. On error nothing is stored
    /// and the current snapshot is unchanged.
    fn import_character_snapshot(
        &mut self,
        envelope: &Envelope<CharacterSnapshot>,
        source: ImportSource,
        received_at: Option<UtcMillis>,
    ) -> Result<ImportOutcome>;

    /// Records that an upstream decode failed. Only the error code and its
    /// static message are kept; snapshots are never touched.
    fn record_failed_import(
        &mut self,
        source: ImportSource,
        error: &AppError,
        received_at: Option<UtcMillis>,
    ) -> Result<()>;

    /// The newest snapshot of the profile, re-validated.
    fn current_snapshot(&self, key: &ProfileKey) -> Result<Option<Envelope<CharacterSnapshot>>>;

    /// Stored snapshots, most recently stored first. `limit` must be in
    /// `1..=MAX_HISTORY_LIMIT`.
    fn snapshot_history(&self, key: &ProfileKey, limit: usize) -> Result<Vec<SnapshotSummary>>;

    fn list_profiles(&self) -> Result<Vec<ProfileSummary>>;

    /// Creates or replaces one profile preference, creating the profile if needed.
    fn set_preference(
        &mut self,
        key: &ProfileKey,
        id: &PreferenceId,
        value: &Observation<bool>,
    ) -> Result<()>;

    fn preferences(&self, key: &ProfileKey) -> Result<BTreeMap<PreferenceId, Observation<bool>>>;

    /// Deletes one profile with its snapshots and preferences. Other profiles
    /// are untouched. Returns whether the profile existed.
    fn delete_profile(&mut self, key: &ProfileKey) -> Result<bool>;
}
