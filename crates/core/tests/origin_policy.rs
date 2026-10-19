//! Accepted-origin policy (ADR 0007 §3 and §5, task V2.8e). Synthetic content
//! only; the repository policy file is read as the shipped input.
use azeroth_core::{
    content_v2::{self, ReferenceContent},
    coverage::{CoverageFinding, RecordRef, content_report},
    origin_policy::*,
    sources::SourceRecord,
    types::{EdgeId, ItemKey},
};
use serde_json::{Value, json};

fn policy_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../release/v1/accepted-origins.json"
    ))
    .unwrap()
}
fn policy() -> OriginPolicy {
    OriginPolicy::decode(&policy_bytes()).unwrap()
}
fn policy_value() -> Value {
    serde_json::from_slice(&policy_bytes()).unwrap()
}
fn refused(value: &Value) -> bool {
    OriginPolicy::decode(&serde_json::to_vec(value).unwrap()).is_err()
}
fn content_value(confidence: &str) -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    value["reference"]["payload"]["evidence"][0]["confidence"] = json!(confidence);
    value
}
fn content(confidence: &str) -> ReferenceContent {
    decode(&content_value(confidence))
}
fn decode(value: &Value) -> ReferenceContent {
    content_v2::decode(&serde_json::to_vec(value).unwrap()).unwrap()
}
fn sources(editions: Value) -> Vec<SourceRecord> {
    let mut value: Value =
        serde_json::from_str(include_str!("../../../fixtures/content-pack/sources.json")).unwrap();
    value["sources"][0]["editions"] = editions;
    serde_json::from_value(value["sources"].clone()).unwrap()
}
/// The integrator's origin rule: a source lists its true origin and the target
/// edition; its origin is every non-target edition.
fn vanilla() -> Vec<SourceRecord> {
    sources(json!({"status":"known","value":["vanilla_1_12","wow_forever"]}))
}
/// The strict count the pack verifier records as `coverage_gaps`.
fn strict(content: &ReferenceContent) -> u64 {
    content_report(&content.reference).gaps.len() as u64 + extension_unavailable(content).unwrap().0
}
fn unverified(content: &ReferenceContent) -> u64 {
    content_report(&content.reference)
        .gaps
        .iter()
        .filter(|g| matches!(g, CoverageFinding::UnverifiedEvidence { .. }))
        .count() as u64
}

#[test]
fn repository_policy_accepts_both_origins_in_every_domain() {
    let policy = policy();
    assert_eq!(policy.target_edition.as_str(), "wow_forever");
    assert!(policy.accept_inferred && policy.accept_unknowns);
    for domain in azeroth_core::release::DOMAINS {
        assert!(policy.accepts_origin(domain, "vanilla_1_12"), "{domain}");
        assert!(policy.accepts_origin(domain, "wow_classic_era"), "{domain}");
        assert!(!policy.accepts_origin(domain, "wow_forever"));
    }
    assert!(!policy.accepts_origin("unknown-domain", "vanilla_1_12"));
    assert_eq!(
        policy.origins().into_iter().collect::<Vec<_>>(),
        ["vanilla_1_12", "wow_classic_era"]
    );
}

