use azeroth_core::predicate::RuleLimits;
mod common;
use azeroth_core::{entities::*, evaluation::*, gear::*, observation::*, types::*};
use serde_json::{Value, from_value, json};
use std::collections::BTreeSet;
fn ms(n: u64) -> Millis {
    Millis::new(n).unwrap()
}
fn known<T: serde::de::DeserializeOwned>(value: Value) -> Observation<T> {
    from_value(json!({"status":"known","value":value})).unwrap()
}
fn source(p: Value, cost: Value) -> AcquisitionSource {
    from_value(json!({"id":"acq1","context":common::context(),"kind":"quest","items":[{"context":common::context(),"item_id":"item1","variant":"base"}],"conditions":{"status":"not_applicable"},"cost":cost,"probability":p,"evidence":[]})).unwrap()
}
fn effort() -> AcquisitionEffort {
    AcquisitionEffort {
        incremental_travel_ms: known(json!(10)),
        formation_ms: known(json!(5)),
        attempt_ms: known(json!(20)),
        stopping_ms: known(json!(5)),
        balance_basis: BalanceBasis::CallerBudget,
        available_copper: known(json!("100")),
        session_budget_ms: ms(100),
        max_attempts: 10,
        probability_model: ProbabilityModel::IndependentPlayerAttempts,
        cost_schedule: CostSchedule::PerAttempt,
    }
}
fn assess(
    source: &AcquisitionSource,
    effort: &AcquisitionEffort,
    counted: &BTreeSet<ItemKey>,
) -> AcquisitionAssessment {
    let snapshot: CharacterSnapshot = from_value(common::snapshot()).unwrap();
    let freshness = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: ms(100),
    };
    let context = EvaluationContext {
        snapshot: &snapshot,
        party: None,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    assess_acquisition(source, &source.items[0], effort, counted, &context).unwrap()
}
#[test]
fn finite_attempts_include_route_formation_stop_and_gold() {
    let s = source(
        json!({"status":"known","value":0.5}),
        json!({"status":"known","value":"30"}),
    );
    let result = assess(&s, &effort(), &BTreeSet::new());
    assert_eq!(result.status, AcquisitionStatus::Uncertain);
    assert_eq!(result.feasible_attempts, Some(3)); // four fit time; three fit gold
    assert_eq!(result.budgeted_cost_copper.unwrap().get(), 90);
    assert_eq!(result.budgeted_effort_ms, Some(ms(80)));
    assert_eq!(result.success_probability, Some(0.875));
    assert_eq!(result.expected_attempts, Some(2.0));
}
#[test]
fn declared_player_guarantee_and_contested_probability_stay_separate() {
    let s = source(
        json!({"status":"known","value":1}),
        json!({"status":"known","value":"0"}),
    );
    let mut e = effort();
    assert_eq!(
        assess(&s, &e, &BTreeSet::new()).status,
        AcquisitionStatus::Guaranteed
    );
    e.probability_model = ProbabilityModel::DeclaredOnly;
    let result = assess(&s, &e, &BTreeSet::new());
    assert_eq!(result.status, AcquisitionStatus::Uncertain);
    assert_eq!(result.success_probability, None);
    assert_eq!(result.expected_attempts, None);
}
#[test]
fn unknown_status_zero_probability_and_zero_time_are_distinct() {
    let s = source(
        json!({"status":"unknown"}),
        json!({"status":"known","value":"0"}),
    );
    let mut e = effort();
    e.attempt_ms = known(json!(0));
    e.max_attempts = 7;
    let unknown = assess(&s, &e, &BTreeSet::new());
    assert_eq!(unknown.feasible_attempts, Some(7));
    assert_eq!(unknown.success_probability, None);
    let zero = source(
        json!({"status":"known","value":0}),
        json!({"status":"known","value":"0"}),
    );
    assert_eq!(
        assess(&zero, &e, &BTreeSet::new()).status,
        AcquisitionStatus::Unavailable
    );
    for status in ["unknown", "restricted", "unsupported"] {
        e.incremental_travel_ms = from_value(json!({"status":status})).unwrap();
        let out = assess(&s, &e, &BTreeSet::new());
        assert_eq!(out.feasible_attempts, None);
        assert!(out.reasons.iter().any(|reason| matches!(
            reason,
            AcquisitionReason::FieldUnavailable {
                field: EffortField::Travel,
                ..
            }
        )));
        assert_eq!(out.effort.incremental_travel_ms, e.incremental_travel_ms);
    }
}
#[test]
fn no_double_credit_and_insufficient_budget_never_guaranteed() {
    let s = source(
        json!({"status":"known","value":1}),
        json!({"status":"known","value":"0"}),
    );
    let result = assess(&s, &effort(), &BTreeSet::from([s.items[0].clone()]));
    assert_eq!(result.status, AcquisitionStatus::Unavailable);
    assert!(result.credited_reward.is_none());
    let mut e = effort();
    e.session_budget_ms = ms(30);
    let result = assess(&s, &e, &BTreeSet::new());
    assert_eq!(result.feasible_attempts, Some(0));
    assert_eq!(result.status, AcquisitionStatus::Unavailable);
}
#[test]
fn unknown_conditions_and_cost_never_become_guaranteed() {
    let s:AcquisitionSource=from_value(json!({"id":"acq1","context":common::context(),"kind":"vendor","items":[{"context":common::context(),"item_id":"item1","variant":"base"}],"conditions":{"status":"restricted"},"cost":{"status":"unknown"},"probability":{"status":"known","value":1},"evidence":[]})).unwrap();
    let out = assess(&s, &effort(), &BTreeSet::new());
    assert_eq!(out.status, AcquisitionStatus::Uncertain);
    assert_eq!(out.feasible_attempts, None);
    assert!(
        out.reasons
            .iter()
            .any(|r| matches!(r, AcquisitionReason::RequirementsUnknown))
    );
}

