use azeroth_core::search_control::{NoopSearchControl, SearchControl, SearchPhase};
use azeroth_core::{
    contract::Envelope,
    entities::{CharacterSnapshot, Objective, ObjectiveTarget, TravelNode},
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, StalePolicy, Truth, evaluate},
    objective_clustering::{
        ClusterPlanningInput, PreparedContribution, PreparedObjective, plan_clustered_controlled,
    },
    observation::{FreshnessContext, ObservationStatus},
    planner::{EligibilityEvidence, PlannerResult},
    predicate::{Predicate, PredicateNode, QuestState, RuleLimits},
    travel::RouteOptions,
    types::*,
};
use azeroth_storage::content::ContentRevision;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Serialize)]
pub struct PlanningCatalog {
    pub snapshot_id: Option<SnapshotId>,
    pub revision: Option<DatasetRevision>,
    pub fixture: bool,
    pub nodes: Vec<TravelNode>,
    pub objectives: Vec<Objective>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateEstimate {
    pub objective: ObjectiveId,
    pub node: NodeId,
    pub utility: u32,
    pub completion_ms: Millis,
    pub service_ms: Millis,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningRequest {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
    pub revision: DatasetRevision,
    pub goal_profile: SourceId,
    pub origin: NodeId,
    pub stopping_node: NodeId,
    pub budget_ms: Millis,
    pub stopping_service_ms: Millis,
    pub candidates: Vec<CandidateEstimate>,
    pub excluded: BTreeSet<ObjectiveId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct FilteredCandidate {
    pub objective: ObjectiveId,
    pub reason: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlanningReport {
    pub request: PlanningRequest,
    pub fixture: bool,
    pub context: DataContext,
    pub result: PlannerResult,
    pub clustering: crate::clustering::ClusteringMetadata,
    pub filtered: Vec<FilteredCandidate>,
    pub assumptions: Vec<&'static str>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
pub(crate) fn calculate(
    request: PlanningRequest,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &ContentRevision,
    clock: &FreshnessContext,
) -> Result<PlanningReport> {
    calculate_controlled(request, snapshot, content, clock, &mut NoopSearchControl)
}
pub(crate) fn calculate_controlled(
    mut request: PlanningRequest,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &ContentRevision,
    clock: &FreshnessContext,
    control: &mut dyn SearchControl,
) -> Result<PlanningReport> {
    control.checkpoint(SearchPhase::ValidateInputs)?;
    if request.candidates.len() > 128 || request.excluded.len() > 128 {
        return Err(invalid(
            "Planning input exceeds 128 candidates or exclusions",
        ));
    }
    request
        .candidates
        .sort_by(|a, b| a.objective.cmp(&b.objective));
    let dataset = &content.envelope.payload;
    if request
        .excluded
        .iter()
        .any(|id| !dataset.objectives.iter().any(|o| &o.id == id))
    {
        return Err(invalid("Excluded objective is absent from active content"));
    }
    let context = EvaluationContext {
        snapshot: &snapshot.payload,
        party: None,
        freshness: clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let mut candidates = Vec::new();
    let mut filtered = Vec::new();
    let mut seen = BTreeSet::new();
    for estimate in &request.candidates {
        control.checkpoint(SearchPhase::CandidateAssessment)?;
        if !seen.insert(&estimate.objective) {
            return Err(invalid("Duplicate planning candidate"));
        }
        let objective = dataset
            .objectives
            .iter()
            .find(|o| o.id == estimate.objective)
            .ok_or_else(|| invalid("Planning objective is absent from active content"))?;
        if !dataset.nodes.iter().any(|n| n.id == estimate.node) {
            return Err(invalid("Planning node is absent from active content"));
        }
        let mut reason = None;
        if objective.participants != [snapshot.payload.character.clone()] {
            reason = Some("Objective requires a different participant or a joint fairness model");
        }
        for target in &objective.targets {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            match target {
                ObjectiveTarget::QuestStage { quest, stage } => {
                    let definition = dataset
                        .quests
                        .iter()
                        .find(|q| &q.id == quest)
                        .ok_or_else(|| invalid("Quest target is absent from active content"))?;
                    if !definition.stages.contains(stage) {
                        return Err(invalid("Stage is not declared by its quest"));
                    }
                    let ready = Predicate::try_new(PredicateNode::All {
                        predicates: vec![
                            PredicateNode::QuestState {
                                quest: quest.clone(),
                                state: QuestState::Accepted,
                                value: true,
                            },
                            PredicateNode::QuestState {
                                quest: quest.clone(),
                                state: QuestState::Completed,
                                value: false,
                            },
                            PredicateNode::QuestState {
                                quest: quest.clone(),
                                state: QuestState::TurnedIn,
                                value: false,
                            },
                            PredicateNode::ObjectiveState {
                                objective: stage.clone(),
                                completed: false,
                            },
                        ],
                    })?;
                    if evaluate(&ready, &context)?.truth != Truth::True {
                        reason =
                            Some("Quest stage is completed, not accepted, or readiness is unknown");
                    }
                }
                ObjectiveTarget::Service { node }
                    if node.id != estimate.node
                        || !dataset
                            .nodes
                            .iter()
                            .any(|n| n.id == node.id && n.position.map == node.map) =>
                {
                    reason = Some("Service objective must use its declared node");
                }
                ObjectiveTarget::Acquisition { .. } => {
                    reason =
                        Some("Acquisition requirements are not yet integrated into this planner");
                }
                _ => {}
            }
        }
        let predicate = objective.requirements.value();
        if predicate.is_none()
            && objective.requirements.status() != ObservationStatus::NotApplicable
        {
            reason = Some("Objective requirements are unavailable");
        }
        if let Some(reason) = reason {
            filtered.push(FilteredCandidate {
                objective: objective.id.clone(),
                reason,
            });
            continue;
        }
        // NotApplicable is an explicit absence of a requirement, not unknown.
        candidates.push((estimate, objective, predicate));
    }
    let always =
        azeroth_core::predicate::Predicate::try_new(azeroth_core::predicate::PredicateNode::All {
            predicates: vec![],
        })?;
    let values = candidates
        .iter()
        .map(|(estimate, objective, predicate)| {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            let evidence = evaluate(predicate.unwrap_or(&always), &context)?;
            Ok(PreparedObjective {
                id: objective.id.clone(),
                node: estimate.node.clone(),
                participants: vec![PreparedContribution {
                    utility: estimate.utility,
                    truth: evidence.truth,
                    evidence: EligibilityEvidence::Predicate { result: evidence },
                }],
                completion_ms: estimate.completion_ms,
                service_ms: estimate.service_ms,
                // One reward identity per target prevents different objectives counting
                // the same quest/acquisition/service contribution twice.
                rewards: objective
                    .targets
                    .iter()
                    .map(|target| match target {
                        ObjectiveTarget::QuestStage { quest, .. } => {
                            SourceId::new(format!("quest:{}", quest.as_str()))
                        }
                        ObjectiveTarget::Acquisition { source } => {
                            SourceId::new(format!("acquisition:{}", source.as_str()))
                        }
                        ObjectiveTarget::Service { node } => {
                            SourceId::new(format!("service:{}", node.id.as_str()))
                        }
                    })
                    .collect::<Result<BTreeSet<_>>>()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let clustered = plan_clustered_controlled(
        &ClusterPlanningInput {
            goal_profile: request.goal_profile.clone(),
            objectives: &values,
            participant_count: 1,
            excluded: &request.excluded,
            nodes: &dataset.nodes,
            edges: &dataset.edges,
            origin: request.origin.clone(),
            stopping_node: request.stopping_node.clone(),
            stopping_service_ms: request.stopping_service_ms,
            budget_ms: request.budget_ms,
            max_steps: 32,
            context: &context,
            travel_options: &RouteOptions::default(),
        },
        control,
    )?;
    Ok(PlanningReport {
        request,
        fixture: snapshot.fixture,
        context: snapshot.data_context(),
        result: clustered.plan.clone(),
        clustering: crate::clustering::ClusteringMetadata::from(clustered),
        filtered,
        assumptions: vec![
            "Utilities, objective node associations and completion/service times are explicit manual inputs",
            "Expected-time greedy baseline with deterministic same-node clusters and singleton fallbacks; estimates are not guaranteed durations",
            "No speculative prerequisite unlocks or party fairness; declared quest predicates only; no shared-kill or service savings inferred",
        ],
    })
}
