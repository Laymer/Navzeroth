use super::{bounded, requirement_context, unique};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::*,
    predicate::Predicate,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    VerifiedAvailable,
    VerifiedUnavailable,
    Restricted,
    Failed,
    Untested,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityCheck {
    pub status: CapabilityStatus,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "GameBuild")]
pub struct GameBuildData {
    pub key: BuildKey,
    pub locale: Locale,
    pub capabilities: Observation<RecordMap<CapabilityId, CapabilityCheck>>,
}
pub type GameBuild = Validated<GameBuildData>;
impl Validate for GameBuildData {
    fn validate(&self) -> Result<()> {
        if let Some(checks) = self.capabilities.value() {
            bounded(checks.len())?;
            for check in checks.values() {
                unique(check.evidence.iter())?;
                if matches!(
                    check.status,
                    CapabilityStatus::VerifiedAvailable | CapabilityStatus::VerifiedUnavailable
                ) && check.evidence.is_empty()
                {
                    return Err(invalid("Verified capability requires evidence"));
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    VerifiedOnBuild,
    PublisherDocumented,
    LimitedSample,
    CrossEditionUnverified,
    Inferred,
    DisputedOrStale,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "EvidenceRecord")]
pub struct EvidenceRecordData {
    pub id: EvidenceId,
    pub context: DataContext,
    pub source: SourceId,
    pub license: Observation<Text>,
    pub observed_at_utc_ms: Observation<UtcMillis>,
    pub confidence: Confidence,
    pub conflicts: Vec<EvidenceId>,
    pub notes: Text,
}
pub type EvidenceRecord = Validated<EvidenceRecordData>;
impl Validate for EvidenceRecordData {
    fn validate(&self) -> Result<()> {
        unique(self.conflicts.iter())?;
        if self.conflicts.contains(&self.id) {
            return Err(invalid("Evidence cannot conflict with itself"));
        }
        if self.context.namespace == DatasetNamespace::Demo
            && self.confidence == Confidence::VerifiedOnBuild
        {
            return Err(invalid(
                "Demo evidence cannot claim live build verification",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Available,
    Unavailable,
    Unverified,
    Restricted,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ProviderCapability")]
pub struct ProviderCapabilityData {
    pub provider: ProviderId,
    pub context: DataContext,
    pub status: ProviderStatus,
    pub datasets: Vec<DatasetRevision>,
    pub endpoints: Vec<Text>,
    pub freshness: ObservationMetadata,
    pub evidence: Vec<EvidenceId>,
}
pub type ProviderCapability = Validated<ProviderCapabilityData>;
impl Validate for ProviderCapabilityData {
    fn validate(&self) -> Result<()> {
        unique(self.datasets.iter())?;
        bounded(self.endpoints.len())?;
        unique(self.endpoints.iter().map(Text::as_str))?;
        if self
            .endpoints
            .iter()
            .any(|endpoint| endpoint.as_str().trim().is_empty())
        {
            return Err(invalid("Provider endpoint labels must not be empty"));
        }
        unique(self.evidence.iter())?;
        self.freshness.validate()?;
        if self.status == ProviderStatus::Available
            && (self.evidence.is_empty() || (self.datasets.is_empty() && self.endpoints.is_empty()))
        {
            return Err(invalid(
                "Available provider requires coverage and verification evidence",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ItemDefinition")]
pub struct ItemDefinitionData {
    pub key: ItemKey,
    pub slots: Vec<SlotId>,
    pub two_handed: Observation<bool>,
    pub unique_group: Observation<Text>,
    pub attributes: Observation<RecordMap<AttributeId, FiniteNumber>>,
    pub requirements: Observation<Predicate>,
    pub effects: Observation<Vec<Text>>,
    pub evidence: Vec<EvidenceId>,
}
pub type ItemDefinition = Validated<ItemDefinitionData>;
impl Validate for ItemDefinitionData {
    fn validate(&self) -> Result<()> {
        requirement_context(&self.requirements, &self.key.context)?;

        unique(self.slots.iter())?;
        unique(self.evidence.iter())?;
        if self.slots.is_empty() {
            return Err(invalid("Item requires at least one slot"));
        }
        if let Some(a) = self.attributes.value() {
            bounded(a.len())?;
        }
        if let Some(e) = self.effects.value() {
            bounded(e.len())?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "EquippedItem")]
pub struct EquippedItemData {
    pub context: DataContext,
    pub item_id: ItemId,
    pub variant: Observation<VariantId>,
    pub instance: Observation<InstanceId>,
    pub modifiers: Observation<RecordMap<AttributeId, FiniteNumber>>,
}
pub type EquippedItem = Validated<EquippedItemData>;
impl Validate for EquippedItemData {
    fn validate(&self) -> Result<()> {
        if let Some(m) = self.modifiers.value() {
            bounded(m.len())?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionKind {
    Quest,
    Vendor,
    Dungeon,
    Crafting,
    Other,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "AcquisitionSource")]
pub struct AcquisitionSourceData {
    pub id: AcquisitionId,
    pub context: DataContext,
    pub kind: AcquisitionKind,
    pub items: Vec<ItemKey>,
    pub conditions: Observation<Predicate>,
    pub cost: Observation<Copper>,
    pub probability: Observation<Normalized>,
    pub evidence: Vec<EvidenceId>,
}
pub type AcquisitionSource = Validated<AcquisitionSourceData>;
impl Validate for AcquisitionSourceData {
    fn validate(&self) -> Result<()> {
        requirement_context(&self.conditions, &self.context)?;

        unique(self.items.iter())?;
        unique(self.evidence.iter())?;
        if self.items.is_empty() {
            return Err(invalid("Acquisition source requires an item reference"));
        }
        for item in &self.items {
            self.context.ensure_compatible(&item.context)?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompletionSemantics {
    Unrepeatable,
    Repeatable,
    ResetBound,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "QuestDefinition")]
pub struct QuestDefinitionData {
    pub id: QuestId,
    pub context: DataContext,
    pub eligibility: Observation<Predicate>,
    pub prerequisites: Observation<Predicate>,
    pub stages: Vec<ObjectiveId>,
    pub locations: Observation<Vec<Position>>,
    pub rewards: Observation<Vec<ItemKey>>,
    pub completion: Observation<CompletionSemantics>,
    pub shared_progress: Observation<bool>,
    pub evidence: Vec<EvidenceId>,
}
pub type QuestDefinition = Validated<QuestDefinitionData>;
impl Validate for QuestDefinitionData {
    fn validate(&self) -> Result<()> {
        requirement_context(&self.eligibility, &self.context)?;
        requirement_context(&self.prerequisites, &self.context)?;

        unique(self.stages.iter())?;
        unique(self.evidence.iter())?;
        if self.stages.is_empty() {
            return Err(invalid("Quest requires at least one stage"));
        }
        if let Some(locations) = self.locations.value() {
            bounded(locations.len())?;
            for p in locations {
                self.context.ensure_compatible(&p.map.context)?;
            }
        }
        if let Some(rewards) = self.rewards.value() {
            bounded(rewards.len())?;
            for item in rewards {
                self.context.ensure_compatible(&item.context)?;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "QuestProgress")]
pub struct QuestProgressData {
    pub quest: QuestId,
    pub context: DataContext,
    pub accepted: Observation<bool>,
    pub completed: Observation<bool>,
    pub turned_in: Observation<bool>,
    pub objectives: RecordMap<ObjectiveId, Observation<bool>>,
}
pub type QuestProgress = Validated<QuestProgressData>;
impl Validate for QuestProgressData {
    fn validate(&self) -> Result<()> {
        bounded(self.objectives.len())
    }
}
