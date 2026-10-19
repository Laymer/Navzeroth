use azeroth_core::{
    content_v2::Dungeon,
    dungeon_composition::{DungeonField, enrich},
};
use serde_json::{Value, json};

fn value() -> Value {
    let catalog: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    serde_json::to_value(dungeon(&catalog["dungeons"][0])).unwrap()
}
fn dungeon(value: &Value) -> Dungeon {
    serde_json::from_value(value.clone()).unwrap()
}
fn known(value: Value) -> Value {
    json!({"status":"known", "value":value, "source":"unrecorded", "clock_trust":"uncertain"})
}

#[test]
fn unknown_fields_inherit_whole_observations_and_known_empty_remains_empty() {
    let mut high = value();
    let mut low = value();
    for field in ["entrance", "preparation_quests", "travel_edges"] {
        high[field] = json!({"status":"unknown"});
    }
    high["rewards"] = known(json!([]));
    low["evidence"] = json!(["historical-proof"]);
    high["bosses"] = json!(["npc:1"]);
    low["bosses"] = json!(["npc:1", "npc:2"]);
    let (result, conflicts) = enrich(&dungeon(&high), &dungeon(&low)).unwrap();
    let result = serde_json::to_value(result).unwrap();
    for field in ["entrance", "preparation_quests", "travel_edges"] {
        assert_eq!(result[field], low[field]);
    }
    assert_eq!(result["rewards"], high["rewards"]);
    assert_eq!(result["bosses"], json!(["npc:1", "npc:2"]));
    assert_eq!(
        result["evidence"],
        json!(["demo-evidence-1", "historical-proof"])
    );
    assert!(conflicts.iter().any(|c| c.field == DungeonField::Rewards));
}

#[test]
fn restricted_unsupported_not_applicable_and_false_never_become_known_true() {
    for status in ["restricted", "unsupported", "not_applicable"] {
        let mut high = value();
        high["entrance"] = json!({"status":status});
        high["checklist"] = json!({"status":status});
        let (result, conflicts) = enrich(&dungeon(&high), &dungeon(&value())).unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap()["entrance"]["status"],
            status
        );
        assert!(
            conflicts
                .iter()
                .any(|c| c.field == DungeonField::ChecklistCollection)
        );
        let mut unknown = value();
        unknown["entrance"] = json!({"status":"unknown"});
        let (result, _) = enrich(&dungeon(&unknown), &dungeon(&high)).unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap()["entrance"]["status"],
            status
        );
    }
    let mut high = value();
    let mut low = value();
    high["access"] = known(json!({"op":"not", "predicate":{"op":"all", "predicates":[]}}));
    low["access"] = known(json!({"op":"all", "predicates":[]}));
    let (result, conflicts) = enrich(&dungeon(&high), &dungeon(&low)).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["access"],
        high["access"]
    );
    assert!(conflicts.iter().any(|c| c.field == DungeonField::Access));
}

#[test]
fn only_the_documented_pending_map_marker_yields_to_historical_geometry() {
    let mut high = value();
    high["id"] = json!("dungeon:the-deadmines");
    high["map"]["map_id"] = json!("authored:the-deadmines");
    high["map"]["floor"] = json!("authored:pending");
    let mut low = value();
    low["id"] = high["id"].clone();
    let (result, conflicts) = enrich(&dungeon(&high), &dungeon(&low)).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap()["map"], low["map"]);
    assert!(
        conflicts
            .iter()
            .any(|c| c.field == DungeonField::Map && c.generated_preferred)
    );
    for (field, changed) in [
        ("map_id", json!("authored:real-map")),
        ("floor", json!("real-floor")),
        ("instance", json!("instance-id")),
    ] {
        let mut real = high.clone();
        real["map"][field] = changed;
        let (result, conflicts) = enrich(&dungeon(&real), &dungeon(&low)).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap()["map"], real["map"]);
        assert!(
            conflicts
                .iter()
                .any(|c| c.field == DungeonField::Map && !c.generated_preferred)
        );
    }
}

#[test]
fn independent_checks_are_additive_and_colliding_checks_preserve_authored_choices() {
    let mut high = value();
    let mut low = value();
    low["title"] = json!("Other source title");
    low["checklist"]["value"][0]["label"] = json!("Other label");
    low["checklist"]["value"][0]["condition"] = known(json!({"op":"all", "predicates":[]}));
    low["checklist"]["value"][0]["evidence"] = json!(["historical-proof"]);
    let mut warning = low["checklist"]["value"][0].clone();
    warning["id"] = json!("estimate-warning");
    warning["label"] = json!("Entrance estimate: portal and elevation unverified");
    low["checklist"]["value"]
        .as_array_mut()
        .unwrap()
        .push(warning);
    let (result, conflicts) = enrich(&dungeon(&high), &dungeon(&low)).unwrap();
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["checklist"]["value"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["checklist"]["value"][0]["label"],
        high["checklist"]["value"][0]["label"]
    );
    assert_eq!(
        result["checklist"]["value"][0]["condition"],
        low["checklist"]["value"][0]["condition"]
    );
    assert!(conflicts.iter().any(|c| c.field == DungeonField::Title));
    high["checklist"]["value"][0]["condition"] =
        known(json!({"op":"not", "predicate":{"op":"all", "predicates":[]}}));
    let (_, conflicts) = enrich(&dungeon(&high), &dungeon(&low)).unwrap();
    assert_eq!(
        conflicts
            .iter()
            .filter(|c| matches!(c.field, DungeonField::Checklist { .. }))
            .count(),
        1
    );
    high["checklist"] = json!({"status":"unknown"});
    let (result, _) = enrich(&dungeon(&high), &dungeon(&low)).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["checklist"],
        low["checklist"]
    );
}

#[test]
fn equal_records_are_idempotent_and_other_identities_or_contexts_refuse() {
    let original = value();
    let (result, conflicts) = enrich(&dungeon(&original), &dungeon(&original)).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), original);
    assert!(conflicts.is_empty());
    let mut other = original.clone();
    other["id"] = json!("different-dungeon");
    assert!(enrich(&dungeon(&original), &dungeon(&other)).is_err());
    other = original.clone();
    other["map"]["context"]["build"]["build"] = json!("different-build");
    assert!(enrich(&dungeon(&original), &dungeon(&other)).is_err());
}
