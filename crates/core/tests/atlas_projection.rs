use azeroth_core::{atlas::*, contract::*, entities::*, observation::ObservationStatus, types::*};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/contracts/v1/reference-dataset-imagery.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn dataset(doc: &Value) -> (DataContext, ReferenceDataset) {
    let envelope = decode_envelope::<ReferenceDataset>(&doc.to_string()).unwrap();
    let context = envelope.data_context();
    (context, envelope.into_inner().payload)
}
fn position(
    map_id: &str,
    floor: &str,
    instance: Option<&str>,
    build: &str,
    x: f64,
    y: f64,
) -> Position {
    Position {
        map: MapKey {
            context: DataContext {
                build: BuildKey {
                    edition: EditionId::new("wow_forever").unwrap(),
                    build: BuildId::new(build).unwrap(),
                    interface: InterfaceId::new("UNVERIFIED_DEMO").unwrap(),
                },
                namespace: DatasetNamespace::Demo,
            },
            map_id: MapId::new(map_id).unwrap(),
            floor: FloorId::new(floor).unwrap(),
            instance: instance.map(|i| InstanceId::new(i).unwrap()),
        },
        x: Normalized::new(x).unwrap(),
        y: Normalized::new(y).unwrap(),
    }
}
fn demo(map_id: &str, x: f64, y: f64) -> Position {
    position(map_id, "0", None, "UNVERIFIED_DEMO", x, y)
}
fn pixel(p: &Projection) -> (f64, f64) {
    (p.pixel.x.get(), p.pixel.y.get())
}

#[test]
fn calibrated_image_places_positions_and_reports_confidence() {
    let (context, data) = dataset(&fixture());
    let projection = project_position(&demo("demo-map-1", 0.5, 0.5), &context, &data).unwrap();
    assert_eq!(pixel(&projection), (32., 48.));
    assert!(projection.notices.is_empty());
    assert_eq!(
        projection.calibration_confidence,
        Some(Confidence::Inferred)
    );
    assert_eq!(projection.image.as_str(), "demo-image-1");
    let again = project_position(&demo("demo-map-1", 0.5, 0.5), &context, &data).unwrap();
    assert_eq!(again, projection, "deterministic");
    // Frame edges are inside the frame.
    assert!(project_position(&demo("demo-map-1", 1., 1.), &context, &data).is_ok());
}

