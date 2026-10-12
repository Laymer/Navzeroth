mod common;
use azeroth_core::{
    entities::*, error::ErrorCode, evaluation::*, observation::*, predicate::*, travel::*, types::*,
};
use common::*;
use serde_json::{Value, from_value, json, to_value};

// All data is synthetic: wow_forever, build SYNTHETIC, namespace demo, fictional ids.
const MAX: u64 = 9_007_199_254_740_991;

fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"observed_at_utc_ms":100,"clock_trust":"trusted","source":"synthetic_fixture"})
}
fn na() -> Value {
    json!({"status":"not_applicable"})
}
fn ms(value: u64) -> Value {
    json!({"status":"known","value":value})
}
fn clock(now: u64) -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(now as i64).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    }
}
fn snapshot_value() -> Value {
    let mut s = snapshot();
    s["level"] = known(json!(22));
    s
}
fn node_map(id: &str) -> Value {
    let (map, floor) = match id {
        "x1" | "x2" => ("m2", "ground"),
        "u1" => ("m1", "upper"),
        _ => ("m1", "ground"),
    };
    json!({"context":context(),"map_id":map,"floor":floor,"instance":null})
}
fn node(id: &str) -> TravelNode {
    from_value(json!({"id":id,"position":{"map":node_map(id),"x":0.1,"y":0.2},"service":unknown()}))
        .unwrap()
}
fn nodes(ids: &[&str]) -> Vec<TravelNode> {
    ids.iter().map(|id| node(id)).collect()
}
fn edge_value(id: &str, from: &str, to: &str, mode: &str, expected: u64) -> Value {
    json!({
        "id":id,"context":context(),
        "from":{"id":from,"map":node_map(from)},"to":{"id":to,"map":node_map(to)},
        "mode":mode,"requirements":na(),"expected_ms":ms(expected),"wait_ms":na(),
        "uncertainty":unknown(),"status":"verified","evidence":["ev1"],
        "consumes_resource":unknown(),"cooldown_ms":unknown()
    })
}
fn walk(id: &str, from: &str, to: &str, expected: u64) -> Value {
    edge_value(id, from, to, "walking", expected)
}
fn edges(values: Vec<Value>) -> Vec<TravelEdge> {
    values.into_iter().map(|v| from_value(v).unwrap()).collect()
}
fn stateful(id: &str, from: &str, to: &str, mode: &str, expected: u64) -> Value {
    let mut value = edge_value(id, from, to, mode, expected);
    value["consumes_resource"] = ms_bool(true);
    value["cooldown_ms"] = ms(600_000);
    value
}
fn ms_bool(value: bool) -> Value {
    json!({"status":"known","value":value})
}
fn run_with(
    n: &[TravelNode],
    e: &[TravelEdge],
    snap: &Value,
    now: u64,
    options: &RouteOptions,
    from: &str,
    to: &str,
) -> Result<RouteSearch, azeroth_core::error::AppError> {
    let snapshot: CharacterSnapshot = from_value(snap.clone()).unwrap();
    let freshness = clock(now);
    shortest_route(
        n,
        e,
        &NodeId::new(from).unwrap(),
        &NodeId::new(to).unwrap(),
        &EvaluationContext {
            snapshot: &snapshot,
            party: None,
            freshness: &freshness,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        },
        options,
    )
}
fn run(n: &[TravelNode], e: &[TravelEdge], from: &str, to: &str) -> RouteSearch {
    run_with(
        n,
        e,
        &snapshot_value(),
        150,
        &RouteOptions::default(),
        from,
        to,
    )
    .unwrap()
}
fn complete(search: &RouteSearch) -> &Route {
    match &search.outcome {
        RouteOutcome::Complete { route } => route,
        other => panic!("expected a complete route, got {other:?}"),
    }
}
fn hop_ids(route: &Route) -> Vec<&str> {
    route.hops.iter().map(|h| h.edge.as_str()).collect()
}
fn reason<'a>(search: &'a RouteSearch, edge: &str) -> &'a TravelExclusion {
    &search
        .exclusions
        .iter()
        .find(|x| x.edge.as_str() == edge)
        .unwrap_or_else(|| panic!("edge {edge} is not excluded"))
        .reason
}
fn is_incomplete(search: &RouteSearch) -> bool {
    matches!(search.outcome, RouteOutcome::Incomplete { .. })
}
fn requirement(predicate: Value) -> Value {
    json!({"status":"known","value":predicate})
}

