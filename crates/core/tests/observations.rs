use azeroth_core::{observation::*, types::*};
use serde_json::{from_str, json, to_value};

#[test]
fn known_false_zero_empty_round_trip_and_additive_fields() {
    for value in [json!(false), json!(0), json!([]), json!("")] {
        let obs: Observation<serde_json::Value> =
            serde_json::from_value(json!({"status":"known","value":value,"future_metadata":true}))
                .unwrap();
        assert_eq!(obs.value(), Some(&value));
        assert_eq!(to_value(obs).unwrap()["value"], value);
    }
    assert!(Observation::known(None::<bool>, ObservationMetadata::default()).is_err());
}
#[test]
fn unavailable_states_never_accept_a_value_even_null() {
    for status in ["unknown", "unsupported", "restricted", "not_applicable"] {
        let obs: Observation<bool> = serde_json::from_value(json!({"status":status})).unwrap();
        assert!(obs.value().is_none());
        assert!(to_value(obs).unwrap().get("value").is_none());
        for value in [json!(null), json!(false)] {
            assert!(
                serde_json::from_value::<Observation<bool>>(json!({"status":status,"value":value}))
                    .is_err()
            );
        }
    }
    for s in [
        r#"{"status":"known"}"#,
        r#"{"status":"known","value":null}"#,
        r#"{"status":"invented"}"#,
        r#"{"status":"unknown","sequence":1}"#,
    ] {
        assert!(from_str::<Observation<bool>>(s).is_err());
    }
}
#[test]
fn freshness_obeys_clock_trust_session_and_threshold() {
    let mut meta = ObservationMetadata {
        observed_at_utc_ms: Some(UtcMillis::new(100).unwrap()),
        clock_trust: ClockTrust::Trusted,
        ..Default::default()
    };
    let mut ctx = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(200).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    };
    assert!(matches!(freshness(&meta, &ctx), Freshness::Fresh { .. }));
    ctx.max_age_ms = Millis::new(99).unwrap();
    assert!(matches!(freshness(&meta, &ctx), Freshness::Stale { .. }));
    ctx.now_utc_ms = Some(UtcMillis::new(0).unwrap());
    assert_eq!(
        freshness(&meta, &ctx),
        Freshness::Uncertain {
            reason: ClockUncertainty::ClockReversed
        }
    );
    meta.clock_trust = ClockTrust::Uncertain;
    meta.received_at_utc_ms = Some(UtcMillis::new(0).unwrap());
    assert!(matches!(
        freshness(&meta, &ctx),
        Freshness::Uncertain { .. }
    ));
    meta.session_id = Some(SessionId::new("one").unwrap());
    meta.session_elapsed_ms = Some(Millis::new(20).unwrap());
    ctx.session = Some((SessionId::new("one").unwrap(), Millis::new(30).unwrap()));
    assert!(matches!(freshness(&meta, &ctx), Freshness::Fresh { .. }));
    ctx.session = Some((SessionId::new("two").unwrap(), Millis::new(30).unwrap()));
    assert!(matches!(
        freshness(&meta, &ctx),
        Freshness::Uncertain { .. }
    ));
}
