use azeroth_core::{
    entities::Confidence,
    error::ErrorCode,
    sources::*,
    types::{DataContext, EvidenceId, ItemKey, SourceId},
};
use serde_json::{Value, json};

fn ctx(edition: &str, namespace: &str) -> Value {
    json!({"build":{"edition":edition,"build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":namespace})
}
fn forever() -> Value {
    ctx("wow_forever", "reference")
}
fn classic() -> Value {
    ctx("wow_classic", "reference")
}
fn unknown() -> Value {
    json!({"status":"unknown"})
}
fn source_json(id: &str, editions: Value) -> Value {
    json!({
        "id": id, "provider": null, "publisher": "Synthetic publisher",
        "location": unknown(), "editions": editions, "access": unknown(),
        "authentication": unknown(), "license": unknown(), "attribution": unknown(),
        "redistribution": unknown(), "retention": unknown(), "allowed_fields": unknown(),
        "transformations": unknown(), "update_schedule": unknown(),
        "conditional_requests": unknown(), "failure_behavior": unknown(),
        "last_retrieval_utc_ms": unknown(), "verified_revision": unknown(),
        "offline_usable": unknown(), "record_class": "secondary_inference"
    })
}
fn editions(list: &[&str]) -> Value {
    json!({"status":"known","value":list})
}
fn source(id: &str, list: &[&str]) -> SourceRecord {
    serde_json::from_value(source_json(id, editions(list))).unwrap()
}
fn registry() -> SourceRegistry {
    let mut registry = SourceRegistry::new();
    registry
        .register(source("src-synthetic-classic", &["wow_classic"]))
        .unwrap();
    registry
        .register(source("src-synthetic-forever", &["wow_forever"]))
        .unwrap();
    registry
        .register(serde_json::from_value(source_json("src-synthetic-unscoped", unknown())).unwrap())
        .unwrap();
    registry
}
fn bool_value(value: bool) -> Value {
    json!({"status":"known","value":value})
}
fn item(context: &Value, id: &str, two_handed: bool, evidence: &str) -> Value {
    json!({"item": {
        "key": {"context": context, "item_id": id, "variant": "base"},
        "slots": ["slot-synthetic"], "two_handed": bool_value(two_handed),
        "unique_group": unknown(), "attributes": unknown(), "requirements": unknown(),
        "effects": unknown(), "evidence": [evidence]
    }})
}
fn quest(context: &Value, id: &str, shared: bool, evidence: &str) -> Value {
    json!({"quest": {
        "id": id, "context": context, "eligibility": unknown(), "prerequisites": unknown(),
        "stages": ["objective-synthetic"], "locations": unknown(), "rewards": unknown(),
        "completion": unknown(), "shared_progress": bool_value(shared), "evidence": [evidence]
    }})
}
fn evidence_json(id: &str, context: &Value, source: &str, confidence: &str) -> Value {
    json!({"id": id, "context": context, "source": source, "license": unknown(),
        "observed_at_utc_ms": unknown(), "confidence": confidence, "conflicts": [],
        "notes": "synthetic"})
}
fn candidate_json(payload: Value, origin: &Value, evidence: Value) -> Value {
    json!({"payload": payload, "origin": origin, "evidence": evidence})
}
fn candidate(payload: Value, origin: &Value, evidence: Value) -> Candidate {
    serde_json::from_value(candidate_json(payload, origin, evidence)).unwrap()
}
/// Item candidate whose key context and origin are both `context`.
fn native_item(
    context: &Value,
    item_id: &str,
    two_handed: bool,
    evidence: &str,
    source: &str,
    confidence: &str,
) -> Candidate {
    candidate(
        item(context, item_id, two_handed, evidence),
        context,
        evidence_json(evidence, context, source, confidence),
    )
}
fn item_key(context: &Value, id: &str) -> RecordKey {
    let key: ItemKey =
        serde_json::from_value(json!({"context": context, "item_id": id, "variant": "base"}))
            .unwrap();
    RecordKey::Item(key)
}
fn id(value: &str) -> EvidenceId {
    EvidenceId::new(value).unwrap()
}
fn state(candidates: Vec<Candidate>) -> NormalizedState {
    NormalizedState::default()
        .ingest(&registry(), candidates)
        .unwrap()
}
fn bytes(state: &NormalizedState) -> Vec<u8> {
    serde_json::to_vec(state).unwrap()
}

#[test]
fn source_record_round_trips_and_keeps_unknown_distinct() {
    let good = source_json("src-synthetic-forever", editions(&["wow_forever"]));
    let record: SourceRecord = serde_json::from_value(good.clone()).unwrap();
    // Unknown policy fields stay unknown; nothing defaults to permitted.
    assert!(record.redistribution.value().is_none());
    assert_eq!(
        serde_json::to_value(&record).unwrap()["redistribution"]["status"],
        "unknown"
    );
    let encoded = serde_json::to_value(&record).unwrap();
    assert_eq!(
        serde_json::from_value::<SourceRecord>(encoded).unwrap(),
        record
    );

    // Unknown is not false: a recorded `false` is a different observation.
    let mut offline_false = good.clone();
    offline_false["offline_usable"] = bool_value(false);
    let offline_false: SourceRecord = serde_json::from_value(offline_false).unwrap();
    assert_ne!(offline_false, record);
    assert_eq!(offline_false.offline_usable.value(), Some(&false));
    assert_eq!(record.offline_usable.value(), None);

    // A recorded negative redistribution decision is preserved as such.
    let mut denied = good.clone();
    denied["redistribution"] = json!({"status":"known","value":"not_permitted"});
    let denied: SourceRecord = serde_json::from_value(denied).unwrap();
    assert_eq!(
        denied.redistribution.value(),
        Some(&Redistribution::NotPermitted)
    );

    // Semantic violations are rejected by Validated deserialization.
    let reject = |edit: &dyn Fn(&mut Value)| {
        let mut bad = good.clone();
        edit(&mut bad);
        assert!(serde_json::from_value::<SourceRecord>(bad).is_err());
    };
    reject(&|v| v["publisher"] = json!("  "));
    reject(&|v| v["editions"] = editions(&[]));
    reject(&|v| v["editions"] = editions(&["wow_forever", "wow_forever"]));
    reject(&|v| v["license"] = json!({"status":"known","value":""}));
    reject(&|v| v["allowed_fields"] = json!({"status":"known","value":["a","a"]}));
    reject(&|v| v["allowed_fields"] = json!({"status":"known","value":[]}));
    reject(&|v| {
        v.as_object_mut().unwrap().remove("redistribution");
    });
    reject(&|v| v["record_class"] = json!("guess"));
}

#[test]
fn registry_rejects_duplicates_and_unregistered_sources() {
    let mut registry = SourceRegistry::new();
    registry
        .register(source("src-synthetic-a", &["wow_forever"]))
        .unwrap();
    let dup = registry
        .register(source("src-synthetic-a", &["wow_classic"]))
        .unwrap_err();
    assert_eq!(dup.code, ErrorCode::InvalidInput);
    assert_eq!(registry.len(), 1);
    // The first registration was not replaced.
    let kept = registry
        .get(&SourceId::new("src-synthetic-a").unwrap())
        .unwrap();
    assert_eq!(kept.editions.value().unwrap()[0].as_str(), "wow_forever");

    // Register-before-ingest: evidence from an unregistered source fails, and
    // nothing is partially applied.
    let orphan = native_item(
        &forever(),
        "1",
        true,
        "e1",
        "src-synthetic-missing",
        "inferred",
    );
    let known = native_item(
        &forever(),
        "2",
        true,
        "e2",
        "src-synthetic-forever",
        "inferred",
    );
    let err = NormalizedState::default()
        .ingest(&self::registry(), vec![known, orphan])
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::DatasetIncomplete);
}

