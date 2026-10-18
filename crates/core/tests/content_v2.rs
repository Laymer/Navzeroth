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
fn recursive_model_predicates_resolve_every_referenced_domain() {
    let mut v = fixture();
    let item = v["reference"]["payload"]["items"][0]["key"].clone();
    let map = v["reference"]["payload"]["map_spaces"][0]["key"].clone();
    let objective = v["reference"]["payload"]["objectives"][0]["id"].clone();
    let predicates = vec![
        json!({"op":"any","predicates":[{"op":"known","field":{"field":"quest","quest":"demo-quest-1","state":"accepted"}},{"op":"known","field":{"field":"objective","objective":objective}}]}),
        json!({"op":"not","predicate":{"op":"equals","field":{"field":"level"},"value":{"type":"integer","value":1}}}),
        json!({"op":"objective_state","objective":objective,"completed":false}),
        json!({"op":"location_in_supported_area","area":{"map":map,"bounds":null}}),
        json!({"op":"equipment_matches","slot":"head","item":item,"modifiers":{}}),
    ];
    v["models"][0]["applicability"] =
        json!({"status":"known","value":{"op":"all","predicates":predicates}});
    parse(&v).unwrap();
    for path in [
        "/models/0/applicability/value/predicates/0/predicates/0/field/quest",
        "/models/0/applicability/value/predicates/0/predicates/1/field/objective",
        "/models/0/applicability/value/predicates/2/objective",
        "/models/0/applicability/value/predicates/3/area/map/map_id",
    ] {
        let mut missing = v.clone();
        *missing.pointer_mut(path).unwrap() = json!("unresolved-synthetic-identity");
        assert!(parse(&missing).is_err());
    }
    let mut missing_item = v.clone();
    missing_item["models"][0]["applicability"]["value"]["predicates"][4]["item"]["item_id"] =
        json!("unresolved-synthetic-item");
    assert!(parse(&missing_item).is_err());
    let mut missing_node = v.clone();
    missing_node["dungeons"][0]["entrance"]["value"]["id"] = json!("unresolved-synthetic-node");
    assert!(parse(&missing_node).is_err());
    for kind in ["documented_model", "permitted_guide"] {
        let mut documented = v.clone();
        documented["models"][0]["profile"]["weight_source"] =
            json!({"type":kind,"source":"demo-source-1"});
        parse(&documented).unwrap();
    }
    v["models"][0]["minimum_level"] = json!(60);
    v["models"][0]["maximum_level"] = json!(1);
    assert!(parse(&v).is_err());
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
fn acquisition_origins_resolve_quest_vendor_and_dungeon_dependencies() {
    let base = fixture();
    let node = base["dungeons"][0]["entrance"]["value"].clone();
    for origin in [
        json!({"kind":"quest","quest":"demo-quest-1"}),
        json!({"kind":"vendor","entity":"synthetic-vendor","location":node}),
        json!({"kind":"drop","entity":"synthetic-drop","dungeon":"dungeon-1"}),
        json!({"kind":"craft","recipe":"synthetic-recipe","profession":"synthetic-profession"}),
    ] {
        let mut v = base.clone();
        v["reference"]["payload"]["acquisitions"][0]["kind"] =
            match origin["kind"].as_str().unwrap() {
                "craft" => json!("crafting"),
                "drop" => json!("dungeon"),
                value => json!(value),
            };
        v["acquisition_facts"][0]["origin"] = json!({"status":"known","value":origin});
        parse(&v).unwrap();
    }
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

#[test]
fn generated_schema_matches_committed_artifact() {
    assert_eq!(
        azeroth_core::content_v2::schema::reference_content_schema_text(),
        include_str!("../../../schemas/v2/reference-content.schema.json")
    );
    let schema = azeroth_core::content_v2::schema::reference_content_schema();
    assert_eq!(schema["properties"]["schema_version"]["const"], json!(2));
    assert_eq!(
        schema["$defs"]["RoleProfileData"]["additionalProperties"],
        json!(false)
    );
}
#[test]
#[ignore = "explicit fixture regeneration only"]
fn regenerate_content_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/contracts/v2");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("reference-content.json"),
        format!("{}\n", serde_json::to_string_pretty(&fixture()).unwrap()),
    )
    .unwrap();
}

#[test]
fn extension_facts_cannot_contradict_known_base_semantics() {
    let mut v = fixture();
    v["quest_facts"][0]["reset"] = json!({"status":"known","value":{"kind":"repeatable"}});
    assert!(parse(&v).is_err());
    let mut v = fixture();
    v["reference"]["payload"]["acquisitions"][0]["kind"] = "quest".into();
    v["acquisition_facts"][0]["origin"] = json!({"status":"known","value":{"kind":"craft","recipe":"recipe-1","profession":"profession-1"}});
    assert!(parse(&v).is_err());
}

#[test]
fn optional_node_facts_preserve_old_serialization_and_reject_invalid_provenance() {
    let original = fixture();
    let old = decode(&serde_json::to_vec(&original).unwrap()).unwrap();
    let canonical = serde_json::to_vec(&old).unwrap();
    assert!(
        !serde_json::to_value(&old)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("node_facts")
    );
    let mut explicit_empty = original.clone();
    explicit_empty["node_facts"] = json!([]);
    assert_eq!(
        serde_json::to_vec(&decode(&serde_json::to_vec(&explicit_empty).unwrap()).unwrap())
            .unwrap(),
        canonical
    );
    let mut populated = original;
    populated["node_facts"] = json!([{"node":"demo-node-1","evidence":["demo-evidence-1"]}]);
    let valid = decode(&serde_json::to_vec(&populated).unwrap()).unwrap();
    assert_eq!(valid.identity_count(), old.identity_count() + 1);
    for evidence in [
        json!([]),
        json!(["missing"]),
        json!(["demo-evidence-1", "demo-evidence-1"]),
    ] {
        let mut bad = populated.clone();
        bad["node_facts"][0]["evidence"] = evidence;
        assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut bad = populated.clone();
    bad["node_facts"][0]["node"] = json!("unrelated-node");
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = populated.clone();
    bad["node_facts"]
        .as_array_mut()
        .unwrap()
        .push(populated["node_facts"][0].clone());
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = populated.clone();
    bad["reference"]["payload"]["evidence"][0]["confidence"] = json!("disputed_or_stale");
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = populated;
    bad["node_facts"][0]["unexpected"] = json!(true);
    assert!(decode(&serde_json::to_vec(&bad).unwrap()).is_err());
}