#[test]
fn zone_positions_fall_back_through_embeddings() {
    let (context, data) = dataset(&fixture());
    let projection = project_position(&demo("demo-zone-1", 0., 1.), &context, &data).unwrap();
    assert_eq!(pixel(&projection), (16., 48.));
    assert_eq!(projection.notices.len(), 1);
    match &projection.notices[0] {
        ProjectionNotice::EmbeddingFallback { via } => {
            assert_eq!(via.len(), 1);
            assert_eq!(via[0].map_id.as_str(), "demo-zone-1");
        }
        other => panic!("{other:?}"),
    }
    // Two levels: a city inside the zone.
    let mut doc = fixture();
    let city = json!({"context":{"build":{"edition":"wow_forever","build":"UNVERIFIED_DEMO","interface":"UNVERIFIED_DEMO"},"namespace":"demo"},"map_id":"demo-city-1","floor":"1","instance":null});
    doc["payload"]["map_spaces"]
        .as_array_mut()
        .unwrap()
        .push(json!({"key":city,"transform":{"status":"unknown"},"evidence":[]}));
    let zone = doc["payload"]["map_embeddings"][0]["child"].clone();
    doc["payload"]["map_embeddings"]
        .as_array_mut()
        .unwrap()
        .push(json!({"child":city,"parent":zone,"bounds":{"min_x":0,"min_y":0,"max_x":0.5,"max_y":0.5},"evidence":[]}));
    let (context, data) = dataset(&doc);
    let projection = project_position(
        &position("demo-city-1", "1", None, "UNVERIFIED_DEMO", 1., 1.),
        &context,
        &data,
    )
    .unwrap();
    assert_eq!(pixel(&projection), (24., 36.));
    match &projection.notices[0] {
        ProjectionNotice::EmbeddingFallback { via } => assert_eq!(via.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_gap_is_explicit() {
    let (context, data) = dataset(&fixture());
    let mut reference = demo("demo-map-1", 0.5, 0.5);
    reference.map.context.namespace = DatasetNamespace::Reference;
    assert_eq!(
        project_position(&reference, &context, &data).unwrap_err(),
        ProjectionGap::NamespaceMismatch
    );
    match project_position(&demo("demo-map-9", 0.5, 0.5), &context, &data).unwrap_err() {
        ProjectionGap::NoImageForMap { map } => assert_eq!(map.map_id.as_str(), "demo-map-9"),
        other => panic!("{other:?}"),
    }
    match project_position(
        &position("demo-map-1", "0", Some("inst"), "UNVERIFIED_DEMO", 0.5, 0.5),
        &context,
        &data,
    )
    .unwrap_err()
    {
        ProjectionGap::InInstance { .. } => {}
        other => panic!("{other:?}"),
    }
    match project_position(
        &position("demo-map-1", "1", None, "UNVERIFIED_DEMO", 0.5, 0.5),
        &context,
        &data,
    )
    .unwrap_err()
    {
        ProjectionGap::DifferentFloor {
            available_floors, ..
        } => {
            assert_eq!(available_floors.len(), 1);
            assert_eq!(available_floors[0].as_str(), "0");
        }
        other => panic!("{other:?}"),
    }
    // A build that is not listed never projects, directly or through an embedding.
    match project_position(
        &position("demo-map-1", "0", None, "LATER", 0.5, 0.5),
        &context,
        &data,
    )
    .unwrap_err()
    {
        ProjectionGap::BuildMismatch {
            image,
            applicable_builds,
            ..
        } => {
            assert_eq!(image.unwrap().as_str(), "demo-image-1");
            assert_eq!(applicable_builds.len(), 1);
        }
        other => panic!("{other:?}"),
    }
    match project_position(
        &position("demo-zone-1", "0", None, "LATER", 0.5, 0.5),
        &context,
        &data,
    )
    .unwrap_err()
    {
        ProjectionGap::BuildMismatch { image: None, .. } => {}
        other => panic!("{other:?}"),
    }
    // Uncalibrated imagery never draws a marker.
    let mut doc = fixture();
    doc["payload"]["map_images"][0]["calibration"] = json!({"status":"unsupported"});
    doc["payload"]["map_images"][0]["control_points"] = json!([]);
    let (context, data) = dataset(&doc);
    match project_position(&demo("demo-map-1", 0.5, 0.5), &context, &data).unwrap_err() {
        ProjectionGap::UncalibratedImage { status, .. } => {
            assert_eq!(status, ObservationStatus::Unsupported)
        }
        other => panic!("{other:?}"),
    }
    // A calibration that lands outside the frame is reported, not clamped onto it.
    let mut doc = fixture();
    doc["payload"]["map_images"][0]["calibration"]["value"]["offset_x"] = json!(60);
    doc["payload"]["map_images"][0]["control_points"] = json!([]);
    let (context, data) = dataset(&doc);
    match project_position(&demo("demo-map-1", 0.5, 0.5), &context, &data).unwrap_err() {
        ProjectionGap::OutOfBounds { pixel, .. } => assert_eq!(pixel.x.get(), 92.),
        other => panic!("{other:?}"),
    }
}

#[test]
fn listed_foreign_builds_project_with_a_visible_notice() {
    let mut doc = fixture();
    doc["payload"]["map_images"][0]["applicable_builds"]
        .as_array_mut()
        .unwrap()
        .push(json!({"build":{"edition":"wow_forever","build":"LATER","interface":"UNVERIFIED_DEMO"},"evidence":["demo-evidence-1"]}));
    let (context, data) = dataset(&doc);
    let projection = project_position(
        &position("demo-map-1", "0", None, "LATER", 0.25, 0.25),
        &context,
        &data,
    )
    .unwrap();
    assert_eq!(pixel(&projection), (16., 24.));
    assert!(matches!(
        projection.notices[0],
        ProjectionNotice::CrossBuildReuse { .. }
    ));
    // Embeddings still require the exact context: the zone of a listed build does not resolve.
    assert!(matches!(
        project_position(
            &position("demo-zone-1", "0", None, "LATER", 0.25, 0.25),
            &context,
            &data
        ),
        Err(ProjectionGap::BuildMismatch { image: None, .. })
    ));
}

#[test]
fn datasets_reject_embedding_cycles_and_deep_chains() {
    let mut doc = fixture();
    let (child, parent) = (
        doc["payload"]["map_embeddings"][0]["child"].clone(),
        doc["payload"]["map_embeddings"][0]["parent"].clone(),
    );
    doc["payload"]["map_embeddings"]
        .as_array_mut()
        .unwrap()
        .push(json!({"child":parent,"parent":child,"bounds":{"min_x":0,"min_y":0,"max_x":1,"max_y":1},"evidence":[]}));
    assert!(decode_envelope::<ReferenceDataset>(&doc.to_string()).is_err());

    let mut doc = fixture();
    let spaces = doc["payload"]["map_spaces"].as_array_mut().unwrap();
    let template = spaces[0].clone();
    for i in 2..=6 {
        let mut space = template.clone();
        space["key"]["map_id"] = json!(format!("demo-level-{i}"));
        spaces.push(space);
    }
    let embeddings = doc["payload"]["map_embeddings"].as_array_mut().unwrap();
    let key = |id: &str| {
        let mut k = embeddings[0]["child"].clone();
        k["map_id"] = json!(id);
        k
    };
    let chain: Vec<_> = ["demo-level-6", "demo-level-5", "demo-level-4", "demo-level-3", "demo-level-2", "demo-map-1"]
        .windows(2)
        .map(|w| json!({"child":key(w[0]),"parent":key(w[1]),"bounds":{"min_x":0,"min_y":0,"max_x":1,"max_y":1},"evidence":[]}))
        .collect();
    embeddings.extend(chain);
    let err = decode_envelope::<ReferenceDataset>(&doc.to_string()).unwrap_err();
    assert_eq!(err.code, azeroth_core::error::ErrorCode::InvalidInput);
}

fn pair(nx: f64, ny: f64, px: f64, py: f64) -> CalibrationPair {
    CalibrationPair {
        normalized_x: Normalized::new(nx).unwrap(),
        normalized_y: Normalized::new(ny).unwrap(),
        pixel: PixelPoint {
            x: FiniteNumber::new(px).unwrap(),
            y: FiniteNumber::new(py).unwrap(),
        },
    }
}

#[test]
fn solver_recovers_transforms_and_reports_residuals() {
    let (sx, sy, ox, oy) = (2000., -3000., 15., 2990.);
    let pairs: Vec<_> = [(0.1, 0.2), (0.4, 0.9), (0.7, 0.3), (0.95, 0.6), (0.2, 0.8)]
        .iter()
        .map(|&(x, y)| pair(x, y, sx * x + ox, sy * y + oy))
        .collect();
    let solution = solve_axis_aligned_calibration(&pairs).unwrap();
    assert!((solution.transform.scale_x.get() - sx).abs() < 1e-6);
    assert!((solution.transform.scale_y.get() - sy).abs() < 1e-6);
    assert!((solution.transform.offset_x.get() - ox).abs() < 1e-6);
    assert!((solution.transform.offset_y.get() - oy).abs() < 1e-6);
    assert!(solution.rms_px.get() < 1e-6 && solution.max_px.get() < 1e-6);
    assert_eq!(solution.transform.units.as_str(), "pixel");
    assert_eq!(solve_axis_aligned_calibration(&pairs).unwrap(), solution);

    let mut perturbed = pairs.clone();
    perturbed[2].pixel.x = FiniteNumber::new(sx * 0.7 + ox + 10.).unwrap();
    let solution = solve_axis_aligned_calibration(&perturbed).unwrap();
    assert!(solution.max_px.get() > 1. && solution.rms_px.get() > 0.5);
    assert_eq!(solution.residuals_px.len(), 5);

    assert_eq!(
        solve_axis_aligned_calibration(&pairs[..2]).unwrap_err(),
        CalibrationError::TooFewPoints { needed: 3, got: 2 }
    );
    let collinear = [
        pair(0.5, 0.1, 1., 1.),
        pair(0.5, 0.5, 2., 2.),
        pair(0.5, 0.9, 3., 3.),
    ];
    assert_eq!(
        solve_axis_aligned_calibration(&collinear).unwrap_err(),
        CalibrationError::DegenerateSpread
    );
}