#[test]
fn bounds_are_enforced_not_truncated() {
    let mut registry = SourceRegistry::new();
    for n in 0..1024 {
        registry
            .register(source(&format!("src-synthetic-{n}"), &["wow_forever"]))
            .unwrap();
    }
    assert!(
        registry
            .register(source("src-synthetic-over", &["wow_forever"]))
            .is_err()
    );
    assert_eq!(registry.len(), 1024);
    let list: Vec<Value> = (0..1025)
        .map(|n| source_json(&format!("src-synthetic-{n}"), editions(&["wow_forever"])))
        .collect();
    assert!(serde_json::from_value::<SourceRegistry>(json!(list)).is_err());
    let dup_list = json!([
        source_json("src-synthetic-a", editions(&["wow_forever"])),
        source_json("src-synthetic-a", editions(&["wow_forever"]))
    ]);
    assert!(serde_json::from_value::<SourceRegistry>(dup_list).is_err());
    let ok = json!([source_json("src-synthetic-a", editions(&["wow_forever"]))]);
    let parsed: SourceRegistry = serde_json::from_value(ok).unwrap();
    assert_eq!(parsed.len(), 1);

    let many: Vec<Candidate> = (0..1025)
        .map(|n| {
            native_item(
                &forever(),
                &n.to_string(),
                true,
                &format!("e{n}"),
                "src-synthetic-forever",
                "inferred",
            )
        })
        .collect();
    let err = NormalizedState::default()
        .ingest(&self::registry(), many)
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn case_005_same_item_id_across_editions_stays_isolated() {
    let classic_candidate = native_item(
        &classic(),
        "9007199254740993",
        true,
        "e-classic",
        "src-synthetic-classic",
        "cross_edition_unverified",
    );
    let forever_candidate = native_item(
        &forever(),
        "9007199254740993",
        false,
        "e-forever",
        "src-synthetic-forever",
        "verified_on_build",
    );
    let state = state(vec![classic_candidate.clone(), forever_candidate.clone()]);
    // Two distinct records, no conflict: different contexts are different keys.
    assert_eq!(state.entries().len(), 2);
    assert!(state.conflicts().is_empty());

    let forever_context: DataContext = serde_json::from_value(forever()).unwrap();
    let classic_context: DataContext = serde_json::from_value(classic()).unwrap();
    let forever_entries: Vec<_> = state.entries_in_context(&forever_context).collect();
    assert_eq!(forever_entries.len(), 1);
    assert_eq!(forever_entries[0].records.len(), 1);
    assert_eq!(forever_entries[0].records[0].evidence.id, id("e-forever"));
    assert_eq!(
        forever_entries[0].records[0].effective_confidence,
        Confidence::VerifiedOnBuild
    );
    let classic_entry = state
        .entry(&item_key(&classic(), "9007199254740993"))
        .unwrap();
    assert_eq!(classic_entry.records[0].evidence.id, id("e-classic"));
    assert_eq!(state.entries_in_context(&classic_context).count(), 1);
    // The Classic record is never reported as verified for any context.
    for entry in state.entries_in_context(&classic_context) {
        for record in &entry.records {
            assert_ne!(record.effective_confidence, Confidence::VerifiedOnBuild);
            assert_eq!(
                record.evidence.confidence,
                Confidence::CrossEditionUnverified
            );
        }
    }
    // The order of ingestion does not matter either.
    let reversed = self::state(vec![forever_candidate, classic_candidate]);
    assert_eq!(bytes(&state), bytes(&reversed));

    // Re-labelling the Classic record's edition string to Forever, while it still
    // comes from Classic and claims verification, is demoted and classified.
    let relabel = candidate(
        item(&forever(), "9007199254740993", true, "e-relabel"),
        &classic(),
        evidence_json(
            "e-relabel",
            &forever(),
            "src-synthetic-classic",
            "verified_on_build",
        ),
    );
    let after = state_with(&state, vec![relabel]);
    let entry = after
        .entry(&item_key(&forever(), "9007199254740993"))
        .unwrap();
    assert_eq!(entry.records.len(), 2);
    assert_eq!(entry.preferred, id("e-forever"));
    let relabelled = entry
        .records
        .iter()
        .find(|r| r.evidence.id == id("e-relabel"))
        .unwrap();
    assert!(relabelled.relabelled);
    assert_eq!(
        relabelled.effective_confidence,
        Confidence::CrossEditionUnverified
    );
    let kinds: Vec<_> = after.conflicts_for(&entry.key).map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ConflictKind::CategoryDowngrade,
            ConflictKind::CrossEditionRelabel
        ]
    );
    // The untouched Classic entry is unchanged.
    assert_eq!(
        after.entry(&item_key(&classic(), "9007199254740993")),
        state.entry(&item_key(&classic(), "9007199254740993"))
    );
}

