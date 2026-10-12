//! Atlas handler, resolver and golden-DTO tests over the synthetic demo atlas.
//! Every `now` is fixed; nothing here reads a clock.
use azeroth_app::DataMode;
use azeroth_app::atlas::*;
use azeroth_core::atlas::{ProjectionGap, ProjectionNotice};
use azeroth_core::contract::{Envelope, ReferenceDataset, decode_envelope};
use azeroth_core::entities::{CharacterSnapshot, Confidence, ImageLayerKind};
use azeroth_core::error::ErrorCode;
use azeroth_core::observation::{ClockUncertainty, Freshness, ObservationStatus};
use azeroth_core::types::{AssetPath, DatasetNamespace, Millis, Position, UtcMillis};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::PathBuf;

const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
const DATASET: &str = include_str!("../../../fixtures/contracts/v1/reference-dataset-imagery.json");
/// The demo snapshot's location was observed at this instant.
const OBSERVED: i64 = 1_791_791_990_000;

fn now(offset_ms: i64) -> Option<UtcMillis> {
    Some(UtcMillis::new(OBSERVED + offset_ms).unwrap())
}

fn position(map: &str, floor: &str, build: &str, namespace: &str, x: f64, y: f64) -> Position {
    serde_json::from_value(json!({
        "map": {
            "context": {
                "build": {"edition": "wow_forever", "build": build, "interface": "UNVERIFIED_DEMO"},
                "namespace": namespace
            },
            "map_id": map, "floor": floor, "instance": null
        },
        "x": x, "y": y
    }))
    .unwrap()
}
fn demo(map: &str, x: f64, y: f64) -> Position {
    position(map, "0", "UNVERIFIED_DEMO", "demo", x, y)
}

fn dataset() -> ReferenceDataset {
    decode_envelope::<ReferenceDataset>(DATASET)
        .unwrap()
        .into_inner()
        .payload
}

fn snapshot_with_location(edit: impl FnOnce(&mut Value)) -> Envelope<CharacterSnapshot> {
    let mut doc: Value = serde_json::from_str(SNAPSHOT).unwrap();
    edit(&mut doc["payload"]["location"]);
    decode_envelope(&doc.to_string()).unwrap()
}

// --- query_atlas ----------------------------------------------------------

#[test]
fn query_atlas_describes_the_labelled_demo_dataset() {
    let view = query_atlas_with(None, SchemeStyle::Custom).unwrap();
    assert_eq!(view.revision.as_str(), "demo-imagery-r1");
    assert_eq!(view.namespace, DatasetNamespace::Demo);
    assert!(view.fixture);
    assert_eq!(view.data_mode, DataMode::Demo);
    let [image] = view.images.as_slice() else {
        panic!("expected one image")
    };
    assert_eq!(image.id.as_str(), "demo-image-1");
    let ids: Vec<_> = image.layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["roads", "flight", "overview"]);
    let dataset = dataset();
    for (layer, declared) in image.layers.iter().zip(&dataset.map_images[0].layers) {
        assert_eq!(layer.availability, LayerAvailability::Available);
        assert_eq!(layer.sha256, declared.sha256);
        assert_eq!(
            layer.url.as_deref(),
            Some(format!("atlas://localhost/demo-image-1/{}", layer.id.as_str()).as_str())
        );
    }
    assert_eq!(image.layers[1].kind, ImageLayerKind::FlightReference);
    assert!(!image.gaps.as_str().is_empty());
    assert!(!image.attribution.as_str().is_empty());
    assert_eq!(image.calibration_status, ObservationStatus::Known);
    assert_eq!(image.calibration_confidence, Some(Confidence::Inferred));
    assert!(image.approximate, "inferred calibration is approximate");
    assert!(!image.pins.is_empty());
    assert!(image.pins.iter().all(|p| p.reference_only));
}

#[test]
fn query_atlas_narrows_to_one_image_and_rejects_unknown_ones_safely() {
    assert_eq!(
        query_atlas_with(Some("demo-image-1"), SchemeStyle::Custom)
            .unwrap()
            .images
            .len(),
        1
    );
    let err = query_atlas_with(Some("../../etc/passwd"), SchemeStyle::Custom).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(!err.to_string().contains("passwd"));
}

