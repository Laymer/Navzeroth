//! Caller-supplied gear inputs: candidates and slot layout. Nothing here
//! hard-codes a game slot table; slot names are opaque identifiers.
use crate::{
    entities::{ItemDefinition, bounded, unique},
    error::{Result, Validate, Validated, invalid},
    observation::Observation,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An item definition plus its armor/weapon category. `ItemDefinition` has no
/// category field and contracts are frozen, so the category is a local,
/// caller-supplied observation. `NotApplicable` means no category applies
/// (for example jewelry); unknown, unsupported and restricted stay distinct.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GearCandidate {
    pub item: ItemDefinition,
    pub category: Observation<SourceId>,
}
impl GearCandidate {
    pub fn key(&self) -> &ItemKey {
        &self.item.key
    }
}

/// The main-hand slot and the off-hand slot a two-handed item also occupies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HandSlots {
    pub main_hand: SlotId,
    pub off_hand: SlotId,
}

/// Slots the scenario may fill, plus the optional hand relationship. Paired
/// slots (two fingers, two trinkets) are expressed by candidates listing both
/// slots in `ItemDefinition.slots`; one candidate is never placed twice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SlotLayoutData {
    pub slots: Vec<SlotId>,
    pub hands: Option<HandSlots>,
}
pub type SlotLayout = Validated<SlotLayoutData>;
impl Validate for SlotLayoutData {
    fn validate(&self) -> Result<()> {
        bounded(self.slots.len())?;
        unique(self.slots.iter())?;
        if self.slots.is_empty() {
            return Err(invalid("Slot layout requires at least one slot"));
        }
        if let Some(hands) = &self.hands
            && (hands.main_hand == hands.off_hand
                || !self.slots.contains(&hands.main_hand)
                || !self.slots.contains(&hands.off_hand))
        {
            return Err(invalid(
                "Hand slots must be two distinct slots of the layout",
            ));
        }
        Ok(())
    }
}
