use azeroth_core::content_v2::{MAX_CONTENT_BYTES, decode};
use serde_json::{Value, json};
fn fixture() -> Value {
    let reference: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap();
    let map = reference["payload"]["map_spaces"][0]["key"].clone();
    let item = reference["payload"]["items"][0]["key"].clone();
    let profile: Value = serde_json::from_str(include_str!(
        "../../../fixtures/gear/case-006-two-hand-shield.json"
    ))
    .unwrap();
    json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,
        "models":[{"id":"model-1","evidence":["demo-evidence-1"],"class":"synthetic-class","minimum_level":1,"maximum_level":60,
        "applicability":{"status":"unknown"},"profile":profile["profile"],"proficiency":{"status":"unknown"},"layout":{"status":"known","value":profile["layout"]},"exclusions":{"status":"known","value":[]}}],
        "equipment":[{"item":item,"evidence":["demo-evidence-1"],"category":{"status":"unknown"},"configuration":{"status":"unknown"}}],
        "quest_facts":[{"quest":"demo-quest-1","evidence":["demo-evidence-1"],"access":{"status":"unknown"},"access_requirements":{"status":"unknown"},"exclusive_groups":{"status":"known","value":[]},"reset":{"status":"known","value":{"kind":"never"}},"shareable":{"status":"known","value":false},"sharing_conditions":{"status":"unknown"}}],
        "acquisition_facts":[{"acquisition":"demo-acq-1","evidence":["demo-evidence-1"],"origin":{"status":"unknown"},"rewards":{"status":"known","value":[{"item":item,"quantity":0,"choice_group":null}]},"costs":{"status":"known","value":[{"kind":"copper","amount":"0"}]},"availability":{"status":"unknown"},"prerequisites":{"status":"unknown"},"entitlement":{"status":"restricted"}}],
        "dungeons":[{"id":"dungeon-1","evidence":["demo-evidence-1"],"title":"Synthetic dungeon","map":map,"entrance":{"status":"known","value":{"id":"demo-node-1","map":map}},"access":{"status":"unknown"},"preparation_quests":{"status":"known","value":["demo-quest-1"]},"travel_edges":{"status":"known","value":[]},"rewards":{"status":"known","value":["demo-acq-1"]},"checklist":{"status":"known","value":[{"id":"check-1","evidence":["demo-evidence-1"],"label":"Check","condition":{"status":"unknown"}}]}}]
    })
}
fn parse(v: &Value) -> azeroth_core::error::Result<azeroth_core::content_v2::ReferenceContent> {
    decode(&serde_json::to_vec(v).unwrap())
}
#[test]
fn synthetic_graph_preserves_false_zero_unknown_restricted() {
    let _: azeroth_core::content_v2::ReferenceContent = serde_json::from_value(fixture()).unwrap();
    let content = parse(&fixture()).unwrap();
    assert_eq!(content.quest_facts[0].shareable.value(), Some(&false));
    assert_eq!(
        content.acquisition_facts[0].rewards.value().unwrap()[0].quantity,
        0
    );
    assert!(content.models[0].proficiency.value().is_none());
    assert_eq!(
        content.acquisition_facts[0].entitlement.status(),
        azeroth_core::observation::ObservationStatus::Restricted
    );
    assert_eq!(content.reference.payload.revision.as_str(), "demo-r1");
}
#[test]
fn rejects_versions_unknown_extension_fields_and_byte_overflow() {
    let mut v = fixture();
    v["schema_version"] = 1.into();
    assert!(parse(&v).is_err());
    let mut v = fixture();
    v["models"][0]["invented"] = true.into();
    assert!(parse(&v).is_err());
    assert!(decode(&vec![b' '; MAX_CONTENT_BYTES + 1]).is_err());
    assert!(
        decode(include_bytes!(
            "../../../fixtures/contracts/v1/reference-dataset.json"
        ))
        .is_err()
    );
}
#[test]
fn rejects_missing_conflicted_and_duplicate_extension_provenance() {
    let mut v = fixture();
    v["models"][0]["evidence"] = json!([]);
    assert!(parse(&v).is_err());
    let mut v = fixture();
    v["reference"]["payload"]["evidence"][0]["confidence"] = "disputed_or_stale".into();
    assert!(parse(&v).is_err());
    let mut v = fixture();
    let duplicate = v["models"][0].clone();
    v["models"].as_array_mut().unwrap().push(duplicate);
    assert!(parse(&v).is_err());
}
#[test]
fn rejects_unresolved_cross_domain_refs_and_mixed_contexts() {
    for path in [
        "/dungeons/0/preparation_quests/value/0",
        "/dungeons/0/rewards/value/0",
        "/quest_facts/0/quest",
    ] {
        let mut v = fixture();
        *v.pointer_mut(path).unwrap() = "missing".into();
        assert!(parse(&v).is_err());
    }
    let mut v = fixture();
    v["equipment"][0]["item"]["context"]["build"]["build"] = "other".into();
    assert!(parse(&v).is_err());
    let mut v = fixture();
    v["dungeons"][0]["access"] = json!({"status":"known","value":{"op":"quest_state","quest":"absent","state":"completed","value":true}});
    assert!(parse(&v).is_err());
    let mut v = fixture();
    v["models"][0]["maximum_level"] = 0.into();
    assert!(parse(&v).is_err());
}
#[test]
fn requires_documented_model_source_and_valid_reset() {
    let mut v = fixture();
    v["models"][0]["profile"]["weight_source"] =
        json!({"type":"documented_model","source":"missing-source"});
    assert!(parse(&v).is_err());
    let mut v = fixture();
    v["quest_facts"][0]["reset"] = json!({"status":"known","value":{"kind":"timed","period_ms":0}});
    assert!(parse(&v).is_err());
}

#[test]
fn exclusive_choices_require_explicit_trigger_and_conflicts_are_bidirectional() {
    let mut v = fixture();
    v["quest_facts"][0]["exclusive_groups"] =
        json!({"status":"known","value":[{"quests":["demo-quest-1","other"]}]});
    assert!(parse(&v).is_err());
    let mut v = fixture();
    let mut other = v["reference"]["payload"]["evidence"][0].clone();
    other["id"] = "other-evidence".into();
    other["conflicts"] = json!(["demo-evidence-1"]);
    v["reference"]["payload"]["evidence"]
        .as_array_mut()
        .unwrap()
        .push(other);
    assert!(parse(&v).is_err());
}

#[test]
fn strict_nested_fields_duplicates_and_known_empty_defaults() {
    for path in [
        "/reference/payload/items/0",
        "/models/0/profile",
        "/models/0/layout/value",
        "/quest_facts/0/access",
    ] {
        let mut v = fixture();
        v.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("invented".into(), json!(true));
        assert!(parse(&v).is_err(), "{path}");
    }
    let text = serde_json::to_string(&fixture()).unwrap().replacen(
        "\"quantity\":0",
        "\"quantity\":0,\"quantity\":0",
        1,
    );
    assert!(decode(text.as_bytes()).is_err());
    let mut v = fixture();
    v["reference"]["payload"]["map_images"] = json!([]);
    v["reference"]["payload"]["map_embeddings"] = json!([]);
    assert!(parse(&v).is_ok());
}
