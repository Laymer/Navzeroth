use azeroth_core::{observation::Freshness, progression::*, types::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Manual input has no caller-controlled provenance or observation metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManualValue<T> {
    #[default]
    Unknown,
    Known {
        value: T,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualProfession {
    pub skill: Option<u16>,
    pub maximum_skill: Option<u16>,
    pub rank: Option<SourceId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualInventory {
    pub item: ItemKey,
    pub quantity: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualEnhancement {
    pub duration: Option<EnhancementDuration>,
    pub expires_at_utc_ms: Option<UtcMillis>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualEquipment {
    pub item: ItemKey,
    pub enhancements: ManualValue<Enumeration<ManualEnhancement>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualProgressionRequest {
    pub profile_id: String,
    pub expected: SnapshotBinding,
    pub id: SnapshotId,
    pub spells: ManualValue<Enumeration<u16>>,
    pub money: ManualValue<Copper>,
    pub talents: ManualValue<Enumeration<u16>>,
    pub available_talent_points: ManualValue<u16>,
    pub professions: ManualValue<Enumeration<ManualProfession>>,
    pub recipes: ManualValue<Enumeration<bool>>,
    pub bags: ManualValue<Enumeration<ManualInventory>>,
    pub bank: ManualValue<Enumeration<ManualInventory>>,
    pub equipment_enhancements: RecordMap<SlotId, ManualValue<ManualEquipment>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportProgressionRequest {
    pub profile_id: String,
    pub expected: SnapshotBinding,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProgressionRecord {
    pub snapshot: ProgressionSnapshot,
    pub freshness: Freshness,
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProgressionView {
    pub profile_id: String,
    pub fixture: bool,
    pub binding: SnapshotBinding,
    pub imported: Vec<ProgressionRecord>,
    pub manual: Vec<ProgressionRecord>,
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProgressionReceipt {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
    pub companion_id: SnapshotId,
    pub stored: bool,
    pub fixture: bool,
}
/// Schema generation root; this is not an executable command dispatcher.
#[derive(JsonSchema)]
pub struct ProgressionApi {
    pub manual_request: ManualProgressionRequest,
    pub import_request: ImportProgressionRequest,
    pub view: ProgressionView,
    pub receipt: ProgressionReceipt,
}
