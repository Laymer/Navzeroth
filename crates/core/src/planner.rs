//! Deterministic greedy baseline over caller-defined, disjoint objective clusters.
//! Utilities must share a declared goal profile/unit. No weights or world facts
//! are invented. Eligibility is reevaluated against the supplied snapshot, never
//! speculative future state. Feasibility is expected time, not a guarantee.
use crate::{
    eligibility::{
        AcceptanceOutcome, EligibilityInputs, ParticipantEligibility, TurnInOutcome,
        evaluate_participant,
    },
    entities::{TravelEdge, TravelNode, bounded, unique},
    error::{Result, invalid},
    evaluation::{EvaluationContext, EvaluationResult, Truth, evaluate},
    predicate::Predicate,
    travel::{Route, RouteOptions, RouteOutcome, RouteSearch, shortest_route},
    types::{Millis, NodeId, ObjectiveId, SourceId},
};
use serde::Serialize;
use std::collections::BTreeSet;

pub enum EligibilityGate<'a> {
    Predicate(&'a Predicate),
    QuestAcceptance(&'a EligibilityInputs<'a>),
    QuestTurnIn(&'a EligibilityInputs<'a>),
}
pub struct Candidate<'a> {
    pub id: ObjectiveId,
    pub node: NodeId,
    pub gate: EligibilityGate<'a>,
    /// Nonnegative integer utility in the caller's versioned goal profile.
    pub utility: u32,
    pub completion_ms: Millis,
    pub service_ms: Millis,
    /// Caller-declared disjoint reward identities; no reward can be selected twice.
    pub rewards: BTreeSet<SourceId>,
}
pub struct PlannerInput<'a> {
    pub goal_profile: SourceId,
    pub candidates: &'a [Candidate<'a>],
    pub excluded: &'a BTreeSet<ObjectiveId>,
    pub nodes: &'a [TravelNode],
    pub edges: &'a [TravelEdge],
    pub origin: NodeId,
    pub stopping_node: NodeId,
    pub stopping_service_ms: Millis,
    pub budget_ms: Millis,
    pub max_steps: usize,
    pub context: &'a EvaluationContext<'a>,
    pub travel_options: &'a RouteOptions,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EligibilityEvidence {
    Predicate { result: EvaluationResult },
    Quest { result: ParticipantEligibility },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Excluded,
    EligibilityFalse,
    EligibilityUnknown,
    RewardAlreadyCounted,
    NoUtility,
    Unreachable,
    StatefulIntermediate,
    OverBudget,
    Feasible,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateTrace {
    pub iteration: usize,
    pub candidate: ObjectiveId,
    pub decision: Decision,
    pub eligibility: Option<EligibilityEvidence>,
    pub travel: Option<RouteSearch>,
    pub stopping: Option<RouteSearch>,
    pub expected_session_ms: Option<Millis>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Choice {
    pub candidate: ObjectiveId,
    pub utility: u32,
    pub incremental_ms: Millis,
    pub completion_ms: Millis,
    pub service_ms: Millis,
    pub travel: Route,
    pub stopping: Route,
    pub expected_session_ms: Millis,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    NoFeasibleCandidate,
    StepLimit,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlannerResult {
    pub goal_profile: SourceId,
    pub selected: Vec<Choice>,
    /// Feasible nonselected choices at each iteration, ranked deterministically.
    pub alternatives: Vec<Vec<Choice>>,
    pub trace: Vec<CandidateTrace>,
    pub stopping: Route,
    pub expected_total_ms: Millis,
    pub termination: Termination,
    pub candidate_count: usize,
    pub candidates_evaluated: usize,
    /// All times are estimates; routes retain supplied ranges and unknowns.
    pub estimated: bool,
}
fn add(values: &[u64]) -> Result<Millis> {
    Millis::new(values.iter().try_fold(0u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| invalid("Planner duration overflows"))
    })?)
}
fn gate(
    candidate: &Candidate<'_>,
    context: &EvaluationContext<'_>,
) -> Result<(Truth, EligibilityEvidence)> {
    match &candidate.gate {
        EligibilityGate::Predicate(predicate) => {
            let result = evaluate(predicate, context)?;
            Ok((result.truth, EligibilityEvidence::Predicate { result }))
        }
        EligibilityGate::QuestAcceptance(inputs) | EligibilityGate::QuestTurnIn(inputs) => {
            let result = evaluate_participant(inputs, context.snapshot, context.party)?;
            let truth = match candidate.gate {
                EligibilityGate::QuestAcceptance(_) => match result.acceptance {
                    AcceptanceOutcome::Eligible => Truth::True,
                    AcceptanceOutcome::Unknown => Truth::Unknown,
                    _ => Truth::False,
                },
                _ => match result.turn_in {
                    TurnInOutcome::Ready => Truth::True,
                    TurnInOutcome::Unknown => Truth::Unknown,
                    _ => Truth::False,
                },
            };
            Ok((truth, EligibilityEvidence::Quest { result }))
        }
    }
}
/// At most 64 greedy selections and 1024 candidates. Ratio comparisons use exact
/// cross products: utility / incremental time (zero time ranks first), then
/// greater utility, shorter time and lexicographic ID. Filters run before score.
pub fn plan(input: &PlannerInput<'_>) -> Result<PlannerResult> {
    unique(input.candidates.iter().map(|c| &c.id))?;
    bounded(input.excluded.len())?;
    if input.max_steps == 0 || input.max_steps > 64 {
        return Err(invalid("Planner step bound must be 1..=64"));
    }
    for c in input.candidates {
        bounded(c.rewards.len())?;
    }
    let route = |from: &NodeId, to: &NodeId| {
        shortest_route(
            input.nodes,
            input.edges,
            from,
            to,
            input.context,
            input.travel_options,
        )
    };
    let initial_stop = route(&input.origin, &input.stopping_node)?;
    let RouteOutcome::Complete {
        route: mut stopping,
    } = initial_stop.outcome
    else {
        return Err(invalid("No supported stopping route"));
    };
    let mut expected_total_ms = add(&[stopping.total_ms.get(), input.stopping_service_ms.get()])?;
    if expected_total_ms > input.budget_ms {
        return Err(invalid("Stopping point exceeds session budget"));
    }
    let mut selected = Vec::new();
    let mut alternatives = Vec::new();
    let mut trace = Vec::new();
    let mut chosen = BTreeSet::new();
    let mut rewards = BTreeSet::new();
    let mut current = input.origin.clone();
    let mut spent = 0;
    let mut termination = Termination::StepLimit;
    for iteration in 0..input.max_steps {
        let mut feasible = Vec::new();
        let mut candidates: Vec<_> = input
            .candidates
            .iter()
            .filter(|c| !chosen.contains(&c.id))
            .collect();
        candidates.sort_by_key(|c| &c.id);
        for candidate in candidates {
            let mut entry = CandidateTrace {
                iteration,
                candidate: candidate.id.clone(),
                decision: Decision::Excluded,
                eligibility: None,
                travel: None,
                stopping: None,
                expected_session_ms: None,
            };
            if !input.excluded.contains(&candidate.id) {
                let (truth, evidence) = gate(candidate, input.context)?;
                entry.eligibility = Some(evidence);
                entry.decision = match truth {
                    Truth::False => Decision::EligibilityFalse,
                    Truth::Unknown => Decision::EligibilityUnknown,
                    Truth::True => Decision::Feasible,
                };
                if truth == Truth::True {
                    if !rewards.is_disjoint(&candidate.rewards) {
                        entry.decision = Decision::RewardAlreadyCounted;
                    } else if candidate.utility == 0 {
                        entry.decision = Decision::NoUtility;
                    } else {
                        let inbound = route(&current, &candidate.node)?;
                        let outbound = route(&candidate.node, &input.stopping_node)?;
                        match (&inbound.outcome, &outbound.outcome) {
                            (
                                RouteOutcome::Complete { route: travel },
                                RouteOutcome::Complete { route: stop },
                            ) => {
                                if travel.hops.iter().any(|hop| hop.stateful) {
                                    entry.decision = Decision::StatefulIntermediate;
                                } else {
                                    let incremental = add(&[
                                        travel.total_ms.get(),
                                        candidate.completion_ms.get(),
                                        candidate.service_ms.get(),
                                    ])?;
                                    let total = add(&[
                                        spent,
                                        incremental.get(),
                                        stop.total_ms.get(),
                                        input.stopping_service_ms.get(),
                                    ])?;
                                    entry.expected_session_ms = Some(total);
                                    if total > input.budget_ms {
                                        entry.decision = Decision::OverBudget;
                                    } else {
                                        feasible.push(Choice {
                                            candidate: candidate.id.clone(),
                                            utility: candidate.utility,
                                            incremental_ms: incremental,
                                            completion_ms: candidate.completion_ms,
                                            service_ms: candidate.service_ms,
                                            travel: travel.clone(),
                                            stopping: stop.clone(),
                                            expected_session_ms: total,
                                        });
                                    }
                                }
                            }
                            _ => entry.decision = Decision::Unreachable,
                        }
                        entry.travel = Some(inbound);
                        entry.stopping = Some(outbound);
                    }
                }
            }
            trace.push(entry);
        }
        feasible.sort_by(|a, b| {
            let ratio = (u128::from(b.utility) * u128::from(a.incremental_ms.get()))
                .cmp(&(u128::from(a.utility) * u128::from(b.incremental_ms.get())));
            ratio
                .then_with(|| b.utility.cmp(&a.utility))
                .then_with(|| a.incremental_ms.cmp(&b.incremental_ms))
                .then_with(|| a.candidate.cmp(&b.candidate))
        });
        if feasible.is_empty() {
            termination = Termination::NoFeasibleCandidate;
            break;
        }
        let best = feasible.remove(0);
        spent = add(&[spent, best.incremental_ms.get()])?.get();
        let candidate = input
            .candidates
            .iter()
            .find(|c| c.id == best.candidate)
            .expect("known candidate");
        rewards.extend(candidate.rewards.iter().cloned());
        chosen.insert(candidate.id.clone());
        current = candidate.node.clone();
        stopping = best.stopping.clone();
        expected_total_ms = best.expected_session_ms;
        selected.push(best);
        alternatives.push(feasible);
    }
    Ok(PlannerResult {
        goal_profile: input.goal_profile.clone(),
        selected,
        alternatives,
        candidates_evaluated: trace.len(),
        trace,
        stopping,
        expected_total_ms,
        termination,
        candidate_count: input.candidates.len(),
        estimated: true,
    })
}
