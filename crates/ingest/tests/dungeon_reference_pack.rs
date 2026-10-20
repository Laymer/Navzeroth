mod common;
use azeroth_ingest::pack;
use serde_json::{Value, json};
use std::fs;
#[test]
fn versioned_dungeon_associations_survive_pack_canonicalization_and_reverification() {
    let scratch = common::Scratch::new("dungeon-reference-pack");
    let input = scratch.0.join("input");
    fs::create_dir(&input).unwrap();
    let mut catalog: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let source = catalog["reference"]["payload"]["acquisitions"][0].clone();
    let mut ids = vec![];
    for n in 0..1025 {
        let id = format!("extra-source-{n}");
        let mut next = source.clone();
        next["id"] = json!(id);
        catalog["reference"]["payload"]["acquisitions"]
            .as_array_mut()
            .unwrap()
            .push(next);
        ids.push(id);
    }
    catalog["dungeons"][0]["rewards"] = json!({"status":"known","value":ids});
    fs::write(
        input.join("sources.json"),
        include_bytes!("../../../fixtures/content-pack/sources.json"),
    )
    .unwrap();
    fs::write(input.join("catalog.json"), catalog.to_string()).unwrap();
    assert!(pack::v2::build(&input, &scratch.0.join("legacy-refused")).is_err());
    catalog["dungeons"][0]["reference_contract_version"] = json!(2);
    fs::write(input.join("catalog.json"), catalog.to_string()).unwrap();
    let output = scratch.0.join("extended");
    pack::v2::build(&input, &output).unwrap();
    pack::verify(&output).unwrap();
    let verified = pack::v2::read_verified(&output).unwrap();
    assert_eq!(
        verified.content.dungeons[0].reference_contract_version,
        Some(2)
    );
    assert_eq!(
        verified.content.dungeons[0].rewards.value().unwrap().len(),
        1025
    );
}
