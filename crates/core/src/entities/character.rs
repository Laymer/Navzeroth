use super::{EquippedItem, QuestProgress, bounded, unique};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::*,
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterProfileData {
    pub id: CharacterId,
    pub context: DataContext,
    pub region: Observation<Text>,
    pub realm: Observation<Text>,
    pub race: Observation<SourceId>,
    pub class: Observation<SourceId>,
    pub level: Observation<Level>,
    pub role: Observation<RoleId>,
    pub preferences: RecordMap<PreferenceId, Observation<bool>>,
}
pub type CharacterProfile = Validated<CharacterProfileData>;
impl Validate for CharacterProfileData {
    fn validate(&self) -> Result<()> {
        bounded(self.preferences.len())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "item", rename_all = "snake_case")]
pub enum EquipmentSlot {
    Empty,
    Item(Box<EquippedItem>),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterSnapshotData {
    pub schema_version: SchemaVersion,
    pub id: SnapshotId,
    pub character: CharacterId,
    pub context: DataContext,
    pub capture: ObservationMetadata,
    pub source_revision: DatasetRevision,
    pub level: Observation<Level>,
    pub role: Observation<RoleId>,
    pub location: Observation<Position>,
    pub preferences: RecordMap<PreferenceId, Observation<bool>>,
    pub equipment: RecordMap<SlotId, Observation<EquipmentSlot>>,
    pub quests: RecordMap<QuestId, QuestProgress>,
    pub objectives: RecordMap<ObjectiveId, Observation<bool>>,
}
pub type CharacterSnapshot = Validated<CharacterSnapshotData>;
impl Validate for CharacterSnapshotData {
    fn validate(&self) -> Result<()> {
        self.capture.validate()?;
        if let Some(location) = self.location.value() {
            self.context.ensure_compatible(&location.map.context)?;
        }
        for slot in self.equipment.values() {
            if let Some(EquipmentSlot::Item(item)) = slot.value() {
                self.context.ensure_compatible(&item.context)?;
            }
        }
        for (key, quest) in self.quests.iter() {
            if key != &quest.quest {
                return Err(invalid("Quest map key disagrees with quest identity"));
            }
            self.context.ensure_compatible(&quest.context)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartySnapshotData {
    pub id: PartyId,
    pub context: DataContext,
    pub participants: Vec<CharacterSnapshot>,
}
pub type PartySnapshot = Validated<PartySnapshotData>;
impl Validate for PartySnapshotData {
    fn validate(&self) -> Result<()> {
        if self.participants.is_empty() {
            return Err(invalid("Party snapshot requires participants"));
        }
        unique(self.participants.iter().map(|p| &p.character))?;
        unique(self.participants.iter().map(|p| &p.id))?;
        for p in &self.participants {
            self.context.ensure_compatible(&p.context)?;
        }
        Ok(())
    }
}
