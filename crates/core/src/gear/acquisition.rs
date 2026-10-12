//! Finite-session acquisition assessment. No drop rates, utility weights, clocks,
//! gold/time conversion or useful-lifespan guesses are supplied by this module.
use crate::{
    entities::AcquisitionSource,
    error::{Result, invalid},
    evaluation::{EvaluationContext, EvaluationResult, Truth, evaluate},
    observation::{Observation, ObservationStatus},
    types::{Copper, ItemKey, Millis},
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbabilityModel {
    /// A declared source/drop probability, with player loot eligibility unknown.
    DeclaredOnly,
    /// Caller explicitly asserts independent identical player acquisition trials.
    IndependentPlayerAttempts,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSchedule {
    Once,
    PerAttempt,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AcquisitionEffort {
    pub incremental_travel_ms: Observation<Millis>,
    pub formation_ms: Observation<Millis>,
    pub attempt_ms: Observation<Millis>,
    pub stopping_ms: Observation<Millis>,
    pub available_copper: Observation<Copper>,
    pub session_budget_ms: Millis,
    pub max_attempts: u32,
    pub probability_model: ProbabilityModel,
    pub cost_schedule: CostSchedule,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionStatus {
    Guaranteed,
    Uncertain,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortField {
    Travel,
    Formation,
    Attempt,
    Stopping,
    AvailableCopper,
    Cost,
    Probability,
    Conditions,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum AcquisitionReason {
    AlreadyCounted,
    RequirementsNotMet,
    RequirementsUnknown,
    FieldUnavailable {
        field: EffortField,
        status: ObservationStatus,
    },
    SessionTooShort,
    InsufficientCopper,
    ZeroProbability,
    ProbabilityModelUnqualified,
    Probabilistic,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AcquisitionAssessment {
    /// Original facts/metadata kept for traceability and uncertainty display.
    pub source: AcquisitionSource,
    pub effort: AcquisitionEffort,
    pub target: ItemKey,
    pub status: AcquisitionStatus,
    pub reasons: Vec<AcquisitionReason>,
    pub requirements: Option<EvaluationResult>,
    /// None when critical timing or cost prevents a finite attempt count.
    pub feasible_attempts: Option<u32>,
    /// Budget reservation for the maximum feasible attempts, not an expected time-to-success.
    pub budgeted_effort_ms: Option<Millis>,
    pub budgeted_cost_copper: Option<Copper>,
    /// Conditional on the explicitly supplied independent player model only.
    pub success_probability: Option<f64>,
    /// Infinite-horizon expectation, never the finite-session attempt count.
    pub expected_attempts: Option<f64>,
    /// Full reward identity eligible for one opportunity credit, not observed loot.
    /// None if already counted, unavailable, or eligibility/effort is indeterminate.
    /// Assessment never adds utility.
    pub credited_reward: Option<ItemKey>,
}
fn duration(
    observation: &Observation<Millis>,
    field: EffortField,
    reasons: &mut Vec<AcquisitionReason>,
) -> Option<u64> {
    if let Some(value) = observation.value() {
        Some(value.get())
    } else if observation.status() == ObservationStatus::NotApplicable {
        Some(0)
    } else {
        reasons.push(AcquisitionReason::FieldUnavailable {
            field,
            status: observation.status(),
        });
        None
    }
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| invalid("Acquisition effort overflows"))
}
/// Assess one source/target. The caller shares `already_counted_rewards` with
/// quest/gear utility accounting; full ItemKey prevents edition/variant mixing.
/// Time feasibility uses declared expected durations. Zero-duration attempts are
/// bounded by max_attempts, never interpreted as infinitely many trials.
pub fn assess_acquisition(
    source: &AcquisitionSource,
    target: &ItemKey,
    effort: &AcquisitionEffort,
    already_counted_rewards: &BTreeSet<ItemKey>,
    context: &EvaluationContext<'_>,
) -> Result<AcquisitionAssessment> {
    source
        .context
        .ensure_compatible(&context.snapshot.context)?;
    source.context.ensure_compatible(&target.context)?;
    if !source.items.contains(target) {
        return Err(invalid("Target item is absent from acquisition source"));
    }
    if effort.max_attempts == 0 || effort.max_attempts > 1024 {
        return Err(invalid("Acquisition attempts must be bounded by 1..=1024"));
    }
    if already_counted_rewards.len() > 1024 {
        return Err(invalid("Too many already counted rewards"));
    }
    let mut out = AcquisitionAssessment {
        source: source.clone(),
        effort: effort.clone(),
        target: target.clone(),
        status: AcquisitionStatus::Uncertain,
        reasons: vec![],
        requirements: None,
        feasible_attempts: None,
        budgeted_effort_ms: None,
        budgeted_cost_copper: None,
        success_probability: None,
        expected_attempts: None,
        credited_reward: None,
    };
    if already_counted_rewards.contains(target) {
        out.status = AcquisitionStatus::Unavailable;
        out.credited_reward = None;
        out.reasons.push(AcquisitionReason::AlreadyCounted);
        return Ok(out);
    }
    let eligible = match source.conditions.value() {
        Some(predicate) => {
            let result = evaluate(predicate, context)?;
            let truth = result.truth;
            out.requirements = Some(result);
            truth
        }
        None if source.conditions.status() == ObservationStatus::NotApplicable => Truth::True,
        None => {
            out.reasons.push(AcquisitionReason::FieldUnavailable {
                field: EffortField::Conditions,
                status: source.conditions.status(),
            });
            Truth::Unknown
        }
    };
    match eligible {
        Truth::False => {
            out.status = AcquisitionStatus::Unavailable;
            out.reasons.push(AcquisitionReason::RequirementsNotMet);
            return Ok(out);
        }
        Truth::Unknown => out.reasons.push(AcquisitionReason::RequirementsUnknown),
        Truth::True => {}
    }
    let travel = duration(
        &effort.incremental_travel_ms,
        EffortField::Travel,
        &mut out.reasons,
    );
    let formation = duration(
        &effort.formation_ms,
        EffortField::Formation,
        &mut out.reasons,
    );
    let stopping = duration(&effort.stopping_ms, EffortField::Stopping, &mut out.reasons);
    let attempt = duration(&effort.attempt_ms, EffortField::Attempt, &mut out.reasons);
    let cost = match source.cost.value() {
        Some(c) => Some(c.get()),
        None if source.cost.status() == ObservationStatus::NotApplicable => Some(0),
        None => {
            out.reasons.push(AcquisitionReason::FieldUnavailable {
                field: EffortField::Cost,
                status: source.cost.status(),
            });
            None
        }
    };
    let mut count = None;
    if let (Some(travel), Some(formation), Some(stopping), Some(attempt)) =
        (travel, formation, stopping, attempt)
    {
        let overhead = add(add(travel, formation)?, stopping)?;
        let time_count = effort
            .session_budget_ms
            .get()
            .checked_sub(overhead)
            .map(|remaining| {
                remaining
                    .checked_div(attempt)
                    .unwrap_or(u64::from(effort.max_attempts))
                    .min(u64::from(effort.max_attempts)) as u32
            })
            .unwrap_or(0);
        if time_count == 0 {
            out.status = AcquisitionStatus::Unavailable;
            out.reasons.push(AcquisitionReason::SessionTooShort);
            count = Some(0);
        } else if let Some(cost) = cost {
            let affordable = if cost == 0 {
                Some(time_count)
            } else if let Some(balance) = effort.available_copper.value() {
                Some(match effort.cost_schedule {
                    CostSchedule::Once => {
                        if balance.get() >= cost {
                            time_count
                        } else {
                            0
                        }
                    }
                    CostSchedule::PerAttempt => {
                        (balance.get() / cost).min(u64::from(time_count)) as u32
                    }
                })
            } else {
                out.reasons.push(AcquisitionReason::FieldUnavailable {
                    field: EffortField::AvailableCopper,
                    status: effort.available_copper.status(),
                });
                None
            };
            if let Some(affordable) = affordable {
                count = Some(affordable);
                out.budgeted_effort_ms = Some(Millis::new(add(
                    overhead,
                    attempt
                        .checked_mul(u64::from(affordable))
                        .ok_or_else(|| invalid("Acquisition effort overflows"))?,
                )?)?);
                out.budgeted_cost_copper = Some(Copper::new(match effort.cost_schedule {
                    CostSchedule::Once => {
                        if affordable > 0 {
                            cost
                        } else {
                            0
                        }
                    }
                    CostSchedule::PerAttempt => cost
                        .checked_mul(u64::from(affordable))
                        .ok_or_else(|| invalid("Acquisition cost overflows"))?,
                }));
                if affordable == 0 {
                    out.status = AcquisitionStatus::Unavailable;
                    out.reasons.push(AcquisitionReason::InsufficientCopper);
                }
            }
        }
    }
    out.feasible_attempts = count;
    match source.probability.value() {
        Some(probability) if probability.get() == 0.0 => {
            out.status = AcquisitionStatus::Unavailable;
            out.reasons.push(AcquisitionReason::ZeroProbability);
            out.success_probability = Some(0.0);
        }
        Some(probability)
            if effort.probability_model == ProbabilityModel::IndependentPlayerAttempts =>
        {
            let p = probability.get();
            let expectation = 1.0 / p;
            if expectation.is_finite() {
                out.expected_attempts = Some(expectation);
            }
            if let Some(n) = count {
                out.success_probability = Some(if p == 1.0 {
                    if n > 0 { 1.0 } else { 0.0 }
                } else {
                    -(f64::from(n) * (-p).ln_1p()).exp_m1()
                });
            }
            if p == 1.0
                && eligible == Truth::True
                && count.is_some_and(|n| n > 0)
                && out.reasons.is_empty()
            {
                out.status = AcquisitionStatus::Guaranteed;
            } else if p < 1.0 {
                out.reasons.push(AcquisitionReason::Probabilistic);
            }
        }
        Some(_) => out
            .reasons
            .push(AcquisitionReason::ProbabilityModelUnqualified),
        None => out.reasons.push(AcquisitionReason::FieldUnavailable {
            field: EffortField::Probability,
            status: source.probability.status(),
        }),
    }
    if out.status != AcquisitionStatus::Unavailable
        && eligible == Truth::True
        && count.is_some_and(|n| n > 0)
    {
        out.credited_reward = Some(target.clone());
    }
    Ok(out)
}
