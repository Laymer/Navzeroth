//! Conservative manual group baseline: every selected objective must contribute
//! positive declared utility to every member and be ready for every member.
//! Group utility is the minimum contribution, so one member cannot dominate.
use crate::search_control::{NoopSearchControl, SearchControl, SearchPhase};
use crate::{
    entities::{TravelEdge, TravelNode},
    error::{Result, invalid},
    evaluation::{EvaluationContext, EvaluationResult, Truth, evaluate},
    observation::{Freshness, ObservationStatus, freshness},
    planner::{Candidate, EligibilityGate, PlannerInput, PlannerResult, plan_controlled},
    predicate::Predicate,
    travel::RouteOptions,
    types::{CharacterId, EdgeId, Millis, NodeId, ObjectiveId, SourceId},
};
use serde::Serialize;
use std::collections::BTreeSet;

pub struct JointCandidate<'a> {
    pub id: ObjectiveId,
    pub node: NodeId,
    pub readiness: Vec<&'a Predicate>,
    pub utilities: Vec<u32>,
    pub completion_ms: Millis,
    pub service_ms: Millis,
    pub rewards: BTreeSet<SourceId>,
}
pub struct JointInput<'a> {
    pub contexts: &'a [EvaluationContext<'a>],
    pub candidates: &'a [JointCandidate<'a>],
    pub goal_profile: SourceId,
    pub excluded: &'a BTreeSet<ObjectiveId>,
    pub nodes: &'a [TravelNode],
    pub edges: &'a [TravelEdge],
    pub origin: NodeId,
    pub stopping_node: NodeId,
    pub budget_ms: Millis,
    pub stopping_service_ms: Millis,
}
#[derive(Clone, Debug, Serialize)]
pub struct MemberReadiness {
    pub character: CharacterId,
    pub utility: u32,
    pub readiness: EvaluationResult,
}
#[derive(Clone, Debug, Serialize)]
pub struct JointCandidateAssessment {
    pub objective: ObjectiveId,
    pub ready: bool,
    pub participants: Vec<MemberReadiness>,
}
#[derive(Clone, Debug, Serialize)]
pub struct JointEdgeExclusion {
    pub edge: EdgeId,
    pub character: CharacterId,
    pub truth: Truth,
}
#[derive(Clone, Debug, Serialize)]
pub struct JointResult {
    pub plan: PlannerResult,
    pub assessments: Vec<JointCandidateAssessment>,
    pub unavailable_edges: Vec<JointEdgeExclusion>,
}
pub fn plan_joint(input: &JointInput<'_>) -> Result<JointResult> {
    plan_joint_controlled(input, &mut NoopSearchControl)
}

/// Cooperative joint readiness and nested route planning.
pub fn plan_joint_controlled(
    input: &JointInput<'_>,
    control: &mut dyn SearchControl,
) -> Result<JointResult> {
    control.checkpoint(SearchPhase::ValidateInputs)?;
    if !(2..=40).contains(&input.contexts.len())
        || input.candidates.len() > 128
        || input.excluded.len() > 128
    {
        return Err(invalid(
            "Joint planning requires 2..=40 participants and at most 128 candidates",
        ));
    }
    let first = &input.contexts[0];
    let mut members = BTreeSet::new();
    for context in input.contexts {
        control.checkpoint(SearchPhase::ParticipantReadiness)?;
        first
            .snapshot
            .context
            .ensure_compatible(&context.snapshot.context)?;
        if !members.insert(&context.snapshot.character)
            || !matches!(
                freshness(&context.snapshot.capture, context.freshness),
                Freshness::Fresh { .. }
            )
        {
            return Err(invalid(
                "Joint participants must be distinct and have fresh trusted captures",
            ));
        }
    }
    let mut assessments = Vec::new();
    let mut candidates = Vec::new();
    let mut ids = BTreeSet::new();
    for candidate in input.candidates {
        control.checkpoint(SearchPhase::CandidateAssessment)?;
        if candidate.readiness.len() != input.contexts.len()
            || candidate.utilities.len() != input.contexts.len()
            || !ids.insert(&candidate.id)
        {
            return Err(invalid(
                "Each joint candidate needs exactly one readiness and utility per participant",
            ));
        }
        let participants = input
            .contexts
            .iter()
            .zip(&candidate.readiness)
            .zip(&candidate.utilities)
            .map(|((context, predicate), utility)| {
                control.checkpoint(SearchPhase::ParticipantReadiness)?;
                Ok(MemberReadiness {
                    character: context.snapshot.character.clone(),
                    utility: *utility,
                    readiness: evaluate(predicate, context)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let ready = participants
            .iter()
            .all(|p| p.utility > 0 && p.readiness.truth == Truth::True);
        if ready {
            candidates.push(Candidate {
                id: candidate.id.clone(),
                node: candidate.node.clone(),
                gate: EligibilityGate::Predicate(candidate.readiness[0]),
                utility: *candidate.utilities.iter().min().expect("validated members"),
                completion_ms: candidate.completion_ms,
                service_ms: candidate.service_ms,
                rewards: candidate.rewards.clone(),
            });
        }
        assessments.push(JointCandidateAssessment {
            objective: candidate.id.clone(),
            ready,
            participants,
        });
    }
    let mut edges = Vec::new();
    let mut unavailable_edges = Vec::new();
    for edge in input.edges {
        control.checkpoint(SearchPhase::EdgeAssessment)?;
        let mut ready = true;
        for context in input.contexts {
            control.checkpoint(SearchPhase::ParticipantReadiness)?;
            let truth = match (edge.requirements.value(), edge.requirements.status()) {
                (Some(predicate), _) => evaluate(predicate, context)?.truth,
                (None, ObservationStatus::NotApplicable) => Truth::True,
                _ => Truth::Unknown,
            };
            if truth != Truth::True {
                ready = false;
                unavailable_edges.push(JointEdgeExclusion {
                    edge: edge.id.clone(),
                    character: context.snapshot.character.clone(),
                    truth,
                });
            }
        }
        if ready {
            edges.push(edge.clone());
        }
    }
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    assessments.sort_by(|a, b| a.objective.cmp(&b.objective));
    unavailable_edges.sort_by(|a, b| (&a.edge, &a.character).cmp(&(&b.edge, &b.character)));
    let plan = plan_controlled(
        &PlannerInput {
            goal_profile: input.goal_profile.clone(),
            candidates: &candidates,
            excluded: input.excluded,
            nodes: input.nodes,
            edges: &edges,
            origin: input.origin.clone(),
            stopping_node: input.stopping_node.clone(),
            budget_ms: input.budget_ms,
            stopping_service_ms: input.stopping_service_ms,
            max_steps: 32,
            context: first,
            travel_options: &RouteOptions::default(),
        },
        control,
    )?;
    control.checkpoint(SearchPhase::Selection)?;
    Ok(JointResult {
        plan,
        assessments,
        unavailable_edges,
    })
}
