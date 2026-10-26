use azeroth_app::{
    catalog_pages::{PageInfo, RecordPage},
    planning::PlanningCatalog,
};
use serde_json::{Value, json};
#[test]
fn hosted_catalog_fixture_is_the_actual_serialized_rust_dto() {
    let info = |collection: &str, hash: &str| -> PageInfo {
        serde_json::from_value(json!({"revision":"actual-r","catalog_sha256":hash.repeat(64),"collection":collection,"offset":0,"total":1,"next_offset":null})).unwrap()
    };
    let nodes = info("nodes", "a");
    let objectives = info("objectives", "b");
    let catalog = PlanningCatalog {
        pages: [
            (nodes.collection, nodes),
            (objectives.collection, objectives),
        ]
        .into_iter()
        .collect(),
        snapshot_id: None,
        revision: Some(serde_json::from_value(json!("actual-r")).unwrap()),
        fixture: false,
        nodes: vec![],
        objectives: vec![],
    };
    let page = RecordPage {
        page: info("quests", "a"),
        records: vec![json!({"id":"actual-source-record"})],
    };
    let actual = json!({"planning_catalog":catalog,"quest_page":page});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/stable_catalog_dto.json");
    if std::env::var_os("UPDATE_STABLE_CATALOG_DTO").is_some() {
        std::fs::write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
    }
    let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        actual["planning_catalog"]["pages"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert!(actual["quest_page"].get("page").is_none());
}
