use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};
fn setup() -> (SnapshotApplication, String) {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let profile = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(1791791990100).unwrap()),
        )
        .unwrap()
        .profile_id;
    let mut v2: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let imagery: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset-imagery.json"
    ))
    .unwrap();
    // Synthetic geometry only; every author/source remains a labelled fixture.
    v2["reference"] = imagery;
    let original = v2["reference"]["payload"]["map_images"][0].clone();
    v2["reference"]["payload"]["map_images"] = json!(
        (0..205)
            .map(|i| {
                let mut image = original.clone();
                image["id"] = json!(format!("atlas-{i:03}"));
                image
            })
            .collect::<Vec<_>>()
    );
    for field in [
        "models",
        "equipment",
        "quest_facts",
        "acquisition_facts",
        "dungeons",
    ] {
        v2[field] = json!([]);
    }
    let v1: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let bundle = json!({"content":v2,"sources":v1["sources"],"conflicts":[]});
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-imagery-r1").unwrap())
        .unwrap();
    (app, profile)
}
#[test]
fn no_profile_has_no_demo_fallback() {
    assert!(
        SnapshotApplication::in_memory()
            .unwrap()
            .stored_atlas(None, None, 0)
            .is_err()
    );
}
#[test]
fn selected_v2_atlas_is_revision_bound_and_missing_material_unavailable() {
    let (app, profile) = setup();
    let view = app.stored_atlas(None, None, 0).unwrap();
    assert_eq!(
        serde_json::to_value(view.data_mode).unwrap(),
        json!("stored")
    );
    assert_eq!(view.images.len(), 200);
    assert!(
        view.images[0]
            .attribution
            .contains("Attribution: unknown. Licence: unknown.")
    );
    assert!(
        view.images
            .iter()
            .flat_map(|i| &i.layers)
            .all(|l| l.url.is_none())
    );
    let page = view.page.unwrap();
    assert_eq!(page.token.profile_id, profile);
    assert_eq!(page.total, 205);
    assert_eq!(page.next_offset, Some(200));
    let next = app.stored_atlas(None, Some(&page.token), 200).unwrap();
    assert_eq!(next.images.len(), 5);
    assert!(next.page.unwrap().next_offset.is_none());
    assert!(app.stored_atlas(None, None, 200).is_err());
    let markers = app
        .atlas_markers(
            Some(&page.token),
            Some(UtcMillis::new(1791791990100).unwrap()),
        )
        .unwrap();
    let projection = serde_json::to_value(&markers[0].projection).unwrap();
    assert_eq!(projection["image"], json!("atlas-000"));
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let position =
        serde_json::from_value(snapshot["payload"]["location"]["value"].clone()).unwrap();
    let projected = app
        .stored_project_positions(&[position], &page.token)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&projected[0]).unwrap()["image"],
        json!("atlas-000")
    );
    let mut stale = page.token;
    stale.revision = DatasetRevision::new("old").unwrap();
    assert!(app.stored_atlas(None, Some(&stale), 0).is_err());
}
