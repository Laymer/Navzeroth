use super::{CapabilityStatus, DurationRange, NodeRef, bounded, requirement_context, unique};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::Observation,
    predicate::Predicate,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotRef {
    pub id: SnapshotId,
    pub character: CharacterId,
    pub context: DataContext,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlanStep {
    pub id: StepId,
    pub objective: ObjectiveId,
    pub participants: Vec<CharacterId>,
    pub alternatives: Vec<ObjectiveId>,
    pub prerequisite: Observation<Predicate>,
    pub required_fields: Vec<Text>,
    pub invalidation: Observation<Predicate>,
    pub expected_ms: Observation<Millis>,
    pub estimate: Observation<DurationRange>,
    pub explanation: Text,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TravelSegment {
    pub from: NodeRef,
    pub to: NodeRef,
    pub expected_ms: Observation<Millis>,
    pub range: Observation<DurationRange>,
}
/// Events after which a plan must be revalidated. Expiry and these triggers
/// never prove that the underlying game state changed.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RevalidationTrigger {
    DatasetRevisionChanged,
    BuildChanged,
    SnapshotSuperseded,
    RulesetChanged,
}
fn within_range(expected: &Observation<Millis>, range: &Observation<DurationRange>) -> Result<()> {
    if let (Some(expected), Some(range)) = (expected.value(), range.value())
        && (*expected < range.min_ms || *expected > range.max_ms)
    {
        return Err(invalid("Expected duration lies outside its estimate range"));
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "RoutePlan")]
pub struct RoutePlanData {
    pub schema_version: SchemaVersion,
    pub id: PlanId,
    pub context: DataContext,
    pub generated_at_utc_ms: UtcMillis,
    pub inputs: Vec<SnapshotRef>,
    pub dataset_revision: DatasetRevision,
    pub ruleset_version: RulesetVersion,
    pub score_model_version: ScoreModelVersion,
    pub capability_assumptions: RecordMap<CapabilityId, CapabilityStatus>,
    pub goal: Text,
    pub time_budget_ms: Millis,
    pub exclusions: Vec<ObjectiveId>,
    pub manual_assumptions: Vec<Text>,
    pub steps: Vec<PlanStep>,
    pub travel: Vec<TravelSegment>,
    pub evidence: Vec<EvidenceId>,
    pub expires_at_utc_ms: Option<UtcMillis>,
    pub revalidation: Vec<RevalidationTrigger>,
}
pub type RoutePlan = Validated<RoutePlanData>;
impl Validate for RoutePlanData {
    fn validate(&self) -> Result<()> {
        unique(self.inputs.iter().map(|s| &s.character))?;
        unique(self.inputs.iter().map(|s| &s.id))?;
        if self.inputs.is_empty() {
            return Err(invalid("Plan requires snapshot inputs"));
        }
        for input in &self.inputs {
            self.context.ensure_compatible(&input.context)?;
        }
        unique(self.steps.iter().map(|s| &s.id))?;
        unique(self.exclusions.iter())?;
        unique(self.evidence.iter())?;
        bounded(self.manual_assumptions.len())?;
        bounded(self.travel.len())?;
        unique(self.revalidation.iter())?;
        for step in &self.steps {
            unique(step.evidence.iter())?;
            if step.evidence.iter().any(|e| !self.evidence.contains(e)) {
                return Err(invalid("Plan step evidence is not listed in plan evidence"));
            }
            within_range(&step.expected_ms, &step.estimate)?;
            requirement_context(&step.prerequisite, &self.context)?;
            requirement_context(&step.invalidation, &self.context)?;
            unique(step.participants.iter())?;
            unique(step.alternatives.iter())?;
            bounded(step.required_fields.len())?;
            if step.participants.is_empty()
                || step
                    .participants
                    .iter()
                    .any(|p| !self.inputs.iter().any(|i| &i.character == p))
            {
                return Err(invalid("Plan step participant is not an input participant"));
            }
            if self.exclusions.contains(&step.objective)
                || step
                    .alternatives
                    .iter()
                    .any(|o| self.exclusions.contains(o))
            {
                return Err(invalid("Plan includes an excluded objective"));
            }
        }
        for segment in &self.travel {
            within_range(&segment.expected_ms, &segment.range)?;
            self.context.ensure_compatible(&segment.from.map.context)?;
            self.context.ensure_compatible(&segment.to.map.context)?;
        }
        for pair in self.travel.windows(2) {
            if pair[0].to != pair[1].from {
                return Err(invalid("Travel segments are discontinuous"));
            }
        }
        if self
            .expires_at_utc_ms
            .is_some_and(|t| t < self.generated_at_utc_ms)
        {
            return Err(invalid("Plan expires before generation"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigurationItem {
    pub item: ItemKey,
    pub occupied_slots: Vec<SlotId>,
    pub unique_group: Observation<Text>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ranking {
    Theoretical,
    Obtainable,
    Practical,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "GearRecommendation")]
pub struct GearRecommendationData {
    pub context: DataContext,
    pub snapshot: SnapshotRef,
    pub dataset_revision: DatasetRevision,
    pub score_model_version: ScoreModelVersion,
    pub scenario: RoleId,
    pub ranking: Ranking,
    pub baseline: RecordMap<SlotId, ConfigurationItem>,
    pub proposed: RecordMap<SlotId, ConfigurationItem>,
    pub eligibility: Observation<bool>,
    pub baseline_score: Observation<FiniteNumber>,
    pub proposed_score: Observation<FiniteNumber>,
    pub acquisitions: Vec<AcquisitionId>,
    pub assumptions: Vec<Text>,
    pub unmodeled_effects: Vec<Text>,
    pub explanation: Text,
    pub evidence: Vec<EvidenceId>,
}
pub type GearRecommendation = Validated<GearRecommendationData>;
impl Validate for GearRecommendationData {
    fn validate(&self) -> Result<()> {
        self.context.ensure_compatible(&self.snapshot.context)?;
        unique(self.acquisitions.iter())?;
        unique(self.evidence.iter())?;
        bounded(self.assumptions.len())?;
        bounded(self.unmodeled_effects.len())?;
        if self.ranking != Ranking::Theoretical && self.eligibility.value() != Some(&true) {
            return Err(invalid(
                "Obtainable and practical rankings require known eligibility",
            ));
        }
        for config in [&self.baseline, &self.proposed] {
            let mut occupied = BTreeSet::new();
            let mut groups = BTreeSet::new();
            for (slot, entry) in config.iter() {
                self.context.ensure_compatible(&entry.item.context)?;
                unique(entry.occupied_slots.iter())?;
                if !entry.occupied_slots.contains(slot) {
                    return Err(invalid("Configuration item must occupy its assigned slot"));
                }
                for occupied_slot in &entry.occupied_slots {
                    if !occupied.insert(occupied_slot) {
                        return Err(invalid("Equipment has overlapping occupied slots"));
                    }
                }
                if let Some(group) = entry.unique_group.value()
                    && !groups.insert(group.as_str())
                {
                    return Err(invalid("Equipment violates a known unique group"));
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleTrigger {
    SnapshotChanged,
    LevelChanged,
    EquipmentChanged,
    LocationChanged,
    PreferenceChanged,
    Manual,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DismissalPolicy {
    Session,
    UntilRulesetChanges,
    Permanent,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ContextRule")]
pub struct ContextRuleData {
    pub id: RuleId,
    pub context: DataContext,
    pub trigger: RuleTrigger,
    pub predicate: Predicate,
    pub result_template: Text,
    pub priority: i16,
    pub deduplication_key: RuleId,
    pub cooldown_ms: Millis,
    pub dismissal: DismissalPolicy,
    pub expires_at_utc_ms: Option<UtcMillis>,
    pub evidence: Vec<EvidenceId>,
}
pub type ContextRule = Validated<ContextRuleData>;
impl Validate for ContextRuleData {
    fn validate(&self) -> Result<()> {
        self.predicate.validate_in_context(&self.context)?;
        unique(self.evidence.iter())?;
        if self.result_template.as_str().trim().is_empty() {
            return Err(invalid("Rule result template must not be empty"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementQuality {
    Observed,
    Estimated,
    Unclassified,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SessionInterval {
    pub start_ms: Millis,
    pub end_ms: Millis,
    pub quality: MeasurementQuality,
    pub label: Text,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SessionGap {
    pub start_ms: Millis,
    pub end_ms: Millis,
    pub reason: Text,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "SessionSummary")]
pub struct SessionSummaryData {
    pub session: SessionId,
    pub character: CharacterId,
    pub context: DataContext,
    pub elapsed_ms: Millis,
    pub starting_level: Observation<Level>,
    pub ending_level: Observation<Level>,
    pub observed_quests: Vec<QuestId>,
    pub changed_equipment: Vec<ItemKey>,
    pub intervals: Vec<SessionInterval>,
    pub gaps: Vec<SessionGap>,
    pub methodology: Text,
}
pub type SessionSummary = Validated<SessionSummaryData>;
impl Validate for SessionSummaryData {
    fn validate(&self) -> Result<()> {
        unique(self.observed_quests.iter())?;
        bounded(self.changed_equipment.len())?;
        bounded(self.intervals.len() + self.gaps.len())?;
        for item in &self.changed_equipment {
            self.context.ensure_compatible(&item.context)?;
        }
        if self.methodology.as_str().trim().is_empty() {
            return Err(invalid("Session summary requires its methodology"));
        }
        let mut ranges: Vec<_> = self
            .intervals
            .iter()
            .map(|i| (i.start_ms, i.end_ms))
            .chain(self.gaps.iter().map(|i| (i.start_ms, i.end_ms)))
            .collect();
        ranges.sort_unstable();
        let mut previous = Millis::new(0)?;
        for (start, end) in ranges {
            if start > end || end > self.elapsed_ms || start < previous {
                return Err(invalid(
                    "Session intervals overlap or exceed session bounds",
                ));
            }
            previous = end;
        }
        Ok(())
    }
}
