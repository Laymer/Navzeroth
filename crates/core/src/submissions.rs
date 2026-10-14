//! Minimal local declarations. No names, free text, transport or confidence inference.
use crate::{
    error::{AppError, ErrorCode, Result, invalid},
    sources::{Redistribution, SourceRecord},
    types::{
        DataContext, DatasetRevision, EvidenceId, ItemId, QuestId, Sequence, SourceId, UtcMillis,
    },
};
use serde::{Deserialize, Serialize};
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024;
pub const EXPORT_TRANSFORMATION: &str = "local_observation_export.v1";
/// A reported value, not an authenticated or automatically captured observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Reported<T> {
    Known(T),
    Unknown,
    Unsupported,
    Restricted,
    Stale,
    NotApplicable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredQuestState {
    NotStarted,
    Accepted,
    Completed,
    TurnedIn,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalPayload {
    QuestState {
        quest: QuestId,
        state: Reported<DeclaredQuestState>,
    },
    Price {
        item: ItemId,
        copper: Reported<Sequence>,
    },
    AcquisitionOutcome {
        item: ItemId,
        acquired: Reported<bool>,
        opportunities: Reported<Sequence>,
        successes: Reported<Sequence>,
        group_eligible: Reported<bool>,
        personally_received: Reported<bool>,
    },
}
impl LocalPayload {
    pub fn policy_token(&self) -> &'static str {
        match self {
            Self::QuestState { .. } => "local_observation.quest_state.v1",
            Self::Price { .. } => "local_observation.price.v1",
            Self::AcquisitionOutcome { .. } => "local_observation.acquisition_outcome.v1",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalObservation {
    pub schema_version: u32,
    pub context: DataContext,
    pub fixture: bool,
    pub base_revision: DatasetRevision,
    pub source: SourceId,
    pub evidence: EvidenceId,
    pub observed_at_utc_ms: Option<UtcMillis>,
    pub payload: LocalPayload,
}
impl LocalObservation {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(AppError::new(
                ErrorCode::UnsupportedSchema,
                "Unsupported local observation schema",
                false,
            ));
        }
        if let LocalPayload::AcquisitionOutcome {
            opportunities: Reported::Known(n),
            successes: Reported::Known(s),
            ..
        } = &self.payload
            && s.get() > n.get()
        {
            return Err(invalid("Successes exceed declared opportunities"));
        }
        Ok(())
    }
}
pub fn decode_local_observation(text: &str) -> Result<LocalObservation> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Local observation exceeds 16 KiB",
            false,
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| invalid("Invalid local observation data"))?;
    let result: LocalObservation =
        serde_json::from_str(text).map_err(|_| invalid("Invalid local observation data"))?;
    result.validate()?;
    // Nested shared domain types may ignore unknown fields. Refuse those too;
    // this local format has no extension/reflection or personal metadata surface.
    if serde_json::to_value(&result).map_err(|_| invalid("Invalid local observation data"))?
        != value
    {
        return Err(invalid("Local observation contains unsupported fields"));
    }
    Ok(result)
}
/// Recorded policy only; this does not independently authenticate legal permission.
pub fn permits_local_export(source: &SourceRecord, observation: &LocalObservation) -> bool {
    source.id == observation.source
        && source.redistribution.value() == Some(&Redistribution::Permitted)
        && source
            .license
            .value()
            .is_some_and(|s| !s.as_str().trim().is_empty())
        && source
            .attribution
            .value()
            .is_some_and(|s| !s.as_str().trim().is_empty())
        && source
            .editions
            .value()
            .is_some_and(|v| v.contains(&observation.context.build.edition))
        && source.verified_revision.value() == Some(&observation.base_revision)
        && source.allowed_fields.value().is_some_and(|v| {
            v.iter()
                .any(|s| s.as_str() == observation.payload.policy_token())
        })
        && source
            .transformations
            .value()
            .is_some_and(|v| v.iter().any(|s| s.as_str() == EXPORT_TRANSFORMATION))
}
