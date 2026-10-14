//! Conservative clustering of independently assessed objectives at the same
//! declared node. No proximity, shared kills, duration saving or future state is
//! inferred. The caller owns snapshot/context validation and readiness evaluation.
//! This module reads no clock and uses the existing controlled greedy planner for
//! exact ranking, supported travel and stopping feasibility.
use crate::{
    entities::{NodeRef, TravelEdge, TravelNode, bounded, unique},
    error::{Result, invalid},
    evaluation::{EvaluationContext, Truth},
    planner::{
        Candidate, CandidateTrace, Choice, Decision, EligibilityEvidence, EligibilityGate,
        PlannerInput, PlannerResult, Termination, plan_controlled,
    },
    predicate::{Predicate, PredicateNode},
    search_control::{NoopSearchControl, SearchControl, SearchPhase},
    travel::{Route, RouteOptions},
    types::{Millis, NodeId, ObjectiveId, SourceId},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_OBJECTIVES: usize = 128;
pub const MAX_PARTICIPANTS: usize = 40;
/// Every original has a singleton; additional disjoint packs have >=2 members.
pub const MAX_VARIANTS: usize = MAX_OBJECTIVES + MAX_OBJECTIVES / 2;
pub const CLUSTER_SCHEMA_VERSION: u16 = 1;

/// A contribution already evaluated against one actual participant's snapshot.
/// All objectives use the same participant order and versioned utility unit.
#[derive(Clone, Debug, Serialize)]
pub struct PreparedContribution {
    pub utility: u32,
    pub truth: Truth,
    pub evidence: EligibilityEvidence,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparedObjective {
    pub id: ObjectiveId,
    pub node: NodeId,
    pub completion_ms: Millis,
    pub service_ms: Millis,
    pub rewards: BTreeSet<SourceId>,
    pub participants: Vec<PreparedContribution>,
}
pub struct ClusterPlanningInput<'a> {
    pub goal_profile: SourceId,
    /// One for solo; 2..=40 for joint. Also explicit for an empty candidate list.
    pub participant_count: usize,
    pub objectives: &'a [PreparedObjective],
    pub excluded: &'a BTreeSet<ObjectiveId>,
    pub nodes: &'a [TravelNode],
    /// For joint planning, the caller supplies the all-participants-usable graph.
    pub edges: &'a [TravelEdge],
    pub origin: NodeId,
    pub stopping_node: NodeId,
    pub stopping_service_ms: Millis,
    pub budget_ms: Millis,
    /// Bound on original objectives, not cluster selections (1..=64).
    pub max_steps: usize,
    pub context: &'a EvaluationContext<'a>,
    pub travel_options: &'a RouteOptions,
}
#[derive(Clone, Debug, Serialize)]
pub struct ObjectiveAssessment {
    pub objective: ObjectiveId,
    pub participants: Vec<PreparedContribution>,
    pub decision: Decision,
}
/// Utility is min(sum of each participant's contributions), not sum of minima.
/// Duration composition is strictly additive. Inbound travel is charged once.
#[derive(Clone, Debug, Serialize)]
pub struct ClusterChoice {
    pub members: Vec<ObjectiveId>,
    pub node: NodeId,
    pub participant_utilities: Vec<u32>,
    pub utility: u32,
    pub completion_ms: Millis,
    pub service_ms: Millis,
    pub inbound_ms: Millis,
    pub stopping_ms: Millis,
    pub incremental_ms: Millis,
    pub expected_session_ms: Millis,
}
#[derive(Clone, Debug, Serialize)]
pub struct VariantTrace {
    pub iteration: usize,
    pub members: Vec<ObjectiveId>,
    pub decision: Decision,
    pub expected_session_ms: Option<Millis>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ClusteredResult {
    pub schema_version: u16,
    /// Original IDs only; alternatives here are single-objective alternatives.
    /// Individual choice utility remains the original minimum contribution.
    pub plan: PlannerResult,
    /// Original readiness, including false/unknown evidence, in ID order.
    pub assessments: Vec<ObjectiveAssessment>,
    pub selected_clusters: Vec<ClusterChoice>,
    /// Includes both singleton and multi-objective alternatives, in rank order.
    pub cluster_alternatives: Vec<Vec<ClusterChoice>>,
    pub variant_trace: Vec<VariantTrace>,
    pub peak_variant_count: usize,
    pub variants_evaluated: usize,
}

fn sum_ms(values: impl IntoIterator<Item = Millis>) -> Result<Millis> {
    Millis::new(values.into_iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(value.get())
            .ok_or_else(|| invalid("Cluster duration overflows"))
    })?)
}
fn decision(objective: &PreparedObjective, excluded: &BTreeSet<ObjectiveId>) -> Decision {
    if excluded.contains(&objective.id) {
        Decision::Excluded
    } else if objective
        .participants
        .iter()
        .any(|p| p.truth == Truth::False)
    {
        Decision::EligibilityFalse
    } else if objective
        .participants
        .iter()
        .any(|p| p.truth == Truth::Unknown)
    {
        Decision::EligibilityUnknown
    } else if objective.participants.iter().any(|p| p.utility == 0) {
        Decision::NoUtility
    } else {
        Decision::Feasible
    }
}
struct Variant<'a> {
    members: Vec<&'a PreparedObjective>,
    utilities: Vec<u32>,
    completion: Millis,
    service: Millis,
    rewards: BTreeSet<SourceId>,
}
impl<'a> Variant<'a> {
    fn new(
        members: Vec<&'a PreparedObjective>,
        participant_count: usize,
        control: &mut dyn SearchControl,
    ) -> Result<Self> {
        let mut utilities = vec![0_u32; participant_count];
        let mut rewards = BTreeSet::new();
        for member in &members {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            if !rewards.is_disjoint(&member.rewards) {
                return Err(invalid("Cluster members share reward identities"));
            }
            rewards.extend(member.rewards.iter().cloned());
            for (sum, contribution) in utilities.iter_mut().zip(&member.participants) {
                control.checkpoint(SearchPhase::ParticipantReadiness)?;
                *sum = sum
                    .checked_add(contribution.utility)
                    .ok_or_else(|| invalid("Cluster utility overflows"))?;
            }
        }
        let completion = sum_ms(members.iter().map(|m| m.completion_ms))?;
        let service = sum_ms(members.iter().map(|m| m.service_ms))?;
        // Bound combined duration too, before the route planner uses it.
        sum_ms([completion, service])?;
        Ok(Self {
            members,
            utilities,
            completion,
            service,
            rewards,
        })
    }
    fn utility(&self) -> u32 {
        *self
            .utilities
            .iter()
            .min()
            .expect("validated participant count")
    }
    fn ids(&self) -> Vec<ObjectiveId> {
        self.members.iter().map(|m| m.id.clone()).collect()
    }
    fn summary(&self, choice: &Choice, spent: Millis) -> Result<ClusterChoice> {
        Ok(ClusterChoice {
            members: self.ids(),
            node: self.members[0].node.clone(),
            participant_utilities: self.utilities.clone(),
            utility: self.utility(),
            completion_ms: self.completion,
            service_ms: self.service,
            inbound_ms: choice.travel.total_ms,
            stopping_ms: choice.stopping.total_ms,
            incremental_ms: choice.incremental_ms,
            expected_session_ms: sum_ms([spent, choice.expected_session_ms])?,
        })
    }
}

