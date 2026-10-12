//! M5.1: role profile structure. A profile is data: a weight per modelled
//! attribute, a label for where the weights came from, and the effects the
//! model knowingly does not cover. No real weights are defined in this
//! repository; a weight table needs an owner decision or a permitted source.
use super::GEAR_MODEL_VERSION;
use crate::{
    entities::{bounded, unique},
    error::{Result, Validate, Validated, invalid},
    types::*,
};
use serde::{Deserialize, Serialize};

/// Where a profile's weights come from. `SyntheticTest` weights are an
/// engineering assumption for tests, not game truth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WeightSource {
    UserPreference,
    DocumentedModel { source: SourceId },
    PermittedGuide { source: SourceId },
    SyntheticTest,
}

/// Weights are integer thousandths (1.000 is stored as 1000), so scoring is
/// exact integer arithmetic that a later Lua evaluator can reproduce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleProfileData {
    pub role: RoleId,
    pub version: ScoreModelVersion,
    pub weight_source: WeightSource,
    pub weights: RecordMap<AttributeId, SafeInteger>,
    pub known_unmodeled: Vec<Text>,
}
pub type RoleProfile = Validated<RoleProfileData>;
impl Validate for RoleProfileData {
    fn validate(&self) -> Result<()> {
        if self.version.as_str() != GEAR_MODEL_VERSION {
            return Err(invalid("Role profile names an unsupported score model"));
        }
        if self.weights.is_empty() {
            return Err(invalid("Role profile requires at least one weight"));
        }
        bounded(self.weights.len())?;
        bounded(self.known_unmodeled.len())?;
        unique(self.known_unmodeled.iter().map(Text::as_str))?;
        Ok(())
    }
}
