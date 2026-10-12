//! Bounded scoring-only inputs for the synthetic Lua foundation (M5.6).
//! These are internal library DTOs, not new exchange schemas. Compilation does
//! not establish equipment eligibility or configuration legality.
use super::{GearCandidate, RoleProfile, WeightSource, score::to_fixed_point};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::{Observation, StateSource},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const LUA_GEAR_FORMAT: &str = "lua-gear-input-v1";
pub const MAX_LUA_GEAR_BYTES: usize = 32 * 1024;
pub const MAX_LUA_GEAR_CANDIDATES: usize = 64;
pub const MAX_LUA_GEAR_WEIGHTS: usize = 64;
pub const MAX_LUA_GEAR_ATTRIBUTES: usize = 128;
pub const MAX_LUA_GEAR_EFFECTS: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LuaGearCandidate {
    pub key: ItemKey,
    /// Attribute values already rounded by Rust to integer thousandths.
    pub attributes_milli: Observation<RecordMap<AttributeId, SafeInteger>>,
    pub effects: Observation<Vec<Text>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LuaGearInputData {
    pub format: String,
    pub fixture: bool,
    pub context: DataContext,
    pub profile: RoleProfile,
    pub candidates: Vec<LuaGearCandidate>,
}
pub type LuaGearInput = Validated<LuaGearInputData>;

impl Validate for LuaGearInputData {
    fn validate(&self) -> Result<()> {
        if self.format != LUA_GEAR_FORMAT
            || !self.fixture
            || self.context.namespace != DatasetNamespace::Demo
            || self.context.build.build.as_str() != "SYNTHETIC"
            || self.context.build.interface.as_str() != "SYNTHETIC"
            || self.profile.weight_source != WeightSource::SyntheticTest
        {
            return Err(invalid(
                "Lua gear foundation accepts explicitly synthetic inputs only",
            ));
        }
        if self.candidates.len() > MAX_LUA_GEAR_CANDIDATES
            || self.profile.weights.len() > MAX_LUA_GEAR_WEIGHTS
            || self.profile.known_unmodeled.len() > MAX_LUA_GEAR_EFFECTS
        {
            return Err(invalid("Lua gear input exceeds collection limits"));
        }
        let mut keys = BTreeSet::new();
        for candidate in &self.candidates {
            self.context.ensure_compatible(&candidate.key.context)?;
            for source in [
                candidate.attributes_milli.metadata().source,
                candidate.effects.metadata().source,
            ] {
                if matches!(
                    source,
                    StateSource::AddonSnapshot
                        | StateSource::ManualEntry
                        | StateSource::OfficialApi
                ) {
                    return Err(invalid(
                        "Synthetic Lua gear input cannot claim live observations",
                    ));
                }
            }
            if !keys.insert(&candidate.key) {
                return Err(invalid("Lua gear candidate keys must be unique"));
            }
            if candidate
                .attributes_milli
                .value()
                .is_some_and(|a| a.len() > MAX_LUA_GEAR_ATTRIBUTES)
                || candidate
                    .effects
                    .value()
                    .is_some_and(|e| e.len() > MAX_LUA_GEAR_EFFECTS)
            {
                return Err(invalid("Lua gear candidate exceeds collection limits"));
            }
        }
        if serde_json::to_vec(self)
            .map_err(|_| invalid("Cannot encode Lua gear input"))?
            .len()
            > MAX_LUA_GEAR_BYTES
        {
            return Err(invalid("Lua gear input exceeds the addon byte budget"));
        }
        Ok(())
    }
}

/// Pure bounded compilation: full key identity, status and metadata are retained.
/// Integer conversion happens once per attribute, before serialization.
pub fn compile_synthetic(
    context: &DataContext,
    profile: &RoleProfile,
    candidates: &[GearCandidate],
) -> Result<LuaGearInput> {
    if candidates.len() > MAX_LUA_GEAR_CANDIDATES || profile.weights.len() > MAX_LUA_GEAR_WEIGHTS {
        return Err(invalid("Lua gear input exceeds collection limits"));
    }
    let mut compiled = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let original = &candidate.item.attributes;
        let attributes_milli = match original.value() {
            Some(attributes) => {
                if attributes.len() > MAX_LUA_GEAR_ATTRIBUTES {
                    return Err(invalid("Lua gear candidate exceeds attribute limits"));
                }
                let values: Vec<_> = attributes
                    .iter()
                    .map(|(key, value)| {
                        Ok((key.clone(), SafeInteger::new(to_fixed_point(*value)?)?))
                    })
                    .collect::<Result<_>>()?;
                Observation::known(RecordMap::new(values)?, original.metadata().clone())?
            }
            None => Observation::unavailable(original.status(), original.metadata().clone())?,
        };
        compiled.push(LuaGearCandidate {
            key: candidate.key().clone(),
            attributes_milli,
            effects: candidate.item.effects.clone(),
        });
    }
    compiled.sort_by(|a, b| a.key.cmp(&b.key));
    LuaGearInput::try_new(LuaGearInputData {
        format: LUA_GEAR_FORMAT.into(),
        fixture: true,
        context: context.clone(),
        profile: profile.clone(),
        candidates: compiled,
    })
}

pub fn encode(input: &LuaGearInput) -> Result<String> {
    serde_json::to_string(input).map_err(|_| invalid("Cannot encode Lua gear input"))
}
