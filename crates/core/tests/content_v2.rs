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

fn with_entity(entity: Value) -> Value {
    let mut v = fixture();
    v["entities"] = json!([entity]);
    v
}
fn entity() -> Value {
    json!({"id":"npc:448","evidence":["demo-evidence-1"],"kind":"npc","name":"Hogger",
        "subname":{"status":"unknown"},"levels":{"status":"known","value":[11,11]},
        "services":["vendor"],"reaction":{"status":"known","value":"hostile"},"nodes":["demo-node-1"]})
}
#[test]
fn entities_resolve_nodes_and_evidence() {
    let content = parse(&with_entity(entity())).unwrap();
    assert_eq!(content.entities.len(), 1);
    assert_eq!(content.entities[0].name.as_str(), "Hogger");
    let mut bad = entity();
    bad["nodes"] = json!(["missing-node"]);
    assert!(parse(&with_entity(bad)).is_err());
    let mut bad = entity();
    bad["levels"] = json!({"status":"known","value":[12,11]});
    assert!(parse(&with_entity(bad)).is_err());
    let mut bad = entity();
    bad["evidence"] = json!(["missing-evidence"]);
    assert!(parse(&with_entity(bad)).is_err());
    let mut twice = fixture();
    twice["entities"] = json!([entity(), entity()]);
    assert!(parse(&twice).is_err());
    let mut empty = fixture();
    empty["entities"] = json!([]);
    assert!(parse(&empty).is_ok());
}

