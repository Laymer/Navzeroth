use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{calibration::*, types::SessionId};
use serde_json::{Value, json};
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
#[test]
fn annotation_revalidates_retained_evidence_and_delete_cascades() {
    let app = SnapshotApplication::in_memory().unwrap();
    let profile = app
        .import_text(&format!("ANAV1:{SNAPSHOT}"), None)
        .unwrap()
        .profile_id;
    let value: Value = serde_json::from_str(SNAPSHOT).unwrap();
    let events = json!([{"session":"calibration","sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}},{"session":"calibration","sequence":2,"elapsed_ms":120000,"kind":{"type":"activity","label":"observed_stage","quality":"observed"}},{"session":"calibration","sequence":3,"elapsed_ms":120000,"kind":{"type":"quest_turn_in","quest":"quest-a"}}]);
    let document = json!({"format":"anav-session-events","version":1,"fixture":true,"batch":{"character":value["payload"]["character"],"context":value["payload"]["context"],"events":events}});
    app.import_session_text(&profile, &format!("ANAVSESSION1:{document}"))
        .unwrap();
    let conditions = CalibrationConditions {
        route_stage: "declared-stage".into(),
        level: 12,
        role: "healer".into(),
        party_size: 2,
    };
    let request = CalibrationRequest {
        conditions: conditions.clone(),
        baseline_ms: 60000,
        baseline_source: "user assumption".into(),
    };
    let id = SessionId::new("calibration").unwrap();
    assert_eq!(
        app.get_calibration(&profile, &request)
            .unwrap()
            .sample_count,
        0
    );
    let mut annotation = SessionCalibrationAnnotation {
        conditions,
        complete: true,
        completion_quest: "quest-a".into(),
    };
    app.set_session_calibration(&profile, &id, &annotation)
        .unwrap();
    let result = app.get_calibration(&profile, &request).unwrap();
    assert_eq!(result.sample_count, 1);
    assert_eq!(result.blended_ms, 70000);
    annotation.complete = false;
    app.set_session_calibration(&profile, &id, &annotation)
        .unwrap();
    assert_eq!(
        app.get_calibration(&profile, &request)
            .unwrap()
            .sample_count,
        0
    );
    assert!(
        app.set_session_calibration("missing", &id, &annotation)
            .is_err()
    );
    app.delete_session(&profile, &id).unwrap();
    app.import_session_text(&profile, &format!("ANAVSESSION1:{document}"))
        .unwrap();
    assert_eq!(
        app.get_calibration(&profile, &request).unwrap().comparisons[0]
            .exclusion
            .as_deref(),
        Some("missing_annotation")
    );
    assert!(app.current(&profile, None).unwrap().is_some());
}
