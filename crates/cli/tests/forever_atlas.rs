//! The committed Eastern Kingdoms dataset says what it claims: 31 flight-master
//! pins measured on the flight layer, a provisional calibration labelled
//! inferred, placeholders declared in gaps, and no fabricated geometry.
use azeroth_core::{contract::*, entities::*};
use std::path::PathBuf;

#[test]
fn forever_atlas_r1_is_honest_about_its_provenance() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/forever/datasets/forever-atlas-r1.json");
    let envelope =
        decode_envelope::<ReferenceDataset>(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(!envelope.fixture);
    let data = &envelope.payload;
    assert!(
        data.nodes.is_empty() && data.edges.is_empty(),
        "no fabricated geometry"
    );
    assert_eq!(data.map_images.len(), 1);
    let image = &data.map_images[0];
    assert_eq!(image.pins.len(), 31);
    assert!(
        image
            .layers
            .iter()
            .any(|l| l.id.as_str() == "flight" && l.kind == ImageLayerKind::FlightReference)
    );
    let mut tower = 0;
    let mut both = 0;
    for pin in &image.pins {
        assert_eq!(pin.measured_on.as_str(), "flight");
        match &pin.kind {
            PinKind::FlightMaster { access } => {
                assert!(!access.is_empty());
                if access.contains(&FlightAccess::TowerRoute) {
                    tower += 1;
                }
                if access.contains(&FlightAccess::Alliance) && access.contains(&FlightAccess::Horde)
                {
                    both += 1;
                }
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(
        (tower, both),
        (4, 3),
        "index: four tower routes, three shared masters"
    );
    let calibration = image
        .calibration
        .value()
        .expect("provisional calibration is known");
    assert_eq!(calibration.units.as_str(), "pixel");
    assert_eq!(
        (calibration.scale_x.get(), calibration.scale_y.get()),
        (2091., 3825.)
    );
    let confidence = data
        .evidence
        .iter()
        .filter(|e| image.calibration_evidence.contains(&e.id))
        .map(|e| e.confidence)
        .collect::<Vec<_>>();
    assert_eq!(confidence, vec![Confidence::Inferred]);
    assert!(image.control_points.is_empty());
    assert_eq!(image.applicable_builds.len(), 1);
    for word in [
        "provisional",
        "approximate",
        "1415",
        "unverified",
        "not unlocks",
    ] {
        assert!(image.gaps.as_str().contains(word), "gaps mentions {word}");
    }
    for evidence in &data.evidence {
        assert!(
            evidence.license.value().is_none(),
            "third-party licences stay unrecorded"
        );
        assert_ne!(evidence.confidence, Confidence::VerifiedOnBuild);
    }
}