#[test]
fn layer_urls_follow_the_platform_scheme_style() {
    let view = query_atlas_with(None, SchemeStyle::HttpLocalhost).unwrap();
    assert_eq!(
        view.images[0].layers[0].url.as_deref(),
        Some("http://atlas.localhost/demo-image-1/roads")
    );
    let platform = query_atlas(None).unwrap();
    let expected = if cfg!(windows) {
        "http://atlas.localhost/demo-image-1/roads"
    } else {
        "atlas://localhost/demo-image-1/roads"
    };
    assert_eq!(platform.images[0].layers[0].url.as_deref(), Some(expected));
}

// --- project_positions ----------------------------------------------------

fn projection_cases() -> Vec<(&'static str, Position)> {
    vec![
        ("placed", demo("demo-map-1", 0.25, 1.0)),
        ("embedding_fallback", demo("demo-zone-1", 0.0, 1.0)),
        (
            "namespace_mismatch",
            position("demo-map-1", "0", "UNVERIFIED_DEMO", "reference", 0.5, 0.5),
        ),
        (
            "different_floor",
            position("demo-map-1", "1", "UNVERIFIED_DEMO", "demo", 0.5, 0.5),
        ),
        (
            "build_mismatch",
            position("demo-map-1", "0", "OTHER_BUILD", "demo", 0.5, 0.5),
        ),
    ]
}

#[test]
fn project_positions_places_and_explains() {
    let cases = projection_cases();
    let positions: Vec<_> = cases.iter().map(|(_, p)| p.clone()).collect();
    let out = project_positions(&positions).unwrap();
    assert_eq!(out.len(), 5);

    let ProjectedPosition::Placed(placed) = &out[0] else {
        panic!("{:?}", out[0])
    };
    assert_eq!(placed.image.as_str(), "demo-image-1");
    // y = 1.0 is the frame edge; edges are inclusive.
    assert_eq!((placed.pixel.x.get(), placed.pixel.y.get()), (16.0, 96.0));
    assert!(placed.notices.is_empty());

    let ProjectedPosition::Placed(zone) = &out[1] else {
        panic!("{:?}", out[1])
    };
    assert!(matches!(
        zone.notices.as_slice(),
        [ProjectionNotice::EmbeddingFallback { .. }]
    ));

    assert!(matches!(
        out[2],
        ProjectedPosition::Gap(ProjectionGap::NamespaceMismatch)
    ));
    assert!(matches!(
        out[3],
        ProjectedPosition::Gap(ProjectionGap::DifferentFloor { .. })
    ));
    assert!(matches!(
        out[4],
        ProjectedPosition::Gap(ProjectionGap::BuildMismatch { .. })
    ));
}

#[test]
fn project_positions_bounds_the_request_without_echoing_it() {
    let one = demo("demo-map-1", 0.5, 0.5);
    assert_eq!(project_positions(&vec![one.clone(); 64]).unwrap().len(), 64);
    for bad in [vec![], vec![one; 65]] {
        let err = project_positions(&bad).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert!(!err.retryable);
        assert_eq!(err.to_string(), "Provide between 1 and 64 positions");
    }
}

// --- get_snapshot_markers -------------------------------------------------

#[test]
fn snapshot_markers_carry_age_freshness_and_the_demo_label() {
    let fresh = &get_snapshot_markers(now(60_000)).unwrap()[0];
    assert_eq!(fresh.observed_at_utc_ms.map(UtcMillis::get), Some(OBSERVED));
    assert_eq!(
        fresh.received_at_utc_ms.map(UtcMillis::get),
        Some(1_791_792_005_000)
    );
    assert_eq!(
        fresh.freshness,
        Freshness::Fresh {
            age_ms: Millis::new(60_000).unwrap()
        }
    );
    assert!(fresh.fixture);
    assert_eq!(fresh.data_mode, DataMode::Demo);
    assert_eq!(fresh.approximate, Some(true));
    assert_eq!(fresh.profile_id.as_str(), "demo-character-paladin");
    let Some(ProjectedPosition::Placed(placed)) = &fresh.projection else {
        panic!("{:?}", fresh.projection)
    };
    assert_eq!(placed.calibration_confidence, Some(Confidence::Inferred));

    let stale = &get_snapshot_markers(now(31 * 60 * 1000)).unwrap()[0];
    assert!(matches!(stale.freshness, Freshness::Stale { .. }));

    let boundary = &get_snapshot_markers(now(30 * 60 * 1000)).unwrap()[0];
    assert!(matches!(boundary.freshness, Freshness::Fresh { .. }));

    let uncertain = &get_snapshot_markers(None).unwrap()[0];
    assert_eq!(
        uncertain.freshness,
        Freshness::Uncertain {
            reason: ClockUncertainty::MissingOrUntrustedTime
        }
    );
}

