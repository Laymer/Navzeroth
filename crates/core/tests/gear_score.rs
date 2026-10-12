//! M5.1/M5.3: profile structure and fixed-point model score breakdowns.
//! All weights here are SYNTHETIC engineering assumptions, not game truth.
use azeroth_core::{error::ErrorCode, gear::*, observation::ObservationStatus, types::*};
use serde_json::{Value, from_value, json};

fn context() -> Value {
    json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"})
}
fn known(v: Value) -> Value {
    json!({"status":"known","value":v})
}
fn na() -> Value {
    json!({"status":"not_applicable"})
}
fn unknown() -> Value {
    json!({"status":"unknown"})
}
fn profile_json(weights: Value) -> Value {
    json!({"role":"synthetic-role","version":"gear-model-v1","weight_source":{"type":"synthetic_test"},"weights":weights,"known_unmodeled":["synthetic set bonuses"]})
}
fn profile(weights: Value) -> RoleProfile {
    from_value(profile_json(weights)).unwrap()
}
fn candidate(id: &str, attributes: Value, effects: Value) -> GearCandidate {
    from_value(json!({"item":{
        "key":{"context":context(),"item_id":id,"variant":"base"},
        "slots":["synthetic-slot"],"two_handed":na(),"unique_group":na(),
        "attributes":attributes,
        "requirements":known(json!({"op":"all","predicates":[]})),
        "effects":effects,"evidence":[]},
        "category":na()}))
    .unwrap()
}
fn plain(id: &str, power: f64) -> GearCandidate {
    candidate(
        id,
        known(json!({"synthetic-power":power})),
        known(json!([])),
    )
}

#[test]
fn fixed_point_rounds_half_away_from_zero() {
    // 0.0625 and 62.5 thousandths are exact in binary floating point.
    assert_eq!(
        to_fixed_point(FiniteNumber::new(0.0625).unwrap()).unwrap(),
        63
    );
    assert_eq!(
        to_fixed_point(FiniteNumber::new(-0.0625).unwrap()).unwrap(),
        -63
    );
    assert_eq!(
        to_fixed_point(FiniteNumber::new(1.25).unwrap()).unwrap(),
        1250
    );
    assert_eq!(to_fixed_point(FiniteNumber::new(-0.0).unwrap()).unwrap(), 0);
    // Products of thousandths: exact halves round away from zero.
    assert_eq!(fixed_mul(1, 500).unwrap(), 1);
    assert_eq!(fixed_mul(-1, 500).unwrap(), -1);
    assert_eq!(fixed_mul(1, 499).unwrap(), 0);
    assert_eq!(fixed_mul(-1, 499).unwrap(), 0);
    assert_eq!(fixed_mul(3, 500).unwrap(), 2);
    assert_eq!(fixed_mul(-3, 500).unwrap(), -2);
    assert_eq!(fixed_mul(1500, 2000).unwrap(), 3000);
}