fn loot_table(id: &str, table: &str, rows: Value) -> Value {
    json!({"id":id,"evidence":["demo-evidence-1"],"table":table,"source":{"status":"unknown"},"rows":rows})
}
fn with_loot(tables: Value) -> Value {
    let mut v = fixture();
    v["loot_tables"] = tables;
    v
}
#[test]
fn loot_tables_keep_rows_and_references_without_expansion() {
    let reference = loot_table(
        "loot:reference:1",
        "reference",
        json!([{"item":"77","min_count":1,"max_count":1,"group":1}]),
    );
    let creature = json!({"id":"loot:creature:448","evidence":["demo-evidence-1"],"table":"creature",
        "source":{"status":"known","value":"npc:448"},
        "rows":[{"item":"1931","min_count":1,"max_count":1,"probability":1.0,"quest_only":true},
            {"item":"2589","min_count":1,"max_count":5,"probability":0.0},
            {"table":"loot:reference:1","min_count":1,"max_count":2,"probability":0.02,"conditional":true}]});
    let content = parse(&with_loot(json!([reference.clone(), creature]))).unwrap();
    assert_eq!(content.loot_tables.len(), 2);
    let rows = &content.loot_tables[1].rows;
    assert!(rows[0].quest_only && !rows[1].quest_only && rows[2].conditional);
    // A stated zero stays zero; an absent probability stays unknown.
    assert_eq!(rows[1].probability.map(|p| p.get()), Some(0.0));
    assert!(content.loot_tables[0].rows[0].probability.is_none());
    // Absent optional fields stay absent when serialized again.
    let encoded = serde_json::to_value(&content.loot_tables[0]).unwrap();
    assert!(encoded["rows"][0].get("probability").is_none());
    assert!(encoded["rows"][0].get("quest_only").is_none());
    assert_eq!(
        content.identity_count(),
        parse(&fixture()).unwrap().identity_count() + 2
    );
    let mut empty = fixture();
    empty["loot_tables"] = json!([]);
    assert!(parse(&empty).is_ok());

    let bad_rows = [
        json!([]),
        json!([{"min_count":1,"max_count":1}]),
        json!([{"item":"1","table":"loot:reference:1","min_count":1,"max_count":1}]),
        json!([{"item":"1","min_count":2,"max_count":1}]),
        json!([{"table":"loot:reference:missing","min_count":1,"max_count":1}]),
        json!([{"table":"loot:creature:448","min_count":1,"max_count":1}]),
        json!([{"item":"1","min_count":1,"max_count":1,"probability":1.5}]),
        json!([{"item":"1","min_count":1,"max_count":1,"unexpected":true}]),
    ];
    for rows in bad_rows {
        let table = loot_table("loot:creature:448", "creature", rows);
        assert!(parse(&with_loot(json!([reference.clone(), table]))).is_err());
    }
    let row = json!({"item":"1","min_count":1,"max_count":1});
    let full = loot_table(
        "loot:game_object:1",
        "game_object",
        Value::Array(vec![row.clone(); azeroth_core::content_v2::MAX_LOOT_ROWS]),
    );
    assert!(parse(&with_loot(json!([full]))).is_ok());
    let over = loot_table(
        "loot:game_object:1",
        "game_object",
        Value::Array(vec![row; azeroth_core::content_v2::MAX_LOOT_ROWS + 1]),
    );
    assert!(parse(&with_loot(json!([over]))).is_err());
    assert!(parse(&with_loot(json!([reference.clone(), reference.clone()]))).is_err());
    let mut unproven = reference;
    unproven["evidence"] = json!(["missing-evidence"]);
    assert!(parse(&with_loot(json!([unproven]))).is_err());
}
#[test]
fn optional_names_bosses_and_quest_parties_resolve_and_keep_old_bytes() {
    let mut v = with_entity(entity());
    v["dungeons"][0]["bosses"] = json!(["npc:448"]);
    v["equipment"][0]["name"] = json!("Synthetic blade");
    v["quest_facts"][0]["title"] = json!("Wanted: Hogger");
    v["quest_facts"][0]["givers"] = json!(["object:68", "item:16305"]);
    v["quest_facts"][0]["enders"] = json!(["npc:240"]);
    let content = parse(&v).unwrap();
    assert_eq!(content.dungeons[0].bosses[0].as_str(), "npc:448");
    assert_eq!(
        content.equipment[0].name.as_ref().map(|n| n.as_str()),
        Some("Synthetic blade")
    );
    let quest = &content.quest_facts[0];
    assert_eq!(
        quest.title.as_ref().map(|t| t.as_str()),
        Some("Wanted: Hogger")
    );
    assert_eq!((quest.givers.len(), quest.enders.len()), (2, 1));
    let mut bad = v.clone();
    bad["quest_facts"][0]["enders"] = json!(["npc:240", "npc:240"]);
    assert!(parse(&bad).is_err());
    v["dungeons"][0]["bosses"] = json!(["npc:999"]);
    assert!(parse(&v).is_err());
    v["dungeons"][0]["bosses"] = json!(["npc:448", "npc:448"]);
    assert!(parse(&v).is_err());
    // Absent optional fields keep the old serialization.
    let encoded = serde_json::to_value(parse(&fixture()).unwrap()).unwrap();
    assert!(encoded["dungeons"][0].get("bosses").is_none());
    assert!(encoded["equipment"][0].get("name").is_none());
    assert!(encoded["quest_facts"][0].get("title").is_none());
    assert!(encoded["quest_facts"][0].get("givers").is_none());
    assert!(encoded.get("loot_tables").is_none());
}
#[test]
fn whole_world_limits_have_headroom_and_stay_bounded() {
    use azeroth_core::contract::{MAX_CATALOG_EVIDENCE, MAX_CATALOG_IDENTITIES};
    // Measured world pack (task V2.8d): 186,665 identities, 27 evidence records
    // and a 273,541,841-byte canonical catalog; the limits keep about 2x headroom.
    const WORLD_IDENTITIES: usize = 186_665;
    const WORLD_CATALOG_BYTES: usize = 273_541_841;
    const { assert!(MAX_CATALOG_IDENTITIES >= 2 * WORLD_IDENTITIES) };
    const { assert!(MAX_CONTENT_BYTES >= 2 * WORLD_CATALOG_BYTES) };
    const { assert!(MAX_CATALOG_EVIDENCE >= 27) };
    assert!(decode(&vec![b' '; MAX_CONTENT_BYTES + 1]).is_err());
}
#[test]
fn loot_tables_bind_their_own_release_identity() {
    let table = loot_table(
        "loot:creature:448",
        "creature",
        json!([{"item":"1931","min_count":1,"max_count":1,"probability":1.0}]),
    );
    let mut v = with_loot(json!([table]));
    v["entities"] = json!([entity()]);
    let content = parse(&v).unwrap();
    let records = azeroth_core::content_v2::records::content_records(&content).unwrap();
    let revision = content.reference.payload.revision.as_str();
    let key = |parts: Value| {
        let mut all = vec![json!(revision)];
        all.extend(parts.as_array().unwrap().iter().cloned());
        serde_json::to_string(&all).unwrap()
    };
    assert!(!records["acquisition"][&key(json!(["loot_tables", "loot:creature:448"]))].is_empty());
    assert!(!records["geography"][&key(json!(["entities", "npc:448"]))].is_empty());
}