fn state_with(base: &NormalizedState, candidates: Vec<Candidate>) -> NormalizedState {
    base.ingest(&registry(), candidates).unwrap()
}

#[test]
fn relabelled_edition_string_alone_never_becomes_verified() {
    // Only the relabelled record exists: no native Forever evidence at all.
    let relabel = candidate(
        item(&forever(), "42", true, "e-relabel"),
        &classic(),
        evidence_json(
            "e-relabel",
            &forever(),
            "src-synthetic-classic",
            "verified_on_build",
        ),
    );
    let state = state(vec![relabel]);
    let entry = state.entry(&item_key(&forever(), "42")).unwrap();
    assert_eq!(entry.records.len(), 1);
    assert_ne!(
        entry.records[0].effective_confidence,
        Confidence::VerifiedOnBuild
    );
    assert_eq!(
        entry.records[0].effective_confidence,
        Confidence::CrossEditionUnverified
    );
    let conflicts = state.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].kind, ConflictKind::CrossEditionRelabel);
    assert_eq!(conflicts[0].preferred, id("e-relabel"));
    assert!(conflicts[0].others.is_empty());

    // A properly labelled cross-edition record is not a relabel.
    let honest = candidate(
        item(&forever(), "43", true, "e-honest"),
        &classic(),
        evidence_json(
            "e-honest",
            &forever(),
            "src-synthetic-classic",
            "cross_edition_unverified",
        ),
    );
    let honest = self::state(vec![honest]);
    assert!(honest.conflicts().is_empty());
    assert!(!honest.entries()[0].records[0].relabelled);
}

