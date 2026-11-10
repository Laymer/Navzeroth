//! Companion observations, not gameplay facts or inferred player state.
use super::{Enumeration, SnapshotBinding};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::{Observation, ObservationMetadata},
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfessionState {
    pub skill: Observation<u16>,
    pub maximum_skill: Observation<u16>,
    pub rank: Observation<SourceId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InventoryQuantity {
    pub item: ItemKey,
    /// Aggregated quantity within this one explicit scope. Never a bag index.
    pub quantity: Observation<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementDuration {
    Permanent,
    Temporary,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedEnhancement {
    pub duration: Observation<EnhancementDuration>,
    /// Unreadable expiry remains unknown/restricted; no inferred duration.
    pub expires_at_utc_ms: Observation<UtcMillis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EquipmentEnhancements {
    pub item: ItemKey,
    pub enhancements: Observation<Enumeration<ObservedEnhancement>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "ProgressionSnapshot")]
pub struct ProgressionSnapshotData {
    pub payload_version: SchemaVersion,
    pub id: SnapshotId,
    pub binding: SnapshotBinding,
    pub capture: ObservationMetadata,
    /// Keys identify exact source spell records; unknown rank is not rank zero.
    pub spells: Observation<Enumeration<Observation<u16>>>,
    pub money: Observation<Copper>,
    pub talents: Observation<Enumeration<Observation<u16>>>,
    pub available_talent_points: Observation<u16>,
    pub professions: Observation<Enumeration<ProfessionState>>,
    pub recipes: Observation<Enumeration<Observation<bool>>>,
    /// Bags and bank are separate enumerations and never silently combined.
    pub bags: Observation<Enumeration<InventoryQuantity>>,
    pub bank: Observation<Enumeration<InventoryQuantity>>,
    pub equipment_enhancements: RecordMap<SlotId, Observation<EquipmentEnhancements>>,
}
pub type ProgressionSnapshot = Validated<ProgressionSnapshotData>;

impl Validate for ProgressionSnapshotData {
    fn validate(&self) -> Result<()> {
        self.capture.validate()?;
        self.binding.capture.validate()?;
        if let Some(professions) = self.professions.value() {
            for profession in professions.entries.values() {
                if let (Some(skill), Some(maximum)) =
                    (profession.skill.value(), profession.maximum_skill.value())
                    && skill > maximum
                {
                    return Err(invalid("Profession skill exceeds its observed maximum"));
                }
            }
        }
        for inventory in [&self.bags, &self.bank] {
            if let Some(inventory) = inventory.value() {
                let mut items = BTreeSet::new();
                for entry in inventory.entries.values() {
                    self.binding
                        .context
                        .ensure_compatible(&entry.item.context)?;
                    if !items.insert(&entry.item) {
                        return Err(invalid(
                            "Inventory scope repeats an aggregated item identity",
                        ));
                    }
                }
            }
        }
        for slot in self.equipment_enhancements.values() {
            if let Some(slot) = slot.value() {
                self.binding.context.ensure_compatible(&slot.item.context)?;
                if let Some(effects) = slot.enhancements.value() {
                    for effect in effects.entries.values() {
                        if effect.duration.value() == Some(&EnhancementDuration::Permanent)
                            && effect.expires_at_utc_ms.value().is_some()
                        {
                            return Err(invalid("Permanent enhancement cannot declare an expiry"));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
