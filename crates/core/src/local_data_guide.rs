//! Private guide-research lineage; observations never prove universal eligibility.
use crate::{
    local_data::is_sha256, local_data_inputs::InputTuple, observation::Observation,
    types::DataContext,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuestObservation {
    pub input_sha256: String,
    pub quest_id: String,
    pub character_id: String,
    pub context: DataContext,
    pub property: String,
    pub value: Observation<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Observations {
    pub format_version: u32,
    pub candidates: Vec<QuestObservation>,
    pub conflicts: Vec<Vec<u32>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GuideFactsManifest {
    pub format_version: u32,
    pub classification: crate::local_data_catalog::DistributionClass,
    pub projection_sha256: String,
    pub catalog_sha256: String,
    pub catalog_tuple: InputTuple,
    pub input_sha256s: Vec<String>,
    pub facts_sha256: String,
    pub observations_sha256: String,
    pub verified_for_planning: bool,
    pub gaps: Vec<String>,
}
impl GuideFactsManifest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.catalog_tuple.validate()?;
        if self.format_version != 1
            || self.classification != crate::local_data_catalog::DistributionClass::LocalOnly
            || self.verified_for_planning
            || [
                &self.projection_sha256,
                &self.catalog_sha256,
                &self.facts_sha256,
                &self.observations_sha256,
            ]
            .iter()
            .any(|v| !is_sha256(v))
            || self.input_sha256s.len() > 16
            || self.input_sha256s.iter().any(|v| !is_sha256(v))
            || !self.input_sha256s.windows(2).all(|w| w[0] < w[1])
            || self.gaps.len() > 64
            || !self.gaps.windows(2).all(|w| w[0] < w[1])
            || self.gaps.iter().any(|v| {
                v.is_empty()
                    || v.len() > 120
                    || !v.bytes().all(|c| c.is_ascii_lowercase() || c == b'_')
            })
        {
            return Err("Invalid guide-research manifest");
        }
        Ok(())
    }
}
