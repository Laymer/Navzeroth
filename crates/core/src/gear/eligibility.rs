//! M5.2: three-way equipment eligibility. Only `Truth::True` from the shared
//! evaluator, together with a definite proficiency, yields `Eligible`. A definite
//! negative is `Ineligible` (or `FutureProgression` when the only gate is a
//! level the character has not reached). Anything unknown stays `Unknown`.
//! Quest, faction, reputation, profession and acquisition checks are not
//! modelled here: they are evaluated only through the item's own requirement
//! predicate, and an item with no readable requirements stays `Unknown`.
use super::inputs::GearCandidate;
use crate::{
    entities::{bounded, unique},
    error::{Result, Validate, Validated, invalid},
    evaluation::{EvaluationContext, EvaluationIssue, Truth, evaluate},
    observation::{Observation, ObservationStatus},
    predicate::PredicateNode,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Categories a class may (`allowed`) or may not (`disallowed`) use. A category
/// in neither set is unmapped, which is unknown rather than disallowed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClassProficiency {
    pub allowed: BTreeSet<SourceId>,
    pub disallowed: BTreeSet<SourceId>,
}
/// Caller-supplied proficiency data keyed by class; core defines no table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProficiencyTableData {
    pub classes: RecordMap<SourceId, ClassProficiency>,
}
pub type ProficiencyTable = Validated<ProficiencyTableData>;
impl Validate for ProficiencyTableData {
    fn validate(&self) -> Result<()> {
        for entry in self.classes.values() {
            bounded(entry.allowed.len())?;
            bounded(entry.disallowed.len())?;
            unique(entry.allowed.iter().chain(entry.disallowed.iter()))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum IneligibleReason {
    CategoryNotProficient { class: SourceId, category: SourceId },
    LevelNotReached { required: Level },
    RequirementNotMet,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum UnknownReason {
    ClassUnavailable {
        status: ObservationStatus,
    },
    ClassNotInProficiencyTable {
        class: SourceId,
    },
    CategoryUnavailable {
        status: ObservationStatus,
    },
    CategoryNotInProficiencyTable {
        class: SourceId,
        category: SourceId,
    },
    RequirementsUnavailable {
        status: ObservationStatus,
    },
    /// Wraps the evaluator's own reason (missing record, stale, clock, unavailable).
    Requirement {
        issue: EvaluationIssue,
    },
    RequirementIndeterminate,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum GearEligibility {
    Eligible,
    /// Not usable now; the only gate is a level not yet reached. Shown as
    /// future progression, never as obtainable now.
    FutureProgression {
        required_level: Level,
    },
    Ineligible {
        reasons: Vec<IneligibleReason>,
    },
    Unknown {
        reasons: Vec<UnknownReason>,
    },
}
impl GearEligibility {
    pub fn is_eligible(&self) -> bool {
        matches!(self, Self::Eligible)
    }
}

/// Inputs for eligibility. Class comes from `CharacterProfile.class` (the
/// snapshot has none); it is judged by status only, not by freshness.
pub struct GearEligibilityContext<'a> {
    pub evaluation: &'a EvaluationContext<'a>,
    pub class: &'a Observation<SourceId>,
    pub proficiency: &'a ProficiencyTable,
}

/// The level named by a predicate made only of `LevelAtLeast` nodes (a lone
/// node or a non-empty `All` of them), which is the highest level required.
fn pure_level_gate(node: &PredicateNode) -> Option<Level> {
    match node {
        PredicateNode::LevelAtLeast { level } => Some(*level),
        PredicateNode::All { predicates } if !predicates.is_empty() => predicates
            .iter()
            .map(pure_level_gate)
            .try_fold(None, |best: Option<Level>, gate| {
                gate.map(|g| Some(best.map_or(g, |b| b.max(g))))
            })
            .flatten(),
        _ => None,
    }
}

/// Assess one candidate. A candidate from another edition, build, interface or
/// dataset namespace is `BuildMismatch`, never merged and never merely excluded.
pub fn assess_candidate(
    candidate: &GearCandidate,
    ctx: &GearEligibilityContext<'_>,
) -> Result<GearEligibility> {
    ctx.evaluation
        .snapshot
        .context
        .ensure_compatible(&candidate.key().context)?;
    let mut ineligible = Vec::new();
    let mut unknown = Vec::new();

    match (candidate.category.value(), candidate.category.status()) {
        (Some(category), _) => match ctx.class.value() {
            None => unknown.push(UnknownReason::ClassUnavailable {
                status: ctx.class.status(),
            }),
            Some(class) => match ctx.proficiency.classes.get(class) {
                None => unknown.push(UnknownReason::ClassNotInProficiencyTable {
                    class: class.clone(),
                }),
                Some(entry) if entry.allowed.contains(category) => {}
                Some(entry) if entry.disallowed.contains(category) => {
                    ineligible.push(IneligibleReason::CategoryNotProficient {
                        class: class.clone(),
                        category: category.clone(),
                    })
                }
                Some(_) => unknown.push(UnknownReason::CategoryNotInProficiencyTable {
                    class: class.clone(),
                    category: category.clone(),
                }),
            },
        },
        (None, ObservationStatus::NotApplicable) => {}
        (None, status) => unknown.push(UnknownReason::CategoryUnavailable { status }),
    }

    let mut level_gate = None;
    match candidate.item.requirements.value() {
        None => unknown.push(UnknownReason::RequirementsUnavailable {
            status: candidate.item.requirements.status(),
        }),
        Some(predicate) => {
            let result = evaluate(predicate, ctx.evaluation)?;
            match result.truth {
                Truth::True => {}
                Truth::False => match pure_level_gate(predicate) {
                    Some(level) => level_gate = Some(level),
                    None => ineligible.push(IneligibleReason::RequirementNotMet),
                },
                Truth::Unknown if result.reasons.is_empty() => {
                    unknown.push(UnknownReason::RequirementIndeterminate)
                }
                Truth::Unknown => unknown.extend(
                    result
                        .reasons
                        .into_iter()
                        .map(|issue| UnknownReason::Requirement { issue }),
                ),
            }
        }
    }

    Ok(
        match (ineligible.is_empty(), level_gate, unknown.is_empty()) {
            (true, None, true) => GearEligibility::Eligible,
            (true, None, false) => GearEligibility::Unknown { reasons: unknown },
            (true, Some(required_level), true) => {
                GearEligibility::FutureProgression { required_level }
            }
            (_, level_gate, _) => {
                // A definite negative dominates unknown findings.
                if let Some(required) = level_gate {
                    ineligible.push(IneligibleReason::LevelNotReached { required });
                }
                GearEligibility::Ineligible {
                    reasons: ineligible,
                }
            }
        },
    )
}

/// Assess every candidate, keyed by full `ItemKey`. Duplicate keys are an error.
pub fn assess_candidates(
    candidates: &[GearCandidate],
    ctx: &GearEligibilityContext<'_>,
) -> Result<BTreeMap<ItemKey, GearEligibility>> {
    bounded(candidates.len())?;
    let mut out = BTreeMap::new();
    for candidate in candidates {
        let assessment = assess_candidate(candidate, ctx)?;
        if out.insert(candidate.key().clone(), assessment).is_some() {
            return Err(invalid("Duplicate record identifier"));
        }
    }
    Ok(out)
}