#[test]
fn policy_decoding_is_strict() {
    let mut cases = vec![];
    let mut value = policy_value();
    value["unexpected"] = json!(true);
    cases.push(value);
    let mut value = policy_value();
    value["domains"]["imaginary"] = json!(["vanilla_1_12"]);
    cases.push(value);
    let mut value = policy_value();
    value["domains"]["quests"] = json!([]);
    cases.push(value);
    let mut value = policy_value();
    value["domains"]["quests"] = json!(["vanilla_1_12", "vanilla_1_12"]);
    cases.push(value);
    let mut value = policy_value();
    value["domains"]["quests"] = json!(["wow_forever"]);
    cases.push(value);
    let mut value = policy_value();
    value["format"] = json!("other");
    cases.push(value);
    let mut value = policy_value();
    value["schema_version"] = json!(2);
    cases.push(value);
    let mut value = policy_value();
    value["decision"] = json!(" ");
    cases.push(value);
    let mut value = policy_value();
    value["precedence"] = json!([]);
    cases.push(value);
    let mut value = policy_value();
    value["precedence"] = json!(["authored", "authored"]);
    cases.push(value);
    let mut value = policy_value();
    value.as_object_mut().unwrap().remove("accept_unknowns");
    cases.push(value);
    for (index, case) in cases.iter().enumerate() {
        assert!(refused(case), "case {index}");
    }
    let duplicate = String::from_utf8(policy_bytes()).unwrap().replacen(
        "\"accept_inferred\": true",
        "\"accept_inferred\": true, \"accept_inferred\": false",
        1,
    );
    assert!(OriginPolicy::decode(duplicate.as_bytes()).is_err());
    assert!(OriginPolicy::decode(&vec![b' '; MAX_POLICY_BYTES + 1]).is_err());
    assert!(OriginPolicy::decode(b"return os.execute('x')").is_err());
}

#[test]
fn accepted_origin_evidence_is_counted_not_a_gap() {
    let content = content("cross_edition_unverified");
    let result = classify(&content, &vanilla(), Some(&policy())).unwrap();
    assert!(result.accepted_origins > 0);
    assert_eq!(result.accepted_origins, unverified(&content));
    assert_eq!(result.total(), strict(&content));
    assert_eq!(
        result
            .accepted_origin_sources
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
        ["demo-source-1"]
    );
    assert!(
        result
            .listed_remaining_gaps
            .iter()
            .all(|g| !matches!(g, CoverageFinding::UnverifiedEvidence { .. }))
    );
    assert_eq!(
        result.listed_accepted_origins.len() as u64,
        result.accepted_origins
    );
}

#[test]
fn non_accepted_origins_and_relabelled_editions_still_gap() {
    for editions in [
        json!({"status":"known","value":["wow_classic_tbc"]}),
        // Cross-edition evidence must name a foreign accepted origin.
        json!({"status":"known","value":["wow_forever"]}),
        json!({"status":"known","value":["vanilla_1_12","wow_classic_tbc"]}),
        json!({"status":"known","value":["wow_classic_tbc","wow_forever"]}),
        json!({"status":"unknown"}),
    ] {
        let content = content("cross_edition_unverified");
        let result = classify(&content, &sources(editions.clone()), Some(&policy())).unwrap();
        assert_eq!(result.accepted_origins, 0, "{editions}");
        assert_eq!(result.total(), strict(&content));
    }
    // A domain the policy does not list keeps its gaps; other domains pass.
    let mut narrow = policy_value();
    narrow["domains"]
        .as_object_mut()
        .unwrap()
        .remove("equipment");
    let narrow = OriginPolicy::decode(&serde_json::to_vec(&narrow).unwrap()).unwrap();
    let content = content("cross_edition_unverified");
    let result = classify(&content, &vanilla(), Some(&narrow)).unwrap();
    assert!(result.accepted_origins > 0 && result.accepted_origins < unverified(&content));
    assert!(result.listed_remaining_gaps.iter().any(|g| matches!(
        g,
        CoverageFinding::UnverifiedEvidence {
            record: RecordRef::Item { .. },
            ..
        }
    )));
    // Unregistered evidence sources are never accepted.
    let result = classify(&content, &[], Some(&policy())).unwrap();
    assert_eq!(result.accepted_origins, 0);
}