#[test]
fn cost_is_expected_plus_wait_plus_caller_service_time() {
    let n = nodes(&["a", "b", "d"]);
    let mut boat = edge_value("boat", "a", "d", "boat", 150);
    boat["wait_ms"] = ms(60);
    let e = edges(vec![
        walk("w1", "a", "b", 100),
        walk("w2", "b", "d", 100),
        boat,
    ]);
    let search = run(&n, &e, "a", "d");
    let route = complete(&search);
    assert_eq!(hop_ids(route), ["w1", "w2"]);
    assert_eq!(route.total_ms.get(), 200);
    assert!(!route.is_estimate());

    // The boat costs 150 + 60 = 210 without service time; 20 more is still dearer.
    let mut options = RouteOptions::default();
    options
        .service_ms
        .insert(EdgeId::new("boat").unwrap(), Millis::new(20).unwrap());
    let with_service = run_with(&n, &e, &snapshot_value(), 150, &options, "a", "d").unwrap();
    assert_eq!(complete(&with_service).total_ms.get(), 200);

    // A cheaper wait makes the boat win; the hop reports each cost term.
    let mut cheap = edge_value("boat", "a", "d", "boat", 150);
    cheap["wait_ms"] = ms(10);
    let e = edges(vec![
        walk("w1", "a", "b", 100),
        walk("w2", "b", "d", 100),
        cheap,
    ]);
    let search = run_with(&n, &e, &snapshot_value(), 150, &options, "a", "d").unwrap();
    let route = complete(&search);
    assert_eq!(hop_ids(route), ["boat"]);
    assert_eq!(route.total_ms.get(), 180);
    assert_eq!(route.hops[0].service_ms.get(), 20);
    assert_eq!(route.hops[0].cost_ms.get(), 180);
}

#[test]
fn case_004_unknown_flight_path_not_assumed() {
    let n = nodes(&["a", "b", "d"]);
    let walking = vec![walk("w1", "a", "b", 400), walk("w2", "b", "d", 400)];

    // A verified flight whose requirement observation is unknown is excluded.
    let mut unknown_requirement = edge_value("fly", "a", "d", "flight", 50);
    unknown_requirement["requirements"] = unknown();
    let mut all = walking.clone();
    all.push(unknown_requirement);
    let search = run(&n, &edges(all), "a", "d");
    assert_eq!(hop_ids(complete(&search)), ["w1", "w2"]);
    assert_eq!(
        reason(&search, "fly"),
        &TravelExclusion::RequirementUnavailable {
            status: ObservationStatus::Unknown
        }
    );

    // A known predicate over a missing snapshot record evaluates to Unknown.
    let mut missing_quest = edge_value("fly", "a", "d", "flight", 50);
    missing_quest["requirements"] = requirement(
        json!({"op":"quest_state","quest":"absent_quest","state":"completed","value":true}),
    );
    let mut all = walking.clone();
    all.push(missing_quest.clone());
    let search = run(&n, &edges(all), "a", "d");
    assert_eq!(hop_ids(complete(&search)), ["w1", "w2"]);
    let TravelExclusion::RequirementUnknown { issues } = reason(&search, "fly") else {
        panic!("expected RequirementUnknown");
    };
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].reason, Reason::MissingRecord);

    // Without a walking alternative the outcome is incomplete, never the flight.
    let search = run(&n, &edges(vec![missing_quest]), "a", "d");
    assert!(is_incomplete(&search));
    let RouteOutcome::Incomplete { reached, .. } = &search.outcome else {
        unreachable!()
    };
    assert!(reached.hops.is_empty());
}