#[test]
fn one_time_cost_and_known_false_requirements_are_explicit() {
    let s = source(
        json!({"status":"known","value":0.5}),
        json!({"status":"known","value":"60"}),
    );
    let mut e = effort();
    e.cost_schedule = CostSchedule::Once;
    let result = assess(&s, &e, &BTreeSet::new());
    assert_eq!(result.feasible_attempts, Some(4));
    assert_eq!(result.budgeted_cost_copper.unwrap().get(), 60);
    let mut value = serde_json::to_value(&s).unwrap();
    value["conditions"] = json!({"status":"known","value":{"op":"any","predicates":[]}});
    let blocked = assess(&from_value(value).unwrap(), &e, &BTreeSet::new());
    assert_eq!(blocked.status, AcquisitionStatus::Unavailable);
    assert!(blocked.credited_reward.is_none());
    assert_eq!(blocked.reasons, vec![AcquisitionReason::RequirementsNotMet]);
}

#[test]
fn captured_money_requires_freshness_and_insufficient_funds_are_unavailable() {
    let s = source(
        json!({"status":"known","value":1}),
        json!({"status":"known","value":"30"}),
    );
    let mut e = effort();
    e.balance_basis = BalanceBasis::ObservedBalance;
    e.available_copper = from_value(
        json!({"status":"known","value":"100","observed_at_utc_ms":1,"clock_trust":"trusted"}),
    )
    .unwrap();
    let stale = assess(&s, &e, &BTreeSet::new());
    assert_eq!(stale.status, AcquisitionStatus::Uncertain);
    assert_eq!(stale.feasible_attempts, None);
    assert!(stale.reasons.iter().any(|r| matches!(
        r,
        AcquisitionReason::BalanceNotFresh {
            freshness: Freshness::Stale { .. }
        }
    )));
    e.available_copper = from_value(
        json!({"status":"known","value":"100","observed_at_utc_ms":100,"clock_trust":"trusted"}),
    )
    .unwrap();
    assert_eq!(
        assess(&s, &e, &BTreeSet::new()).status,
        AcquisitionStatus::Guaranteed
    );
    e.available_copper = from_value(
        json!({"status":"known","value":"10","observed_at_utc_ms":100,"clock_trust":"trusted"}),
    )
    .unwrap();
    let poor = assess(&s, &e, &BTreeSet::new());
    assert_eq!(poor.status, AcquisitionStatus::Unavailable);
    assert_eq!(poor.feasible_attempts, Some(0));
    assert!(poor.credited_reward.is_none());
    assert!(
        poor.reasons
            .contains(&AcquisitionReason::InsufficientCopper)
    );
}
