//! Joint planning uses the caller's travel policy (ADR 0007 §5, §8).
#[path = "../examples/support/mod.rs"]
mod support;
use azeroth_core::{entities::TravelEdge, search_control::NoopSearchControl, travel::RouteOptions};
use serde_json::{from_value, json, to_value};

fn inferred_unknown_fixture() -> support::Fixture {
    let mut f = support::Fixture::new(4, 6, 2);
    f.edges = f
        .edges
        .iter()
        .map(|edge| {
            let mut value = to_value(edge).unwrap();
            value["status"] = json!("unverified");
            value["requirements"] = json!({"status":"unknown"});
            from_value::<TravelEdge>(value).unwrap()
        })
        .collect();
    f
}

#[test]
fn strict_policy_leaves_unverified_unknown_edges_unusable() {
    let f = inferred_unknown_fixture();
    // Unknown requirements make every edge unavailable for every member, so
    // the stopping route (origin to origin) still exists but nothing is reached.
    let result = f.joint(2, &mut NoopSearchControl).unwrap();
    assert!(result.plan.selected.is_empty());
    assert!(!result.unavailable_edges.is_empty());
}

#[test]
fn accepted_policy_uses_them_as_estimates() {
    let f = inferred_unknown_fixture();
    let options = RouteOptions {
        inferred_edges: f.edges.iter().map(|e| e.id.clone()).collect(),
        accept_unknown_requirements: true,
        ..RouteOptions::default()
    };
    let result = f.joint_with(2, &options, &mut NoopSearchControl).unwrap();
    assert!(result.unavailable_edges.is_empty());
    assert!(!result.plan.selected.is_empty());
    assert!(result.plan.selected[0].travel.is_estimate());
    assert!(result.plan.selected[0].travel.assumes_requirements());
}