#[test]
fn stale_requirement_inputs_stay_unknown_not_false() {
    let n = nodes(&["a", "d"]);
    let mut fly = edge_value("fly", "a", "d", "flight", 50);
    fly["requirements"] = requirement(json!({"op":"level_at_least","level":10}));
    let e = edges(vec![fly]);
    let fresh = run(&n, &e, "a", "d");
    assert_eq!(hop_ids(complete(&fresh)), ["fly"]);
    // Observed at 100 with max age 100: at 1000 the level is stale.
    let search = run_with(
        &n,
        &e,
        &snapshot_value(),
        1000,
        &RouteOptions::default(),
        "a",
        "d",
    )
    .unwrap();
    assert!(is_incomplete(&search));
    let TravelExclusion::RequirementUnknown { issues } = reason(&search, "fly") else {
        panic!("stale input must stay Unknown");
    };
    assert_eq!(issues[0].reason, Reason::Stale);
}

#[test]
fn locked_flight_path_excluded() {
    let n = nodes(&["a", "d"]);
    let mut locked = edge_value("fly", "a", "d", "flight", 50);
    locked["requirements"] = requirement(json!({"op":"level_at_least","level":30}));
    let search = run(&n, &edges(vec![locked]), "a", "d");
    assert!(is_incomplete(&search));
    assert_eq!(reason(&search, "fly"), &TravelExclusion::RequirementNotMet);

    let mut open = edge_value("fly", "a", "d", "flight", 50);
    open["requirements"] = requirement(json!({"op":"level_at_least","level":20}));
    let search = run(&n, &edges(vec![open]), "a", "d");
    assert_eq!(hop_ids(complete(&search)), ["fly"]);
    assert!(search.exclusions.is_empty());
}

#[test]
fn directed_edge_not_reversed() {
    let n = nodes(&["a", "b"]);
    let e = edges(vec![walk("ab", "a", "b", 10)]);
    assert_eq!(hop_ids(complete(&run(&n, &e, "a", "b"))), ["ab"]);
    let back = run(&n, &e, "b", "a");
    let RouteOutcome::Incomplete {
        target,
        reached,
        reachable,
    } = &back.outcome
    else {
        panic!("reverse direction must not be fabricated");
    };
    assert_eq!(target.as_str(), "a");
    assert!(reached.hops.is_empty());
    assert_eq!(reached.end().id.as_str(), "b");
    assert_eq!(reachable, &[NodeId::new("b").unwrap()]);
}

#[test]
fn map_floor_boundary_requires_explicit_edge() {
    // a is on m1/ground, u1 on m1/upper, x1 on m2/ground.
    let n = nodes(&["a", "u1", "x1"]);
    let none = run(&n, &[], "a", "u1");
    assert!(is_incomplete(&none));
    let none = run(&n, &[], "a", "x1");
    assert!(is_incomplete(&none));

    let e = edges(vec![
        walk("a_u1", "a", "u1", 30),
        edge_value("u1_x1", "u1", "x1", "boat", 90),
    ]);
    let search = run(&n, &e, "a", "x1");
    let route = complete(&search);
    assert_eq!(hop_ids(route), ["a_u1", "u1_x1"]);
    assert_eq!(route.hops[0].to.map.floor.as_str(), "upper");
    assert_eq!(route.hops[1].to.map.map_id.as_str(), "m2");

    // A reference whose map disagrees with the node's position is excluded.
    let mut wrong = walk("a_u1", "a", "u1", 30);
    wrong["to"]["map"] = node_map("a");
    let search = run(&n, &edges(vec![wrong]), "a", "u1");
    assert!(is_incomplete(&search));
    assert_eq!(reason(&search, "a_u1"), &TravelExclusion::MapMismatch);
}

