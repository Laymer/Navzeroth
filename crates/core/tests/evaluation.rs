mod common;
use azeroth_core::{entities::*, evaluation::*, observation::*, predicate::*, types::*};
use common::*;
use serde_json::{Value, from_value, json};
fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"observed_at_utc_ms":100,"clock_trust":"trusted","source":"synthetic_fixture"})
}
fn map() -> Value {
    json!({"context":context(),"map_id":"map1","floor":"ground","instance":null})
}
fn populated() -> Value {
    let mut s = snapshot();
    s["level"] = known(json!(22));
    s["role"] = known(json!("tank"));
    s["preferences"] = json!({"tips":known(json!(true))});
    s["objectives"] = json!({"o1":known(json!(false))});
    s["quests"] = json!({"q1":{"quest":"q1","context":context(),"accepted":known(json!(true)),"completed":known(json!(false)),"turned_in":unknown(),"objectives":{}}});
    s["location"] = known(json!({"map":map(),"x":0.5,"y":0.5}));
    s["equipment"] = json!({"main_hand":known(json!({"state":"item","item":{"context":context(),"item_id":"item1","variant":known(json!("base")),"instance":unknown(),"modifiers":known(json!({"strength":0}))}}))});
    s
}
fn clock() -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    }
}
fn run(value: Value, s: &Value) -> EvaluationResult {
    let predicate: Predicate = from_value(value).unwrap();
    let snapshot: CharacterSnapshot = from_value(s.clone()).unwrap();
    evaluate(
        &predicate,
        &EvaluationContext {
            snapshot: &snapshot,
            party: None,
            freshness: &clock(),
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        },
    )
    .unwrap()
}
fn equipment() -> Value {
    json!({"op":"equipment_matches","slot":"main_hand","item":{"context":context(),"item_id":"item1","variant":"base"},"modifiers":{"strength":0}})
}
#[test]
fn every_operator_has_concrete_semantic_cases() {
    let s = populated();
    let yes = json!({"op":"level_at_least","level":22});
    let no = json!({"op":"level_at_least","level":23});
    let missing = json!({"op":"quest_state","quest":"absent","state":"completed","value":true});
    let area = json!({"map":map(),"bounds":{"min_x":0,"min_y":0,"max_x":1,"max_y":1}});
    let cases = vec![
        (json!({"op":"all","predicates":[]}), Truth::True),
        (json!({"op":"any","predicates":[]}), Truth::False),
        (
            json!({"op":"all","predicates":[yes,missing]}),
            Truth::Unknown,
        ),
        (json!({"op":"all","predicates":[no,missing]}), Truth::False),
        (json!({"op":"any","predicates":[missing,yes]}), Truth::True),
        (json!({"op":"not","predicate":missing}), Truth::Unknown),
        (json!({"op":"not","predicate":yes}), Truth::False),
        (json!({"op":"known","field":{"field":"level"}}), Truth::True),
        (
            json!({"op":"known","field":{"field":"preference","key":"absent"}}),
            Truth::False,
        ),
        (
            json!({"op":"equals","field":{"field":"level"},"value":{"type":"integer","value":22}}),
            Truth::True,
        ),
        (
            json!({"op":"equals","field":{"field":"level"},"value":{"type":"integer","value":23}}),
            Truth::False,
        ),
        (
            json!({"op":"greater_than_or_equal","field":{"field":"level"},"value":{"type":"integer","value":22}}),
            Truth::True,
        ),
        (
            json!({"op":"greater_than_or_equal","field":{"field":"level"},"value":{"type":"integer","value":23}}),
            Truth::False,
        ),
        (
            json!({"op":"quest_state","quest":"q1","state":"accepted","value":true}),
            Truth::True,
        ),
        (
            json!({"op":"quest_state","quest":"q1","state":"completed","value":true}),
            Truth::False,
        ),
        (
            json!({"op":"quest_state","quest":"q1","state":"turned_in","value":true}),
            Truth::Unknown,
        ),
        (
            json!({"op":"objective_state","objective":"o1","completed":false}),
            Truth::True,
        ),
        (
            json!({"op":"objective_state","objective":"o1","completed":true}),
            Truth::False,
        ),
        (
            json!({"op":"objective_state","objective":"absent","completed":false}),
            Truth::Unknown,
        ),
        (yes, Truth::True),
        (no, Truth::False),
        (equipment(), Truth::True),
        (json!({"op":"role_is","role":"tank"}), Truth::True),
        (json!({"op":"role_is","role":"mage"}), Truth::False),
        (
            json!({"op":"location_in_supported_area","area":area}),
            Truth::True,
        ),
        (
            json!({"op":"user_preference","key":"tips","value":true}),
            Truth::True,
        ),
        (
            json!({"op":"user_preference","key":"tips","value":false}),
            Truth::False,
        ),
        (
            json!({"op":"user_preference","key":"absent","value":false}),
            Truth::Unknown,
        ),
        (
            json!({"op":"participant_state","participant":"absent","predicate":{"op":"all","predicates":[]}}),
            Truth::Unknown,
        ),
    ];
    for (predicate, truth) in cases {
        let result = run(predicate.clone(), &s);
        assert_eq!(result.truth, truth, "{predicate}");
        assert_eq!(result.should_trigger(), truth == Truth::True);
    }
    let outside = json!({"op":"location_in_supported_area","area":{"map":map(),"bounds":{"min_x":0.6,"min_y":0.6,"max_x":1,"max_y":1}}});
    assert_eq!(run(outside, &s).truth, Truth::False);
}
#[test]
fn unavailable_inputs_never_become_false_or_zero() {
    for status in ["unknown", "restricted", "unsupported", "not_applicable"] {
        let mut s = populated();
        s["level"] = json!({"status":status});
        let r = run(json!({"op":"level_at_least","level":1}), &s);
        assert_eq!(r.truth, Truth::Unknown);
        assert!(!r.should_trigger());
        assert!(matches!(r.reasons[0].reason, Reason::Unavailable { .. }));
        assert_eq!(
            run(json!({"op":"known","field":{"field":"level"}}), &s).truth,
            Truth::False
        );
    }
    let mut s = populated();
    s["equipment"]["main_hand"]["value"]["item"]["variant"] = unknown();
    assert_eq!(run(equipment(), &s).truth, Truth::Unknown);
    s = populated();
    s["equipment"]["main_hand"]["value"]["item"]["modifiers"] = unknown();
    assert_eq!(run(equipment(), &s).truth, Truth::Unknown);
    s = populated();
    s["equipment"]["main_hand"] = known(json!({"state":"empty"}));
    assert_eq!(run(equipment(), &s).truth, Truth::False);
    let mut mismatch = equipment();
    mismatch["modifiers"]["strength"] = json!(1);
    assert_eq!(run(mismatch, &populated()).truth, Truth::False);
    s = populated();
    s["role"] = unknown();
    s["location"] = unknown();
    assert_eq!(
        run(json!({"op":"role_is","role":"tank"}), &s).truth,
        Truth::Unknown
    );
    assert_eq!(
        run(
            json!({"op":"location_in_supported_area","area":{"map":map(),"bounds":null}}),
            &s
        )
        .truth,
        Truth::Unknown
    );
}
#[test]
fn stale_values_require_explicit_opt_in_and_keep_warnings() {
    let mut data = populated();
    data["level"]["observed_at_utc_ms"] = json!(0);
    let snapshot: CharacterSnapshot = from_value(data.clone()).unwrap();
    let p: Predicate = from_value(json!({"op":"level_at_least","level":22})).unwrap();
    let freshness = clock();
    let mut ctx = EvaluationContext {
        snapshot: &snapshot,
        party: None,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    assert_eq!(evaluate(&p, &ctx).unwrap().truth, Truth::Unknown);
    ctx.stale_policy = StalePolicy::AllowStale;
    let r = evaluate(&p, &ctx).unwrap();
    assert_eq!(r.truth, Truth::True);
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(r.warnings[0].reason, Reason::Stale);
    assert_eq!(snapshot.level.status(), ObservationStatus::Known);
    data["level"]["clock_trust"] = json!("uncertain");
    data["level"]["received_at_utc_ms"] = json!(150);
    let snapshot = from_value(data).unwrap();
    ctx.snapshot = &snapshot;
    assert_eq!(evaluate(&p, &ctx).unwrap().truth, Truth::Unknown);
}
#[test]
fn party_uses_each_participants_observation_and_time() {
    let root: CharacterSnapshot = from_value(populated()).unwrap();
    let mut peer = populated();
    peer["id"] = json!("snap2");
    peer["character"] = json!("char2");
    peer["level"] = known(json!(15));
    let make_party = |peer: Value| {
        from_value::<PartySnapshot>(
            json!({"id":"party1","context":context(),"participants":[root,peer]}),
        )
        .unwrap()
    };
    let party = make_party(peer.clone());
    let freshness = clock();
    let mut ctx = EvaluationContext {
        snapshot: &root,
        party: Some(&party),
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let test = |threshold| {
        from_value::<Predicate>(json!({"op":"participant_state","participant":"char2","predicate":{"op":"level_at_least","level":threshold}})).unwrap()
    };
    assert_eq!(evaluate(&test(10), &ctx).unwrap().truth, Truth::True);
    assert_eq!(evaluate(&test(20), &ctx).unwrap().truth, Truth::False);
    peer["level"]["observed_at_utc_ms"] = json!(0);
    let stale = make_party(peer);
    ctx.party = Some(&stale);
    let r = evaluate(&test(10), &ctx).unwrap();
    assert_eq!(r.truth, Truth::Unknown);
    assert_eq!(r.reasons[0].participant.as_str(), "char2");
}
#[test]
fn truth_tables_cover_all_three_values() {
    let values = [Truth::True, Truth::False, Truth::Unknown];
    let and = [
        [Truth::True, Truth::False, Truth::Unknown],
        [Truth::False, Truth::False, Truth::False],
        [Truth::Unknown, Truth::False, Truth::Unknown],
    ];
    let or = [
        [Truth::True, Truth::True, Truth::True],
        [Truth::True, Truth::False, Truth::Unknown],
        [Truth::True, Truth::Unknown, Truth::Unknown],
    ];
    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            assert_eq!(a.and(*b), and[i][j]);
            assert_eq!(a.or(*b), or[i][j]);
        }
    }
    assert_eq!(Truth::Unknown.negate(), Truth::Unknown);
}
#[test]
fn execution_limits_and_context_validation_cannot_be_short_circuited() {
    let snapshot: CharacterSnapshot = from_value(populated()).unwrap();
    let p: Predicate = from_value(json!({"op":"level_at_least","level":22})).unwrap();
    let freshness = clock();
    let mut ctx = EvaluationContext {
        snapshot: &snapshot,
        party: None,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::new(32, 1024, 3).unwrap(),
    };
    assert_eq!(evaluate(&p, &ctx).unwrap().steps, 3);
    ctx.limits = RuleLimits::new(32, 1024, 2).unwrap();
    assert!(evaluate(&p, &ctx).is_err());
    let mut wrong = equipment();
    wrong["item"]["context"]["namespace"] = json!("reference");
    let p =
        from_value(json!({"op":"any","predicates":[{"op":"all","predicates":[]},wrong]})).unwrap();
    ctx.limits = RuleLimits::default();
    assert!(evaluate(&p, &ctx).is_err());
}
#[test]
fn rule_expiry_is_separate_from_observation_status() {
    let snapshot: CharacterSnapshot = from_value(populated()).unwrap();
    let mut freshness = clock();
    let rule:ContextRule=from_value(json!({"id":"rule1","context":context(),"trigger":"manual","predicate":{"op":"level_at_least","level":22},"result_template":"synthetic","priority":0,"deduplication_key":"rule1","cooldown_ms":0,"dismissal":"session","expires_at_utc_ms":150,"evidence":[]})).unwrap();
    let run_rule = |freshness: &FreshnessContext| {
        evaluate_rule(
            &rule,
            &EvaluationContext {
                snapshot: &snapshot,
                party: None,
                freshness,
                stale_policy: StalePolicy::Reject,
                limits: RuleLimits::default(),
            },
        )
        .unwrap()
    };
    assert_eq!(run_rule(&freshness).truth, Truth::False);
    freshness.now_utc_ms = Some(UtcMillis::new(149).unwrap());
    assert_eq!(run_rule(&freshness).truth, Truth::True);
    freshness.clock_trust = ClockTrust::Uncertain;
    assert_eq!(run_rule(&freshness).truth, Truth::Unknown);
    assert_eq!(snapshot.level.status(), ObservationStatus::Known);
}
