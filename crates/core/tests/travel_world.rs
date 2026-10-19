//! Measurement over the generated whole-world pack (ADR 0007 §2, task V2.8g).
//! Ignored by default: it needs the Git LFS catalog. Run with
//!
//!     NAVZEROTH_WORLD_CATALOG=content/world/1.60.1.70450/catalog.json \
//!         cargo test --release -p azeroth-core --test travel_world -- --ignored --nocapture
mod common;
use azeroth_core::{
    entities::{EvidenceRecord, TravelEdge, TravelNode},
    evaluation::{EvaluationContext, StalePolicy},
    observation::{ClockTrust, FreshnessContext},
    predicate::RuleLimits,
    search_control::NoopSearchControl,
    travel::{RouteOptions, RouteOutcome, TravelGraph, inferred_edges, shortest_route},
    types::{Millis, NodeId, UtcMillis},
};
use serde_json::{Value, from_value};
use std::time::Instant;

fn peak_rss_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches(" kB").trim().parse().ok())
}

#[test]
#[ignore = "needs the Git LFS whole-world catalog"]
fn whole_world_route_queries_stay_interactive() {
    let Ok(path) = std::env::var("NAVZEROTH_WORLD_CATALOG") else {
        eprintln!("NAVZEROTH_WORLD_CATALOG is not set");
        return;
    };
    let started = Instant::now();
    let mut catalog: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let payload = catalog["reference"]["payload"].take();
    let nodes: Vec<TravelNode> = from_value(payload["nodes"].clone()).unwrap();
    let edges: Vec<TravelEdge> = from_value(payload["edges"].clone()).unwrap();
    let evidence: Vec<EvidenceRecord> = from_value(payload["evidence"].clone()).unwrap();
    eprintln!(
        "loaded {} nodes, {} edges in {:?}",
        nodes.len(),
        edges.len(),
        started.elapsed()
    );
    let mut snapshot = common::snapshot();
    snapshot["context"] = serde_json::to_value(&nodes[0].position.map.context).unwrap();
    let snapshot = from_value(snapshot).unwrap();
    let freshness = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    };
    let context = EvaluationContext {
        snapshot: &snapshot,
        party: None,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let options = RouteOptions {
        inferred_edges: inferred_edges(&edges, &evidence),
        accept_unknown_requirements: true,
        ..RouteOptions::default()
    };
    eprintln!("accepted inferred edges: {}", options.inferred_edges.len());
    let node = |id: &str| NodeId::new(id).unwrap();
    let started = Instant::now();
    let graph =
        TravelGraph::prepare(&nodes, &edges, &context, &options, &mut NoopSearchControl).unwrap();
    eprintln!("prepare {:?}", started.elapsed());
    // The farthest node a start reaches is a long query by construction:
    // Stormwind's and Orgrimmar's flight masters, and a quest-giver spawn.
    // An unreachable target makes the search settle everything it reaches.
    let island = node("node:graveyard:1");
    let farthest = |from: &NodeId| match graph
        .route(from, &island, &mut NoopSearchControl)
        .unwrap()
        .outcome
    {
        RouteOutcome::Incomplete { reached, .. } => reached.end().id.clone(),
        RouteOutcome::Complete { route } => route.end().id.clone(),
    };
    let mut pairs = vec![];
    for start in ["node:taxi:2", "node:taxi:23", "node:graveyard:105"] {
        let start = node(start);
        let end = farthest(&start);
        pairs.push((start, end));
    }
    pairs.push((node("node:taxi:2"), node("node:taxi:23")));
    for (from, to) in &pairs {
        let started = Instant::now();
        let search = graph.route(from, to, &mut NoopSearchControl).unwrap();
        let elapsed = started.elapsed();
        let started_single = Instant::now();
        let single = shortest_route(&nodes, &edges, from, to, &context, &options).unwrap();
        let single_elapsed = started_single.elapsed();
        assert_eq!(search, single);
        match &search.outcome {
            RouteOutcome::Complete { route } => eprintln!(
                "{} -> {}: complete, {} hops, {} ms, estimate {}, assumed {}; query {:?}, single search {:?}",
                from.as_str(),
                to.as_str(),
                route.hops.len(),
                route.total_ms.get(),
                route.is_estimate(),
                route.assumes_requirements(),
                elapsed,
                single_elapsed
            ),
            RouteOutcome::Incomplete { reachable, .. } => eprintln!(
                "{} -> {}: incomplete, {} reachable; query {:?}, single search {:?}",
                from.as_str(),
                to.as_str(),
                reachable.len(),
                elapsed,
                single_elapsed
            ),
        }
    }
    eprintln!("peak RSS {:?} KiB", peak_rss_kib());
}