#[test]
fn disconnected_graph_returns_incomplete_known_segment() {
    let n = nodes(&["a", "b", "c", "d"]);
    let e = edges(vec![walk("ab", "a", "b", 10), walk("bc", "b", "c", 20)]);
    let search = run(&n, &e, "a", "d");
    let RouteOutcome::Incomplete {
        target,
        reached,
        reachable,
    } = &search.outcome
    else {
        panic!("target is unreachable");
    };
    assert_eq!(target.as_str(), "d");
    assert_eq!(reached.end().id.as_str(), "c");
    assert_eq!(reached.total_ms.get(), 30);
    assert_eq!(hop_ids(reached), ["ab", "bc"]);
    let ids: Vec<_> = reachable.iter().map(|id| id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c"]);

    // The known segment is a valid RoutePlan travel list and stops short of d.
    let segments = reached.to_travel_segments();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].to, segments[1].from);
    assert!(segments.iter().all(|s| s.to.id.as_str() != "d"));
    let plan = json!({
        "schema_version":1,"id":"plan1","context":context(),"generated_at_utc_ms":100,
        "inputs":[{"id":"snap1","character":"char1","context":context()}],
        "dataset_revision":"demo-r1","ruleset_version":"rules1","score_model_version":"scores1",
        "capability_assumptions":{},"goal":"synthetic","time_budget_ms":1000,"exclusions":[],
        "manual_assumptions":[],"steps":[],"travel":to_value(&segments).unwrap(),
        "evidence":[],"expires_at_utc_ms":null,"revalidation":[]
    });
    from_value::<RoutePlan>(plan).expect("segments satisfy RoutePlan continuity and ranges");
}

#[test]
fn complete_route_converts_to_valid_plan_segments_with_ranges() {
    let n = nodes(&["a", "b", "c"]);
    let mut first = walk("ab", "a", "b", 10);
    first["uncertainty"] = json!({"status":"known","value":{"min_ms":5,"max_ms":20}});
    let mut second = walk("bc", "b", "c", 20);
    second["wait_ms"] = ms(7);
    let search = run(&n, &edges(vec![first, second]), "a", "c");
    let route = complete(&search);
    assert_eq!(route.total_ms.get(), 37);
    let segments = route.to_travel_segments();
    assert_eq!(segments[0].expected_ms.value().unwrap().get(), 10);
    assert_eq!(segments[0].range.value().unwrap().max_ms.get(), 20);
    // Segments carry travel time; the wait is only in the route total.
    assert_eq!(segments[1].expected_ms.value().unwrap().get(), 20);
    let plan = json!({
        "schema_version":1,"id":"plan1","context":context(),"generated_at_utc_ms":100,
        "inputs":[{"id":"snap1","character":"char1","context":context()}],
        "dataset_revision":"demo-r1","ruleset_version":"rules1","score_model_version":"scores1",
        "capability_assumptions":{},"goal":"synthetic","time_budget_ms":1000,"exclusions":[],
        "manual_assumptions":[],"steps":[],"travel":to_value(&segments).unwrap(),
        "evidence":[],"expires_at_utc_ms":null,"revalidation":[]
    });
    from_value::<RoutePlan>(plan).unwrap();
}

#[test]
fn unavailable_and_unverified_transport_excluded() {
    let n = nodes(&["a", "b", "d"]);
    let mut closed = edge_value("fly_closed", "a", "d", "flight", 10);
    closed["status"] = json!("temporarily_unavailable");
    let mut unverified = edge_value("fly_unverified", "a", "d", "flight", 10);
    unverified["status"] = json!("unverified");
    unverified["evidence"] = json!([]);
    let mut estimated = edge_value("boat_estimated", "a", "b", "boat", 50);
    estimated["status"] = json!("estimated");
    estimated["evidence"] = json!([]);
    let e = edges(vec![
        closed,
        unverified,
        estimated,
        walk("w2", "b", "d", 100),
    ]);
    let search = run(&n, &e, "a", "d");
    assert_eq!(
        reason(&search, "fly_closed"),
        &TravelExclusion::TemporarilyUnavailable
    );
    assert_eq!(
        reason(&search, "fly_unverified"),
        &TravelExclusion::Unverified
    );
    let route = complete(&search);
    assert_eq!(hop_ids(route), ["boat_estimated", "w2"]);
    assert!(route.is_estimate());
    assert_eq!(route.hops[0].status, EdgeStatus::Estimated);
    assert_eq!(search.exclusions.len(), 2);
}