#[test]
fn unavailable_locations_never_become_a_pixel() {
    for status in ["unknown", "unsupported", "restricted", "not_applicable"] {
        let snapshot = snapshot_with_location(|location| {
            let object = location.as_object_mut().unwrap();
            object.remove("value");
            object.insert("status".into(), json!(status));
        });
        let marker = snapshot_marker_for(&snapshot, now(60_000)).unwrap();
        assert_ne!(marker.location_status, ObservationStatus::Known, "{status}");
        assert!(marker.projection.is_none(), "{status}");
        assert!(marker.approximate.is_none(), "{status}");
    }
}

#[test]
fn a_location_on_another_floor_is_a_gap_not_a_marker() {
    let snapshot = snapshot_with_location(|l| l["value"]["map"]["floor"] = json!("1"));
    let marker = snapshot_marker_for(&snapshot, now(60_000)).unwrap();
    assert!(matches!(
        marker.projection,
        Some(ProjectedPosition::Gap(ProjectionGap::DifferentFloor { .. }))
    ));
    assert!(marker.approximate.is_none());
}

// --- resolver --------------------------------------------------------------

#[test]
fn resolver_serves_only_declared_hash_verified_layers() {
    for layer in ["roads", "flight", "overview"] {
        let asset = resolve_atlas_asset(&format!("demo-image-1/{layer}")).unwrap();
        assert_eq!(asset.content_type, "image/png");
        assert!(asset.bytes.starts_with(b"\x89PNG"), "{layer}");
    }
}

#[test]
fn resolver_refuses_everything_else() {
    let refused = [
        "",
        "demo-image-1",
        "demo-image-1/",
        "/demo-image-1/roads",
        "demo-image-1/roads/",
        "demo-image-1/roads/extra",
        "demo-image-9/roads",
        "demo-image-1/terrain",
        "DEMO-IMAGE-1/roads",
        "demo-image-1/Roads",
        "demo-image-1/..",
        "../roads",
        "demo-image-1/roads.png",
        "demo-image-1%2Froads",
        "demo-image-1/%2e%2e",
        "demo-image-1/roads%2F..%2Fflight",
        "demo-image-1/ro%61ds%252F",
        "demo-image-1/%",
        "demo-image-1/%zz",
        "demo-image-1\\roads",
        "demo-image-1/roads?x=1",
        "demo-image-1/roads%00",
    ];
    for path in refused {
        let err = resolve_atlas_asset(path).unwrap_err();
        assert!(
            matches!(
                err,
                AtlasAssetError::InvalidPath | AtlasAssetError::NotFound
            ),
            "{path:?} gave {err:?}"
        );
        assert_eq!(err.http_status(), 404, "{path:?}");
    }
}

#[test]
fn resolver_decodes_exactly_once() {
    // %72 is "r": one decode gives a declared name, and no second decode happens.
    assert!(resolve_atlas_asset("demo-image-1/%72oads").is_ok());
    assert_eq!(
        resolve_atlas_asset("demo-image-1/%2572oads").unwrap_err(),
        AtlasAssetError::InvalidPath
    );
}

