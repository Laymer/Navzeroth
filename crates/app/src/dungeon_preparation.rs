//! Explicit local preparation references; no built-in production dungeon facts.
//! Capture immutable inputs under the store lock, evaluate outside it, and verify
//! current identities before returning a bounded explanatory report.
use crate::{party::QuestAssessment, snapshots::SnapshotApplication};
use azeroth_core::{
    entities::{Confidence, DurationRange, NodeRef},
    evaluation::EvaluationResult,
    observation::{Freshness, Observation, ObservationStatus},
    predicate::Predicate,
    travel::RouteSearch,
    types::*,
};
use serde::{Deserialize, Serialize};

mod json;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SECTION_RECORDS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declared<T> {
    pub observation: Observation<T>,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCategory {
    Role,
    Consumables,
    Equipment,
    BeforeDeparture,
    BeforeEntry,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationCheck {
    pub id: SourceId,
    pub label: Text,
    pub category: CheckCategory,
    pub requirement: Declared<Predicate>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RewardReference {
    pub item: ItemKey,
    pub source: AcquisitionId,
    /// A local label, not an inferred role benefit or recommendation.
    pub declared_role: Observation<RoleId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPreparationPackage {
    pub schema_version: u32,
    pub id: SourceId,
    pub title: Text,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: DatasetRevision,
    pub entrance: Declared<NodeRef>,
    pub access: Declared<Predicate>,
    pub quests: Declared<Vec<QuestId>>,
    pub rewards: Declared<Vec<RewardReference>>,
    pub checks: Declared<Vec<PreparationCheck>>,
    pub formation_ms: Declared<DurationRange>,
    pub run_ms: Declared<DurationRange>,
    pub before_departure: Declared<Vec<Text>>,
    pub before_entry: Declared<Vec<Text>>,
    pub encounter_notes: Declared<Vec<Text>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparationSubject {
    Character { profile_id: String },
    Party { party_id: PartyId },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationRequest {
    pub schema_version: u32,
    pub subject: PreparationSubject,
    pub package: Option<LocalPreparationPackage>,
    /// Manually declared shared starting node; never inferred from labels.
    pub origin: Option<NodeId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConditionAssessment {
    pub availability: ObservationStatus,
    pub evaluation: Option<EvaluationResult>,
}
#[derive(Clone, Debug, Serialize)]
pub struct QuestPreparation {
    pub quest: QuestId,
    pub assessment: Option<QuestAssessment>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CheckPreparation {
    pub check: SourceId,
    pub condition: ConditionAssessment,
}
#[derive(Clone, Debug, Serialize)]
pub struct RewardPreparation {
    pub item: ItemKey,
    pub source: AcquisitionId,
    pub item_requirement: ConditionAssessment,
    pub acquisition_requirement: ConditionAssessment,
    pub probability: Observation<Normalized>,
    pub cost: Observation<Copper>,
}
#[derive(Clone, Debug, Serialize)]
pub struct MemberPreparation {
    pub profile_id: String,
    pub character: CharacterId,
    pub snapshot_id: Option<SnapshotId>,
    pub freshness: Option<Freshness>,
    pub access: Option<ConditionAssessment>,
    pub quests: Vec<QuestPreparation>,
    pub checks: Vec<CheckPreparation>,
    pub rewards: Vec<RewardPreparation>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationEvidence {
    pub id: EvidenceId,
    pub source: SourceId,
    pub confidence: Confidence,
    pub license: Observation<Text>,
    pub redistribution: Observation<azeroth_core::sources::Redistribution>,
    pub offline_usable: Observation<bool>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapCode {
    PackageMissing,
    LocalPackageUnverified,
    ActiveContentMissing,
    SectionUnavailable,
    EvidenceMissing,
    SnapshotMissing,
    SnapshotNotFresh,
    OriginMissing,
    EntranceUnavailable,
    RouteNeedsFreshMembers,
    NoSupportedRoute,
    ConsumableInventoryUnavailable,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationGap {
    pub code: GapCode,
    pub section: &'static str,
    pub availability: Option<ObservationStatus>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationReport {
    pub schema_version: u16,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: Option<DatasetRevision>,
    pub package: Option<LocalPreparationPackage>,
    pub members: Vec<MemberPreparation>,
    pub evidence: Vec<PreparationEvidence>,
    pub gaps: Vec<PreparationGap>,
    pub route: Option<RouteSearch>,
}