#[test]
fn unknown_cost_terms_exclude_the_edge_and_not_applicable_wait_is_zero() {
    let n = nodes(&["a", "d"]);
    let mut no_expected = walk("no_expected", "a", "d", 1);
    no_expected["expected_ms"] = unknown();
    let mut no_wait = edge_value("no_wait", "a", "d", "boat", 5);
    no_wait["wait_ms"] = unknown();
    let mut restricted_wait = edge_value("restricted_wait", "a", "d", "tram", 5);
    restricted_wait["wait_ms"] = json!({"status":"restricted"});
    let search = run(
        &n,
        &edges(vec![no_expected, no_wait, restricted_wait]),
        "a",
        "d",
    );
    assert!(is_incomplete(&search));
    assert_eq!(
        reason(&search, "no_expected"),
        &TravelExclusion::CostUnknown {
            term: CostTerm::Expected
        }
    );
    assert_eq!(
        reason(&search, "no_wait"),
        &TravelExclusion::CostUnknown {
            term: CostTerm::Wait
        }
    );
    assert_eq!(
        reason(&search, "restricted_wait"),
        &TravelExclusion::CostUnknown {
            term: CostTerm::Wait
        }
    );

    let ok = run(&n, &edges(vec![walk("ok", "a", "d", 5)]), "a", "d");
    assert_eq!(complete(&ok).total_ms.get(), 5);
}

#[test]
fn return_action_used_at_most_once_at_end() {
    let n = nodes(&["a", "b", "t"]);
    // The return is cheap, but it may only close the route.
    let e = edges(vec![
        walk("w_ab", "a", "b", 100),
        walk("w_bt", "b", "t", 500),
        stateful("ret_bt", "b", "t", "return", 10),
        // A mid-route return toward b is never relaxed, even though it is free.
        stateful("ret_ab", "a", "b", "return", 0),
    ]);
    let search = run(&n, &e, "a", "t");
    let route = complete(&search);
    assert_eq!(hop_ids(route), ["w_ab", "ret_bt"]);
    assert_eq!(route.total_ms.get(), 110);
    assert_eq!(route.hops.iter().filter(|h| h.stateful).count(), 1);
    assert!(route.hops.last().unwrap().stateful);
    assert_eq!(
        reason(&search, "ret_ab"),
        &TravelExclusion::StatefulNotFinalHop
    );

    // Two chained returns cannot both be used.
    let chain = edges(vec![
        stateful("ret1", "a", "b", "return", 1),
        stateful("ret2", "b", "t", "return", 1),
    ]);
    let search = run(&n, &chain, "a", "t");
    assert!(is_incomplete(&search));
    assert_eq!(
        reason(&search, "ret1"),
        &TravelExclusion::StatefulNotFinalHop
    );

    // A resource-consuming or cooldown-bound edge of an ordinary mode is stateful too.
    let mut consuming = edge_value("fly_use", "b", "t", "flight", 5);
    consuming["consumes_resource"] = ms_bool(true);
    consuming["cooldown_ms"] = ms(0);
    let mut cooldown = edge_value("fly_cd", "b", "t", "flight", 6);
    cooldown["consumes_resource"] = ms_bool(false);
    cooldown["cooldown_ms"] = ms(30_000);
    let mid = edges(vec![
        walk("w_ab", "a", "b", 100),
        consuming.clone(),
        cooldown.clone(),
    ]);
    let search = run(&n, &mid, "a", "t");
    assert_eq!(hop_ids(complete(&search)), ["w_ab", "fly_use"]);
    // Used as a mid-route hop toward b they would be excluded.
    let mid_edge = {
        let mut v = consuming;
        v["from"] = json!({"id":"a","map":node_map("a")});
        v["to"] = json!({"id":"b","map":node_map("b")});
        v["id"] = json!("fly_mid");
        v
    };
    let search = run(&n, &edges(vec![mid_edge, cooldown]), "a", "t");
    assert!(is_incomplete(&search));
    assert_eq!(
        reason(&search, "fly_mid"),
        &TravelExclusion::StatefulNotFinalHop
    );
}

