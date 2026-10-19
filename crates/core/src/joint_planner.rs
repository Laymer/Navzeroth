//! Conservative manual group baseline: every selected objective must contribute
//! positive declared utility to every member and be ready for every member.
//! Group utility is the minimum contribution, so one member cannot dominate.
use crate::search_control::{NoopSearchControl, SearchControl, SearchPhase};
use crate::{
    entities::{EdgeStatus, TravelEdge, TravelNode},
    error::{Result, invalid},
    evaluation::{EvaluationContext, EvaluationResult, Truth, evaluate},
    objective_clustering::{
        ClusterPlanningInput, ClusteredResult, PreparedContribution, PreparedObjective,
        plan_clustered_controlled,
    },
    observation::{Freshness, ObservationStatus, freshness},
    planner::{
        Candidate, EligibilityEvidence, EligibilityGate, PlannerInput, PlannerResult,
        plan_controlled,
    },
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
    /// Travel policy of the active content (accepted inferred edges and, with
    /// `accept_unknown_requirements`, their unknown requirements). Default:
    /// strict.
    pub travel_options: &'a RouteOptions,
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
    Ok(calculate_joint(input, control, false)?.0)
}

/// Same conservative member/edge checks, with bounded same-node clustering.
/// Flattened choices retain original objective IDs; explanations are separate.
pub fn plan_joint_clustered_controlled(
    input: &JointInput<'_>,
    control: &mut dyn SearchControl,
) -> Result<(JointResult, ClusteredResult)> {
    let (result, clustered) = calculate_joint(input, control, true)?;
    Ok((result, clustered.expect("clustering requested")))
}

fn calculate_joint(
    input: &JointInput<'_>,
    control: &mut dyn SearchControl,
    automatic_clustering: bool,
) -> Result<(JointResult, Option<ClusteredResult>)> {
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
        let assumes_unknown = input.travel_options.accept_unknown_requirements
            && edge.status == EdgeStatus::Unverified
            && input.travel_options.inferred_edges.contains(&edge.id);
        for context in input.contexts {
            control.checkpoint(SearchPhase::ParticipantReadiness)?;
            let truth = match (edge.requirements.value(), edge.requirements.status()) {
                (Some(predicate), _) => evaluate(predicate, context)?.truth,
                (None, ObservationStatus::NotApplicable) => Truth::True,
                // An accepted inferred edge's unknown requirement stays an
                // assumption: the route search marks the hop, nothing here
                // calls it satisfied.
                _ if assumes_unknown => Truth::True,
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
    let mut prepared = Vec::new();
    if automatic_clustering {
        for (candidate, assessment) in input.candidates.iter().zip(&assessments) {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            prepared.push(PreparedObjective {
                id: candidate.id.clone(),
                node: candidate.node.clone(),
                completion_ms: candidate.completion_ms,
                service_ms: candidate.service_ms,
                rewards: candidate.rewards.clone(),
                participants: assessment
                    .participants
                    .iter()
                    .map(|member| PreparedContribution {
                        utility: member.utility,
                        truth: member.readiness.truth,
                        evidence: EligibilityEvidence::Predicate {
                            result: member.readiness.clone(),
                        },
                    })
                    .collect(),
            });
        }
    }
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    assessments.sort_by(|a, b| a.objective.cmp(&b.objective));
    unavailable_edges.sort_by(|a, b| (&a.edge, &a.character).cmp(&(&b.edge, &b.character)));
    let clustered = if automatic_clustering {
        Some(plan_clustered_controlled(
            &ClusterPlanningInput {
                goal_profile: input.goal_profile.clone(),
                participant_count: input.contexts.len(),
                objectives: &prepared,
                excluded: input.excluded,
                nodes: input.nodes,
                edges: &edges,
                origin: input.origin.clone(),
                stopping_node: input.stopping_node.clone(),
                budget_ms: input.budget_ms,
                stopping_service_ms: input.stopping_service_ms,
                max_steps: 32,
                context: first,
                travel_options: input.travel_options,
            },
            control,
        )?)
    } else {
        None
    };
    let plan = if let Some(result) = &clustered {
        result.plan.clone()
    } else {
        plan_controlled(
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
                travel_options: input.travel_options,
            },
            control,
        )?
    };
    control.checkpoint(SearchPhase::Selection)?;
    Ok((
        JointResult {
            plan,
            assessments,
            unavailable_edges,
        },
        clustered,
    ))
}
