mod common;
use azeroth_core::{
    entities::*, evaluation::*, observation::*, planner::*, predicate::*, travel::*, types::*,
};
use serde_json::{from_value, json};
use std::collections::BTreeSet;
fn ms(n: u64) -> Millis {
    Millis::new(n).unwrap()
}
fn node(id: &str) -> TravelNode {
    from_value(json!({"id":id,"position":{"map":{"context":common::context(),"map_id":"m","floor":"ground","instance":null},"x":0.1,"y":0.2},"service":{"status":"unknown"}})).unwrap()
}
fn edge(id: &str, from: &str, to: &str, n: u64) -> TravelEdge {
    from_value(json!({"id":id,"context":common::context(),"from":{"id":from,"map":{"context":common::context(),"map_id":"m","floor":"ground","instance":null}},"to":{"id":to,"map":{"context":common::context(),"map_id":"m","floor":"ground","instance":null}},"mode":"walking","requirements":{"status":"not_applicable"},"expected_ms":{"status":"known","value":n},"wait_ms":{"status":"not_applicable"},"uncertainty":{"status":"unknown"},"status":"verified","evidence":["synthetic-edge-evidence"],"consumes_resource":{"status":"unknown"},"cooldown_ms":{"status":"unknown"}})).unwrap()
}
fn candidate<'a>(
    id: &str,
    predicate: &'a Predicate,
    utility: u32,
    completion: u64,
    service: u64,
) -> Candidate<'a> {
    Candidate {
        id: ObjectiveId::new(id).unwrap(),
        node: NodeId::new("b").unwrap(),
        gate: EligibilityGate::Predicate(predicate),
        utility,
        completion_ms: ms(completion),
        service_ms: ms(service),
        rewards: BTreeSet::new(),
    }
}
fn replay(
    candidates: &[Candidate<'_>],
    budget: u64,
    excluded: &BTreeSet<ObjectiveId>,
) -> PlannerResult {
    let snapshot: CharacterSnapshot = from_value(common::snapshot()).unwrap();
    let freshness = FreshnessContext {
        now_utc_ms: None,
        clock_trust: ClockTrust::Uncertain,
        session: None,
        max_age_ms: ms(1000),
    };
    let context = EvaluationContext {
        snapshot: &snapshot,
        party: None,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let nodes = vec![node("a"), node("b")];
    let edges = vec![edge("ab", "a", "b", 10), edge("ba", "b", "a", 20)];
    plan(&PlannerInput {
        goal_profile: SourceId::new("synthetic-goal-v1").unwrap(),
        candidates,
        excluded,
        nodes: &nodes,
        edges: &edges,
        origin: NodeId::new("a").unwrap(),
        stopping_node: NodeId::new("a").unwrap(),
        stopping_service_ms: ms(5),
        budget_ms: ms(budget),
        max_steps: 8,
        context: &context,
        travel_options: &RouteOptions::default(),
    })
    .unwrap()
}
fn always() -> Predicate {
    from_value(json!({"op":"all","predicates":[]})).unwrap()
}
#[test]
fn replay_is_deterministic_and_reserves_completion_services_and_stop() {
    let yes = always();
    let mut candidates = vec![
        candidate("z", &yes, 10, 10, 5),
        candidate("a", &yes, 10, 10, 5),
    ];
    let result = replay(&candidates, 65, &BTreeSet::new());
    assert_eq!(result.selected[0].candidate.as_str(), "a");
    assert_eq!(result.selected.len(), 2);
    assert_eq!(result.expected_total_ms, ms(65));
    assert_eq!(result.alternatives[0].len(), 1);
    candidates.reverse();
    assert_eq!(result, replay(&candidates, 65, &BTreeSet::new()));
    let short = replay(&candidates, 49, &BTreeSet::new());
    assert!(short.selected.is_empty());
    assert!(
        short
            .trace
            .iter()
            .all(|t| t.decision == Decision::OverBudget)
    );
}
#[test]
fn exclusions_and_unknown_eligibility_never_recommend_despite_high_utility() {
    let yes = always();
    let unknown: Predicate = from_value(json!({"op":"level_at_least","level":2})).unwrap();
    let candidates = vec![
        candidate("unknown", &unknown, u32::MAX, 0, 0),
        candidate("excluded", &yes, u32::MAX, 0, 0),
        candidate("eligible", &yes, 1, 10, 0),
    ];
    let excluded = BTreeSet::from([ObjectiveId::new("excluded").unwrap()]);
    let result = replay(&candidates, 100, &excluded);
    assert_eq!(result.selected.len(), 1);
    assert_eq!(result.selected[0].candidate.as_str(), "eligible");
    assert!(
        result
            .trace
            .iter()
            .any(|t| t.decision == Decision::EligibilityUnknown)
    );
    let entry = result
        .trace
        .iter()
        .find(|t| t.candidate.as_str() == "excluded")
        .unwrap();
    assert!(entry.eligibility.is_none());
    assert!(entry.travel.is_none());
}
#[test]
fn rewards_count_once_and_zero_utility_does_not_enter_route() {
    let yes = always();
    let reward = SourceId::new("reward1").unwrap();
    let mut candidates = vec![
        candidate("a", &yes, 5, 10, 0),
        candidate("b", &yes, 4, 10, 0),
        candidate("zero", &yes, 0, 0, 0),
    ];
    candidates[0].rewards.insert(reward.clone());
    candidates[1].rewards.insert(reward);
    let result = replay(&candidates, 100, &BTreeSet::new());
    assert_eq!(result.selected.len(), 1);
    assert!(
        result
            .trace
            .iter()
            .any(|t| t.decision == Decision::RewardAlreadyCounted)
    );
    assert!(
        result
            .trace
            .iter()
            .any(|t| t.decision == Decision::NoUtility)
    );
}
#[test]
fn exact_ratio_uses_incremental_travel_and_produces_ranked_alternatives() {
    let yes = always();
    let candidates = vec![
        candidate("slow", &yes, 10, 90, 0),
        candidate("fast", &yes, 3, 10, 0),
    ];
    let result = replay(&candidates, 200, &BTreeSet::new());
    assert_eq!(result.selected[0].candidate.as_str(), "fast");
    assert_eq!(result.selected[1].travel.total_ms, ms(0));
    assert_eq!(result.alternatives[0][0].candidate.as_str(), "slow");
    assert!(result.estimated);
}

#[test]
fn existing_quest_eligibility_unknown_history_remains_uncertain() {
    use azeroth_core::eligibility::*;
    use std::collections::BTreeMap;
    let definition: QuestDefinition = from_value(json!({"id":"quest1","context":common::context(),"eligibility":{"status":"known","value":{"op":"all","predicates":[]}},"prerequisites":{"status":"not_applicable"},"stages":["stage1"],"locations":{"status":"unknown"},"rewards":{"status":"unknown"},"completion":{"status":"known","value":"unrepeatable"},"shared_progress":{"status":"unknown"},"evidence":[]})).unwrap();
    let freshness = FreshnessContext {
        now_utc_ms: None,
        clock_trust: ClockTrust::Uncertain,
        session: None,
        max_age_ms: ms(1000),
    };
    let facts = BTreeMap::new();
    let inputs = EligibilityInputs {
        definition: &definition,
        access: None,
        exclusive_groups: &[],
        facts: &facts,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let candidates = [Candidate {
        id: ObjectiveId::new("quest-cluster").unwrap(),
        node: NodeId::new("b").unwrap(),
        gate: EligibilityGate::QuestAcceptance(&inputs),
        utility: 100,
        completion_ms: ms(1),
        service_ms: ms(0),
        rewards: BTreeSet::new(),
    }];
    let result = replay(&candidates, 100, &BTreeSet::new());
    assert!(result.selected.is_empty());
    assert_eq!(result.trace[0].decision, Decision::EligibilityUnknown);
    assert!(matches!(
        result.trace[0].eligibility,
        Some(EligibilityEvidence::Quest { .. })
    ));
}