#[test]
fn precedence_prefers_verified_and_keeps_every_record() {
    let verified = native_item(
        &forever(),
        "7",
        false,
        "e-verified",
        "src-synthetic-forever",
        "verified_on_build",
    );
    let cross = candidate(
        item(&forever(), "7", true, "e-cross"),
        &classic(),
        evidence_json(
            "e-cross",
            &forever(),
            "src-synthetic-classic",
            "cross_edition_unverified",
        ),
    );
    let inferred = native_item(
        &forever(),
        "7",
        true,
        "e-inferred",
        "src-synthetic-unscoped",
        "inferred",
    );
    let state = state(vec![inferred, cross, verified]);
    let entry = state.entry(&item_key(&forever(), "7")).unwrap();
    assert_eq!(entry.preferred, id("e-verified"));
    // Count preserved; best first.
    let order: Vec<_> = entry
        .records
        .iter()
        .map(|r| r.evidence.id.as_str())
        .collect();
    assert_eq!(order, ["e-verified", "e-cross", "e-inferred"]);
    assert_eq!(state.conflicts().len(), 1);
    let conflict = &state.conflicts()[0];
    assert_eq!(conflict.kind, ConflictKind::CategoryDowngrade);
    assert_eq!(conflict.preferred, id("e-verified"));
    assert_eq!(conflict.others, vec![id("e-cross"), id("e-inferred")]);

    // Agreeing lower-precedence evidence is kept and is not a conflict.
    let agree = state_with(
        &self::state(vec![native_item(
            &forever(),
            "8",
            true,
            "e-a",
            "src-synthetic-forever",
            "verified_on_build",
        )]),
        vec![native_item(
            &forever(),
            "8",
            true,
            "e-b",
            "src-synthetic-unscoped",
            "inferred",
        )],
    );
    assert_eq!(agree.entries()[0].records.len(), 2);
    assert!(agree.conflicts().is_empty());
}