/// Deterministic first-fit disjoint-reward packs per declared node, plus every
/// singleton. Rebuilt after each choice so remaining originals can regroup.
fn variants<'a>(
    ready: &[&'a PreparedObjective],
    participant_count: usize,
    remaining_steps: usize,
    control: &mut dyn SearchControl,
) -> Result<Vec<Variant<'a>>> {
    let mut result = Vec::new();
    let mut packs: BTreeMap<NodeId, Vec<Vec<&PreparedObjective>>> = BTreeMap::new();
    for objective in ready {
        control.checkpoint(SearchPhase::CandidateAssessment)?;
        result.push(Variant::new(vec![objective], participant_count, control)?);
        let node_packs = packs.entry(objective.node.clone()).or_default();
        let mut destination = None;
        for (index, pack) in node_packs.iter().enumerate() {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            if pack.len() < remaining_steps
                && pack
                    .iter()
                    .all(|m| m.rewards.is_disjoint(&objective.rewards))
            {
                destination = Some(index);
                break;
            }
        }
        match destination {
            Some(index) => node_packs[index].push(objective),
            None => node_packs.push(vec![objective]),
        }
    }
    for pack in packs.into_values().flatten().filter(|p| p.len() > 1) {
        control.checkpoint(SearchPhase::CandidateAssessment)?;
        result.push(Variant::new(pack, participant_count, control)?);
    }
    // All identifiers used by the underlying scorer are private numeric slots.
    // The original member lists are the deterministic structural tie-breaker.
    result.sort_by_key(|v| v.ids());
    if result.len() > MAX_VARIANTS {
        return Err(invalid("Cluster variant bound exceeded"));
    }
    Ok(result)
}

