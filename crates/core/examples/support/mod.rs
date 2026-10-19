#![allow(dead_code)]
use azeroth_core::{
    entities::*, evaluation::*, joint_planner::*, observation::*, planner::*, predicate::*,
    search_control::*, types::*,
};
use serde_json::{from_value, json};
use std::collections::BTreeSet;
pub fn ms(n: u64) -> Millis {
    Millis::new(n).unwrap()
}
pub struct Fixture {
    pub nodes: Vec<TravelNode>,
    pub edges: Vec<TravelEdge>,
    pub snapshots: Vec<CharacterSnapshot>,
    pub freshness: FreshnessContext,
    pub predicate: Predicate,
}
impl Fixture {
    pub fn new(n: usize, e: usize, members: usize) -> Self {
        let context = json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"});
        let map = json!({"context":context,"map_id":"m","floor":"ground","instance":null});
        let nodes=(0..n).map(|i| from_value(json!({"id":format!("n{i:04}"),"position":{"map":map,"x":0.1,"y":0.2},"service":{"status":"unknown"}})).unwrap()).collect();
        // Connected focused star; larger fixture has a partial star so 1024
        // edges reach 513 nodes and explicitly leave the rest disconnected.
        let edges=(0..e).map(|i| { let pair=i/2+1; let (a,b)=if pair<n {(0,pair)} else {(1,2)}; let (a,b)=if i%2==0{(a,b)}else{(b,a)};
   from_value(json!({"id":format!("e{i:04}"),"context":context,"from":{"id":format!("n{a:04}"),"map":map},"to":{"id":format!("n{b:04}"),"map":map},"mode":"walking","requirements":{"status":"not_applicable"},"expected_ms":{"status":"known","value":1},"wait_ms":{"status":"not_applicable"},"uncertainty":{"status":"unknown"},"status":"estimated","evidence":[],"consumes_resource":{"status":"unknown"},"cooldown_ms":{"status":"unknown"}})).unwrap()
  }).collect();
        let snapshots=(0..members).map(|i|from_value(json!({"schema_version":1,"id":format!("snap{i}"),"character":format!("char{i}"),"context":context,"capture":{"clock_trust":"trusted","observed_at_utc_ms":100},"source_revision":"synthetic-r1","level":{"status":"unknown"},"role":{"status":"unknown"},"location":{"status":"unknown"},"preferences":{},"equipment":{},"quests":{},"objectives":{}})).unwrap()).collect();
        Self {
            nodes,
            edges,
            snapshots,
            freshness: FreshnessContext {
                now_utc_ms: Some(UtcMillis::new(100).unwrap()),
                clock_trust: ClockTrust::Trusted,
                session: None,
                max_age_ms: ms(1000),
            },
            predicate: from_value(json!({"op":"all","predicates":[]})).unwrap(),
        }
    }
    pub fn contexts(&self) -> Vec<EvaluationContext<'_>> {
        self.snapshots
            .iter()
            .map(|snapshot| EvaluationContext {
                snapshot,
                party: None,
                freshness: &self.freshness,
                stale_policy: StalePolicy::Reject,
                limits: RuleLimits::default(),
            })
            .collect()
    }
    pub fn solo(
        &self,
        count: usize,
        control: &mut dyn SearchControl,
    ) -> azeroth_core::error::Result<PlannerResult> {
        let contexts = self.contexts();
        let excluded = BTreeSet::new();
        let options = azeroth_core::travel::RouteOptions::default();
        let candidates = (0..count)
            .map(|i| Candidate {
                id: ObjectiveId::new(format!("o{i:04}")).unwrap(),
                node: NodeId::new(format!("n{:04}", i + 1)).unwrap(),
                gate: EligibilityGate::Predicate(&self.predicate),
                utility: 1,
                completion_ms: ms(1000),
                service_ms: ms(0),
                rewards: BTreeSet::from([SourceId::new(format!("r{i:04}")).unwrap()]),
            })
            .collect::<Vec<_>>();
        plan_controlled(
            &PlannerInput {
                goal_profile: SourceId::new("synthetic-v1").unwrap(),
                candidates: &candidates,
                excluded: &excluded,
                nodes: &self.nodes,
                edges: &self.edges,
                origin: NodeId::new("n0000").unwrap(),
                stopping_node: NodeId::new("n0000").unwrap(),
                stopping_service_ms: ms(0),
                budget_ms: ms(2500),
                max_steps: 32,
                context: &contexts[0],
                travel_options: &options,
            },
            control,
        )
    }
    pub fn joint(
        &self,
        count: usize,
        control: &mut dyn SearchControl,
    ) -> azeroth_core::error::Result<JointResult> {
        self.joint_with(
            count,
            &azeroth_core::travel::RouteOptions::default(),
            control,
        )
    }
    pub fn joint_with(
        &self,
        count: usize,
        travel_options: &azeroth_core::travel::RouteOptions,
        control: &mut dyn SearchControl,
    ) -> azeroth_core::error::Result<JointResult> {
        let contexts = self.contexts();
        let excluded = BTreeSet::new();
        let candidates = (0..count)
            .map(|i| JointCandidate {
                id: ObjectiveId::new(format!("o{i:04}")).unwrap(),
                node: NodeId::new(format!("n{:04}", i + 1)).unwrap(),
                readiness: vec![&self.predicate; contexts.len()],
                utilities: vec![1; contexts.len()],
                completion_ms: ms(1000),
                service_ms: ms(0),
                rewards: BTreeSet::from([SourceId::new(format!("r{i:04}")).unwrap()]),
            })
            .collect::<Vec<_>>();
        plan_joint_controlled(
            &JointInput {
                contexts: &contexts,
                candidates: &candidates,
                goal_profile: SourceId::new("synthetic-v1").unwrap(),
                excluded: &excluded,
                nodes: &self.nodes,
                edges: &self.edges,
                origin: NodeId::new("n0000").unwrap(),
                stopping_node: NodeId::new("n0000").unwrap(),
                stopping_service_ms: ms(0),
                budget_ms: ms(2500),
                travel_options,
            },
            control,
        )
    }
}
