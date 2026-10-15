mod common;
use azeroth_core::{content_v2::decode, types::AssetPath};
use azeroth_storage::{
    SqliteStore,
    catalog_assets::{CatalogAsset, ImageRequest},
    content::{ContentRevision, ContentRevisionV2, catalog_content_sha256},
};
use common::{TempDb, raw};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Cursor;
fn bundle() -> (ContentRevisionV2, Vec<CatalogAsset>, ImageRequest) {
    let old: ContentRevision = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let mut reference = serde_json::to_value(&old.envelope).unwrap();
    reference["payload"]["revision"] = "asset-catalog".into();
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::new(2, 2))
        .write_to(&mut png, ImageFormat::Png)
        .unwrap();
    let bytes = png.into_inner();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let map = reference["payload"]["map_spaces"][0]["key"].clone();
    reference["payload"]["map_images"] = json!([{"id":"asset-image","map":map,"source":"demo-source-1","evidence":["demo-evidence-1"],"applicable_builds":[{"build":map["context"]["build"],"evidence":[]}],"width_px":2,"height_px":2,"layers":[{"id":"base","kind":"roads_traced","file":"atlas/base.png","sha256":hash,"width_px":2,"height_px":2,"frame_divisor":1,"registration_offset_px":{"x":0,"y":0}}],"pins":[],"calibration":{"status":"unknown"},"calibration_evidence":[],"control_points":[],"gaps":"Synthetic image"}]);
    let document = json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,"models":[],"equipment":[],"quest_facts":[],"acquisition_facts":[],"dungeons":[]});
    let revision = ContentRevisionV2 {
        content: decode(&serde_json::to_vec(&document).unwrap()).unwrap(),
        sources: old.sources,
        conflicts: vec![],
    };
    let request:ImageRequest=serde_json::from_value(json!({"revision":"asset-catalog","catalog_sha256":catalog_content_sha256(&revision.content).unwrap(),"image_id":"asset-image","layer_id":"base"})).unwrap();
    (
        revision,
        vec![CatalogAsset {
            file: AssetPath::new("atlas/base.png").unwrap(),
            bytes,
        }],
        request,
    )
}
#[test]
fn verified_bundle_serves_only_exact_active_revision_and_survives_backup() {
    let db = TempDb::new("asset-store");
    let output = TempDb::new("asset-recovered");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let (revision, assets, mut request) = bundle();
    store.stage_content_v2_bundle(&revision, &assets).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    assert!(
        store
            .catalog_image_present(&revision.key().scope, &request)
            .unwrap()
    );
    assert_eq!(
        store
            .active_catalog_image(&revision.key().scope, &request)
            .unwrap()
            .unwrap(),
        assets[0].bytes
    );
    azeroth_storage::recovery::reconstruct_backup(&db.path, &output.path).unwrap();
    let recovered = SqliteStore::open(&output.path).unwrap();
    assert_eq!(
        recovered
            .active_catalog_image(&revision.key().scope, &request)
            .unwrap()
            .unwrap(),
        assets[0].bytes
    );
    request.catalog_sha256 = serde_json::from_value(json!("f".repeat(64))).unwrap();
    assert!(
        store
            .active_catalog_image(&revision.key().scope, &request)
            .is_err()
    );
}
#[test]
fn json_only_has_explicit_missing_bytes_and_incomplete_or_tampered_bundle_is_atomic() {
    let db = TempDb::new("asset-atomic");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let (revision, mut assets, request) = bundle();
    assert!(store.stage_content_v2_bundle(&revision, &[]).is_err());
    assert_eq!(
        raw(&db.path)
            .query_row("SELECT count(*) FROM dataset_revisions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    store.stage_content_v2_revision(&revision).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    assert!(
        !store
            .catalog_image_present(&revision.key().scope, &request)
            .unwrap()
    );
    assert!(
        store
            .active_catalog_image(&revision.key().scope, &request)
            .unwrap()
            .is_none()
    );
    assets[0].bytes[0] = 0;
    assert!(store.stage_content_v2_bundle(&revision, &assets).is_err());
    assert!(
        store
            .active_catalog_image(&revision.key().scope, &request)
            .unwrap()
            .is_none()
    );
}
#[test]
fn installed_blob_tampering_refuses_serving_and_recovery() {
    let db = TempDb::new("asset-tamper");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let (revision, assets, request) = bundle();
    store.stage_content_v2_bundle(&revision, &assets).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    raw(&db.path).execute_batch("DROP TRIGGER catalog_asset_blobs_no_update; UPDATE catalog_asset_blobs SET bytes=x'00'").unwrap();
    assert!(
        store
            .active_catalog_image(&revision.key().scope, &request)
            .is_err()
    );
    assert!(azeroth_storage::recovery::validate(&store).is_err());
}
#[test]
fn matching_hash_non_png_and_wrong_dimensions_cannot_be_installed() {
    let (mut revision, mut assets, _) = bundle();
    let mut store = SqliteStore::open_in_memory().unwrap();
    assets[0].bytes = b"not a PNG".to_vec();
    let mut value = serde_json::to_value(&revision.content).unwrap();
    value["reference"]["payload"]["map_images"][0]["layers"][0]["sha256"] =
        format!("{:x}", Sha256::digest(&assets[0].bytes)).into();
    revision.content = decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(store.stage_content_v2_bundle(&revision, &assets).is_err());
    let (mut revision, assets, _) = bundle();
    let mut value = serde_json::to_value(&revision.content).unwrap();
    value["reference"]["payload"]["map_images"][0]["width_px"] = 3.into();
    value["reference"]["payload"]["map_images"][0]["layers"][0]["width_px"] = 3.into();
    revision.content = decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(store.stage_content_v2_bundle(&revision, &assets).is_err());
}

#[test]
fn orphan_blob_refuses_recovery_and_staged_catalog_getter_preserves_v2() {
    let db = TempDb::new("asset-orphan");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let (revision, _, _) = bundle();
    store.stage_content_v2_revision(&revision).unwrap();
    assert!(
        store
            .catalog_revision(&revision.key())
            .unwrap()
            .unwrap()
            .content_v2()
            .is_some()
    );
    raw(&db.path)
        .execute(
            "INSERT INTO catalog_asset_blobs(sha256,bytes) VALUES (?1,x'00')",
            ["0".repeat(64)],
        )
        .unwrap();
    assert!(azeroth_storage::recovery::validate(&store).is_err());
}
