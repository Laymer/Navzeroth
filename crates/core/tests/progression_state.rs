use azeroth_core::{
    entities::CharacterSnapshot,
    observation::{
        ClockTrust, Freshness, FreshnessContext, Observation, ObservationMetadata,
        ObservationStatus, StateSource,
    },
    progression::{Completeness, Enumeration, Lookup, SnapshotBinding, lookup},
    types::{Millis, RecordMap, Sha256Hex, SourceId, UtcMillis},
};
use serde_json::json;

fn time() -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(100).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(10).unwrap(),
    }
}
fn metadata() -> ObservationMetadata {
    ObservationMetadata {
        observed_at_utc_ms: Some(UtcMillis::new(100).unwrap()),
        clock_trust: ClockTrust::Trusted,
        source: StateSource::ManualEntry,
        ..Default::default()
    }
}
fn enumeration(
    completeness: Completeness,
    entries: &[(&str, u32)],
) -> Observation<Enumeration<u32>> {
    Observation::known(
        Enumeration {
            completeness,
            entries: RecordMap::new(
                entries
                    .iter()
                    .map(|(id, value)| (SourceId::new(*id).unwrap(), *value)),
            )
            .unwrap(),
        },
        metadata(),
    )
    .unwrap()
}

#[test]
fn absence_requires_complete_fresh_enumeration() {
    let key = SourceId::new("spell-1").unwrap();
    assert_eq!(lookup::<u32>(None, &key, &time()), Lookup::Unknown);
    for completeness in [Completeness::Partial, Completeness::Complete] {
        let empty = enumeration(completeness, &[]);
        assert_eq!(
            lookup(Some(&empty), &key, &time()),
            if completeness == Completeness::Complete {
                Lookup::Absent
            } else {
                Lookup::Unknown
            }
        );
        let zero = enumeration(completeness, &[("spell-1", 0)]);
        assert_eq!(lookup(Some(&zero), &key, &time()), Lookup::Present(&0));
        let mut stale = time();
        stale.now_utc_ms = Some(UtcMillis::new(111).unwrap());
        assert!(matches!(
            lookup(Some(&empty), &key, &stale),
            Lookup::NotFresh(Freshness::Stale { .. })
        ));
        assert!(matches!(
            lookup(Some(&zero), &key, &stale),
            Lookup::NotFresh(Freshness::Stale { .. })
        ));
        let mut uncertain = time();
        uncertain.clock_trust = ClockTrust::Uncertain;
        assert!(matches!(
            lookup(Some(&empty), &key, &uncertain),
            Lookup::NotFresh(Freshness::Uncertain { .. })
        ));
        let mut boundary = time();
        boundary.now_utc_ms = Some(UtcMillis::new(110).unwrap());
        assert_eq!(lookup(Some(&zero), &key, &boundary), Lookup::Present(&0));
    }
}

#[test]
fn availability_and_false_are_preserved() {
    let key = SourceId::new("recipe-1").unwrap();
    for status in [
        ObservationStatus::Unknown,
        ObservationStatus::Unsupported,
        ObservationStatus::Restricted,
        ObservationStatus::NotApplicable,
    ] {
        let value = Observation::<Enumeration<bool>>::unavailable(status, metadata()).unwrap();
        assert_eq!(
            lookup(Some(&value), &key, &time()),
            Lookup::Unavailable(status)
        );
        let roundtrip: Observation<Enumeration<bool>> =
            serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(roundtrip, value);
    }
    let value = Observation::known(
        Enumeration {
            completeness: Completeness::Complete,
            entries: RecordMap::new([(key.clone(), false)]).unwrap(),
        },
        metadata(),
    )
    .unwrap();
    assert_eq!(lookup(Some(&value), &key, &time()), Lookup::Present(&false));
    assert_eq!(value.metadata().source, StateSource::ManualEntry);
}

#[test]
fn collection_wire_requires_completeness_and_bounded_unique_keys() {
    for bad in [
        r#"{"entries":{}}"#,
        r#"{"completeness":"unavailable","entries":{}}"#,
        r#"{"completeness":"complete","entries":{"s":0,"s":1}}"#,
        r#"{"completeness":"complete","entries":{},"assumed":true}"#,
    ] {
        assert!(serde_json::from_str::<Enumeration<u32>>(bad).is_err());
    }
    let entries = (0..1025)
        .map(|i| (i.to_string(), json!(0)))
        .collect::<serde_json::Map<_, _>>();
    assert!(
        serde_json::from_value::<Enumeration<u32>>(
            json!({"completeness":"complete","entries":entries})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<Observation<Enumeration<u32>>>(
            json!({"status":"restricted","value":{"completeness":"complete","entries":{}}})
        )
        .is_err()
    );
}

fn snapshot() -> CharacterSnapshot {
    serde_json::from_value(json!({
        "schema_version":1,"id":"snapshot-1","character":"character-1",
        "context":{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},
        "capture":{},"source_revision":"fixture-v1","level":{"status":"unknown"},"role":{"status":"unknown"},"location":{"status":"unknown"},"preferences":{},"equipment":{},"quests":{},"objectives":{}
    })).unwrap()
}

#[test]
fn binding_rejects_every_changed_dependency() {
    let snapshot = snapshot();
    let digest = Sha256Hex::new("a".repeat(64)).unwrap();
    let binding = SnapshotBinding::from_snapshot(&snapshot, digest.clone());
    binding.ensure_matches(&snapshot, &digest).unwrap();
    let roundtrip: SnapshotBinding =
        serde_json::from_str(&serde_json::to_string(&binding).unwrap()).unwrap();
    assert_eq!(binding, roundtrip);
    assert!(
        binding
            .ensure_matches(&snapshot, &Sha256Hex::new("b".repeat(64)).unwrap())
            .is_err()
    );
    for (pointer, value) in [
        ("/id", json!("snapshot-2")),
        ("/character", json!("character-2")),
        ("/source_revision", json!("fixture-v2")),
        ("/capture/source", json!("manual_entry")),
        ("/context/build/edition", json!("wow_classic")),
        ("/context/build/build", json!("OTHER")),
        ("/context/build/interface", json!("OTHER")),
        ("/context/namespace", json!("reference")),
    ] {
        let mut changed = serde_json::to_value(&snapshot).unwrap();
        *changed.pointer_mut(pointer).unwrap() = value;
        let changed: CharacterSnapshot = serde_json::from_value(changed).unwrap();
        assert!(
            binding.ensure_matches(&changed, &digest).is_err(),
            "{pointer}"
        );
    }
}
