//! M5.2: three-way gear eligibility. CASE-005 (edition/variant isolation),
//! CASE-015 (demo vs reference namespace) and the section 18.1 gear rows.
//! All identifiers are SYNTHETIC; nothing here is game data.
mod common;
use azeroth_core::{
    entities::CharacterSnapshot, error::ErrorCode, evaluation::*, gear::*, observation::*,
    predicate::RuleLimits, types::*,
};
use common::*;
use serde_json::{Value, from_value, json};

fn known(v: Value) -> Value {
    json!({"status":"known","value":v})
}
fn observed(v: Value) -> Value {
    json!({"status":"known","value":v,"observed_at_utc_ms":100,"clock_trust":"trusted","source":"synthetic_fixture"})
}
fn na() -> Value {
    json!({"status":"not_applicable"})
}
fn item_with(
    id: &str,
    variant: &str,
    ctx: Value,
    requirements: Value,
    category: Value,
) -> GearCandidate {
    from_value(json!({"item":{
        "key":{"context":ctx,"item_id":id,"variant":variant},
        "slots":["synthetic-slot"],"two_handed":na(),"unique_group":na(),
        "attributes":known(json!({"synthetic-power":1})),
        "requirements":requirements,
        "effects":known(json!([])),"evidence":[]},
        "category":category}))
    .unwrap()
}
fn gated(id: &str, requirement: Value) -> GearCandidate {
    item_with(id, "base", context(), known(requirement), na())
}
fn categorized(id: &str, category: Value) -> GearCandidate {
    item_with(
        id,
        "base",
        context(),
        known(json!({"op":"all","predicates":[]})),
        category,
    )
}
fn snapshot_at(level: Value) -> CharacterSnapshot {
    let mut s = snapshot();
    s["level"] = level;
    from_value(s).unwrap()
}
fn clock() -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    }
}
fn table() -> ProficiencyTable {
    from_value(json!({"classes":{
        "synthetic-class-a":{"allowed":["synthetic-heavy"],"disallowed":["synthetic-light"]},
        "synthetic-class-b":{"allowed":["synthetic-light"],"disallowed":[]}}}))
    .unwrap()
}
fn class(name: &str) -> Observation<SourceId> {
    from_value(known(json!(name))).unwrap()
}
fn assess(
    candidate: &GearCandidate,
    snapshot: &CharacterSnapshot,
    class: &Observation<SourceId>,
    policy: StalePolicy,
) -> azeroth_core::error::Result<GearEligibility> {
    let clock = clock();
    let evaluation = EvaluationContext {
        snapshot,
        party: None,
        freshness: &clock,
        stale_policy: policy,
        limits: RuleLimits::default(),
    };
    let table = table();
    assess_candidate(
        candidate,
        &GearEligibilityContext {
            evaluation: &evaluation,
            class,
            proficiency: &table,
        },
    )
}
fn assess_simple(candidate: &GearCandidate, level: Value) -> GearEligibility {
    assess(
        candidate,
        &snapshot_at(level),
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap()
}
fn level_gate(level: u16) -> Value {
    json!({"op":"level_at_least","level":level})
}

#[test]
fn level_requirement_above_the_character_is_future_progression() {
    let item = gated("synthetic-late", level_gate(20));
    assert_eq!(
        assess_simple(&item, observed(json!(10))),
        GearEligibility::FutureProgression {
            required_level: Level::new(20).unwrap()
        }
    );
    // Reaching the level makes it eligible; the boundary is inclusive.
    assert_eq!(
        assess_simple(&item, observed(json!(20))),
        GearEligibility::Eligible
    );
    assert_eq!(
        assess_simple(&item, observed(json!(19))),
        GearEligibility::FutureProgression {
            required_level: Level::new(20).unwrap()
        }
    );
}

#[test]
fn unknown_snapshot_level_is_unknown_never_eligible() {
    let item = gated("synthetic-late", level_gate(20));
    let outcome = assess_simple(&item, unknown());
    let GearEligibility::Unknown { reasons } = outcome else {
        panic!("expected Unknown, got {outcome:?}");
    };
    assert!(reasons.iter().any(|r| matches!(
        r,
        UnknownReason::Requirement { issue }
            if issue.reason == Reason::Unavailable { status: ObservationStatus::Unknown }
    )));
}

#[test]
fn stale_level_is_unknown_unless_staleness_is_allowed() {
    let item = gated("synthetic-late", level_gate(20));
    let mut stale = snapshot();
    stale["level"] = json!({"status":"known","value":30,"observed_at_utc_ms":0,"clock_trust":"trusted","source":"synthetic_fixture"});
    let snap: CharacterSnapshot = from_value(stale).unwrap();
    let rejected = assess(
        &item,
        &snap,
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap();
    assert!(matches!(
        rejected,
        GearEligibility::Unknown { ref reasons }
            if reasons.iter().any(|r| matches!(r, UnknownReason::Requirement { issue } if issue.reason == Reason::Stale))
    ));
    let allowed = assess(
        &item,
        &snap,
        &class("synthetic-class-a"),
        StalePolicy::AllowStale,
    )
    .unwrap();
    assert_eq!(allowed, GearEligibility::Eligible);
}

#[test]
fn requirements_that_are_not_known_leave_the_item_unknown() {
    for status in ["unknown", "unsupported", "restricted", "not_applicable"] {
        let item = item_with(
            "synthetic-opaque",
            "base",
            context(),
            json!({"status":status}),
            na(),
        );
        let outcome = assess_simple(&item, observed(json!(60)));
        assert!(
            matches!(&outcome, GearEligibility::Unknown { reasons }
                if matches!(reasons.as_slice(), [UnknownReason::RequirementsUnavailable { .. }])),
            "{status}: {outcome:?}"
        );
    }
}

#[test]
fn a_requirement_that_is_not_only_a_level_gate_is_ineligible_not_future() {
    let role = json!({"op":"role_is","role":"synthetic-other-role"});
    let mut s = snapshot();
    s["level"] = observed(json!(10));
    s["role"] = observed(json!("synthetic-role"));
    let snap: CharacterSnapshot = from_value(s).unwrap();
    let item = gated("synthetic-role-gated", role);
    let outcome = assess(
        &item,
        &snap,
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap();
    assert_eq!(
        outcome,
        GearEligibility::Ineligible {
            reasons: vec![IneligibleReason::RequirementNotMet]
        }
    );
    // A level gate mixed with another condition is not pure future progression.
    let mixed = gated(
        "synthetic-mixed",
        json!({"op":"all","predicates":[level_gate(20),{"op":"role_is","role":"synthetic-role"}]}),
    );
    let outcome = assess(
        &mixed,
        &snap,
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap();
    assert_eq!(
        outcome,
        GearEligibility::Ineligible {
            reasons: vec![IneligibleReason::RequirementNotMet]
        }
    );
    // A conjunction of level gates reports the highest one.
    let both = gated(
        "synthetic-two-levels",
        json!({"op":"all","predicates":[level_gate(15),level_gate(25)]}),
    );
    assert_eq!(
        assess(
            &both,
            &snap,
            &class("synthetic-class-a"),
            StalePolicy::Reject
        )
        .unwrap(),
        GearEligibility::FutureProgression {
            required_level: Level::new(25).unwrap()
        }
    );
}

#[test]
fn missing_proficiency_is_ineligible_and_distinct_from_unknown() {
    let level = observed(json!(60));
    let light = categorized("synthetic-light-item", known(json!("synthetic-light")));
    let heavy = categorized("synthetic-heavy-item", known(json!("synthetic-heavy")));
    let odd = categorized("synthetic-odd-item", known(json!("synthetic-unmapped")));

    assert_eq!(
        assess_simple(&heavy, level.clone()),
        GearEligibility::Eligible
    );
    let ineligible = assess_simple(&light, level.clone());
    assert_eq!(
        ineligible,
        GearEligibility::Ineligible {
            reasons: vec![IneligibleReason::CategoryNotProficient {
                class: SourceId::new("synthetic-class-a").unwrap(),
                category: SourceId::new("synthetic-light").unwrap(),
            }]
        }
    );
    // An unmapped category is unknown, not a refusal.
    let unmapped = assess_simple(&odd, level.clone());
    assert!(matches!(
        &unmapped,
        GearEligibility::Unknown { reasons }
            if matches!(reasons.as_slice(), [UnknownReason::CategoryNotInProficiencyTable { .. }])
    ));
    assert_ne!(
        std::mem::discriminant(&ineligible),
        std::mem::discriminant(&unmapped)
    );
}

#[test]
fn unknown_class_or_category_stays_unknown_and_not_applicable_needs_no_check() {
    let snap = snapshot_at(observed(json!(60)));
    let heavy = categorized("synthetic-heavy-item", known(json!("synthetic-heavy")));
    let unknown_class: Observation<SourceId> = from_value(unknown()).unwrap();
    let restricted_class: Observation<SourceId> =
        from_value(json!({"status":"restricted"})).unwrap();
    for (class, status) in [
        (&unknown_class, ObservationStatus::Unknown),
        (&restricted_class, ObservationStatus::Restricted),
    ] {
        let outcome = assess(&heavy, &snap, class, StalePolicy::Reject).unwrap();
        assert_eq!(
            outcome,
            GearEligibility::Unknown {
                reasons: vec![UnknownReason::ClassUnavailable { status }]
            }
        );
    }
    // A class that is absent from the supplied table is unknown, not ineligible.
    let outcome = assess(
        &heavy,
        &snap,
        &class("synthetic-class-z"),
        StalePolicy::Reject,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        GearEligibility::Unknown { ref reasons }
            if matches!(reasons.as_slice(), [UnknownReason::ClassNotInProficiencyTable { .. }])
    ));
    // An unavailable category is unknown; an unknown class does not matter when
    // the category is not applicable (for example jewelry).
    let unknown_category = categorized("synthetic-unknown-category", unknown());
    assert!(matches!(
        assess_simple(&unknown_category, observed(json!(60))),
        GearEligibility::Unknown { ref reasons }
            if matches!(reasons.as_slice(), [UnknownReason::CategoryUnavailable { status: ObservationStatus::Unknown }])
    ));
    let jewelry = categorized("synthetic-jewelry", na());
    assert_eq!(
        assess(&jewelry, &snap, &unknown_class, StalePolicy::Reject).unwrap(),
        GearEligibility::Eligible
    );
}

#[test]
fn a_definite_negative_dominates_unknown_findings() {
    // Not proficient AND level not reached: definitely not usable now, and both
    // findings are reported.
    let item = item_with(
        "synthetic-both",
        "base",
        context(),
        known(level_gate(50)),
        known(json!("synthetic-light")),
    );
    let outcome = assess_simple(&item, observed(json!(10)));
    assert_eq!(
        outcome,
        GearEligibility::Ineligible {
            reasons: vec![
                IneligibleReason::CategoryNotProficient {
                    class: SourceId::new("synthetic-class-a").unwrap(),
                    category: SourceId::new("synthetic-light").unwrap(),
                },
                IneligibleReason::LevelNotReached {
                    required: Level::new(50).unwrap()
                },
            ]
        }
    );
    // A level gate plus an unknown category is ineligible now, not eligible.
    let unknown_category = item_with(
        "synthetic-gated-unknown",
        "base",
        context(),
        known(level_gate(50)),
        unknown(),
    );
    assert!(matches!(
        assess_simple(&unknown_category, observed(json!(10))),
        GearEligibility::Ineligible { .. }
    ));
}

#[test]
fn case_005_candidate_from_another_build_is_a_build_mismatch() {
    let mut other_build = context();
    other_build["build"]["build"] = json!("SYNTHETIC-OTHER");
    let foreign = item_with(
        "synthetic-shared-id",
        "base",
        other_build,
        known(json!({"op":"all","predicates":[]})),
        na(),
    );
    let snap = snapshot_at(observed(json!(60)));
    let err = assess(
        &foreign,
        &snap,
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);

    let mut other_edition = context();
    other_edition["build"]["edition"] = json!("wow_other");
    let foreign = item_with(
        "synthetic-shared-id",
        "base",
        other_edition,
        known(json!({"op":"all","predicates":[]})),
        na(),
    );
    let err = assess(
        &foreign,
        &snap,
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
}

#[test]
fn case_015_demo_candidate_with_a_reference_snapshot_is_rejected() {
    // Reference-namespace data is built in memory only, never committed.
    let mut reference_context = context();
    reference_context["namespace"] = json!("reference");
    let mut s = snapshot();
    s["context"] = reference_context;
    s["level"] = observed(json!(60));
    let reference_snapshot: CharacterSnapshot = from_value(s).unwrap();
    let demo_item = gated("synthetic-demo-item", json!({"op":"all","predicates":[]}));
    let err = assess(
        &demo_item,
        &reference_snapshot,
        &class("synthetic-class-a"),
        StalePolicy::Reject,
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
}

#[test]
fn same_item_id_with_different_variants_is_assessed_separately() {
    let low = item_with(
        "synthetic-shared-id",
        "low",
        context(),
        known(level_gate(5)),
        na(),
    );
    let high = item_with(
        "synthetic-shared-id",
        "high",
        context(),
        known(level_gate(30)),
        na(),
    );
    let snap = snapshot_at(observed(json!(10)));
    let clock = clock();
    let evaluation = EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let table = table();
    let class = class("synthetic-class-a");
    let ctx = GearEligibilityContext {
        evaluation: &evaluation,
        class: &class,
        proficiency: &table,
    };
    let forward = assess_candidates(&[low.clone(), high.clone()], &ctx).unwrap();
    let backward = assess_candidates(&[high.clone(), low.clone()], &ctx).unwrap();
    assert_eq!(forward, backward);
    assert_eq!(forward.len(), 2);
    assert_eq!(forward[low.key()], GearEligibility::Eligible);
    assert!(matches!(
        forward[high.key()],
        GearEligibility::FutureProgression { .. }
    ));
    // The same full key twice is an error rather than a silent merge.
    assert_eq!(
        assess_candidates(&[low.clone(), low], &ctx)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn evaluator_errors_propagate_and_are_not_folded_into_unknown() {
    let item = gated("synthetic-late", level_gate(20));
    let snap = snapshot_at(observed(json!(60)));
    let clock = clock();
    let evaluation = EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::new(1, 1, 1).unwrap(),
    };
    let table = table();
    let class = class("synthetic-class-a");
    let result = assess_candidate(
        &item,
        &GearEligibilityContext {
            evaluation: &evaluation,
            class: &class,
            proficiency: &table,
        },
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::InvalidInput);
}

#[test]
fn proficiency_table_rejects_a_category_that_is_both_allowed_and_disallowed() {
    let bad = from_value::<ProficiencyTable>(json!({"classes":{
        "synthetic-class-a":{"allowed":["synthetic-heavy"],"disallowed":["synthetic-heavy"]}}}));
    assert!(bad.is_err());
}
