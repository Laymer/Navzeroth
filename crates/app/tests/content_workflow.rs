use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::DatasetRevision;
use serde_json::{Value, json};
const BUNDLE: &str = include_str!("../../../fixtures/desktop/content/demo-revision.json");
fn revision(name: &str) -> String {
    let mut value: Value = serde_json::from_str(BUNDLE).unwrap();
    value["envelope"]["payload"]["revision"] = json!(name);
    value.to_string()
}
fn profile(app: &SnapshotApplication) -> String {
    app.import_text(
        &format!(
            "ANAV1:{}",
            include_str!("../../../fixtures/contracts/v1/character-snapshot.json")
        ),
        None,
    )
    .unwrap()
    .profile_id
}
#[test]
fn staging_never_activates_and_invalid_updates_preserve_current_content() {
    let app = SnapshotApplication::in_memory().unwrap();
    let profile = profile(&app);
    let staged = app.stage_content(BUNDLE).unwrap();
    assert!(staged.report.passes);
    assert!(!staged.report.gaps.is_empty());
    assert!(app.active_content(&profile).unwrap().is_none());
    let r1 = DatasetRevision::new("demo-r1").unwrap();
    app.activate_content(&profile, &r1).unwrap();
    assert!(app.stage_content("not JSON").is_err());
    assert!(
        app.stage_content(&" ".repeat(azeroth_storage::content::MAX_CONTENT_REVISION_BYTES + 1))
            .is_err()
    );
    let mut bad: Value = serde_json::from_str(BUNDLE).unwrap();
    bad["sources"] = json!([]);
    assert!(app.stage_content(&bad.to_string()).is_err());
    assert!(
        app.activate_content(&profile, &DatasetRevision::new("absent").unwrap())
            .is_err()
    );
    assert_eq!(
        app.active_content(&profile).unwrap().unwrap().key.revision,
        r1
    );
    let result = app.rollback_content(&profile).unwrap();
    assert!(!result.changed);
    assert_eq!(result.active.unwrap().key.revision, r1);
}
#[test]
fn activation_is_context_scoped_and_rollback_preserves_character_state() {
    let app = SnapshotApplication::in_memory().unwrap();
    let profile = profile(&app);
    app.stage_content(BUNDLE).unwrap();
    app.stage_content(&revision("r2")).unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    app.activate_content(&profile, &DatasetRevision::new("r2").unwrap())
        .unwrap();
    assert_eq!(
        app.rollback_content(&profile)
            .unwrap()
            .active
            .unwrap()
            .key
            .revision
            .as_str(),
        "demo-r1"
    );
    assert_eq!(
        app.rollback_content(&profile)
            .unwrap()
            .active
            .unwrap()
            .key
            .revision
            .as_str(),
        "r2"
    );
    let reference = app
        .import_text(
            &format!(
                "ANAV1:{}",
                include_str!("../../../fixtures/contracts/v1/reference-namespace-fixture.json")
            ),
            None,
        )
        .unwrap()
        .profile_id;
    assert!(
        app.activate_content(&reference, &DatasetRevision::new("r2").unwrap())
            .is_err()
    );
    assert!(app.active_content(&reference).unwrap().is_none());
    assert!(app.current(&profile, None).unwrap().is_some());
    assert_eq!(app.history(&profile, 32, None).unwrap().len(), 1);
}
