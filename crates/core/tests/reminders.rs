//! M7.1 reminder engine behavior. All data is synthetic (namespace demo, build
//! SYNTHETIC); no rule content is shipped.
mod common;
use azeroth_core::{
    entities::*, error::ErrorCode, evaluation::*, observation::*, predicate::*, reminders::*,
    types::*,
};
use common::*;
use serde_json::{Value, from_value, json};

fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"observed_at_utc_ms":100,"clock_trust":"trusted","source":"synthetic_fixture","session_id":"s1","session_elapsed_ms":10})
}
fn snap(id: &str) -> CharacterSnapshot {
    let mut s = snapshot();
    s["id"] = json!(id);
    s["level"] = known(json!(22));
    s["role"] = known(json!("tank"));
    from_value(s).unwrap()
}
fn snap_level(id: &str, level: Value) -> CharacterSnapshot {
    let mut s = snapshot();
    s["id"] = json!(id);
    s["level"] = level;
    from_value(s).unwrap()
}
/// Session-relative freshness works with or without a trusted clock.
fn clock(now: Option<i64>, trust: ClockTrust) -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: now.map(|n| UtcMillis::new(n).unwrap()),
        clock_trust: trust,
        session: Some((SessionId::new("s1").unwrap(), Millis::new(20).unwrap())),
        max_age_ms: Millis::new(1000).unwrap(),
    }
}
fn trusted(now: i64) -> FreshnessContext {
    clock(Some(now), ClockTrust::Trusted)
}
fn rule_json(id: &str, priority: i16, key: &str, predicate: Value) -> Value {
    json!({"id":id,"context":context(),"trigger":"manual","predicate":predicate,"result_template":format!("synthetic reminder {id}"),"priority":priority,"deduplication_key":key,"cooldown_ms":0,"dismissal":"session","expires_at_utc_ms":null,"evidence":[]})
}
fn is_true() -> Value {
    json!({"op":"level_at_least","level":22})
}
fn is_false() -> Value {
    json!({"op":"level_at_least","level":23})
}
fn is_unknown() -> Value {
    json!({"op":"quest_state","quest":"absent","state":"completed","value":true})
}
fn rules(values: Vec<Value>) -> Vec<ContextRule> {
    values.into_iter().map(|v| from_value(v).unwrap()).collect()
}
fn id(value: &str) -> RuleId {
    RuleId::new(value).unwrap()
}
fn run(
    engine: &ReminderEngine,
    rules: &[ContextRule],
    snapshot: &CharacterSnapshot,
    freshness: &FreshnessContext,
    inputs: &ReminderInputs,
    state: &ReminderState,
) -> azeroth_core::error::Result<ReminderPlan> {
    engine.plan(
        rules,
        &EvaluationContext {
            snapshot,
            party: None,
            freshness,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        },
        inputs,
        state,
    )
}
/// Normal-mode inputs: every disruptive signal is fresh and known false.
fn clear_inputs() -> ReminderInputs {
    ReminderInputs {
        suppression: suppression(false, false, false),
        ..ReminderInputs::default()
    }
}
fn suppression(combat: bool, loading: bool, cutscene: bool) -> SuppressionState {
    from_value(json!({"combat":known(json!(combat)),"loading":known(json!(loading)),"cutscene":known(json!(cutscene))})).unwrap()
}
fn plan_clear(rules: &[ContextRule], state: &ReminderState) -> ReminderPlan {
    run(
        &ReminderEngine::default(),
        rules,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        state,
    )
    .unwrap()
}
fn queued(plan: &ReminderPlan) -> Vec<&str> {
    plan.queue.iter().map(|i| i.rule.as_str()).collect()
}
fn scope(session: Option<&str>, ruleset: Option<&str>) -> ReminderScope {
    ReminderScope {
        session: session.map(|s| SessionId::new(s).unwrap()),
        ruleset: ruleset.map(|r| DatasetRevision::new(r).unwrap()),
    }
}

#[test]
fn only_true_produces_advice_and_false_produces_nothing() {
    let set = rules(vec![
        rule_json("t", 0, "t", is_true()),
        rule_json("f", 0, "f", is_false()),
    ]);
    let plan = plan_clear(&set, &ReminderState::new());
    assert_eq!(queued(&plan), ["t"]);
    assert!(
        matches!(&plan.queue[0].kind, ReminderKind::Advice { text } if text.as_str() == "synthetic reminder t")
    );
    assert_eq!(plan.excluded[&id("f")], ReminderReason::ConditionFalse);
    assert!(plan.rearm().contains(&id("f")));
}

