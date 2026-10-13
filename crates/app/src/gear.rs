//! Explicit manual model assumptions over exact active content and captured state.
use crate::snapshots::SnapshotApplication;
use azeroth_core::{
    contract::Envelope,
    entities::{AcquisitionSource, CharacterSnapshot, EquipmentSlot, ItemDefinition, Ranking},
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, StalePolicy, Truth},
    gear::*,
    observation::{FreshnessContext, Observation, ObservationStatus},
    predicate::RuleLimits,
    types::*,
};
use azeroth_storage::SnapshotStore;
use azeroth_storage::content::{ContentRevision, ContentScope};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_CANDIDATES: usize = 128;
pub const PRACTICAL_MODEL_VERSION: &str = "gear-practical-v1";

#[derive(Clone, Debug, Serialize)]
pub struct GearCatalog {
    pub snapshot_id: Option<SnapshotId>,
    pub revision: Option<DatasetRevision>,
    pub fixture: bool,
    pub context: DataContext,
    pub equipment: RecordMap<SlotId, Observation<EquipmentSlot>>,
    pub items: Vec<ItemDefinition>,
    pub acquisitions: Vec<AcquisitionSource>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffortInput {
    pub incremental_travel_ms: Observation<Millis>,
    pub formation_ms: Observation<Millis>,
    pub attempt_ms: Observation<Millis>,
    pub stopping_ms: Observation<Millis>,
    pub available_copper: Observation<Copper>,
    pub session_budget_ms: Millis,
    pub max_attempts: u32,
    /// Explicit independent, identical player-acquisition trial assumption.
    pub independent_player_attempts: bool,
    pub cost_per_attempt: bool,
}
impl EffortInput {
    fn effort(&self) -> AcquisitionEffort {
        AcquisitionEffort {
            incremental_travel_ms: self.incremental_travel_ms.clone(),
            formation_ms: self.formation_ms.clone(),
            attempt_ms: self.attempt_ms.clone(),
            stopping_ms: self.stopping_ms.clone(),
            available_copper: self.available_copper.clone(),
            balance_basis: BalanceBasis::CallerBudget,
            session_budget_ms: self.session_budget_ms,
            max_attempts: self.max_attempts,
            probability_model: if self.independent_player_attempts {
                ProbabilityModel::IndependentPlayerAttempts
            } else {
                ProbabilityModel::DeclaredOnly
            },
            cost_schedule: if self.cost_per_attempt {
                CostSchedule::PerAttempt
            } else {
                CostSchedule::Once
            },
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionInput {
    pub source: AcquisitionId,
    pub effort: EffortInput,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GearInput {
    pub item: ItemKey,
    pub category: Observation<SourceId>,
    pub acquisition: Option<AcquisitionInput>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GearRequest {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
    pub revision: DatasetRevision,
    pub model: RoleProfileData,
    pub layout: SlotLayoutData,
    pub class: Observation<SourceId>,
    pub proficiency: ProficiencyTableData,
    pub candidates: Vec<GearInput>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CandidateComparison {
    pub item: ItemKey,
    pub eligibility: GearEligibility,
    pub score: ScoreBreakdown,
    pub acquisition: Option<AcquisitionAssessment>,
    pub practical_reasons: Vec<&'static str>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PracticalUpgrade {
    pub item: ItemKey,
    pub slot: SlotId,
    pub proposed: RankedConfiguration,
    pub gain_milli: i64,
    pub probability_millionths: u32,
    pub conditional_expected_gain_milli: i64,
    pub reserved_effort_ms: Millis,
    pub conditional: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct GearReport {
    pub request: GearRequest,
    pub fixture: bool,
    pub context: DataContext,
    pub current: BaselineResolution,
    pub current_score: ScoreBreakdown,
    pub baseline_gaps: Vec<&'static str>,
    pub theoretical: ConfigurationRanking,
    pub equip_eligible: ConfigurationRanking,
    pub obtainable: ConfigurationRanking,
    pub candidates: Vec<CandidateComparison>,
    pub practical: Vec<PracticalUpgrade>,
    pub practical_model_version: &'static str,
    pub assumptions: Vec<&'static str>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}

impl SnapshotApplication {
    pub fn gear_catalog(&self, profile_id: &str) -> Result<GearCatalog> {
        self.with_profile_store_mut(profile_id, |store, key| {
            let snapshot = store.current_snapshot(key)?;
            let content = store.active_content_revision(&ContentScope {
                context: key.context.clone(),
                fixture: key.fixture,
            })?;
            Ok(GearCatalog {
                snapshot_id: snapshot.as_ref().map(|s| s.payload.id.clone()),
                revision: content
                    .as_ref()
                    .map(|c| c.envelope.payload.revision.clone()),
                fixture: key.fixture,
                context: key.context.clone(),
                equipment: snapshot
                    .map(|s| s.payload.equipment.clone())
                    .unwrap_or_default(),
                items: content
                    .as_ref()
                    .map(|c| c.envelope.payload.items.clone())
                    .unwrap_or_default(),
                acquisitions: content
                    .map(|c| c.envelope.payload.acquisitions.clone())
                    .unwrap_or_default(),
            })
        })
    }
    pub fn compare_gear(&self, text: &str, now: Option<UtcMillis>) -> Result<GearReport> {
        if text.len() > MAX_REQUEST_BYTES {
            return Err(AppError::new(
                ErrorCode::ImportTooLarge,
                "Gear request exceeds 1 MiB",
                false,
            ));
        }
        let request: GearRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid gear request"))?;
        self.with_profile_store_mut(&request.profile_id, |store, key| {
            let snapshot = store
                .current_snapshot(key)?
                .ok_or_else(|| invalid("Profile has no current snapshot"))?;
            if snapshot.payload.id != request.snapshot_id {
                return Err(invalid("Snapshot changed; refresh gear inputs"));
            }
            let content = store
                .active_content_revision(&ContentScope {
                    context: key.context.clone(),
                    fixture: key.fixture,
                })?
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::DatasetIncomplete,
                        "No active content for this profile",
                        false,
                    )
                })?;
            if content.envelope.payload.revision != request.revision {
                return Err(invalid("Active content changed; refresh gear inputs"));
            }
            calculate(
                request.clone(),
                &snapshot,
                &content,
                &crate::snapshots::context(now),
            )
        })
    }
}

pub fn calculate(
    request: GearRequest,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &ContentRevision,
    clock: &FreshnessContext,
) -> Result<GearReport> {
    if request.candidates.len() > MAX_CANDIDATES {
        return Err(invalid("Gear input exceeds 128 candidates"));
    }
    snapshot
        .data_context()
        .ensure_compatible(&content.envelope.data_context())?;
    if snapshot.fixture != content.envelope.fixture {
        return Err(invalid("Fixture identity mismatch"));
    }
    if request.snapshot_id != snapshot.payload.id
        || request.revision != content.envelope.payload.revision
    {
        return Err(invalid("Gear inputs changed"));
    }
    let model = RoleProfile::try_new(request.model.clone())?;
    let layout = SlotLayout::try_new(request.layout.clone())?;
    let proficiency = ProficiencyTable::try_new(request.proficiency.clone())?;
    let evaluation = EvaluationContext {
        snapshot: &snapshot.payload,
        party: None,
        freshness: clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let eligibility_context = GearEligibilityContext {
        class: &request.class,
        proficiency: &proficiency,
        evaluation: &evaluation,
    };
    let mut inputs = request.candidates.clone();
    inputs.sort_by(|a, b| a.item.cmp(&b.item));
    let mut candidates = Vec::new();
    for input in &inputs {
        let item = content
            .envelope
            .payload
            .items
            .iter()
            .find(|i| i.key == input.item)
            .ok_or_else(|| invalid("Gear candidate is absent from active content"))?;
        candidates.push(GearCandidate {
            item: item.clone(),
            category: input.category.clone(),
        });
    }
    let context = snapshot.data_context();
    let scenario = GearScenario {
        context: &context,
        profile: &model,
        layout: &layout,
        candidates: &candidates,
    };
    let theoretical = rank_theoretical(&scenario)?;
    let equip_eligible = rank_obtainable(&scenario, &eligibility_context)?;
    let current = resolve_baseline(&scenario, &evaluation)?;
    let current_score = score_baseline(&scenario, &current)?;
    let baseline_keys: BTreeMap<_, _> = current
        .matched()
        .into_iter()
        .map(|(s, k)| (s.clone(), k.clone()))
        .collect();
    let mut baseline_gaps = Vec::new();
    if !current.is_complete() {
        baseline_gaps.push("Captured baseline slots or variants are unavailable, stale, or absent from the selected catalog");
    }
    if current_score.certainty != Certainty::Complete {
        baseline_gaps.push("Baseline model score is incomplete");
    }
    if current.is_complete()
        && !baseline_keys.is_empty()
        && !theoretical
            .valid
            .iter()
            .any(|c| configuration_keys(c) == baseline_keys)
    {
        baseline_gaps.push("Baseline configuration legality is not established");
    }
    for slot in &layout.slots {
        if let Some(EquipmentSlot::Item(item)) = snapshot
            .payload
            .equipment
            .get(slot)
            .and_then(Observation::value)
            && !(item.modifiers.value().is_some_and(|m| m.is_empty())
                || item.modifiers.status() == ObservationStatus::NotApplicable)
        {
            baseline_gaps.push("Captured modifiers are unavailable or require a model this comparison does not implement");
            break;
        }
    }
    if candidates
        .iter()
        .filter(|c| baseline_keys.values().any(|key| key == c.key()))
        .any(|c| !assess_candidate(c, &eligibility_context).is_ok_and(|e| e.is_eligible()))
    {
        baseline_gaps.push(
            "Baseline equipment eligibility is not established under the supplied assumptions",
        );
    }
    let mut comparisons = Vec::new();
    let mut practical = Vec::new();
    let mut obtainable_candidates = Vec::new();
    for (candidate, input) in candidates.iter().zip(&inputs) {
        let eligibility = assess_candidate(candidate, &eligibility_context)?;
        let score = score_item(&model, candidate)?;
        let acquisition = input
            .acquisition
            .as_ref()
            .map(|a| {
                let source = content
                    .envelope
                    .payload
                    .acquisitions
                    .iter()
                    .find(|s| s.id == a.source)
                    .ok_or_else(|| invalid("Acquisition source is absent from active content"))?;
                assess_acquisition(
                    source,
                    candidate.key(),
                    &a.effort.effort(),
                    &BTreeSet::new(),
                    &evaluation,
                )
            })
            .transpose()?;
        if eligibility.is_eligible()
            && acquisition
                .as_ref()
                .is_some_and(|a| a.status == AcquisitionStatus::Guaranteed)
        {
            obtainable_candidates.push(candidate.clone());
        }
        let mut reasons = baseline_gaps.clone();
        if candidate.item.two_handed.value() == Some(&true) && layout.hands.is_none() {
            reasons.push("A two-handed candidate requires an explicit hand-slot relationship");
        }

        if !eligibility.is_eligible() {
            reasons.push("Equipment eligibility is not established");
        }
        if score.certainty != Certainty::Complete {
            reasons.push("Candidate model score is incomplete");
        }
        let qualified = acquisition.as_ref().filter(|a| {
            a.status != AcquisitionStatus::Unavailable
                && a.requirements
                    .as_ref()
                    .is_none_or(|r| r.truth == Truth::True)
                && a.credited_reward.is_some()
                && a.reasons
                    .iter()
                    .all(|r| matches!(r, AcquisitionReason::Probabilistic))
                && a.success_probability.is_some()
                && a.budgeted_effort_ms.is_some()
        });
        if qualified.is_none() {
            reasons.push("Acquisition eligibility, probability, or reserved effort is unavailable");
        }
        if reasons.is_empty() {
            let acquisition = qualified.expect("qualified acquisition checked");
            let probability_millionths = (acquisition.success_probability.unwrap().clamp(0.0, 1.0)
                * 1_000_000.0)
                .floor() as u32;
            for slot in &candidate.item.slots {
                if baseline_keys.get(slot) == Some(candidate.key()) {
                    continue;
                }
                let mut expected = baseline_keys.clone();
                expected.insert(slot.clone(), candidate.key().clone());
                if candidate.item.two_handed.value() == Some(&true)
                    && let Some(hands) = &layout.hands
                {
                    expected.remove(&hands.off_hand);
                }
                let proposed = theoretical.valid.iter().find(|c| {
                    configuration_keys(c) == expected && c.score.certainty == Certainty::Complete
                });
                if let Some(proposed) = proposed {
                    let gain = proposed
                        .score
                        .total
                        .checked_sub(current_score.total)
                        .filter(|n| n.unsigned_abs() <= MAX_SAFE_INTEGER)
                        .ok_or_else(|| invalid("Practical gain leaves safe arithmetic range"))?;
                    if gain <= 0 {
                        continue;
                    }
                    let expected_gain = i64::try_from(
                        i128::from(gain) * i128::from(probability_millionths) / 1_000_000,
                    )
                    .map_err(|_| invalid("Expected gain overflow"))?;
                    if expected_gain <= 0 {
                        continue;
                    }
                    practical.push(PracticalUpgrade {
                        item: candidate.key().clone(),
                        slot: slot.clone(),
                        proposed: proposed.clone(),
                        gain_milli: gain,
                        probability_millionths,
                        conditional_expected_gain_milli: expected_gain,
                        reserved_effort_ms: acquisition.budgeted_effort_ms.unwrap(),
                        conditional: acquisition.status != AcquisitionStatus::Guaranteed,
                    });
                }
            }
            if !practical.iter().any(|p| p.item == *candidate.key()) {
                reasons.push("No known legal positive single-acquisition upgrade under the complete baseline");
            }
        }
        comparisons.push(CandidateComparison {
            item: candidate.key().clone(),
            eligibility,
            score,
            acquisition,
            practical_reasons: reasons,
        });
    }
    practical.sort_by(|a, b| {
        let ae = a.reserved_effort_ms.get();
        let be = b.reserved_effort_ms.get();
        match (ae == 0, be == 0) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let left = i128::from(a.conditional_expected_gain_milli) * i128::from(be);
                let right = i128::from(b.conditional_expected_gain_milli) * i128::from(ae);
                right.cmp(&left)
            }
        }
        .then_with(|| a.item.cmp(&b.item))
        .then_with(|| a.slot.cmp(&b.slot))
    });
    let obtainable_scenario = GearScenario {
        candidates: &obtainable_candidates,
        ..scenario
    };
    let mut obtainable = rank_theoretical(&obtainable_scenario)?;
    obtainable.ranking = Ranking::Obtainable;
    obtainable.scope_note = "Individually guaranteed acquisition and equipment eligibility only; combined acquisition time and cost are not a joint plan.";
    for comparison in &comparisons {
        if !obtainable_candidates
            .iter()
            .any(|c| c.key() == &comparison.item)
        {
            obtainable.excluded.push(ExcludedCandidate {
                item: comparison.item.clone(),
                eligibility: comparison.eligibility.clone(),
            });
        }
    }
    Ok(GearReport {
        request,
        fixture: snapshot.fixture,
        context,
        current,
        current_score,
        baseline_gaps,
        theoretical,
        equip_eligible,
        obtainable,
        candidates: comparisons,
        practical,
        practical_model_version: PRACTICAL_MODEL_VERSION,
        assumptions: vec![
            "Weights, class, categories, proficiency, durations, spending ceilings and trial semantics are explicit manual assumptions; no game tables are supplied",
            "Model scores are not measured damage or survivability; useful lifespan is unknown",
            "Obtainable includes only individually guaranteed acquisitions; probabilistic candidates remain separately inspectable",
            "Practical estimates evaluate one acquisition against a complete legal baseline; independent upgrade probabilities are never combined",
            "gear-practical-v1 floors conditional success probability to millionths and expected positive gain to score thousandths; ordering uses exact bounded cross-products",
            "Zero reserved effort with positive expected gain sorts first; reserved effort covers maximum feasible trials, not expected time-to-success",
        ],
    })
}
fn configuration_keys(configuration: &RankedConfiguration) -> BTreeMap<SlotId, ItemKey> {
    configuration
        .configuration
        .items
        .iter()
        .map(|(s, i)| (s.clone(), i.item.clone()))
        .collect()
}