#[test]
fn arithmetic_overflow_is_an_error_not_a_wrap() {
    let huge = FiniteNumber::new(9_000_000_000_000_000.0).unwrap();
    assert_eq!(
        to_fixed_point(huge).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let large = MAX_SAFE_INTEGER as i64;
    assert!(fixed_mul(large, large).is_err());
    let profile = profile(json!({"synthetic-power":9_000_000_000_000_000_i64}));
    let item = plain("synthetic-big", 9_000_000_000_000.0);
    assert_eq!(
        score_item(&profile, &item).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn contributions_sum_exactly_to_the_total() {
    let profile =
        profile(json!({"synthetic-power":1500,"synthetic-haste":-1000,"synthetic-unused":2000}));
    let item = candidate(
        "synthetic-a",
        known(json!({"synthetic-power":10,"synthetic-haste":2.5})),
        known(json!([])),
    );
    let score = score_item(&profile, &item).unwrap();
    let by_attribute: Vec<(&str, i64, i64, i64)> = score
        .contributions
        .iter()
        .map(|c| (c.attribute.as_str(), c.value, c.weight, c.contribution))
        .collect();
    assert_eq!(
        by_attribute,
        vec![
            ("synthetic-haste", 2500, -1000, -2500),
            ("synthetic-power", 10000, 1500, 15000),
            ("synthetic-unused", 0, 2000, 0),
        ]
    );
    assert_eq!(score.total, 12500);
    assert_eq!(
        score.total,
        score
            .contributions
            .iter()
            .map(|c| c.contribution)
            .sum::<i64>()
    );
    assert_eq!(score.certainty, Certainty::Complete);
    assert!(score.certainty_reasons.is_empty());
    assert!(score.unmodeled_effects.is_empty());
}

#[test]
fn configuration_total_is_the_sum_of_item_contributions_after_rounding() {
    // 0.0625 is 63 thousandths (62.5 rounds away from zero); times 0.5 that is
    // 31.5, which rounds to 32 per item. The pair sums to 64, not the 63 a single
    // rounding of the summed value (126 thousandths) would give.
    let profile = profile(json!({"synthetic-power":500}));
    let a = plain("synthetic-a", 0.0625);
    let b = plain("synthetic-b", 0.0625);
    let pair = score_candidates(&profile, &[&a, &b], vec![]).unwrap();
    assert_eq!(pair.contributions[0].value, 126);
    assert_eq!(pair.contributions[0].contribution, 64);
    assert_eq!(pair.total, 64);
    assert_eq!(fixed_mul(126, 500).unwrap(), 63);
    assert_eq!(
        score_item(&profile, &a).unwrap().total + score_item(&profile, &b).unwrap().total,
        pair.total
    );
}

#[test]
fn scoring_does_not_depend_on_candidate_order() {
    let profile = profile(json!({"synthetic-power":1000}));
    let a = candidate(
        "synthetic-a",
        known(json!({"synthetic-power":1.5})),
        known(json!(["synthetic proc a"])),
    );
    let b = candidate("synthetic-b", unknown(), known(json!([])));
    let forward = score_candidates(&profile, &[&a, &b], vec![]).unwrap();
    let backward = score_candidates(&profile, &[&b, &a], vec![]).unwrap();
    assert_eq!(forward, backward);
}

#[test]
fn unknown_attributes_are_low_certainty_and_never_complete() {
    let profile = profile(json!({"synthetic-power":1000}));
    for status in [
        unknown(),
        json!({"status":"unsupported"}),
        json!({"status":"restricted"}),
    ] {
        let item = candidate("synthetic-opaque", status.clone(), known(json!([])));
        let score = score_item(&profile, &item).unwrap();
        assert_eq!(score.certainty, Certainty::Low);
        assert_ne!(score.certainty, Certainty::Complete);
        // The statuses stay distinct in the reasons.
        let expected = match status["status"].as_str().unwrap() {
            "unknown" => ObservationStatus::Unknown,
            "unsupported" => ObservationStatus::Unsupported,
            _ => ObservationStatus::Restricted,
        };
        assert!(score.certainty_reasons.iter().any(|r| matches!(
            r,
            CertaintyReason::AttributesUnavailable { status, .. } if *status == expected
        )));
    }
}

#[test]
fn unknown_or_listed_effects_are_unmodeled_and_reduce_certainty() {
    let profile = profile(json!({"synthetic-power":1000}));
    let attrs = known(json!({"synthetic-power":10}));
    let unknown_effects = candidate("synthetic-x", attrs.clone(), unknown());
    let score = score_item(&profile, &unknown_effects).unwrap();
    assert_eq!(score.certainty, Certainty::Reduced);
    assert!(matches!(
        score.unmodeled_effects.as_slice(),
        [UnmodeledEffect::Unavailable {
            status: ObservationStatus::Unknown,
            ..
        }]
    ));
    // The unknown effect contributes nothing, and the score says so.
    assert_eq!(score.total, 10000);

    let listed = candidate(
        "synthetic-y",
        attrs.clone(),
        known(json!(["synthetic proc"])),
    );
    let score = score_item(&profile, &listed).unwrap();
    assert_eq!(score.certainty, Certainty::Reduced);
    assert!(matches!(
        score.unmodeled_effects.as_slice(),
        [UnmodeledEffect::Listed { text, .. }] if text.as_str() == "synthetic proc"
    ));

    let none = candidate("synthetic-z", attrs, known(json!([])));
    assert_eq!(
        score_item(&profile, &none).unwrap().certainty,
        Certainty::Complete
    );
}

#[test]
fn unweighted_attributes_are_reported_not_silently_dropped() {
    let profile = profile(json!({"synthetic-power":1000}));
    let item = candidate(
        "synthetic-w",
        known(json!({"synthetic-power":4,"synthetic-novel":9})),
        known(json!([])),
    );
    let score = score_item(&profile, &item).unwrap();
    assert_eq!(score.total, 4000);
    assert_eq!(score.certainty, Certainty::Reduced);
    assert!(score.certainty_reasons.iter().any(|r| matches!(
        r,
        CertaintyReason::UnweightedAttribute { attribute, .. } if attribute.as_str() == "synthetic-novel"
    )));
}

#[test]
fn incomplete_scores_are_never_definitively_better() {
    let profile = profile(json!({"synthetic-power":1000}));
    let certain = plain("synthetic-certain", 5.0);
    let certain_low = plain("synthetic-certain-low", 3.0);
    // Higher modelled total, but an unknown effect (counted as zero in the sum).
    let uncertain = candidate(
        "synthetic-uncertain",
        known(json!({"synthetic-power":10})),
        unknown(),
    );
    let a = score_item(&profile, &certain).unwrap();
    let b = score_item(&profile, &certain_low).unwrap();
    let u = score_item(&profile, &uncertain).unwrap();
    assert!(a.definitively_exceeds(&b));
    assert!(!b.definitively_exceeds(&a));
    assert!(u.total > a.total);
    assert!(!u.definitively_exceeds(&a));
    assert!(!a.definitively_exceeds(&u));
}

#[test]
fn breakdown_is_labelled_as_a_model_score_with_its_version_and_source() {
    let profile = profile(json!({"synthetic-power":1000}));
    let score = score_item(&profile, &plain("synthetic-a", 1.0)).unwrap();
    assert_eq!(score.model_version, profile.version);
    assert_eq!(score.weight_source, WeightSource::SyntheticTest);
    assert_eq!(score.profile_unmodeled.len(), 1);
    let text = serde_json::to_string(&score).unwrap();
    assert!(text.contains("\"label\":\"model score\""));
    assert!(text.contains("\"model_version\":\"gear-model-v1\""));
    assert!(!text.contains('%'));
    assert!(!text.to_lowercase().contains("best in slot"));
}

#[test]
fn profile_structure_is_validated() {
    let ok = profile_json(json!({"synthetic-power":1000}));
    assert!(from_value::<RoleProfile>(ok.clone()).is_ok());

    let mut other_version = ok.clone();
    other_version["version"] = json!("gear-model-v2");
    assert!(from_value::<RoleProfile>(other_version).is_err());

    assert!(from_value::<RoleProfile>(profile_json(json!({}))).is_err());

    let mut duplicate_notes = ok.clone();
    duplicate_notes["known_unmodeled"] = json!(["same", "same"]);
    assert!(from_value::<RoleProfile>(duplicate_notes).is_err());

    // A fractional weight is not representable: weights are integer thousandths.
    assert!(from_value::<RoleProfile>(profile_json(json!({"synthetic-power":1.5}))).is_err());

    for source in [
        json!({"type":"user_preference"}),
        json!({"type":"documented_model","source":"synthetic-source"}),
        json!({"type":"permitted_guide","source":"synthetic-source"}),
        json!({"type":"synthetic_test"}),
    ] {
        let mut labelled = ok.clone();
        labelled["weight_source"] = source.clone();
        let parsed: RoleProfile = from_value(labelled).unwrap();
        assert_eq!(serde_json::to_value(&parsed.weight_source).unwrap(), source);
    }
    let mut unlabelled = ok;
    unlabelled["weight_source"] = json!({"type":"measured_truth"});
    assert!(from_value::<RoleProfile>(unlabelled).is_err());
}
