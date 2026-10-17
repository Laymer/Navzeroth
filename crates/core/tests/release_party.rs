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
    matrix.target.as_mut().unwrap().build = "70338".into();
    assert!(matrix.validate().is_err());
    let mut matrix = sample();
    matrix.rows[0].supporting_evidence.clear();
    assert!(matrix.validate().is_err());
    let mut value = serde_json::to_value(sample()).unwrap();
    value["schema_version"] = json!(3);
    assert!(SupportMatrix::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value["schema_version"] = json!(2);
    assert!(SupportMatrix::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value["schema_version"] = json!(1);
    value["unexpected"] = json!(false);
    assert!(SupportMatrix::decode(&serde_json::to_vec(&value).unwrap()).is_err());
}

fn sample_v2(sizes: &[u8]) -> SupportMatrix {
    let rows: Vec<_> = sizes
        .iter()
        .flat_map(|s| workflows_for(*s).map(move |w| (*s, w)))
        .enumerate()
        .map(|(n, (size, w))| {
            json!({"id":format!("row-{n}"),"size":size,"workflow":w,
            "dependencies":[{"domain":"quests","record_id":"[\"revision\",\"quests\",\"q1\"]"}],
            "procedure":{"path":"docs/scenario.txt","bytes":1,"sha256":HASH},
            "supporting_evidence":[{"path":"docs/facts.txt","bytes":2,"sha256":HASH}]})
        })
        .collect();
    serde_json::from_value(
        json!({"schema_version":2,"target":null,"catalog_sha256":HASH,
        "supported_sizes":sizes,"rows":rows}),
    )
    .unwrap()
}
#[test]
fn schema_two_admits_solo_with_code_fixed_workflows_and_a_pending_target() {
    let matrix = sample_v2(&[1, 2]);
    assert_eq!(matrix.rows.len(), 4 + 8);
    assert!(matrix.validate().is_ok());
    assert_eq!(matrix.bind(HASH, &catalog()).unwrap().len(), 12);
    let solo: Vec<_> = workflows_for(1).collect();
    assert_eq!(
        solo,
        [
            Workflow::JointPlanning,
            Workflow::StaleMembers,
            Workflow::Advancement,
            Workflow::Review
        ]
    );
    assert!(WORKFLOWS.iter().all(|w| applies(2, *w)));
    // A not-applicable solo workflow cannot be declared, and none can be omitted.
    let mut extra = sample_v2(&[1]);
    extra.rows[0].workflow = Workflow::Sharing;
    assert!(extra.validate().is_err());
    let mut missing = sample_v2(&[1, 2]);
    missing.rows.pop();
    assert!(missing.validate().is_err());
    // Schema 1 keeps its historical bounds; schema 2 cannot claim a target
    // before the code freezes one, and the member must be present.
    let mut v1_solo = serde_json::to_value(sample_v2(&[1])).unwrap();
    v1_solo["schema_version"] = json!(1);
    v1_solo["target"] = serde_json::to_value(Target::forever_70374()).unwrap();
    assert!(SupportMatrix::decode(&serde_json::to_vec(&v1_solo).unwrap()).is_err());
    let mut claimed = sample_v2(&[2]);
    claimed.target = Some(Target::forever_70374());
    assert!(claimed.validate().is_err());
    let mut absent = serde_json::to_value(sample_v2(&[2])).unwrap();
    absent.as_object_mut().unwrap().remove("target");
    assert!(SupportMatrix::decode(&serde_json::to_vec(&absent).unwrap()).is_err());
    for size in [0, 41] {
        let mut matrix = sample_v2(&[2]);
        matrix.supported_sizes[0] = size;
        assert!(matrix.validate().is_err());
    }
}
