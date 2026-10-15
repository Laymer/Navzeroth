use azeroth_core::release::Target;
use azeroth_core::release_party::*;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
fn sample() -> SupportMatrix {
    serde_json::from_value(json!({"schema_version":1,"target":Target::forever_70374(),
        "catalog_sha256":HASH,"supported_sizes":[2],"rows":WORKFLOWS.iter().enumerate().map(|(n,w)|
        json!({"id":format!("row-{n}"),"size":2,"workflow":w,
            "dependencies":[{"domain":"quests","record_id":"[\"revision\",\"quests\",\"q1\"]"}],
            "procedure":{"path":"docs/scenario.txt","bytes":1,"sha256":HASH},
            "supporting_evidence":[{"path":"docs/facts.txt","bytes":2,"sha256":HASH}]})).collect::<Vec<_>>()})).unwrap()
}
fn catalog() -> ConcreteCatalog {
    BTreeMap::from([(
        "quests".into(),
        BTreeMap::from([(
            "[\"revision\",\"quests\",\"q1\"]".into(),
            BTreeSet::from(["source-1".into()]),
        )]),
    )])
}
#[test]
fn all_workflows_bind_concrete_content_and_exact_sources() {
    let matrix = sample();
    let rows = matrix.bind(HASH, &catalog()).unwrap();
    assert_eq!(rows.len(), 8);
    assert!(
        rows.values()
            .all(|s| s == &BTreeSet::from(["source-1".into()]))
    );
    assert_eq!(matrix.evidence_files().count(), 16);
    let bytes = serde_json::to_vec(&matrix).unwrap();
    assert!(SupportMatrix::decode(&bytes).is_ok());
}
#[test]
fn local_profiles_and_synthetic_scenario_ids_cannot_replace_content() {
    let mut matrix = sample();
    assert!(matrix.bind(HASH, &ConcreteCatalog::new()).is_err());
    matrix.rows[0].dependencies[0].record_id = "local-profile".into();
    assert!(matrix.bind(HASH, &catalog()).is_err());
    matrix.rows[0].dependencies[0].domain = "party".into();
    assert!(matrix.validate().is_err());
}
#[test]
fn every_size_requires_every_workflow_once() {
    let mut matrix = sample();
    matrix.supported_sizes.push(3);
    assert!(matrix.validate().is_err());
    matrix.supported_sizes.pop();
    matrix.rows[0].workflow = matrix.rows[1].workflow;
    assert!(matrix.validate().is_err());
    for size in [0, 1, 41, 255] {
        let mut matrix = sample();
        matrix.supported_sizes[0] = size;
        assert!(matrix.validate().is_err());
    }
}
#[test]
fn stale_catalog_mixed_target_empty_evidence_and_unknown_version_refuse() {
    let matrix = sample();
    assert!(matrix.bind(&"b".repeat(64), &catalog()).is_err());
    let mut matrix = sample();
    matrix.target.build = "70338".into();
    assert!(matrix.validate().is_err());
    let mut matrix = sample();
    matrix.rows[0].supporting_evidence.clear();
    assert!(matrix.validate().is_err());
    let mut value = serde_json::to_value(sample()).unwrap();
    value["schema_version"] = json!(2);
    assert!(SupportMatrix::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value["schema_version"] = json!(1);
    value["unexpected"] = json!(false);
    assert!(SupportMatrix::decode(&serde_json::to_vec(&value).unwrap()).is_err());
}