pub fn plan_clustered(input: &ClusterPlanningInput<'_>) -> Result<ClusteredResult> {
    plan_clustered_controlled(input, &mut NoopSearchControl)
}
pub fn plan_clustered_controlled(
    input: &ClusterPlanningInput<'_>,
    control: &mut dyn SearchControl,
) -> Result<ClusteredResult> {
    control.checkpoint(SearchPhase::ValidateInputs)?;
    if input.objectives.len() > MAX_OBJECTIVES
        || input.excluded.len() > MAX_OBJECTIVES
        || !(1..=MAX_PARTICIPANTS).contains(&input.participant_count)
        || !(1..=64).contains(&input.max_steps)
    {
        return Err(invalid("Cluster planning input exceeds bounds"));
    }
    unique(input.objectives.iter().map(|o| &o.id))?;
    let mut originals: Vec<_> = input.objectives.iter().collect();
    originals.sort_by_key(|o| &o.id);
    let mut assessments = Vec::new();
    for objective in &originals {
        control.checkpoint(SearchPhase::ParticipantReadiness)?;
        bounded(objective.rewards.len())?;
        if objective.participants.len() != input.participant_count
            || !input.nodes.iter().any(|n| n.id == objective.node)
        {
            return Err(invalid(
                "Cluster objective needs declared node and one contribution per participant",
            ));
        }
        assessments.push(ObjectiveAssessment {
            objective: objective.id.clone(),
            participants: objective.participants.clone(),
            decision: decision(objective, input.excluded),
        });
    }
    // Neutral gate solely for reusing the baseline's routing/ranking engine.
    // It is never original readiness evidence: prepared truth remains authoritative.
    let always = Predicate::try_new(PredicateNode::All { predicates: vec![] })?;
    let excluded = BTreeSet::new();
    let scorer = |candidates: &[Candidate<'_>],
                  current: &NodeId,
                  spent: Millis,
                  control: &mut dyn SearchControl| {
        plan_controlled(
            &PlannerInput {
                goal_profile: input.goal_profile.clone(),
                candidates,
                excluded: &excluded,
                nodes: input.nodes,
                edges: input.edges,
                origin: current.clone(),
                stopping_node: input.stopping_node.clone(),
                stopping_service_ms: input.stopping_service_ms,
                budget_ms: Millis::new(
                    input
                        .budget_ms
                        .get()
                        .checked_sub(spent.get())
                        .ok_or_else(|| invalid("Cluster spent time exceeds budget"))?,
                )?,
                max_steps: 1,
                context: input.context,
                travel_options: input.travel_options,
            },
            control,
        )
    };
    let mut result = scorer(&[], &input.origin, Millis::new(0)?, control)?;
    result.candidate_count = originals.len();
    result.termination = Termination::StepLimit;
    let mut current = input.origin.clone();
    let mut spent = Millis::new(0)?;
    let mut chosen = BTreeSet::new();
    let mut rewards = BTreeSet::new();
    let mut selected_clusters = Vec::new();
    let mut cluster_alternatives = Vec::new();
    let mut variant_trace = Vec::new();
    let mut peak_variant_count = 0;
    let mut variants_evaluated = 0;
    while chosen.len() < input.max_steps {
        control.checkpoint(SearchPhase::Selection)?;
        let iteration = selected_clusters.len();
        let mut ready = Vec::new();
        for (objective, assessment) in originals.iter().zip(&assessments) {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            if chosen.contains(&objective.id) {
                continue;
            }
            let decision = if assessment.decision != Decision::Feasible {
                assessment.decision
            } else if !rewards.is_disjoint(&objective.rewards) {
                Decision::RewardAlreadyCounted
            } else {
                ready.push(*objective);
                continue;
            };
            result.trace.push(CandidateTrace {
                iteration,
                candidate: objective.id.clone(),
                decision,
                eligibility: (decision != Decision::Excluded)
                    .then(|| objective.participants[0].evidence.clone()),
                travel: None,
                stopping: None,
                expected_session_ms: None,
            });
        }
        let variants = variants(
            &ready,
            input.participant_count,
            input.max_steps - chosen.len(),
            control,
        )?;
        peak_variant_count = peak_variant_count.max(variants.len());
        let slots = (0..variants.len())
            .map(|i| ObjectiveId::new(format!("cluster-slot-{i:03}")))
            .collect::<Result<Vec<_>>>()?;
        let candidates = variants
            .iter()
            .zip(&slots)
            .map(|(v, id)| Candidate {
                id: id.clone(),
                node: v.members[0].node.clone(),
                gate: EligibilityGate::Predicate(&always),
                utility: v.utility(),
                completion_ms: v.completion,
                service_ms: v.service,
                rewards: v.rewards.clone(),
            })
            .collect::<Vec<_>>();
        let ranking = scorer(&candidates, &current, spent, control)?;
        variants_evaluated += ranking.trace.len();
        let slot = |id: &ObjectiveId| slots.binary_search(id).expect("scorer returns known slot");
        for trace in &ranking.trace {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            let variant = &variants[slot(&trace.candidate)];
            let total = trace
                .expected_session_ms
                .map(|v| sum_ms([spent, v]))
                .transpose()?;
            variant_trace.push(VariantTrace {
                iteration,
                members: variant.ids(),
                decision: trace.decision,
                expected_session_ms: total,
            });
            if variant.members.len() == 1 {
                let mut original_trace = trace.clone();
                original_trace.iteration = iteration;
                original_trace.candidate = variant.members[0].id.clone();
                original_trace.eligibility =
                    Some(variant.members[0].participants[0].evidence.clone());
                original_trace.expected_session_ms = total;
                result.trace.push(original_trace);
            }
        }
        let Some(choice) = ranking.selected.first() else {
            result.termination = Termination::NoFeasibleCandidate;
            break;
        };
        let selected_variant = &variants[slot(&choice.candidate)];
        let mut alternatives = Vec::new();
        let mut cluster_choices = Vec::new();
        for alternate in &ranking.alternatives[0] {
            control.checkpoint(SearchPhase::Selection)?;
            let variant = &variants[slot(&alternate.candidate)];
            cluster_choices.push(variant.summary(alternate, spent)?);
            if variant.members.len() == 1
                && !selected_variant
                    .members
                    .iter()
                    .any(|m| m.id == variant.members[0].id)
            {
                let mut original = alternate.clone();
                original.candidate = variant.members[0].id.clone();
                original.expected_session_ms = sum_ms([spent, original.expected_session_ms])?;
                alternatives.push(original);
            }
        }
        selected_clusters.push(selected_variant.summary(choice, spent)?);
        cluster_alternatives.push(cluster_choices);
        let mut inbound = choice.travel.clone();
        for original in &selected_variant.members {
            control.checkpoint(SearchPhase::Selection)?;
            let incremental = sum_ms([
                inbound.total_ms,
                original.completion_ms,
                original.service_ms,
            ])?;
            spent = sum_ms([spent, incremental])?;
            result.selected.push(Choice {
                candidate: original.id.clone(),
                utility: *original
                    .participants
                    .iter()
                    .map(|p| &p.utility)
                    .min()
                    .expect("validated contributions"),
                incremental_ms: incremental,
                completion_ms: original.completion_ms,
                service_ms: original.service_ms,
                travel: inbound,
                stopping: choice.stopping.clone(),
                expected_session_ms: sum_ms([
                    spent,
                    choice.stopping.total_ms,
                    input.stopping_service_ms,
                ])?,
            });
            // Keep v1 alternatives aligned with original selected steps. Atomic
            // cluster alternatives belong only on its first original step.
            result.alternatives.push(std::mem::take(&mut alternatives));
            chosen.insert(original.id.clone());
            rewards.extend(original.rewards.iter().cloned());
            let node = input
                .nodes
                .iter()
                .find(|n| n.id == original.node)
                .expect("validated node");
            inbound = Route {
                start: NodeRef {
                    id: node.id.clone(),
                    map: node.position.map.clone(),
                },
                hops: vec![],
                total_ms: Millis::new(0)?,
            };
        }
        current = selected_variant.members[0].node.clone();
        result.stopping = choice.stopping.clone();
        result.expected_total_ms =
            sum_ms([spent, result.stopping.total_ms, input.stopping_service_ms])?;
    }
    result
        .trace
        .sort_by(|a, b| (a.iteration, &a.candidate).cmp(&(b.iteration, &b.candidate)));
    result.candidates_evaluated = result.trace.len();
    control.checkpoint(SearchPhase::Selection)?;
    Ok(ClusteredResult {
        schema_version: CLUSTER_SCHEMA_VERSION,
        plan: result,
        assessments,
        selected_clusters,
        cluster_alternatives,
        variant_trace,
        peak_variant_count,
        variants_evaluated,
    })
}