#[test]
fn disputed_limited_and_conflicting_evidence_still_fail() {
    // Content v2 also refuses disputed provenance at decoding (checked below).
    let content = content("limited_sample");
    let result = classify(&content, &vanilla(), Some(&policy())).unwrap();
    assert_eq!(result.accepted_origins, 0);
    assert!(result.remaining_gaps >= unverified(&content));
    // Content v2 refuses conflicted provenance at decoding; the policy refuses
    // it independently, as well as evidence of another source or target.
    let mut evidence =
        content_value("cross_edition_unverified")["reference"]["payload"]["evidence"][0].clone();
    evidence["context"]["namespace"] = json!("reference");
    let source = &vanilla()[0];
    let record = |value: &Value| -> azeroth_core::entities::EvidenceRecord {
        serde_json::from_value(value.clone()).unwrap()
    };
    assert!(policy().evidence_accepted("travel", &record(&evidence), Some(source)));
    assert!(!policy().evidence_accepted("travel", &record(&evidence), None));
    let mut conflicted = evidence.clone();
    conflicted["conflicts"] = json!(["other-evidence"]);
    assert!(!policy().evidence_accepted("travel", &record(&conflicted), Some(source)));
    let mut foreign_source = evidence.clone();
    foreign_source["source"] = json!("another-source");
    assert!(!policy().evidence_accepted("travel", &record(&foreign_source), Some(source)));
    let mut other_target = evidence.clone();
    other_target["context"]["build"]["edition"] = json!("wow_classic_tbc");
    assert!(!policy().evidence_accepted("travel", &record(&other_target), Some(source)));
    for confidence in [
        "verified_on_build",
        "publisher_documented",
        "limited_sample",
        "disputed_or_stale",
    ] {
        let mut strong = evidence.clone();
        strong["confidence"] = json!(confidence);
        // Strong evidence is never a gap; weak evidence other than
        // cross-edition or inferred is never accepted.
        assert!(!policy().evidence_accepted("travel", &record(&strong), Some(source)));
    }
}

#[test]
fn inferred_evidence_counts_only_when_the_policy_accepts_inferred() {
    let content = content("inferred");
    // Inferred values may also come from the target edition's own derivation.
    let forever = sources(json!({"status":"known","value":["wow_forever"]}));
    let accepted = classify(&content, &forever, Some(&policy())).unwrap();
    assert_eq!(accepted.accepted_origins, unverified(&content));
    let mut strict_policy = policy_value();
    strict_policy["accept_inferred"] = json!(false);
    let strict_policy = OriginPolicy::decode(&serde_json::to_vec(&strict_policy).unwrap()).unwrap();
    let refused = classify(&content, &forever, Some(&strict_policy)).unwrap();
    assert_eq!(refused.accepted_origins, 0);
    assert_eq!(refused.total(), strict(&content));
}

#[test]
fn unknowns_are_accepted_only_by_policy_and_stay_distinct() {
    let content = content("cross_edition_unverified");
    let (extensions, fields) = extension_unavailable(&content).unwrap();
    assert!(extensions > 0);
    assert!(
        fields
            .iter()
            .all(|f| ["unknown", "unsupported", "restricted"].contains(&f.status.as_str()))
    );
    assert!(fields.iter().all(|f| f.path.starts_with('/')));
    let accepted = classify(&content, &vanilla(), Some(&policy())).unwrap();
    assert!(accepted.accepted_unknowns > extensions);
    assert!(accepted.listed_accepted_unknowns.iter().any(|u| matches!(
        u,
        AcceptedUnknown::Finding { finding } if matches!(**finding, CoverageFinding::UnknownMapTransform { .. })
    )));
    assert!(
        accepted
            .listed_accepted_unknowns
            .iter()
            .any(|u| matches!(u, AcceptedUnknown::Extension { .. }))
    );
    let mut off = policy_value();
    off["accept_unknowns"] = json!(false);
    let off = OriginPolicy::decode(&serde_json::to_vec(&off).unwrap()).unwrap();
    let strict_unknowns = classify(&content, &vanilla(), Some(&off)).unwrap();
    assert_eq!(strict_unknowns.accepted_unknowns, 0);
    assert_eq!(strict_unknowns.accepted_origins, accepted.accepted_origins);
    assert_eq!(
        strict_unknowns.remaining_gaps,
        accepted.remaining_gaps + accepted.accepted_unknowns
    );
    // Without any policy, or for another target edition, every finding is a gap.
    let none = classify(&content, &vanilla(), None).unwrap();
    assert_eq!(none.remaining_gaps, strict(&content));
    assert_eq!(none.accepted_origins + none.accepted_unknowns, 0);
    let mut other = policy_value();
    other["target_edition"] = json!("wow_classic_tbc");
    let other = OriginPolicy::decode(&serde_json::to_vec(&other).unwrap()).unwrap();
    assert_eq!(
        classify(&content, &vanilla(), Some(&other))
            .unwrap()
            .remaining_gaps,
        strict(&content)
    );
}