#[test]
fn unknown_return_availability_stays_unknown() {
    let n = nodes(&["a", "t"]);
    let mut unknown_use = stateful("ret", "a", "t", "return", 10);
    unknown_use["consumes_resource"] = unknown();
    let search = run(&n, &edges(vec![unknown_use]), "a", "t");
    assert!(is_incomplete(&search));
    assert_eq!(
        reason(&search, "ret"),
        &TravelExclusion::StatefulAvailabilityUnknown {
            consumes_resource: ObservationStatus::Unknown,
            cooldown: ObservationStatus::Known,
        }
    );

    let mut unknown_cooldown = stateful("ret", "a", "t", "teleport", 10);
    unknown_cooldown["cooldown_ms"] = json!({"status":"restricted"});
    let search = run(&n, &edges(vec![unknown_cooldown]), "a", "t");
    assert_eq!(
        reason(&search, "ret"),
        &TravelExclusion::StatefulAvailabilityUnknown {
            consumes_resource: ObservationStatus::Known,
            cooldown: ObservationStatus::Restricted,
        }
    );

    // A return mode without any resource observation is not assumed free.
    let search = run(
        &n,
        &edges(vec![edge_value("ret", "a", "t", "return", 1)]),
        "a",
        "t",
    );
    assert!(is_incomplete(&search));

    // Estimated stateful edges are not the reserved verified action.
    let mut estimated = stateful("ret", "a", "t", "return", 10);
    estimated["status"] = json!("estimated");
    let search = run(&n, &edges(vec![estimated]), "a", "t");
    assert_eq!(
        reason(&search, "ret"),
        &TravelExclusion::StatefulNotVerified
    );

    // An unmet requirement on the verified return wins over its resource state.
    let mut gated = stateful("ret", "a", "t", "return", 10);
    gated["requirements"] = requirement(json!({"op":"level_at_least","level":60}));
    let search = run(&n, &edges(vec![gated]), "a", "t");
    assert_eq!(reason(&search, "ret"), &TravelExclusion::RequirementNotMet);
}

#[test]
fn deterministic_tie_break_independent_of_input_order() {
    let n = nodes(&["a", "b", "c", "t"]);
    let base = vec![
        walk("e_b1", "a", "b", 50),
        walk("e_b2", "b", "t", 50),
        walk("e_a1", "a", "c", 50),
        walk("e_a2", "c", "t", 50),
        // Same cost with fewer hops beats both two-hop routes.
        walk("e_z", "a", "t", 100),
    ];
    let reference = run(&n, &edges(base.clone()), "a", "t");
    assert_eq!(hop_ids(complete(&reference)), ["e_z"]);

    // Without the direct edge, the smallest edge-id sequence wins.
    let two_hops: Vec<Value> = base[..4].to_vec();
    let reference = run(&n, &edges(two_hops.clone()), "a", "t");
    assert_eq!(hop_ids(complete(&reference)), ["e_a1", "e_a2"]);

    // All 24 edge orderings and a reversed node order give an identical result.
    let mut orders = vec![vec![]];
    for _ in 0..two_hops.len() {
        let mut next = vec![];
        for order in &orders {
            for index in 0..two_hops.len() {
                if !order.contains(&index) {
                    let mut extended = order.clone();
                    extended.push(index);
                    next.push(extended);
                }
            }
        }
        orders = next;
    }
    assert_eq!(orders.len(), 24);
    let mut reversed_nodes = n.clone();
    reversed_nodes.reverse();
    for order in orders {
        let shuffled: Vec<Value> = order.iter().map(|i| two_hops[*i].clone()).collect();
        let e = edges(shuffled);
        assert_eq!(run(&n, &e, "a", "t"), reference);
        assert_eq!(run(&reversed_nodes, &e, "a", "t"), reference);
    }
}