#[test]
fn precedence_rank_is_the_documented_total_order() {
    use Confidence::*;
    let order = [
        VerifiedOnBuild,
        PublisherDocumented,
        LimitedSample,
        CrossEditionUnverified,
        Inferred,
        DisputedOrStale,
    ];
    for pair in order.windows(2) {
        assert!(precedence_rank(pair[0]) > precedence_rank(pair[1]));
    }
}

#[test]
fn disagreement_is_classified_not_last_writer_wins() {
    let a = native_item(
        &forever(),
        "9",
        true,
        "e-a",
        "src-synthetic-forever",
        "limited_sample",
    );
    let b = native_item(
        &forever(),
        "9",
        false,
        "e-b",
        "src-synthetic-forever",
        "limited_sample",
    );
    let forward = state(vec![a.clone(), b.clone()]);
    let backward = state(vec![b, a.clone()]);
    for state in [&forward, &backward] {
        let entry = state.entry(&item_key(&forever(), "9")).unwrap();
        assert_eq!(entry.records.len(), 2);
        // Same category: the deterministic tie-break is the lower evidence ID.
        assert_eq!(entry.preferred, id("e-a"));
        assert_eq!(state.conflicts().len(), 1);
        assert_eq!(state.conflicts()[0].kind, ConflictKind::ValueDisagreement);
        assert_eq!(state.conflicts()[0].others, vec![id("e-b")]);
    }
    assert_eq!(bytes(&forward), bytes(&backward));

    // An identical candidate is deduplicated, not a conflict.
    let twice = state(vec![a.clone(), a.clone()]);
    assert_eq!(twice.entries()[0].records.len(), 1);
    assert!(twice.conflicts().is_empty());

    // The same evidence ID with different content is an error, never an overwrite.
    let changed = native_item(
        &forever(),
        "9",
        false,
        "e-a",
        "src-synthetic-forever",
        "limited_sample",
    );
    let err = NormalizedState::default()
        .ingest(&registry(), vec![a, changed])
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn reapplying_the_same_input_is_idempotent() {
    let input = || {
        vec![
            native_item(
                &forever(),
                "1",
                true,
                "e1",
                "src-synthetic-forever",
                "verified_on_build",
            ),
            native_item(
                &forever(),
                "1",
                false,
                "e2",
                "src-synthetic-unscoped",
                "inferred",
            ),
            native_item(
                &classic(),
                "1",
                true,
                "e3",
                "src-synthetic-classic",
                "cross_edition_unverified",
            ),
        ]
    };
    let once = state(input());
    let twice = state_with(&once, input());
    assert_eq!(once, twice);
    assert_eq!(bytes(&once), bytes(&twice));
    // Ingesting nothing returns the same state.
    assert_eq!(state_with(&once, Vec::new()), once);
}

#[test]
fn output_is_deterministic_for_shuffled_input() {
    let make = || {
        vec![
            native_item(
                &forever(),
                "1",
                true,
                "e1",
                "src-synthetic-forever",
                "verified_on_build",
            ),
            native_item(
                &forever(),
                "1",
                false,
                "e2",
                "src-synthetic-unscoped",
                "inferred",
            ),
            native_item(
                &forever(),
                "1",
                false,
                "e3",
                "src-synthetic-unscoped",
                "inferred",
            ),
            native_item(
                &classic(),
                "1",
                true,
                "e4",
                "src-synthetic-classic",
                "cross_edition_unverified",
            ),
            native_item(
                &forever(),
                "2",
                true,
                "e5",
                "src-synthetic-forever",
                "limited_sample",
            ),
            candidate(
                quest(&forever(), "quest-synthetic", true, "e6"),
                &forever(),
                evidence_json(
                    "e6",
                    &forever(),
                    "src-synthetic-forever",
                    "publisher_documented",
                ),
            ),
        ]
    };
    let base = bytes(&state(make()));
    let n = make().len();
    for shift in 0..n {
        let mut rotated = make();
        rotated.rotate_left(shift);
        assert_eq!(base, bytes(&state(rotated)));
        let mut reversed = make();
        reversed.rotate_left(shift);
        reversed.reverse();
        assert_eq!(base, bytes(&state(reversed)));
    }
}

#[test]
fn tie_break_uses_evidence_id_then_source_id() {
    let state = state(vec![
        native_item(
            &forever(),
            "5",
            true,
            "e-b",
            "src-synthetic-forever",
            "inferred",
        ),
        native_item(
            &forever(),
            "5",
            true,
            "e-a",
            "src-synthetic-unscoped",
            "inferred",
        ),
    ]);
    let entry = state.entry(&item_key(&forever(), "5")).unwrap();
    assert_eq!(entry.preferred, id("e-a"));
    // Same evidence ID cannot coexist in a state, so the source tie-break is
    // checked on the comparison itself.
    let mut left = entry.records[0].clone();
    let mut right = left.clone();
    left.evidence = serde_json::from_value(evidence_json(
        "e-same",
        &forever(),
        "src-synthetic-forever",
        "inferred",
    ))
    .unwrap();
    right.evidence = serde_json::from_value(evidence_json(
        "e-same",
        &forever(),
        "src-synthetic-unscoped",
        "inferred",
    ))
    .unwrap();
    assert_eq!(compare_precedence(&left, &right), std::cmp::Ordering::Less);
    assert_eq!(
        compare_precedence(&right, &left),
        std::cmp::Ordering::Greater
    );
    assert_eq!(compare_precedence(&left, &left), std::cmp::Ordering::Equal);
}

#[test]
fn demo_and_reference_namespaces_never_merge() {
    let demo = ctx("wow_forever", "demo");
    let demo_candidate = native_item(
        &demo,
        "1",
        true,
        "e-demo",
        "src-synthetic-forever",
        "inferred",
    );
    let reference = native_item(
        &forever(),
        "1",
        false,
        "e-ref",
        "src-synthetic-forever",
        "verified_on_build",
    );
    let state = state(vec![demo_candidate, reference]);
    assert_eq!(state.entries().len(), 2);
    assert!(state.conflicts().is_empty());
    let demo_context: DataContext = serde_json::from_value(demo.clone()).unwrap();
    let in_demo: Vec<_> = state.entries_in_context(&demo_context).collect();
    assert_eq!(in_demo.len(), 1);
    assert_eq!(in_demo[0].records[0].evidence.id, id("e-demo"));
    assert_ne!(
        in_demo[0].records[0].effective_confidence,
        Confidence::VerifiedOnBuild
    );

    // Demo evidence cannot claim verification, even with a custom origin.
    let verified_demo = candidate_json(
        item(&demo, "1", true, "e-bad"),
        &demo,
        evidence_json("e-bad", &demo, "src-synthetic-forever", "verified_on_build"),
    );
    assert!(serde_json::from_value::<Candidate>(verified_demo).is_err());

    // Reference data cannot claim demo origin (or the reverse) in the same edition.
    let mixed = candidate_json(
        item(&forever(), "1", true, "e-mix"),
        &demo,
        evidence_json("e-mix", &forever(), "src-synthetic-forever", "inferred"),
    );
    let err = serde_json::from_value::<Candidate>(mixed).unwrap_err();
    assert!(err.to_string().contains("mismatch"));
    // Cross-edition does not excuse a namespace mix either.
    let mixed = candidate_json(
        item(&forever(), "1", true, "e-mix"),
        &ctx("wow_classic", "demo"),
        evidence_json("e-mix", &forever(), "src-synthetic-classic", "inferred"),
    );
    assert!(serde_json::from_value::<Candidate>(mixed).is_err());
}

#[test]
fn candidates_are_validated_for_context_and_citation() {
    // Evidence from another context than the record it supports.
    let wrong_context = candidate_json(
        item(&forever(), "1", true, "e1"),
        &forever(),
        evidence_json("e1", &classic(), "src-synthetic-forever", "inferred"),
    );
    assert!(serde_json::from_value::<Candidate>(wrong_context).is_err());
    // A record that does not cite its evidence.
    let uncited = candidate_json(
        item(&forever(), "1", true, "other"),
        &forever(),
        evidence_json("e1", &forever(), "src-synthetic-forever", "inferred"),
    );
    assert!(serde_json::from_value::<Candidate>(uncited).is_err());
    // Same edition but different build is not the same origin.
    let mut other_build = forever();
    other_build["build"]["build"] = json!("OTHER");
    let different_build = candidate_json(
        item(&forever(), "1", true, "e1"),
        &other_build,
        evidence_json("e1", &forever(), "src-synthetic-forever", "inferred"),
    );
    assert!(serde_json::from_value::<Candidate>(different_build).is_err());
}

#[test]
fn source_edition_coverage_is_enforced_when_known() {
    // A Forever-only source cannot supply Classic-origin evidence.
    let c = candidate(
        item(&forever(), "1", true, "e1"),
        &classic(),
        evidence_json(
            "e1",
            &forever(),
            "src-synthetic-forever",
            "cross_edition_unverified",
        ),
    );
    let err = NormalizedState::default()
        .ingest(&registry(), vec![c])
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
    // An unknown coverage is not coerced into a refusal.
    let ok = candidate(
        item(&forever(), "1", true, "e2"),
        &classic(),
        evidence_json(
            "e2",
            &forever(),
            "src-synthetic-unscoped",
            "cross_edition_unverified",
        ),
    );
    assert!(
        NormalizedState::default()
            .ingest(&registry(), vec![ok])
            .is_ok()
    );
}

#[test]
fn quests_use_the_same_conflict_model() {
    let key = RecordKey::Quest {
        context: serde_json::from_value(forever()).unwrap(),
        quest: azeroth_core::types::QuestId::new("quest-synthetic").unwrap(),
    };
    let documented = candidate(
        quest(&forever(), "quest-synthetic", true, "q-a"),
        &forever(),
        evidence_json(
            "q-a",
            &forever(),
            "src-synthetic-forever",
            "publisher_documented",
        ),
    );
    let inferred = candidate(
        quest(&forever(), "quest-synthetic", false, "q-b"),
        &forever(),
        evidence_json("q-b", &forever(), "src-synthetic-unscoped", "inferred"),
    );
    let state = state(vec![inferred, documented]);
    let entry = state.entry(&key).unwrap();
    assert_eq!(entry.preferred, id("q-a"));
    assert_eq!(entry.records.len(), 2);
    assert_eq!(state.conflicts()[0].kind, ConflictKind::CategoryDowngrade);
    // Item and quest keys never collide.
    assert!(
        state
            .entry(&item_key(&forever(), "quest-synthetic"))
            .is_none()
    );
}

#[test]
fn conflict_records_round_trip_as_plain_data() {
    let state = state(vec![
        native_item(
            &forever(),
            "1",
            true,
            "e1",
            "src-synthetic-forever",
            "verified_on_build",
        ),
        native_item(
            &forever(),
            "1",
            false,
            "e2",
            "src-synthetic-unscoped",
            "inferred",
        ),
    ]);
    let encoded = serde_json::to_value(state.conflicts()).unwrap();
    let decoded: Vec<SourceConflict> = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, state.conflicts());
    assert_eq!(
        serde_json::to_value(decoded[0].kind).unwrap(),
        json!("category_downgrade")
    );
}

#[test]
fn no_numeric_confidence_in_the_sources_module() {
    let source = include_str!("../src/sources.rs");
    for forbidden in ["f32", "f64", "percent"] {
        assert!(!source.contains(forbidden), "found {forbidden}");
    }
}