#[test]
fn listings_are_bounded_while_counts_stay_complete() {
    let mut value = content_value("cross_edition_unverified");
    let item = value["reference"]["payload"]["items"][0].clone();
    let fact = value["equipment"][0].clone();
    let mut items = vec![item.clone()];
    let mut facts = vec![fact.clone()];
    for n in 0..(MAX_LISTED + 5) {
        let mut record = item.clone();
        record["key"]["item_id"] = json!(format!("bulk-item-{n}"));
        items.push(record.clone());
        let mut f = fact.clone();
        f["item"] = record["key"].clone();
        facts.push(f);
    }
    value["reference"]["payload"]["items"] = Value::Array(items);
    value["equipment"] = Value::Array(facts);
    let content = decode(&value);
    let none = classify(&content, &vanilla(), None).unwrap();
    assert!(none.remaining_gaps > MAX_LISTED as u64);
    assert_eq!(none.listed_remaining_gaps.len(), MAX_LISTED);
    let all = classify(&content, &vanilla(), Some(&policy())).unwrap();
    assert!(all.accepted_origins > MAX_LISTED as u64);
    assert_eq!(all.listed_accepted_origins.len(), MAX_LISTED);
    assert_eq!(all.listed_accepted_unknowns.len(), MAX_LISTED);
    assert_eq!(none.remaining_gaps, strict(&content));
    let (_, fields) = extension_unavailable(&content).unwrap();
    assert!(fields.len() <= MAX_LISTED);
}

#[test]
fn record_domains_follow_the_release_domains() {
    let item: ItemKey = serde_json::from_value(
        content_value("inferred")["reference"]["payload"]["items"][0]["key"].clone(),
    )
    .unwrap();
    assert_eq!(
        record_domain(&RecordRef::Item { key: item }),
        Some("equipment")
    );
    assert_eq!(
        record_domain(&RecordRef::Edge {
            id: EdgeId::new("e").unwrap()
        }),
        Some("travel")
    );
    assert_eq!(
        record_domain(&RecordRef::Evidence {
            id: azeroth_core::types::EvidenceId::new("e").unwrap()
        }),
        None
    );
}

#[test]
fn origin_only_sources_are_accepted_too_and_relabels_are_flagged() {
    // An origin-only listing (no target edition) is also read as that origin.
    let content = content("cross_edition_unverified");
    let only = sources(json!({"status":"known","value":["wow_classic_era"]}));
    let result = classify(&content, &only, Some(&policy())).unwrap();
    assert_eq!(result.accepted_origins, unverified(&content));
    assert_eq!(result.relabelled_evidence, 0);
    // Stronger claims from a non-target origin are relabels, policy or not.
    let mut value = content_value("publisher_documented");
    value["reference"]["namespace"] = json!("demo");
    let strong = decode(&value);
    for policy in [Some(policy()), None] {
        let result = classify(&strong, &vanilla(), policy.as_ref()).unwrap();
        assert_eq!(result.relabelled_evidence, 1);
        assert_eq!(
            result
                .listed_relabelled_evidence
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
            ["demo-evidence-1"]
        );
    }
    let forever = sources(json!({"status":"known","value":["wow_forever"]}));
    let result = classify(&strong, &forever, Some(&policy())).unwrap();
    assert_eq!(result.relabelled_evidence, 0);
}
