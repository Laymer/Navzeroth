mod common;
use azeroth_core::{
    entities::*,
    error::{AppError, ErrorCode, Result},
    evaluation::*,
    objective_clustering::*,
    observation::*,
    planner::{
        self, Candidate, Decision, EligibilityEvidence, EligibilityGate, PlannerInput, Termination,
    },
    predicate::*,
    search_control::{SearchControl, SearchPhase},
    travel::*,
    types::*,
};
use serde_json::{Value, from_value, json};
use std::collections::BTreeSet;
fn ms(value: u64) -> Millis {
    Millis::new(value).unwrap()
}
fn node(id: &str) -> TravelNode {
    from_value(json!({"id":id,"position":{"map":{"context":common::context(),"map_id":"m","floor":"ground","instance":null},"x":0.1,"y":0.2},"service":{"status":"unknown"}})).unwrap()
}
fn edge(id: &str, from: &str, to: &str, cost: u64) -> TravelEdge {
    from_value(json!({"id":id,"context":common::context(),"from":{"id":from,"map":{"context":common::context(),"map_id":"m","floor":"ground","instance":null}},"to":{"id":to,"map":{"context":common::context(),"map_id":"m","floor":"ground","instance":null}},"mode":"walking","requirements":{"status":"not_applicable"},"expected_ms":{"status":"known","value":cost},"wait_ms":{"status":"not_applicable"},"uncertainty":{"status":"unknown"},"status":"verified","evidence":["synthetic-edge-evidence"],"consumes_resource":{"status":"unknown"},"cooldown_ms":{"status":"unknown"}})).unwrap()
}
fn objective(
    id: &str,
    at: &str,
    utilities: &[u32],
    completion: u64,
    service: u64,
) -> PreparedObjective {
    PreparedObjective {
        id: ObjectiveId::new(id).unwrap(),
        node: NodeId::new(at).unwrap(),
        completion_ms: ms(completion),
        service_ms: ms(service),
        rewards: BTreeSet::new(),
        participants: utilities
            .iter()
            .map(|utility| PreparedContribution {
                utility: *utility,
                truth: Truth::True,
                evidence: EligibilityEvidence::Predicate {
                    result: EvaluationResult {
                        truth: Truth::True,
                        reasons: vec![],
                        warnings: vec![],
                        steps: 1,
                    },
                },
            })
            .collect(),
    }
}
struct Harness {
    snapshot: CharacterSnapshot,
    clock: FreshnessContext,
    nodes: Vec<TravelNode>,
    edges: Vec<TravelEdge>,
    excluded: BTreeSet<ObjectiveId>,
    options: RouteOptions,
}
impl Harness {
    fn new() -> Self {
        Self {
            snapshot: from_value(common::snapshot()).unwrap(),
            clock: FreshnessContext {
                now_utc_ms: None,
                clock_trust: ClockTrust::Uncertain,
                session: None,
                max_age_ms: ms(1000),
            },
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![
                edge("ab", "a", "b", 100),
                edge("ba", "b", "a", 100),
                edge("ac", "a", "c", 10),
                edge("ca", "c", "a", 10),
            ],
            excluded: BTreeSet::new(),
            options: RouteOptions::default(),
        }
    }
    fn controlled(
        &self,
        objectives: &[PreparedObjective],
        participants: usize,
        budget: u64,
        steps: usize,
        control: &mut dyn SearchControl,
    ) -> Result<ClusteredResult> {
        let context = EvaluationContext {
            snapshot: &self.snapshot,
            party: None,
            freshness: &self.clock,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        };
        plan_clustered_controlled(
            &ClusterPlanningInput {
                goal_profile: SourceId::new("synthetic-goal-v1").unwrap(),
                participant_count: participants,
                objectives,
                excluded: &self.excluded,
                nodes: &self.nodes,
                edges: &self.edges,
                origin: NodeId::new("a").unwrap(),
                stopping_node: NodeId::new("a").unwrap(),
                stopping_service_ms: ms(5),
                budget_ms: ms(budget),
                max_steps: steps,
                context: &context,
                travel_options: &self.options,
            },
            control,
        )
    }
    fn run(
        &self,
        objectives: &[PreparedObjective],
        participants: usize,
        budget: u64,
        steps: usize,
    ) -> Result<ClusteredResult> {
        self.controlled(
            objectives,
            participants,
            budget,
            steps,
            &mut azeroth_core::search_control::NoopSearchControl,
        )
    }
}
fn selected(result: &ClusteredResult) -> Vec<&str> {
    result
        .plan
        .selected
        .iter()
        .map(|s| s.candidate.as_str())
        .collect()
}
#[test]
fn declared_replay_changes_selection_by_sharing_only_inbound_travel() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/planner/clustering-basic.json"
    ))
    .unwrap();
    let mut harness = Harness::new();
    harness.edges = fixture["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            edge(
                e[0].as_str().unwrap(),
                e[1].as_str().unwrap(),
                e[2].as_str().unwrap(),
                e[3].as_u64().unwrap(),
            )
        })
        .collect();
    let objectives = fixture["objectives"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            objective(
                o["id"].as_str().unwrap(),
                o["node"].as_str().unwrap(),
                &[o["utility"].as_u64().unwrap() as u32],
                o["completion_ms"].as_u64().unwrap(),
                o["service_ms"].as_u64().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let clustered = harness
        .run(&objectives, 1, fixture["budget_ms"].as_u64().unwrap(), 32)
        .unwrap();
    assert_eq!(
        json!(selected(&clustered)),
        fixture["expected"]["clustered_selected"]
    );
    assert_eq!(
        clustered.plan.expected_total_ms.get(),
        fixture["expected"]["clustered_total_ms"].as_u64().unwrap()
    );
    assert_eq!(
        clustered.selected_clusters[0].utility,
        fixture["expected"]["clustered_utility"].as_u64().unwrap() as u32
    );
    assert_eq!(clustered.selected_clusters[0].completion_ms.get(), 20);
    assert_eq!(clustered.plan.selected[0].travel.total_ms.get(), 100);
    assert_eq!(clustered.plan.selected[1].travel.total_ms.get(), 0);
    assert!(clustered.plan.selected[1].travel.hops.is_empty());
    assert_eq!(
        clustered.plan.selected[1].expected_session_ms,
        clustered.plan.expected_total_ms
    );
    for (index, choice) in clustered.plan.selected.iter().enumerate() {
        assert!(
            clustered.plan.alternatives[index]
                .iter()
                .all(|a| a.candidate != choice.candidate)
        );
        assert!(
            clustered.plan.alternatives[index]
                .iter()
                .all(|a| !["one", "two"].contains(&a.candidate.as_str()))
        );
    }
    assert_eq!(clustered.plan.termination, Termination::NoFeasibleCandidate);
    assert!(
        clustered
            .plan
            .selected
            .iter()
            .all(|c| !c.candidate.as_str().starts_with("cluster-slot-"))
    );
    let always = Predicate::try_new(PredicateNode::All { predicates: vec![] }).unwrap();
    let candidates = objectives
        .iter()
        .map(|o| Candidate {
            id: o.id.clone(),
            node: o.node.clone(),
            gate: EligibilityGate::Predicate(&always),
            utility: o.participants[0].utility,
            completion_ms: o.completion_ms,
            service_ms: o.service_ms,
            rewards: o.rewards.clone(),
        })
        .collect::<Vec<_>>();
    let context = EvaluationContext {
        snapshot: &harness.snapshot,
        party: None,
        freshness: &harness.clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let baseline = planner::plan(&PlannerInput {
        goal_profile: SourceId::new("synthetic-goal-v1").unwrap(),
        candidates: &candidates,
        excluded: &harness.excluded,
        nodes: &harness.nodes,
        edges: &harness.edges,
        origin: NodeId::new("a").unwrap(),
        stopping_node: NodeId::new("a").unwrap(),
        stopping_service_ms: ms(5),
        budget_ms: ms(250),
        max_steps: 32,
        context: &context,
        travel_options: &harness.options,
    })
    .unwrap();
    assert_eq!(
        json!(
            baseline
                .selected
                .iter()
                .map(|c| c.candidate.as_str())
                .collect::<Vec<_>>()
        ),
        fixture["expected"]["baseline_selected"]
    );
}
#[test]
fn permutation_and_noop_wrapper_are_deterministic() {
    let mut harness = Harness::new();
    let mut objectives = vec![
        objective("z", "b", &[3], 10, 7),
        objective("a", "b", &[4], 20, 9),
        objective("c", "c", &[1], 10, 0),
    ];
    let first = harness.run(&objectives, 1, 1000, 32).unwrap();
    objectives.reverse();
    harness.nodes.reverse();
    harness.edges.reverse();
    let second = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
    let group = first
        .selected_clusters
        .iter()
        .find(|c| c.members.len() == 2)
        .unwrap();
    assert_eq!(group.service_ms.get(), 16);
    assert_eq!(group.completion_ms.get(), 30);
    assert_eq!(group.incremental_ms.get(), group.inbound_ms.get() + 46);
    let context = EvaluationContext {
        snapshot: &harness.snapshot,
        party: None,
        freshness: &harness.clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let wrapper = plan_clustered(&ClusterPlanningInput {
        goal_profile: SourceId::new("synthetic-goal-v1").unwrap(),
        participant_count: 1,
        objectives: &objectives,
        excluded: &harness.excluded,
        nodes: &harness.nodes,
        edges: &harness.edges,
        origin: NodeId::new("a").unwrap(),
        stopping_node: NodeId::new("a").unwrap(),
        stopping_service_ms: ms(5),
        budget_ms: ms(1000),
        max_steps: 32,
        context: &context,
        travel_options: &harness.options,
    })
    .unwrap();
    assert_eq!(
        serde_json::to_string(&second).unwrap(),
        serde_json::to_string(&wrapper).unwrap()
    );
}
#[test]
fn over_budget_pack_keeps_feasible_singleton_and_exact_stopping_cost() {
    let harness = Harness::new();
    let objectives = vec![
        objective("a", "b", &[10], 10, 0),
        objective("b", "b", &[10], 10, 0),
    ];
    let result = harness.run(&objectives, 1, 215, 32).unwrap();
    assert_eq!(selected(&result), vec!["a"]);
    assert_eq!(result.plan.expected_total_ms.get(), 215);
    assert!(
        result
            .variant_trace
            .iter()
            .any(|t| t.members.len() == 2 && t.decision == Decision::OverBudget)
    );
    assert!(harness.run(&[], 1, 4, 32).is_err());
}
#[test]
fn joint_fairness_is_minimum_of_participant_sums_and_not_sum_of_minima() {
    let harness = Harness::new();
    let objectives = vec![
        objective("a", "b", &[9, 1], 10, 0),
        objective("b", "b", &[1, 9], 10, 0),
    ];
    let result = harness.run(&objectives, 2, 1000, 32).unwrap();
    assert_eq!(
        result.selected_clusters[0].participant_utilities,
        vec![10, 10]
    );
    assert_eq!(result.selected_clusters[0].utility, 10);
    assert_eq!(
        result.plan.selected.iter().map(|c| c.utility).sum::<u32>(),
        2
    );
    assert_eq!(result.assessments[0].participants[1].utility, 1);
    let mut blocked = objectives.clone();
    blocked[0].participants[1].utility = 0;
    let result = harness.run(&blocked, 2, 1000, 32).unwrap();
    assert_eq!(selected(&result), vec!["b"]);
    assert_eq!(result.assessments[0].decision, Decision::NoUtility);
    blocked[0].participants[1].utility = 1;
    blocked[0].participants[1].truth = Truth::Unknown;
    if let EligibilityEvidence::Predicate { result } = &mut blocked[0].participants[1].evidence {
        result.truth = Truth::Unknown;
    }
    let result = harness.run(&blocked, 2, 1000, 32).unwrap();
    assert_eq!(selected(&result), vec!["b"]);
    assert_eq!(result.assessments[0].decision, Decision::EligibilityUnknown);
}
#[test]
fn exclusions_false_unknown_and_zero_keep_original_evidence_and_cannot_group() {
    let mut harness = Harness::new();
    let mut objectives = vec![
        objective("excluded", "b", &[5], 0, 0),
        objective("false", "b", &[5], 0, 0),
        objective("unknown", "b", &[5], 0, 0),
        objective("zero", "b", &[0], 0, 0),
        objective("ready", "b", &[5], 0, 0),
    ];
    harness.excluded.insert(objectives[0].id.clone());
    for (index, truth) in [(1, Truth::False), (2, Truth::Unknown)] {
        objectives[index].participants[0].truth = truth;
        if let EligibilityEvidence::Predicate { result } =
            &mut objectives[index].participants[0].evidence
        {
            result.truth = truth;
            result.steps = 7;
        }
    }
    let result = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert_eq!(selected(&result), vec!["ready"]);
    for (id, decision) in [
        ("excluded", Decision::Excluded),
        ("false", Decision::EligibilityFalse),
        ("unknown", Decision::EligibilityUnknown),
        ("zero", Decision::NoUtility),
    ] {
        assert!(
            result
                .plan
                .trace
                .iter()
                .any(|t| t.candidate.as_str() == id && t.decision == decision)
        );
    }
    let evidence = &result
        .assessments
        .iter()
        .find(|a| a.objective.as_str() == "unknown")
        .unwrap()
        .participants[0]
        .evidence;
    assert!(
        matches!(evidence,EligibilityEvidence::Predicate {result} if result.truth==Truth::Unknown && result.steps==7)
    );
    assert!(
        result
            .selected_clusters
            .iter()
            .all(|c| c.members.len() == 1)
    );
}
#[test]
fn overlapping_rewards_and_empty_rewards_never_repeat_originals() {
    let harness = Harness::new();
    let mut objectives = vec![
        objective("a", "b", &[5], 10, 0),
        objective("b", "b", &[5], 10, 0),
        objective("c", "b", &[5], 10, 0),
    ];
    objectives[0]
        .rewards
        .insert(SourceId::new("shared").unwrap());
    objectives[1].rewards = objectives[0].rewards.clone();
    let result = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert_eq!(selected(&result), vec!["a", "c"]);
    assert!(
        result
            .plan
            .trace
            .iter()
            .any(|t| t.candidate.as_str() == "b" && t.decision == Decision::RewardAlreadyCounted)
    );
    for o in &mut objectives {
        o.rewards.clear();
    }
    let result = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert_eq!(selected(&result), vec!["a", "b", "c"]);
    assert_eq!(
        result.plan.selected.len(),
        result
            .plan
            .selected
            .iter()
            .map(|c| &c.candidate)
            .collect::<BTreeSet<_>>()
            .len()
    );
}
#[test]
fn original_step_limit_and_declared_nodes_are_respected() {
    let harness = Harness::new();
    let objectives = vec![
        objective("a", "b", &[5], 1, 0),
        objective("b", "b", &[5], 1, 0),
        objective("c", "c", &[1], 1, 0),
    ];
    let result = harness.run(&objectives, 1, 1000, 2).unwrap();
    assert_eq!(selected(&result), vec!["a", "b"]);
    assert_eq!(result.plan.termination, Termination::StepLimit);
    let result = harness.run(&objectives, 1, 1000, 1).unwrap();
    assert_eq!(result.plan.selected.len(), 1);
    assert!(
        result
            .selected_clusters
            .iter()
            .all(|c| c.members.len() == 1)
    );
    let result = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert!(
        result
            .selected_clusters
            .iter()
            .all(|c| !c.members.iter().any(|id| id.as_str() == "c") || c.members.len() == 1)
    );
}
#[test]
fn unknown_transport_and_stateful_intermediate_do_not_become_available() {
    let mut harness = Harness::new();
    let objectives = vec![
        objective("a", "b", &[5], 1, 0),
        objective("b", "b", &[5], 1, 0),
    ];
    let mut raw = serde_json::to_value(&harness.edges[0]).unwrap();
    raw["requirements"] = json!({"status":"unknown"});
    harness.edges[0] = from_value(raw).unwrap();
    let result = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert!(result.plan.selected.is_empty());
    assert!(
        result
            .variant_trace
            .iter()
            .all(|t| t.decision == Decision::Unreachable)
    );
    let mut raw = serde_json::to_value(edge("ab", "a", "b", 1)).unwrap();
    raw["mode"] = json!("return");
    raw["consumes_resource"] = json!({"status":"known","value":false});
    raw["cooldown_ms"] = json!({"status":"known","value":0});
    harness.edges[0] = from_value(raw).unwrap();
    let result = harness.run(&objectives, 1, 1000, 32).unwrap();
    assert!(result.plan.selected.is_empty());
    assert!(
        result
            .variant_trace
            .iter()
            .all(|t| t.decision == Decision::StatefulIntermediate)
    );
}
#[test]
fn duration_utility_and_shape_bounds_fail_without_partial_report() {
    let harness = Harness::new();
    assert!(
        harness
            .run(
                &[
                    objective("a", "b", &[u32::MAX], 1, 0),
                    objective("b", "b", &[1], 1, 0)
                ],
                1,
                1000,
                32
            )
            .is_err()
    );
    assert!(
        harness
            .run(
                &[
                    objective("a", "b", &[1], MAX_SAFE_INTEGER, 0),
                    objective("b", "b", &[1], 1, 0)
                ],
                1,
                MAX_SAFE_INTEGER,
                32
            )
            .is_err()
    );
    assert!(
        harness
            .run(&[objective("a", "b", &[1], 1, 0)], 2, 1000, 32)
            .is_err()
    );
    assert!(harness.run(&[], 0, 1000, 32).is_err());
    assert!(harness.run(&[], 41, 1000, 32).is_err());
    assert!(harness.run(&[], 1, 1000, 0).is_err());
    assert!(harness.run(&[], 1, 1000, 65).is_err());
    assert!(
        harness
            .run(&[objective("a", "missing", &[1], 1, 0)], 1, 1000, 32)
            .is_err()
    );
    assert!(
        harness
            .run(
                &[
                    objective("a", "b", &[1], 1, 0),
                    objective("a", "c", &[1], 1, 0)
                ],
                1,
                1000,
                32
            )
            .is_err()
    );
    let too_many = (0..129)
        .map(|i| objective(&format!("o{i:03}"), "b", &[1], 1, 0))
        .collect::<Vec<_>>();
    assert!(harness.run(&too_many, 1, 1000, 32).is_err());
}
struct CancelPhase {
    phase: SearchPhase,
    remaining: usize,
}
impl SearchControl for CancelPhase {
    fn checkpoint(&mut self, phase: SearchPhase) -> Result<()> {
        if phase == self.phase {
            if self.remaining == 0 {
                return Err(AppError::new(
                    ErrorCode::OperationCanceled,
                    "Synthetic cancellation",
                    false,
                ));
            }
            self.remaining -= 1;
        }
        Ok(())
    }
}
#[test]
fn cancellation_in_grouping_participant_sum_routes_and_terminal_selection_propagates() {
    let harness = Harness::new();
    let objectives = vec![
        objective("a", "b", &[9, 1], 1, 0),
        objective("b", "b", &[1, 9], 1, 0),
    ];
    for (phase, remaining) in [
        (SearchPhase::ValidateInputs, 0),
        (SearchPhase::CandidateAssessment, 3),
        (SearchPhase::ParticipantReadiness, 3),
        (SearchPhase::RouteExpansion, 1),
        (SearchPhase::Selection, 0),
    ] {
        let error = harness
            .controlled(
                &objectives,
                2,
                1000,
                32,
                &mut CancelPhase { phase, remaining },
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::OperationCanceled);
    }
    let error = harness
        .controlled(
            &[],
            1,
            1000,
            32,
            &mut CancelPhase {
                phase: SearchPhase::Selection,
                remaining: 5,
            },
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::OperationCanceled);
}
#[test]
fn bounded_128_objective_40_participant_workload_preserves_flattened_limit() {
    let harness = Harness::new();
    let objectives = (0..128)
        .map(|i| objective(&format!("o{i:03}"), "b", &[1; 40], 1, 0))
        .collect::<Vec<_>>();
    let result = harness.run(&objectives, 40, 1000, 64).unwrap();
    assert_eq!(result.plan.selected.len(), 64);
    assert_eq!(result.selected_clusters.len(), 1);
    assert_eq!(
        result.selected_clusters[0].participant_utilities,
        vec![64; 40]
    );
    assert_eq!(result.plan.termination, Termination::StepLimit);
    assert!(result.peak_variant_count <= MAX_VARIANTS);
    assert_eq!(result.variants_evaluated, result.variant_trace.len());
    assert_eq!(result.plan.candidates_evaluated, result.plan.trace.len());
}

#[test]
fn only_one_final_stateful_return_is_reserved_after_flattening() {
    let mut harness = Harness::new();
    let mut raw = serde_json::to_value(edge("ba", "b", "a", 1)).unwrap();
    raw["mode"] = json!("return");
    raw["consumes_resource"] = json!({"status":"known","value":true});
    raw["cooldown_ms"] = json!({"status":"known","value":60000});
    harness.edges[1] = from_value(raw).unwrap();
    let result = harness
        .run(
            &[
                objective("a", "b", &[5], 10, 0),
                objective("b", "b", &[5], 10, 0),
            ],
            1,
            1000,
            32,
        )
        .unwrap();
    assert_eq!(selected(&result), vec!["a", "b"]);
    assert_eq!(result.plan.stopping.hops.len(), 1);
    assert!(result.plan.stopping.hops[0].stateful);
    assert_eq!(result.plan.expected_total_ms.get(), 126);
    assert!(
        result
            .plan
            .selected
            .iter()
            .flat_map(|c| &c.travel.hops)
            .all(|h| !h.stateful)
    );
}

#[test]
fn maximum_variant_count_is_finite_even_when_every_two_objectives_have_their_own_node() {
    let mut harness = Harness::new();
    let mut objectives = Vec::new();
    for index in 0..64 {
        let at = format!("node{index:03}");
        harness.nodes.push(node(&at));
        for member in 0..2 {
            objectives.push(objective(&format!("o{index:03}-{member}"), &at, &[1], 1, 0));
        }
    }
    let result = harness.run(&objectives, 1, 1000, 64).unwrap();
    assert_eq!(result.peak_variant_count, MAX_VARIANTS);
    assert_eq!(result.variants_evaluated, 192);
    assert!(result.plan.selected.is_empty());
    assert_eq!(result.plan.termination, Termination::NoFeasibleCandidate);
}
