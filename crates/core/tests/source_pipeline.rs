use azeroth_core::{entities::Confidence, source_admission::Origin, source_pipeline::*};
use serde_json::json;
fn origin(edition: &str, build: Option<&str>) -> Origin {
    Origin {
        edition: Some(edition.into()),
        build: build.map(str::to_owned),
    }
}
fn target() -> Origin {
    origin("wow_forever", Some("1.60.1.70374"))
}
fn candidate(
    id: &str,
    time: Option<i64>,
    confidence: Confidence,
    value: serde_json::Value,
) -> Candidate {
    Candidate {
        source_id: id.into(),
        source_revision: "a".repeat(40),
        source_file: "facts.json".into(),
        source_sha256: "b".repeat(64),
        origin: target(),
        fact: Fact {
            domain: "quests".into(),
            record: "quest/1".into(),
            field: "shareable".into(),
            datum: Datum::Known { value },
            confidence,
            observed_at_utc_ms: time,
            clock_trust: azeroth_core::observation::ClockTrust::Trusted,
        },
    }
}
#[test]
fn reliability_precedes_freshness_and_freshness_precedes_lexical_ids() {
    let old = candidate("a", Some(1), Confidence::VerifiedOnBuild, json!(false));
    let new = candidate("z", Some(2), Confidence::VerifiedOnBuild, json!(false));
    let weaker = candidate("0", Some(3), Confidence::LimitedSample, json!(true));
    let rows = vec![old, new, weaker];
    let catalog = converge(target(), 10, rows.clone()).unwrap();
    assert_eq!(catalog.entries[0].candidates[0].source_id, "z");
    assert_eq!(catalog.entries[0].candidates.len(), 3);
    assert!(catalog.entries[0].conflict);
    assert!(!catalog.entries[0].usable_for_target);
    let mut reversed = rows;
    reversed.reverse();
    assert_eq!(
        serde_json::to_vec(&catalog).unwrap(),
        serde_json::to_vec(&converge(target(), 10, reversed).unwrap()).unwrap()
    );
    assert!(
        catalog
            .domains
            .iter()
            .all(|d| d.required_inventory.is_none())
    );
    assert!(!catalog.publication_authorized);
}
#[test]
fn different_and_unknown_origins_never_join_or_promote() {
    let live = candidate("live", Some(1), Confidence::VerifiedOnBuild, json!(1));
    let mut classic = candidate(
        "classic",
        Some(9),
        Confidence::PublisherDocumented,
        json!(2),
    );
    classic.origin = origin("wow_classic", Some("1.12.1"));
    let mut old = candidate("old", Some(9), Confidence::PublisherDocumented, json!(3));
    old.origin = origin("wow_forever", Some("1.60.1.70338"));
    let mut unknown = candidate(
        "unknown",
        None,
        Confidence::CrossEditionUnverified,
        json!(4),
    );
    unknown.origin.build = None;
    let mut another = unknown.clone();
    another.source_id = "another".into();
    let catalog = converge(target(), 10, vec![live, classic, old, unknown, another]).unwrap();
    assert_eq!(catalog.entries.len(), 5);
    assert_eq!(
        catalog
            .entries
            .iter()
            .filter(|e| e.usable_for_target)
            .count(),
        1
    );
}
#[test]
fn known_false_zero_and_empty_are_not_unavailable() {
    for value in [json!(false), json!(0), json!([]), json!("")] {
        let row = candidate(
            "source",
            Some(0),
            Confidence::PublisherDocumented,
            value.clone(),
        );
        let catalog = converge(target(), 0, vec![row.clone(), row]).unwrap();
        assert_eq!(catalog.entries[0].candidates.len(), 1);
        assert!(catalog.entries[0].usable_for_target);
        assert_eq!(
            catalog.entries[0].candidates[0].fact.datum,
            Datum::Known { value }
        );
    }
    for datum in [
        Datum::Unknown,
        Datum::Unsupported,
        Datum::Restricted,
        Datum::Stale {
            last_value: Some(json!(0)),
        },
        Datum::NotApplicable,
    ] {
        let mut row = candidate("source", None, Confidence::PublisherDocumented, json!(1));
        row.fact.datum = datum;
        assert!(!converge(target(), 1, vec![row]).unwrap().entries[0].usable_for_target);
    }
}
#[test]
fn stale_values_never_outrank_current_evidence() {
    let good = candidate("old", Some(1), Confidence::LimitedSample, json!(0));
    let mut stale = candidate("new", Some(9), Confidence::VerifiedOnBuild, json!(1));
    stale.fact.datum = Datum::Stale {
        last_value: Some(json!(1)),
    };
    let catalog = converge(target(), 10, vec![stale, good]).unwrap();
    assert_eq!(catalog.entries[0].candidates[0].source_id, "old");
}
#[test]
fn strict_documents_and_dates_refuse_invalid_claims() {
    assert!(Document::parse(br#"{"schema_version":2,"facts":[]}"#, 10).is_err());
    assert!(Document::parse(br#"{"schema_version":1,"schema_version":1,"facts":[]}"#, 10).is_err());
    assert!(Document::parse(br#"{"schema_version":1,"facts":[],"allow":true}"#, 10).is_err());
    assert!(Document::parse(&vec![b' '; MAX_DOCUMENT_BYTES + 1], 10).is_err());
    assert!(Document::parse(br#"{"schema_version":1,"facts":[{"domain":"quests","record":"q","field":"f","datum":{"status":"known","value":{"a":1,"a":2}},"confidence":"publisher_documented","observed_at_utc_ms":null}]}"#, 10).is_err());
    let excessive = candidate(
        "source",
        None,
        Confidence::PublisherDocumented,
        json!(9_007_199_254_740_992_u64),
    );
    assert!(excessive.fact.validate(10).is_err());
    let row = candidate("source", Some(11), Confidence::VerifiedOnBuild, json!(1));
    assert!(converge(target(), 10, vec![row]).is_err());
    let mut row = candidate("source", Some(0), Confidence::VerifiedOnBuild, json!(1));
    row.origin.build = None;
    assert!(converge(target(), 10, vec![row]).is_err());
    let mut row = candidate("source", None, Confidence::PublisherDocumented, json!(null));
    assert!(row.fact.validate(10).is_err());
    row.fact.datum = Datum::Known {
        value: json!("x".repeat(65537)),
    };
    assert!(row.fact.validate(10).is_err());
}
#[test]
fn source_identity_collision_refuses_and_large_catalogs_are_not_truncated() {
    let a = candidate(
        "source",
        None,
        Confidence::PublisherDocumented,
        json!(false),
    );
    let mut b = a.clone();
    b.fact.datum = Datum::Known { value: json!(true) };
    assert!(converge(target(), 10, vec![a, b]).is_err());
    let rows = (0..1100)
        .map(|i| {
            let mut row = candidate("source", None, Confidence::PublisherDocumented, json!(0));
            row.fact.record = format!("quest/{i}");
            row
        })
        .collect();
    assert_eq!(converge(target(), 10, rows).unwrap().entries.len(), 1100);
    let mut raw = json!({"schema_version":1,"facts":[candidate("source",None,Confidence::PublisherDocumented,json!(0)).fact]});
    let duplicate = raw["facts"][0].clone();
    raw["facts"].as_array_mut().unwrap().push(duplicate);
    assert!(Document::parse(&serde_json::to_vec(&raw).unwrap(), 10).is_err());
}

#[test]
fn uncertain_dates_cannot_claim_freshness() {
    let old = candidate("old", Some(1), Confidence::PublisherDocumented, json!(true));
    let mut uncertain = candidate(
        "a-uncertain",
        Some(9),
        Confidence::PublisherDocumented,
        json!(true),
    );
    uncertain.fact.clock_trust = azeroth_core::observation::ClockTrust::Uncertain;
    let catalog = converge(target(), 10, vec![uncertain, old]).unwrap();
    assert_eq!(catalog.entries[0].candidates[0].source_id, "old");
}

#[test]
fn float_spellings_cannot_bypass_safe_numeric_range() {
    for value in [json!(9007199254740992.0), json!(1e100), json!(-1e100)] {
        assert!(
            candidate("source", None, Confidence::PublisherDocumented, value)
                .fact
                .validate(10)
                .is_err()
        );
    }
}
#[test]
fn exact_context_does_not_promote_explicitly_unvalidated_facts() {
    for confidence in [
        Confidence::CrossEditionUnverified,
        Confidence::Inferred,
        Confidence::DisputedOrStale,
    ] {
        let catalog = converge(
            target(),
            10,
            vec![candidate("source", None, confidence, json!(1))],
        )
        .unwrap();
        assert!(!catalog.entries[0].usable_for_target);
        assert_eq!(
            catalog
                .domains
                .iter()
                .find(|d| d.domain == "quests")
                .unwrap()
                .target_facts,
            0
        );
    }
}