fn dungeon_refs(count: usize) -> Value {
    let mut value = fixture();
    for (collection, field) in [
        ("quests", "preparation_quests"),
        ("acquisitions", "rewards"),
    ] {
        let template = value["reference"]["payload"][collection][0].clone();
        let mut ids = vec![];
        for n in 0..count {
            let id = format!("extra-{collection}-{n}");
            let mut record = template.clone();
            record["id"] = json!(id);
            value["reference"]["payload"][collection]
                .as_array_mut()
                .unwrap()
                .push(record);
            ids.push(id);
        }
        value["dungeons"][0][field] = json!({"status":"known", "value":ids});
    }
    value
}
#[test]
fn dungeon_reference_versions_preserve_canonical_legacy_and_explicit_one() {
    let legacy = parse(&fixture()).unwrap();
    assert!(
        serde_json::to_value(&legacy).unwrap()["dungeons"][0]
            .get("reference_contract_version")
            .is_none()
    );
    for version in [1, 2] {
        let mut v = fixture();
        v["dungeons"][0]["reference_contract_version"] = json!(version);
        let decoded = parse(&v).unwrap();
        let encoded = serde_json::to_vec(&decoded).unwrap();
        let roundtrip = decode(&encoded).unwrap();
        assert_eq!(
            roundtrip.dungeons[0].reference_contract_version,
            Some(version)
        );
    }
    for invalid in [
        json!(null),
        json!(0),
        json!(3),
        json!("2"),
        json!(-1),
        json!(2.5),
        json!(true),
    ] {
        let mut v = fixture();
        v["dungeons"][0]["reference_contract_version"] = invalid;
        assert!(parse(&v).is_err());
    }
}
#[test]
fn dungeon_reference_bounds_are_scoped_and_keep_resolution_and_uniqueness() {
    let mut v = dungeon_refs(4096);
    assert!(parse(&v).is_err());
    v["dungeons"][0]["reference_contract_version"] = json!(2);
    let content = parse(&v).unwrap();
    assert_eq!(content.dungeons[0].rewards.value().unwrap().len(), 4096);
    assert_eq!(
        content.dungeons[0]
            .preparation_quests
            .value()
            .unwrap()
            .len(),
        4096
    );
    let extended = content.dungeons[0].clone();
    let mut legacy = extended.clone();
    legacy.reference_contract_version = None;
    legacy.preparation_quests = azeroth_core::observation::Observation::unknown();
    legacy.rewards = azeroth_core::observation::Observation::unknown();
    for (a, b) in [(&legacy, &extended), (&extended, &legacy)] {
        let (merged, _) = azeroth_core::dungeon_composition::enrich(a, b).unwrap();
        let mut composed = content.clone();
        composed.dungeons[0] = merged;
        let decoded = decode(&serde_json::to_vec(&composed).unwrap()).unwrap();
        assert_eq!(decoded.dungeons[0].rewards.value().unwrap().len(), 4096);
    }
    for field in ["preparation_quests", "rewards"] {
        let original = v["dungeons"][0][field].clone();
        let refs = v["dungeons"][0][field]["value"].as_array_mut().unwrap();
        refs[1] = refs[0].clone();
        assert!(parse(&v).is_err());
        v["dungeons"][0][field] = original.clone();
        v["dungeons"][0][field]["value"][0] = json!("unresolved");
        assert!(parse(&v).is_err());
        v["dungeons"][0][field] = original;
    }
    v["dungeons"][0]["travel_edges"] = json!({"status":"known", "value":vec!["demo-edge-1";1025]});
    assert!(parse(&v).is_err());
    let mut oversized = dungeon_refs(4097);
    oversized["dungeons"][0]["reference_contract_version"] = json!(2);
    assert!(parse(&oversized).is_err());
    assert!(parse(&dungeon_refs(1024)).is_ok());
    assert!(parse(&dungeon_refs(1025)).is_err());
}

#[test]
fn dungeon_reference_schema_is_optional_and_nonnullable() {
    let schema = azeroth_core::content_v2::schema::reference_content_schema();
    let dungeon = &schema["$defs"]["Dungeon"];
    assert_eq!(
        dungeon["properties"]["reference_contract_version"]["type"],
        json!("integer")
    );
    assert_eq!(
        dungeon["properties"]["reference_contract_version"]["enum"],
        json!([1, 2])
    );
    assert!(
        !dungeon["required"]
            .as_array()
            .unwrap()
            .contains(&json!("reference_contract_version"))
    );
}

#[test]
fn borrowed_value_decode_matches_historical_consuming_deserializer_for_optional_forms() {
    let original = fixture();
    for explicit_empty in [false, true] {
        let mut value = original.clone();
        if explicit_empty {
            for key in ["node_facts", "entities", "loot_tables"] {
                value[key] = json!([]);
            }
            value["reference"]["payload"]["map_images"] = json!([]);
            value["reference"]["payload"]["map_embeddings"] = json!([]);
            value["reference"]["dependencies"] = json!([]);
        }
        let legacy: azeroth_core::content_v2::ReferenceContent =
            serde_json::from_value(value.clone()).unwrap();
        let current = parse(&value).unwrap();
        assert_eq!(
            serde_json::to_vec(&current).unwrap(),
            serde_json::to_vec(&legacy).unwrap()
        );
    }
}
