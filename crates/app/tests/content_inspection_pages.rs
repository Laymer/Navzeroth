use azeroth_app::snapshots::SnapshotApplication;
use serde_json::{Value, json};
#[test]
fn staged_inspection_is_bounded_and_hash_bound_before_activation() {
    let app = SnapshotApplication::in_memory().unwrap();
    let content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let source = old["sources"][0].clone();
    let mut sources = vec![source.clone()];
    for n in 0..204 {
        let mut extra = source.clone();
        extra["id"] = json!(format!("extra-source-{n:03}"));
        sources.push(extra);
    }
    let inspected = app
        .stage_content(&json!({"content":content,"sources":sources,"conflicts":[]}).to_string())
        .unwrap();
    assert_eq!(inspected.sources.len(), 200);
    let pages = inspected.pages.unwrap();
    assert_eq!(pages.sources, 205);
    let mut request = json!({"key":inspected.key,"content_sha256":pages.content_sha256,"collection":"sources","offset":200});
    let second = app.content_inspection_page(&request.to_string()).unwrap();
    assert_eq!(second.records.len(), 5);
    assert_eq!(second.total, 205);
    assert_eq!(second.next_offset, None);
    request["offset"] = json!(206);
    assert!(app.content_inspection_page(&request.to_string()).is_err());
    request["offset"] = json!(0);
    request["content_sha256"] = json!("f".repeat(64));
    assert!(app.content_inspection_page(&request.to_string()).is_err());
}
#[test]
fn verified_pack_import_stages_without_activation_and_rejects_tampered_catalog() {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "nav-app-bundle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let input = root.join("input");
    fs::create_dir(&input).unwrap();
    let reference: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset-imagery.json"
    ))
    .unwrap();
    let catalog = json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,"models":[],"equipment":[],"quest_facts":[],"acquisition_facts":[],"dungeons":[]});
    fs::write(input.join("catalog.json"), catalog.to_string()).unwrap();
    let layers = input.join("atlas/synthetic-continent/layers");
    fs::create_dir_all(&layers).unwrap();
    for (name, bytes) in [
        (
            "overview",
            include_bytes!("../../../content/demo/atlas/synthetic-continent/layers/overview.png")
                .as_slice(),
        ),
        (
            "roads",
            include_bytes!("../../../content/demo/atlas/synthetic-continent/layers/roads.png")
                .as_slice(),
        ),
        (
            "flight",
            include_bytes!("../../../content/demo/atlas/synthetic-continent/layers/flight.png")
                .as_slice(),
        ),
    ] {
        fs::write(layers.join(format!("{name}.png")), bytes).unwrap();
    }
    fs::write(
        input.join("sources.json"),
        include_bytes!("../../../fixtures/content-pack/sources.json"),
    )
    .unwrap();
    let output = root.join("pack");
    azeroth_ingest::pack::v2::build(&input, &output).unwrap();
    let app = SnapshotApplication::in_memory().unwrap();
    let receipt = app
        .import_text(
            &format!(
                "ANAV1:{}",
                include_str!("../../../fixtures/contracts/v1/character-snapshot.json")
            ),
            None,
        )
        .unwrap();
    let inspection = app.stage_verified_content_bundle(&output).unwrap();
    assert!(inspection.report.passes);
    assert!(app.active_content(&receipt.profile_id).unwrap().is_none());
    fs::write(output.join("catalog.json"), b"{}").unwrap();
    assert!(app.stage_verified_content_bundle(&output).is_err());
    assert!(app.active_content(&receipt.profile_id).unwrap().is_none());
    app.activate_content(&receipt.profile_id, &inspection.key.revision)
        .unwrap();
    let atlas = app.stored_atlas(None, None, 0).unwrap();
    let url = atlas.images[0].layers[0].url.as_ref().unwrap();
    let path = &url[url.find("/stored/").unwrap()..];
    assert!(!app.stored_atlas_asset(path).unwrap().unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}