#[test]
fn build_mismatch_rejected() {
    let n = nodes(&["a", "b"]);
    let other = json!({"build":{"edition":"wow_forever","build":"OTHER","interface":"SYNTHETIC"},"namespace":"demo"});
    let reference = json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"reference"});
    for foreign in [other, reference] {
        let map = json!({"context":foreign,"map_id":"m1","floor":"ground","instance":null});
        let mut value = walk("ab", "a", "b", 10);
        value["context"] = foreign.clone();
        value["from"]["map"] = map.clone();
        value["to"]["map"] = map;
        let e = edges(vec![value]);
        let err = run_with(
            &n,
            &e,
            &snapshot_value(),
            150,
            &RouteOptions::default(),
            "a",
            "b",
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::BuildMismatch);

        // A node from another context is rejected the same way.
        let foreign_node: TravelNode = from_value(json!({
            "id":"f","position":{"map":{"context":foreign,"map_id":"m1","floor":"ground","instance":null},"x":0.1,"y":0.2},
            "service":unknown()
        }))
        .unwrap();
        let err = run_with(
            &[foreign_node],
            &[],
            &snapshot_value(),
            150,
            &RouteOptions::default(),
            "f",
            "f",
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::BuildMismatch);
    }
}

#[test]
fn cost_overflow_rejected() {
    let n = nodes(&["a", "b", "c"]);
    // One edge: expected at the ceiling plus any wait overflows the safe range.
    let mut over = walk("ab", "a", "b", MAX);
    over["wait_ms"] = ms(1);
    let err = run_with(
        &n,
        &edges(vec![over]),
        &snapshot_value(),
        150,
        &RouteOptions::default(),
        "a",
        "b",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);

    // Service time counts toward the ceiling.
    let mut options = RouteOptions::default();
    options
        .service_ms
        .insert(EdgeId::new("ab").unwrap(), Millis::new(1).unwrap());
    let err = run_with(
        &n,
        &edges(vec![walk("ab", "a", "b", MAX)]),
        &snapshot_value(),
        150,
        &options,
        "a",
        "b",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);

    // Each edge fits, their sum does not.
    let half = MAX / 2 + 1;
    let err = run_with(
        &n,
        &edges(vec![walk("ab", "a", "b", half), walk("bc", "b", "c", half)]),
        &snapshot_value(),
        150,
        &RouteOptions::default(),
        "a",
        "c",
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);

    // The ceiling itself is allowed.
    let ok = run(&n, &edges(vec![walk("ab", "a", "b", MAX)]), "a", "b");
    assert_eq!(complete(&ok).total_ms.get(), MAX);
}

#[test]
fn zero_cost_edges_and_cycles_terminate() {
    let n = nodes(&["a", "b", "c", "t"]);
    let e = edges(vec![
        walk("a_b", "a", "b", 0),
        walk("b_a", "b", "a", 0),
        walk("b_c", "b", "c", 0),
        walk("c_a", "c", "a", 0),
        walk("c_b", "c", "b", 0),
        walk("c_t", "c", "t", 0),
        walk("self_loop", "t", "t", 0),
    ]);
    let search = run(&n, &e, "a", "t");
    let route = complete(&search);
    assert_eq!(hop_ids(route), ["a_b", "b_c", "c_t"]);
    assert_eq!(route.total_ms.get(), 0);

    let search = run(&n, &e[..5], "a", "t");
    assert!(is_incomplete(&search));
}

