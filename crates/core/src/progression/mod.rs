//! Pure progression state primitives. Imported and manual observations retain
//! their own metadata; neither lookup nor binding changes observed game state.
use crate::{
    entities::CharacterSnapshot,
    error::{Result, invalid},
    observation::{
        Freshness, FreshnessContext, Observation, ObservationMetadata, ObservationStatus, freshness,
    },
    types::{
        CharacterId, DataContext, DatasetRevision, RecordMap, Sha256Hex, SnapshotId, SourceId,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    Partial,
}

/// Availability and capture metadata live on the enclosing Observation. An
/// unavailable enumeration is never represented as an empty complete map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Enumeration<T> {
    pub completeness: Completeness,
    pub entries: RecordMap<SourceId, T>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lookup<'a, T> {
    Present(&'a T),
    Absent,
    Unknown,
    Unavailable(ObservationStatus),
    NotFresh(Freshness),
}

/// Missing companions/partial entries cannot prove absence. Even a complete
/// enumeration cannot establish current absence if its time is stale/uncertain.
pub fn lookup<'a, T>(
    observation: Option<&'a Observation<Enumeration<T>>>,
    key: &SourceId,
    time: &FreshnessContext,
) -> Lookup<'a, T> {
    let Some(observation) = observation else {
        return Lookup::Unknown;
    };
    let Some(enumeration) = observation.value() else {
        return Lookup::Unavailable(observation.status());
    };
    let age = freshness(observation.metadata(), time);
    if !matches!(age, Freshness::Fresh { .. }) {
        return Lookup::NotFresh(age);
    }
    match enumeration.entries.get(key) {
        Some(value) => Lookup::Present(value),
        None if enumeration.completeness == Completeness::Complete => Lookup::Absent,
        None => Lookup::Unknown,
    }
}

/// Exact predecessor identity for progression companions and saved advice.
/// The I/O layer supplies the persisted snapshot hash; this module does not hash
/// or normalize imported bytes and never accepts a nearest-build substitution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotBinding {
    pub snapshot: SnapshotId,
    pub character: CharacterId,
    pub context: DataContext,
    pub source_revision: DatasetRevision,
    pub capture: ObservationMetadata,
    pub snapshot_sha256: Sha256Hex,
}

impl SnapshotBinding {
    pub fn from_snapshot(snapshot: &CharacterSnapshot, snapshot_sha256: Sha256Hex) -> Self {
        Self {
            snapshot: snapshot.id.clone(),
            character: snapshot.character.clone(),
            context: snapshot.context.clone(),
            source_revision: snapshot.source_revision.clone(),
            capture: snapshot.capture.clone(),
            snapshot_sha256,
        }
    }

    pub fn ensure_matches(
        &self,
        snapshot: &CharacterSnapshot,
        snapshot_sha256: &Sha256Hex,
    ) -> Result<()> {
        self.context.ensure_compatible(&snapshot.context)?;
        if self.snapshot != snapshot.id
            || self.character != snapshot.character
            || self.source_revision != snapshot.source_revision
            || self.capture != snapshot.capture
            || &self.snapshot_sha256 != snapshot_sha256
        {
            return Err(invalid(
                "Progression state belongs to a different character snapshot",
            ));
        }
        Ok(())
    }
}

mod snapshot;
pub use snapshot::*;