#[test]
fn unknown_never_produces_strong_advice() {
    let set = rules(vec![rule_json("u", 100, "u", is_unknown())]);
    let omitted = plan_clear(&set, &ReminderState::new());
    assert!(omitted.queue.is_empty());
    assert!(matches!(
        &omitted.excluded[&id("u")],
        ReminderReason::InputsUnknown { reasons } if reasons.iter().all(|i| i.reason == Reason::MissingRecord)
    ));
    let engine = ReminderEngine::new(ReminderPolicy {
        unknown: UnknownPolicy::Informational,
        ..ReminderPolicy::default()
    });
    let plan = run(
        &engine,
        &set,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &ReminderState::new(),
    )
    .unwrap();
    let item = &plan.queue[0];
    assert_eq!(item.strength(), ReminderStrength::Informational);
    // CASE-001: no template, so absence of history is never asserted.
    assert!(matches!(&item.kind, ReminderKind::CannotEvaluate { reasons } if !reasons.is_empty()));
    assert!(
        !serde_json::to_string(&plan)
            .unwrap()
            .contains("synthetic reminder")
    );
}

#[test]
fn restricted_and_unsupported_inputs_disable_only_dependent_rules() {
    let engine = ReminderEngine::new(ReminderPolicy {
        unknown: UnknownPolicy::Informational,
        ..ReminderPolicy::default()
    });
    let set = rules(vec![
        rule_json("needs_level", 5, "a", is_true()),
        rule_json("needs_role", 1, "b", json!({"op":"role_is","role":"tank"})),
    ]);
    for status in ["restricted", "unsupported", "unknown"] {
        let mut s = snapshot();
        s["level"] = json!({"status":status});
        s["role"] = known(json!("tank"));
        let snapshot: CharacterSnapshot = from_value(s).unwrap();
        let plan = run(
            &engine,
            &set,
            &snapshot,
            &trusted(150),
            &clear_inputs(),
            &ReminderState::new(),
        )
        .unwrap();
        assert_eq!(queued(&plan), ["needs_role", "needs_level"]);
        assert_eq!(plan.queue[0].strength(), ReminderStrength::Strong);
        let ReminderKind::CannotEvaluate { reasons } = &plan.queue[1].kind else {
            panic!("expected an informational item");
        };
        let serialized = serde_json::to_string(reasons).unwrap();
        assert!(serialized.contains(&format!(r#""status":"{status}""#)));
    }
}

#[test]
fn context_mismatch_is_an_error_not_a_silent_exclusion() {
    let mut value = rule_json("r", 0, "r", is_true());
    value["context"]["namespace"] = json!("reference");
    let mixed: Vec<ContextRule> = vec![from_value(value).unwrap()];
    let err = run(
        &ReminderEngine::default(),
        &mixed,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &ReminderState::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
}

#[test]
fn expired_rules_drop_and_uncertain_clocks_never_give_strong_advice() {
    let mut value = rule_json("r", 0, "r", is_true());
    value["expires_at_utc_ms"] = json!(150);
    let set = rules(vec![value]);
    let engine = ReminderEngine::new(ReminderPolicy {
        unknown: UnknownPolicy::Informational,
        ..ReminderPolicy::default()
    });
    let plan = |fresh: &FreshnessContext| {
        run(
            &engine,
            &set,
            &snap("snap1"),
            fresh,
            &clear_inputs(),
            &ReminderState::new(),
        )
        .unwrap()
    };
    let expired = plan(&trusted(150));
    assert!(expired.queue.is_empty());
    assert_eq!(expired.excluded[&id("r")], ReminderReason::RuleExpired);
    assert_eq!(queued(&plan(&trusted(149))), ["r"]);
    for fresh in [
        clock(Some(150), ClockTrust::Uncertain),
        clock(None, ClockTrust::Trusted),
    ] {
        let uncertain = plan(&fresh);
        assert_eq!(uncertain.queue.len(), 1);
        let item = &uncertain.queue[0];
        assert_eq!(item.strength(), ReminderStrength::Informational);
        let ReminderKind::CannotEvaluate { reasons } = &item.kind else {
            panic!("expected cannot-evaluate");
        };
        assert_eq!(reasons[0].reason, Reason::ClockUncertain);
        assert_eq!(reasons[0].input, InputPath::RuleExpiry);
    }
}

fn permutations(items: Vec<Value>) -> Vec<Vec<Value>> {
    if items.len() <= 1 {
        return vec![items];
    }
    let mut out = vec![];
    for i in 0..items.len() {
        let mut rest = items.clone();
        let head = rest.remove(i);
        for mut tail in permutations(rest) {
            tail.insert(0, head.clone());
            out.push(tail);
        }
    }
    out
}

#[test]
fn ordering_is_priority_then_rule_id_and_independent_of_input_order() {
    let values = vec![
        rule_json("b", 5, "kb", is_true()),
        rule_json("a", 5, "ka", is_true()),
        rule_json("c", 9, "kc", is_true()),
        rule_json("d", -3, "kd", is_true()),
    ];
    let mut plans = vec![];
    for order in permutations(values) {
        plans.push(plan_clear(&rules(order), &ReminderState::new()));
    }
    assert_eq!(plans.len(), 24);
    assert_eq!(queued(&plans[0]), ["c", "a", "b", "d"]);
    assert!(plans.iter().all(|p| p == &plans[0]));
    let json = serde_json::to_string(&plans[0]).unwrap();
    assert!(
        plans
            .iter()
            .all(|p| serde_json::to_string(p).unwrap() == json)
    );
}

#[test]
fn advice_ranks_before_informational_regardless_of_priority() {
    let engine = ReminderEngine::new(ReminderPolicy {
        unknown: UnknownPolicy::Informational,
        ..ReminderPolicy::default()
    });
    let set = rules(vec![
        rule_json("loud_unknown", 1000, "x", is_unknown()),
        rule_json("quiet_advice", -1000, "y", is_true()),
    ]);
    let plan = run(
        &engine,
        &set,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &ReminderState::new(),
    )
    .unwrap();
    assert_eq!(queued(&plan), ["quiet_advice", "loud_unknown"]);
}

#[test]
fn rules_sharing_a_deduplication_key_collapse_to_the_best_ranked() {
    let set = rules(vec![
        rule_json("low", 1, "shared", is_true()),
        rule_json("high", 7, "shared", is_true()),
        rule_json("tie_b", 3, "tie", is_true()),
        rule_json("tie_a", 3, "tie", is_true()),
    ]);
    let plan = plan_clear(&set, &ReminderState::new());
    assert_eq!(queued(&plan), ["high", "tie_a"]);
    assert_eq!(
        plan.excluded[&id("low")],
        ReminderReason::Deduplicated { kept: id("high") }
    );
    assert_eq!(
        plan.excluded[&id("tie_b")],
        ReminderReason::Deduplicated { kept: id("tie_a") }
    );
}

#[test]
fn an_affirmative_rule_beats_an_unknown_one_in_the_same_group() {
    let engine = ReminderEngine::new(ReminderPolicy {
        unknown: UnknownPolicy::Informational,
        ..ReminderPolicy::default()
    });
    let set = rules(vec![
        rule_json("unknown_high", 50, "g", is_unknown()),
        rule_json("true_low", 1, "g", is_true()),
    ]);
    let plan = run(
        &engine,
        &set,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &ReminderState::new(),
    )
    .unwrap();
    assert_eq!(queued(&plan), ["true_low"]);
}

#[test]
fn unchanged_state_is_not_repeated_but_changed_state_is() {
    let set = rules(vec![rule_json("a", 0, "a", is_true())]);
    let mut state = ReminderState::new();
    let first = plan_clear(&set, &state);
    assert_eq!(queued(&first), ["a"]);
    state.commit(&first, &[id("a")], &trusted(150)).unwrap();
    // Same snapshot, same truth, same reasons: no repeat.
    let again = plan_clear(&set, &state);
    assert!(again.queue.is_empty());
    assert_eq!(again.excluded[&id("a")], ReminderReason::StateUnchanged);
    // A new snapshot is new state under the default basis.
    let engine = ReminderEngine::default();
    let newer = run(
        &engine,
        &set,
        &snap("snap2"),
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert_eq!(queued(&newer), ["a"]);
    // The outcome-only basis treats the same outcome on a newer snapshot as unchanged.
    let outcome = ReminderEngine::new(ReminderPolicy {
        fingerprint: FingerprintBasis::OutcomeOnly,
        ..ReminderPolicy::default()
    });
    let mut state = ReminderState::new();
    let first = run(
        &outcome,
        &set,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    state.commit(&first, &[id("a")], &trusted(150)).unwrap();
    let newer = run(
        &outcome,
        &set,
        &snap("snap2"),
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert_eq!(newer.excluded[&id("a")], ReminderReason::StateUnchanged);
    // A definite False re-arms the rule, so True again is a change.
    let off = run(
        &outcome,
        &set,
        &snap_level("snap3", known(json!(5))),
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert!(off.rearm().contains(&id("a")));
    state.commit(&off, &[], &trusted(150)).unwrap();
    let on = run(
        &outcome,
        &set,
        &snap("snap4"),
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert_eq!(queued(&on), ["a"]);
}

#[test]
fn changed_unknown_reasons_change_the_fingerprint() {
    let engine = ReminderEngine::new(ReminderPolicy {
        unknown: UnknownPolicy::Informational,
        fingerprint: FingerprintBasis::OutcomeOnly,
    });
    let set = rules(vec![rule_json("a", 0, "a", is_true())]);
    let mut state = ReminderState::new();
    let level = |status: &str| snap_level("snap1", json!({"status":status}));
    let plan = |level: &CharacterSnapshot, state: &ReminderState| {
        run(&engine, &set, level, &trusted(150), &clear_inputs(), state).unwrap()
    };
    let first = plan(&level("unknown"), &state);
    state.commit(&first, &[id("a")], &trusted(150)).unwrap();
    assert!(plan(&level("unknown"), &state).queue.is_empty());
    let restricted = plan(&level("restricted"), &state);
    assert_eq!(queued(&restricted), ["a"]);
}

#[test]
fn cooldown_blocks_until_elapsed_on_a_trusted_clock_and_otherwise_stays_conservative() {
    let mut value = rule_json("a", 0, "a", is_true());
    value["cooldown_ms"] = json!(1000);
    let set = rules(vec![value]);
    let engine = ReminderEngine::default();
    let at = |snapshot: &str, fresh: &FreshnessContext, state: &ReminderState| {
        run(
            &engine,
            &set,
            &snap(snapshot),
            fresh,
            &clear_inputs(),
            state,
        )
        .unwrap()
    };
    let mut state = ReminderState::new();
    let first = at("s_1", &trusted(2000), &state);
    state.commit(&first, &[id("a")], &trusted(2000)).unwrap();
    // A changed snapshot alone does not beat the cooldown.
    let early = at("s_2", &trusted(2999), &state);
    assert_eq!(early.excluded[&id("a")], ReminderReason::CooldownActive);
    assert_eq!(queued(&at("s_2", &trusted(3000), &state)), ["a"]);
    // Clock reversal, an untrusted clock and a missing clock never re-emit.
    for fresh in [
        trusted(1500),
        clock(Some(9000), ClockTrust::Uncertain),
        clock(None, ClockTrust::Trusted),
    ] {
        let plan = at("s_2", &fresh, &state);
        assert_eq!(
            plan.excluded[&id("a")],
            ReminderReason::CooldownUnverifiable
        );
    }
    // A record made without a trusted clock cannot prove the cooldown elapsed.
    let mut blind = ReminderState::new();
    let first = at("s_1", &clock(None, ClockTrust::Uncertain), &blind);
    assert_eq!(queued(&first), ["a"]);
    blind
        .commit(&first, &[id("a")], &clock(None, ClockTrust::Uncertain))
        .unwrap();
    let later = at("s_2", &trusted(99_999), &blind);
    assert_eq!(
        later.excluded[&id("a")],
        ReminderReason::CooldownUnverifiable
    );
}

#[test]
fn zero_cooldown_does_not_depend_on_the_clock() {
    let set = rules(vec![rule_json("a", 0, "a", is_true())]);
    let engine = ReminderEngine::default();
    let blind = clock(None, ClockTrust::Uncertain);
    let mut state = ReminderState::new();
    let first = run(&engine, &set, &snap("s_1"), &blind, &clear_inputs(), &state).unwrap();
    state.commit(&first, &[id("a")], &blind).unwrap();
    let second = run(&engine, &set, &snap("s_2"), &blind, &clear_inputs(), &state).unwrap();
    assert_eq!(queued(&second), ["a"]);
}

fn with_policy(mut value: Value, policy: &str) -> Value {
    value["dismissal"] = json!(policy);
    value
}

#[test]
fn dismissal_policies_clear_on_exactly_their_own_boundary() {
    let session = rules(vec![with_policy(
        rule_json("s", 0, "s", is_true()),
        "session",
    )]);
    let ruleset = rules(vec![with_policy(
        rule_json("u", 0, "u", is_true()),
        "until_ruleset_changes",
    )]);
    let permanent = rules(vec![with_policy(
        rule_json("p", 0, "p", is_true()),
        "permanent",
    )]);
    let engine = ReminderEngine::default();
    let blocked = |set: &[ContextRule], state: &ReminderState, scope: ReminderScope| {
        let inputs = ReminderInputs {
            scope,
            ..clear_inputs()
        };
        let plan = run(&engine, set, &snap("snap1"), &trusted(150), &inputs, state).unwrap();
        plan.queue.is_empty()
    };
    let now = trusted(150);

    let mut state = ReminderState::new();
    state
        .dismiss(&session[0], &scope(Some("s1"), Some("r1")), &now)
        .unwrap();
    assert!(blocked(&session, &state, scope(Some("s1"), Some("r1"))));
    assert!(blocked(&session, &state, scope(Some("s1"), Some("r2"))));
    assert!(
        blocked(&session, &state, scope(None, None)),
        "unknown session stays dismissed"
    );
    assert!(!blocked(&session, &state, scope(Some("s2"), Some("r1"))));
    let plan = run(
        &engine,
        &session,
        &snap("snap1"),
        &now,
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert_eq!(
        plan.excluded[&id("s")],
        ReminderReason::Dismissed {
            policy: DismissalPolicy::Session
        },
        "the default scope is unknown, so the dismissal still applies"
    );

    let mut state = ReminderState::new();
    state
        .dismiss(&ruleset[0], &scope(Some("s1"), Some("r1")), &now)
        .unwrap();
    assert!(blocked(&ruleset, &state, scope(Some("s2"), Some("r1"))));
    assert!(blocked(&ruleset, &state, scope(Some("s1"), None)));
    assert!(!blocked(&ruleset, &state, scope(Some("s1"), Some("r2"))));

    let mut state = ReminderState::new();
    state
        .dismiss(&permanent[0], &scope(None, None), &now)
        .unwrap();
    for next in [scope(Some("s9"), Some("r9")), scope(None, None)] {
        assert!(blocked(&permanent, &state, next));
    }

    // Restoring (the preference control) re-enables the reminder.
    assert!(state.restore(&permanent[0]));
    assert!(!state.restore(&permanent[0]));
    assert!(!blocked(&permanent, &state, scope(None, None)));
}

#[test]
fn dismissal_needs_the_scope_its_policy_depends_on_and_prunes_inactive_records() {
    let session = rules(vec![with_policy(
        rule_json("s", 0, "s", is_true()),
        "session",
    )]);
    let ruleset = rules(vec![with_policy(
        rule_json("u", 0, "u", is_true()),
        "until_ruleset_changes",
    )]);
    let now = trusted(150);
    let mut state = ReminderState::new();
    let err = state
        .dismiss(&session[0], &scope(None, Some("r1")), &now)
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(
        state
            .dismiss(&ruleset[0], &scope(Some("s1"), None), &now)
            .is_err()
    );
    assert_eq!(state.dismissal_count(), 0);
    state
        .dismiss(&session[0], &scope(Some("s1"), None), &now)
        .unwrap();
    state
        .dismiss(&ruleset[0], &scope(None, Some("r1")), &now)
        .unwrap();
    state.prune_inactive(&scope(Some("s2"), Some("r1")));
    let left: Vec<_> = state.dismissals().map(|d| d.rule.as_str()).collect();
    assert_eq!(left, ["u"]);
    // Only a trusted clock is recorded.
    assert_eq!(
        state.dismissals().next().unwrap().dismissed_at_utc_ms,
        Some(UtcMillis::new(150).unwrap())
    );
    let mut blind = ReminderState::new();
    blind
        .dismiss(
            &session[0],
            &scope(Some("s1"), None),
            &clock(None, ClockTrust::Uncertain),
        )
        .unwrap();
    assert_eq!(blind.dismissals().next().unwrap().dismissed_at_utc_ms, None);
}

#[test]
fn dismissals_are_isolated_by_data_context() {
    let demo = rules(vec![with_policy(
        rule_json("same", 0, "same", is_true()),
        "permanent",
    )]);
    let mut reference = rule_json("same", 0, "same", is_true());
    reference["context"]["namespace"] = json!("reference");
    reference["dismissal"] = json!("permanent");
    let reference = rules(vec![reference]);
    let mut s = snapshot();
    s["context"]["namespace"] = json!("reference");
    s["level"] = known(json!(22));
    let reference_snapshot: CharacterSnapshot = from_value(s).unwrap();
    let mut state = ReminderState::new();
    state
        .dismiss(&demo[0], &scope(None, None), &trusted(150))
        .unwrap();
    let engine = ReminderEngine::default();
    let demo_plan = run(
        &engine,
        &demo,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert!(demo_plan.queue.is_empty());
    let reference_plan = run(
        &engine,
        &reference,
        &reference_snapshot,
        &trusted(150),
        &clear_inputs(),
        &state,
    )
    .unwrap();
    assert_eq!(queued(&reference_plan), ["same"]);
}

fn many(prefix: &str, count: usize, policy: &str) -> Vec<ContextRule> {
    rules(
        (0..count)
            .map(|n| {
                let name = format!("{prefix}{n:04}");
                with_policy(rule_json(&name, 0, &name, is_true()), policy)
            })
            .collect(),
    )
}

#[test]
fn dismissal_history_is_bounded_and_evicts_oldest_non_permanent_first() {
    let now = trusted(150);
    let scope = scope(Some("s1"), None);
    let set = many("d", 1025, "session");
    let mut state = ReminderState::new();
    for rule in &set[..1024] {
        state.dismiss(rule, &scope, &now).unwrap();
    }
    assert_eq!(state.dismissal_count(), 1024);
    state.dismiss(&set[1024], &scope, &now).unwrap();
    assert_eq!(state.dismissal_count(), 1024);
    let names: Vec<_> = state
        .dismissals()
        .map(|d| d.rule.as_str().to_owned())
        .collect();
    assert!(!names.contains(&"d0000".to_owned()));
    assert!(names.contains(&"d1024".to_owned()));
    // Re-dismissing an existing rule never grows the history.
    state.dismiss(&set[500], &scope, &now).unwrap();
    assert_eq!(state.dismissal_count(), 1024);

    // Permanent dismissals are never evicted: a full permanent history refuses more.
    let permanent = many("p", 1025, "permanent");
    let mut state = ReminderState::new();
    for rule in &permanent[..1024] {
        state.dismiss(rule, &scope, &now).unwrap();
    }
    let before = state.clone();
    let err = state.dismiss(&permanent[1024], &scope, &now).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert_eq!(state, before);
    // A single non-permanent entry is the one that gives way.
    let mut state = ReminderState::new();
    for rule in &permanent[..1023] {
        state.dismiss(rule, &scope, &now).unwrap();
    }
    state.dismiss(&set[0], &scope, &now).unwrap();
    state.dismiss(&permanent[1023], &scope, &now).unwrap();
    assert_eq!(state.dismissal_count(), 1024);
    assert!(
        state
            .dismissals()
            .all(|d| d.policy == DismissalPolicy::Permanent)
    );
}

#[test]
fn shown_history_is_bounded() {
    let now = trusted(150);
    let set = many("s", 1024, "session");
    let mut state = ReminderState::new();
    let plan = plan_clear(&set, &state);
    assert_eq!(plan.queue.len(), 1024);
    let all: Vec<RuleId> = plan.queue.iter().map(|i| i.rule.clone()).collect();
    state.commit(&plan, &all, &now).unwrap();
    assert_eq!(state.shown_count(), 1024);
    let extra = many("x", 1, "session");
    let plan = plan_clear(&extra, &state);
    state.commit(&plan, &[id("x0000")], &now).unwrap();
    assert_eq!(state.shown_count(), 1024);
    let shown: Vec<_> = state.shown().map(|r| r.rule.as_str().to_owned()).collect();
    assert!(!shown.contains(&"s0000".to_owned()));
    assert!(shown.contains(&"x0000".to_owned()));
}

#[test]
fn suppression_known_true_holds_reminders_back_without_dropping_them() {
    let set = rules(vec![
        rule_json("a", 2, "a", is_true()),
        rule_json("b", 1, "b", is_true()),
    ]);
    let engine = ReminderEngine::default();
    for (combat, loading, cutscene) in [
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let inputs = ReminderInputs {
            suppression: suppression(combat, loading, cutscene),
            ..ReminderInputs::default()
        };
        let mut state = ReminderState::new();
        let plan = run(
            &engine,
            &set,
            &snap("snap1"),
            &trusted(150),
            &inputs,
            &state,
        )
        .unwrap();
        assert_eq!(plan.mode, PresentationMode::Suppressed);
        assert!(plan.queue.is_empty());
        assert_eq!(
            plan.held
                .iter()
                .map(|i| i.rule.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(plan.excluded[&id("a")], ReminderReason::Suppressed);
        let selection = plan.select_hud();
        assert!(selection.presented().is_empty());
        // Held items are not presentable, so they cannot be recorded as shown.
        let before = state.clone();
        assert!(state.commit(&plan, &[id("a")], &trusted(150)).is_err());
        assert_eq!(state, before);
    }
    // Known true wins even when the other signals are unknown.
    let mixed: SuppressionState = from_value(json!({"combat":known(json!(true)),"loading":{"status":"unknown"},"cutscene":{"status":"restricted"}})).unwrap();
    let inputs = ReminderInputs {
        suppression: mixed,
        ..ReminderInputs::default()
    };
    let state = ReminderState::new();
    let plan = run(
        &engine,
        &set,
        &snap("snap1"),
        &trusted(150),
        &inputs,
        &state,
    )
    .unwrap();
    assert_eq!(plan.mode, PresentationMode::Suppressed);
    // Once the state is known false, the held reminders come out.
    let released = plan_clear(&set, &state);
    assert_eq!(released.mode, PresentationMode::Normal);
    assert_eq!(queued(&released), ["a", "b"]);
}

#[test]
fn unconfirmed_suppression_state_selects_the_non_intrusive_mode() {
    let set = rules(vec![
        rule_json("a", 2, "a", is_true()),
        rule_json("b", 1, "b", is_true()),
    ]);
    let engine = ReminderEngine::default();
    let stale =
        json!({"status":"known","value":false,"observed_at_utc_ms":0,"clock_trust":"trusted"});
    let cases: Vec<(&str, Value)> = vec![
        ("unknown", json!({"status":"unknown"})),
        ("unsupported", json!({"status":"unsupported"})),
        ("restricted", json!({"status":"restricted"})),
        ("not_applicable", json!({"status":"not_applicable"})),
        ("stale", stale),
        ("uncertain clock", json!({"status":"known","value":false})),
    ];
    for (label, combat) in cases {
        let suppression: SuppressionState = from_value(
            json!({"combat":combat,"loading":known(json!(false)),"cutscene":known(json!(false))}),
        )
        .unwrap();
        let inputs = ReminderInputs {
            suppression,
            ..ReminderInputs::default()
        };
        let plan = run(
            &engine,
            &set,
            &snap("snap1"),
            &trusted(5000),
            &inputs,
            &ReminderState::new(),
        )
        .unwrap();
        assert_eq!(plan.mode, PresentationMode::NonIntrusive, "{label}");
        assert_eq!(plan.queue.len(), 2, "{label}: nothing is dropped");
        let hud = plan.select_hud();
        assert!(hud.primary.is_none(), "{label}");
        assert_eq!(hud.secondary.as_ref().unwrap().rule, id("a"), "{label}");
        assert_eq!(hud.remaining, [id("b")], "{label}");
    }
    // The default input is wholly unknown.
    let plan = run(
        &engine,
        &set,
        &snap("snap1"),
        &trusted(150),
        &ReminderInputs::default(),
        &ReminderState::new(),
    )
    .unwrap();
    assert_eq!(plan.mode, PresentationMode::NonIntrusive);
}

#[test]
fn hud_budget_is_one_primary_and_one_secondary_with_the_rest_retained() {
    let set = rules(vec![
        rule_json("c", 1, "c", is_true()),
        rule_json("a", 4, "a", is_true()),
        rule_json("d", 0, "d", is_true()),
        rule_json("b", 2, "b", is_true()),
    ]);
    let mut state = ReminderState::new();
    let plan = plan_clear(&set, &state);
    assert_eq!(plan.mode, PresentationMode::Normal);
    let hud = plan.select_hud();
    assert_eq!(hud.primary.as_ref().unwrap().rule, id("a"));
    assert_eq!(hud.secondary.as_ref().unwrap().rule, id("b"));
    assert_eq!(hud.remaining, [id("c"), id("d")]);
    assert_eq!(
        queued(&plan),
        ["a", "b", "c", "d"],
        "desktop sees the full queue"
    );
    state
        .commit(&plan, &hud.presented(), &trusted(150))
        .unwrap();
    // Unpresented items stay queued, in the same order.
    let next = plan_clear(&set, &state);
    assert_eq!(queued(&next), ["c", "d"]);
    let hud = next.select_hud();
    assert_eq!(hud.primary.as_ref().unwrap().rule, id("c"));
    assert_eq!(hud.secondary.as_ref().unwrap().rule, id("d"));
    assert!(hud.remaining.is_empty());
}

#[test]
fn commit_rejects_unknown_or_repeated_presentations_without_changing_state() {
    let set = rules(vec![rule_json("a", 0, "a", is_true())]);
    let mut state = ReminderState::new();
    let plan = plan_clear(&set, &state);
    let before = state.clone();
    assert!(state.commit(&plan, &[id("nope")], &trusted(150)).is_err());
    assert!(
        state
            .commit(&plan, &[id("a"), id("a")], &trusted(150))
            .is_err()
    );
    assert_eq!(state, before);
}

#[test]
fn empty_duplicate_and_oversized_rule_sets_are_handled() {
    let empty = plan_clear(&[], &ReminderState::new());
    assert!(empty.queue.is_empty() && empty.held.is_empty() && empty.excluded.is_empty());
    let hud = empty.select_hud();
    assert!(hud.primary.is_none() && hud.secondary.is_none() && hud.remaining.is_empty());

    let duplicate = rules(vec![
        rule_json("a", 0, "k1", is_true()),
        rule_json("a", 1, "k2", is_true()),
    ]);
    let err = run(
        &ReminderEngine::default(),
        &duplicate,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &ReminderState::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);

    let oversized = many("o", 1025, "session");
    let err = run(
        &ReminderEngine::default(),
        &oversized,
        &snap("snap1"),
        &trusted(150),
        &clear_inputs(),
        &ReminderState::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn trigger_filter_skips_unfired_rules_without_evaluating_them() {
    let mut level = rule_json("lvl", 0, "lvl", is_true());
    level["trigger"] = json!("level_changed");
    let mut gear = rule_json("gear", 0, "gear", is_true());
    gear["trigger"] = json!("equipment_changed");
    let set = rules(vec![level, gear]);
    let inputs = ReminderInputs {
        fired_triggers: Some(vec![RuleTrigger::LevelChanged]),
        ..clear_inputs()
    };
    let plan = run(
        &ReminderEngine::default(),
        &set,
        &snap("snap1"),
        &trusted(150),
        &inputs,
        &ReminderState::new(),
    )
    .unwrap();
    assert_eq!(queued(&plan), ["lvl"]);
    assert_eq!(plan.excluded[&id("gear")], ReminderReason::TriggerNotFired);
    assert!(!plan.rearm().contains(&id("gear")));
}

#[test]
fn state_round_trips_and_rejects_inconsistent_records() {
    let session = rules(vec![with_policy(
        rule_json("s", 0, "s", is_true()),
        "session",
    )]);
    let mut state = ReminderState::new();
    state
        .dismiss(&session[0], &scope(Some("s1"), None), &trusted(150))
        .unwrap();
    let plan = plan_clear(&rules(vec![rule_json("a", 0, "a", is_true())]), &state);
    state.commit(&plan, &[id("a")], &trusted(150)).unwrap();
    let value = serde_json::to_value(&state).unwrap();
    let restored: ReminderState = from_value(value.clone()).unwrap();
    assert_eq!(restored, state);
    assert_eq!(serde_json::to_value(&restored).unwrap(), value);

    let mutate = |edit: &dyn Fn(&mut Value)| {
        let mut broken = value.clone();
        edit(&mut broken);
        from_value::<ReminderState>(broken).is_err()
    };
    assert!(mutate(&|v| v["dismissals"][0]["session"] = Value::Null));
    assert!(mutate(&|v| v["dismissals"][0]["serial"] = json!(99)));
    assert!(mutate(&|v| v["next_serial"] = json!(0)));
    assert!(mutate(&|v| {
        let copy = v["dismissals"][0].clone();
        v["dismissals"].as_array_mut().unwrap().push(copy);
    }));
    assert!(mutate(&|v| v["shown"][0]["rule"] = json!("other")));
    assert!(mutate(
        &|v| v["shown"][0]["fingerprint"]["issues"] = json!(["b", "a"])
    ));
    assert!(mutate(&|v| v["unexpected"] = json!(1)));
}