#[test]
fn origin_equal_to_target_is_an_empty_complete_route() {
    let n = nodes(&["a"]);
    let search = run(&n, &[], "a", "a");
    let route = complete(&search);
    assert!(route.hops.is_empty());
    assert_eq!(route.total_ms.get(), 0);
    assert_eq!(route.end().id.as_str(), "a");
    assert!(route.to_travel_segments().is_empty());
}

#[test]
fn invalid_graph_inputs_are_errors_not_exclusions() {
    let n = nodes(&["a", "b"]);
    let call = |n: &[TravelNode], e: &[TravelEdge], from: &str, to: &str| {
        run_with(
            n,
            e,
            &snapshot_value(),
            150,
            &RouteOptions::default(),
            from,
            to,
        )
    };
    // Unknown endpoints.
    assert_eq!(
        call(&n, &[], "a", "zzz").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        call(&n, &[], "zzz", "a").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    // Duplicate node and edge ids.
    let dup_nodes = nodes(&["a", "a"]);
    assert_eq!(
        call(&dup_nodes, &[], "a", "a").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let dup_edges = edges(vec![walk("e", "a", "b", 1), walk("e", "b", "a", 1)]);
    assert_eq!(
        call(&n, &dup_edges, "a", "b").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    // An edge naming a node that is not in the graph.
    let dangling = edges(vec![walk("e", "a", "c", 1)]);
    assert_eq!(
        call(&n, &dangling, "a", "b").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    // More than 1024 edges are rejected rather than truncated.
    let many = edges(
        (0..1025)
            .map(|i| walk(&format!("e{i:04}"), "a", "b", 1))
            .collect(),
    );
    assert_eq!(
        call(&n, &many, "a", "b").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    // Service time naming an unknown edge.
    let mut options = RouteOptions::default();
    options
        .service_ms
        .insert(EdgeId::new("ghost").unwrap(), Millis::new(1).unwrap());
    assert_eq!(
        run_with(&n, &[], &snapshot_value(), 150, &options, "a", "b")
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn evaluation_errors_propagate_instead_of_becoming_unknown() {
    let n = nodes(&["a", "d"]);
    let mut gated = edge_value("fly", "a", "d", "flight", 5);
    gated["requirements"] = requirement(json!({"op":"level_at_least","level":10}));
    let e = edges(vec![gated]);
    let snapshot: CharacterSnapshot = from_value(snapshot_value()).unwrap();
    let freshness = clock(150);
    let result = shortest_route(
        &n,
        &e,
        &NodeId::new("a").unwrap(),
        &NodeId::new("d").unwrap(),
        &EvaluationContext {
            snapshot: &snapshot,
            party: None,
            freshness: &freshness,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::new(32, 1024, 1).unwrap(),
        },
        &RouteOptions::default(),
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::InvalidInput);
}

#[test]
fn exclusions_are_sorted_and_carry_codes_only() {
    let n = nodes(&["a", "d"]);
    let mut b = edge_value("z_edge", "a", "d", "flight", 5);
    b["status"] = json!("unverified");
    b["evidence"] = json!([]);
    let mut a = edge_value("a_edge", "a", "d", "flight", 5);
    a["status"] = json!("temporarily_unavailable");
    let search = run(&n, &edges(vec![b, a]), "a", "d");
    let ids: Vec<_> = search.exclusions.iter().map(|x| x.edge.as_str()).collect();
    assert_eq!(ids, ["a_edge", "z_edge"]);
    assert_eq!(
        to_value(&search.exclusions[0]).unwrap(),
        json!({"edge":"a_edge","reason":{"code":"temporarily_unavailable"}})
    );
    assert_eq!(
        to_value(TravelExclusion::RequirementUnavailable {
            status: ObservationStatus::Restricted
        })
        .unwrap(),
        json!({"code":"requirement_unavailable","status":"restricted"})
    );
    assert_eq!(
        to_value(TravelExclusion::CostUnknown {
            term: CostTerm::Wait
        })
        .unwrap(),
        json!({"code":"cost_unknown","term":"wait"})
    );
}