#[test]
fn resolver_refuses_bytes_that_do_not_match_the_declared_digest() {
    let dataset = dataset();
    let genuine = b"genuine layer bytes";
    let tampered = b"tampered layer bytes";
    let source = |_: &AssetPath| Some(&genuine[..]);
    // Neither buffer hashes to the declared digest.
    for bytes in [&genuine[..], &tampered[..]] {
        let err = resolve_asset_with(&dataset, "demo-image-1/roads", &|_| Some(bytes)).unwrap_err();
        assert_eq!(err, AtlasAssetError::IntegrityMismatch);
        assert_eq!(err.http_status(), 403);
    }
    assert_eq!(
        resolve_asset_with(&dataset, "demo-image-1/roads", &|_| None).unwrap_err(),
        AtlasAssetError::NotEmbedded
    );
    assert_eq!(
        resolve_asset_with(&dataset, "demo-image-9/roads", &source).unwrap_err(),
        AtlasAssetError::NotFound
    );
}

#[test]
fn resolver_accepts_the_genuine_bytes_through_an_injected_source() {
    let dataset = dataset();
    let genuine = resolve_atlas_asset("demo-image-1/roads").unwrap().bytes;
    let asset = resolve_asset_with(&dataset, "demo-image-1/roads", &|_| Some(genuine)).unwrap();
    assert_eq!(asset.bytes, genuine);
}

#[test]
fn scheme_replies_are_get_only_and_empty_on_refusal() {
    let ok = serve_atlas("GET", "/demo-image-1/overview");
    assert_eq!((ok.status, ok.content_type), (200, Some("image/png")));
    assert!(!ok.body.is_empty());
    for (method, path, status) in [
        ("POST", "/demo-image-1/overview", 405),
        ("GET", "demo-image-1/overview", 404),
        ("GET", "//demo-image-1/overview", 404),
        ("GET", "/demo-image-1/nope", 404),
        ("GET", "/demo-image-1/..%2Foverview", 404),
    ] {
        let reply = serve_atlas(method, path);
        assert_eq!(reply.status, status, "{method} {path}");
        assert!(reply.body.is_empty() && reply.content_type.is_none());
    }
}

// --- golden DTO JSON -------------------------------------------------------

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/desktop/atlas")
}

/// Compare `value` with the committed file; rewrite it only when
/// `UPDATE_DESKTOP_FIXTURES=1`.
fn check_golden(name: &str, value: &impl Serialize) {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    let path = golden_dir().join(name);
    if std::env::var("UPDATE_DESKTOP_FIXTURES").as_deref() == Ok("1") {
        std::fs::create_dir_all(golden_dir()).unwrap();
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_DESKTOP_FIXTURES=1"));
    assert_eq!(
        committed, text,
        "{name} drifted; rerun with UPDATE_DESKTOP_FIXTURES=1 and review the diff"
    );
}

#[derive(Serialize)]
struct ProjectionGolden<'a> {
    cases: Vec<&'static str>,
    positions: &'a [Position],
    results: Vec<ProjectedPosition>,
}

#[test]
fn golden_dto_json_matches_the_committed_files() {
    check_golden(
        "query-atlas.json",
        &query_atlas_with(None, SchemeStyle::Custom).unwrap(),
    );

    let cases = projection_cases();
    let positions: Vec<_> = cases.iter().map(|(_, p)| p.clone()).collect();
    check_golden(
        "project-positions.json",
        &ProjectionGolden {
            cases: cases.iter().map(|(name, _)| *name).collect(),
            results: project_positions(&positions).unwrap(),
            positions: &positions,
        },
    );

    check_golden(
        "snapshot-markers-fresh.json",
        &get_snapshot_markers(now(60_000)).unwrap(),
    );
    check_golden(
        "snapshot-markers-stale.json",
        &get_snapshot_markers(now(31 * 60 * 1000)).unwrap(),
    );
    check_golden(
        "snapshot-markers-uncertain.json",
        &get_snapshot_markers(None).unwrap(),
    );
    let restricted = snapshot_with_location(|location| {
        let object = location.as_object_mut().unwrap();
        object.remove("value");
        object.insert("status".into(), json!("restricted"));
    });
    check_golden(
        "snapshot-markers-location-restricted.json",
        &vec![snapshot_marker_for(&restricted, now(60_000)).unwrap()],
    );
    let other_floor = snapshot_with_location(|l| l["value"]["map"]["floor"] = json!("1"));
    check_golden(
        "snapshot-markers-gap-different-floor.json",
        &vec![snapshot_marker_for(&other_floor, now(60_000)).unwrap()],
    );
}
