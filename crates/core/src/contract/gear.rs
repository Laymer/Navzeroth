//! A set of gear comparisons for one snapshot, dataset revision and score model.
use super::{ContractKind, EnvelopeHeader, Payload, same_context};
use crate::{
    entities::{GearRecommendation, SnapshotRef, bounded},
    error::{Result, Validate, Validated, invalid},
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "GearRecommendationSet")]
pub struct GearRecommendationSetData {
    pub snapshot: SnapshotRef,
    pub dataset_revision: DatasetRevision,
    pub score_model_version: ScoreModelVersion,
    /// Empty means no recommendation was produced, not that none exists.
    pub recommendations: Vec<GearRecommendation>,
}
pub type GearRecommendationSet = Validated<GearRecommendationSetData>;
impl Validate for GearRecommendationSetData {
    fn validate(&self) -> Result<()> {
        bounded(self.recommendations.len())?;
        for r in &self.recommendations {
            if r.snapshot != self.snapshot
                || r.dataset_revision != self.dataset_revision
                || r.score_model_version != self.score_model_version
            {
                return Err(invalid(
                    "Recommendation snapshot, dataset or score model differs from its set",
                ));
            }
        }
        Ok(())
    }
}
impl Payload for GearRecommendationSet {
    const KIND: ContractKind = ContractKind::GearRecommendations;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        same_context(header, &self.snapshot.context)?;
        for r in &self.recommendations {
            same_context(header, &r.context)?;
        }
        let deps = header.dependencies;
        if deps.dataset_revision.as_ref() != Some(&self.dataset_revision)
            || deps.score_model_version.as_ref() != Some(&self.score_model_version)
            || deps.ruleset_version.is_some()
            || deps.snapshots.as_slice() != std::slice::from_ref(&self.snapshot)
        {
            return Err(invalid("Gear dependencies disagree with the payload"));
        }
        Ok(())
    }
}
