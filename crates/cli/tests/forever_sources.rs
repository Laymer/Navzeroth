//! Existing source declarations are policy data, not newly verified permissions.
use azeroth_cli::pack::{MAX_SOURCES_BYTES, build, verify};
use azeroth_core::{
    contract::{ReferenceDataset, decode_envelope},
    observation::ObservationStatus,
    sources::{RecordClass, Redistribution, SourceRecord, SourceRegistry},
};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sources {
    schema_version: u32,
    sources: Vec<SourceRecord>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn sources() -> Vec<SourceRecord> {
    let bytes = fs::read(root().join("content/forever/sources.json")).unwrap();
    assert!(bytes.len() <= MAX_SOURCES_BYTES);
    let sources: Sources = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(sources.schema_version, 1);
    assert_eq!(sources.sources.len(), 4);
    sources.sources
}
#[test]
fn all_existing_dataset_citations_are_registered_without_relabeling() {
    let registry = SourceRegistry::try_from(sources()).unwrap();
    let data = decode_envelope::<ReferenceDataset>(
        &fs::read_to_string(root().join("content/forever/datasets/forever-atlas-r1.json")).unwrap(),
    )
    .unwrap();
    assert!(!data.fixture);
    assert_eq!(data.game.edition.as_str(), "wow_forever");
    assert_eq!(data.game.build.as_str(), "1.60.1.70205");
    assert_eq!(data.game.interface.as_str(), "16001");
    assert_eq!(data.payload.revision.as_str(), "forever-atlas-r1");
    for id in data
        .payload
        .evidence
        .iter()
        .map(|e| &e.source)
        .chain(data.payload.map_images.iter().map(|image| &image.source))
    {
        let source = registry.ensure_registered(id).unwrap();
        assert!(
            source
                .editions
                .value()
                .unwrap()
                .contains(&data.game.edition)
        );
    }
}
#[test]
fn owner_assertion_and_unknown_third_party_rights_remain_distinct() {
    for source in sources() {
        assert_eq!(source.license.status(), ObservationStatus::Unknown);
        assert_eq!(
            source.verified_revision.status(),
            ObservationStatus::Unknown
        );
        assert_eq!(source.record_class, RecordClass::SecondaryInference);
        assert_eq!(source.offline_usable.value(), Some(&true));
        if source.id.as_str() == "src-owner-ek-composite-2026-10-07" {
            assert_eq!(
                source.redistribution.value(),
                Some(&Redistribution::Conditional)
            );
            assert_eq!(
                source.last_retrieval_utc_ms.status(),
                ObservationStatus::Unknown
            );
        } else {
            assert_eq!(source.redistribution.status(), ObservationStatus::Unknown);
        }
        if source.id.as_str() == "src-blizzard-map-artwork" {
            assert_eq!(
                source.last_retrieval_utc_ms.status(),
                ObservationStatus::NotApplicable
            );
            assert_eq!(
                source.allowed_fields.status(),
                ObservationStatus::NotApplicable
            );
            assert_eq!(
                source.transformations.status(),
                ObservationStatus::NotApplicable
            );
        } else {
            assert_eq!(
                source.last_retrieval_utc_ms.status(),
                ObservationStatus::Unknown
            );
        }
    }
}
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn each_existing_policy_blocks_distribution_in_a_valid_synthetic_pack() {
    let temp =
        Temp(std::env::temp_dir().join(format!("anav-forever-sources-{}", std::process::id())));
    fs::create_dir(&temp.0).unwrap();
    for (index, source) in sources().iter().enumerate() {
        let input = temp.0.join(format!("input-{index}"));
        fs::create_dir_all(input.join("datasets")).unwrap();
        fs::copy(
            root().join("content/forever/sources.json"),
            input.join("sources.json"),
        )
        .unwrap();
        let mut fixture: serde_json::Value = serde_json::from_slice(
            &fs::read(root().join("fixtures/contracts/v1/reference-dataset-imagery.json")).unwrap(),
        )
        .unwrap();
        for evidence in fixture["payload"]["evidence"].as_array_mut().unwrap() {
            evidence["source"] = source.id.as_str().into();
        }
        for image in fixture["payload"]["map_images"].as_array_mut().unwrap() {
            image["source"] = source.id.as_str().into();
        }
        fs::write(
            input.join("datasets/synthetic.json"),
            serde_json::to_vec(&fixture).unwrap(),
        )
        .unwrap();
        for name in ["roads", "flight", "overview"] {
            let relative = format!("atlas/synthetic-continent/layers/{name}.png");
            let target = input.join(&relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(root().join("content/demo").join(relative), target).unwrap();
        }
        let output = temp.0.join(format!("output-{index}"));
        assert!(!build(&input, &output).unwrap().distribution_eligible);
        assert!(!verify(&output).unwrap().distribution_eligible);
    }
}
