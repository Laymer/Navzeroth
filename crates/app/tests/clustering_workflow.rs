use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, PartyId, UtcMillis};
use serde_json::{Value, json};
const NOW: i64 = 1791791990100;
fn setup(joint: bool, unknown_stage: bool) -> (SnapshotApplication, Value) {
    let app = SnapshotApplication::in_memory().unwrap();
    let members = if joint {
        vec!["one", "two"]
    } else {
        vec!["one"]
    };
    let mut profiles = Vec::new();
    let observed = |value: bool| json!({"status":"known","value":value,"observed_at_utc_ms":NOW-100,"clock_trust":"trusted","source":"synthetic_fixture"});
    for who in &members {
        let mut s: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v1/character-snapshot.json"
        ))
        .unwrap();
        s["payload"]["character"] = json!(who);
        s["payload"]["id"] = json!(format!("cluster-snapshot-{who}"));
        s["payload"]["quests"]["cluster-quest"] = json!({"quest":"cluster-quest","context":s["payload"]["context"],"accepted":observed(true),"completed":observed(false),"turned_in":observed(false),"objectives":{"cluster-stage":observed(false)}});
        s["payload"]["objectives"]["cluster-stage"] = if unknown_stage {
            json!({"status":"restricted"})
        } else {
            observed(false)
        };
        profiles.push(
            app.import_text(&format!("ANAV1:{s}"), Some(UtcMillis::new(NOW).unwrap()))
                .unwrap()
                .profile_id,
        );
    }
    let mut c: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let d = &mut c["envelope"]["payload"];
    d["objectives"][0]["participants"] = json!(members);
    let mut objective = d["objectives"][0].clone();
    objective["id"] = json!("cluster-stage");
    objective["targets"] =
        json!([{"kind":"quest_stage","quest":"cluster-quest","stage":"cluster-stage"}]);
    d["objectives"].as_array_mut().unwrap().push(objective);
    let reference: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    let mut quest = reference["envelope"]["payload"]["quests"][0].clone();
    quest["id"] = json!("cluster-quest");
    quest["stages"] = json!(["cluster-stage"]);
    quest["rewards"] = json!({"status":"unknown"});
    d["quests"] = json!([quest]);
    app.stage_content(&c.to_string()).unwrap();
    app.activate_content(
        &profiles[0],
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    let estimates = ["cluster-stage", "synthetic-service"].iter().map(|id| json!({"objective":id,"node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0})).collect::<Vec<_>>();
    let request = if joint {
        let party = PartyId::new("cluster-party").unwrap();
        app.save_party(&party, &profiles).unwrap();
        let catalog = app
            .joint_planning_catalog(&party, Some(UtcMillis::new(NOW).unwrap()))
            .unwrap();
        let candidates = estimates.into_iter().enumerate().map(|(i, mut e)| {
            e.as_object_mut().unwrap().remove("utility");
            e["utilities"] = json!({profiles[0].clone():if i==0 {9} else {1},profiles[1].clone():if i==0 {1} else {9}}); e
        }).collect::<Vec<_>>();
        json!({"party_id":party,"participants":catalog.participants.iter().map(|p|json!({"profile_id":p.profile_id,"snapshot_id":p.snapshot_id})).collect::<Vec<_>>(),"revision":"synthetic-planning-v1","goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":62000,"stopping_service_ms":0,"candidates":candidates,"excluded":[]})
    } else {
        let catalog = app.planning_catalog(&profiles[0]).unwrap();
        json!({"profile_id":profiles[0],"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":62000,"stopping_service_ms":0,"candidates":estimates,"excluded":[]})
    };
    (app, request)
}
#[test]
fn solo_cluster_shares_travel_and_exports_original_steps_with_singleton_fallback() {
    let (app, mut request) = setup(false, false);
    let now = Some(UtcMillis::new(NOW).unwrap());
    let report = app.plan_session(&request.to_string(), now).unwrap();
    assert_eq!(report.result.selected.len(), 2);
    assert_eq!(report.result.expected_total_ms.get(), 62000);
    assert_eq!(report.clustering.selected_clusters.len(), 1);
    assert_eq!(report.clustering.selected_clusters[0].members.len(), 2);
    assert_eq!(
        report
            .result
            .selected
            .iter()
            .map(|c| c.travel.total_ms.get())
            .sum::<u64>(),
        30000
    );
    let exported = app.export_plan(&request.to_string(), now).unwrap();
    let value: Value = serde_json::from_str(&exported.text[6..]).unwrap();
    assert_eq!(value["payload"]["steps"].as_array().unwrap().len(), 2);
    assert!(!exported.text.contains("cluster-slot-"));
    request["budget_ms"] = json!(61000);
    let fallback = app.plan_session(&request.to_string(), now).unwrap();
    assert_eq!(fallback.result.selected.len(), 1);
    assert_eq!(fallback.clustering.selected_clusters[0].members.len(), 1);
}
#[test]
fn joint_cluster_uses_minimum_participant_sum_and_preserves_original_utilities() {
    let (app, request) = setup(true, false);
    let report = app
        .plan_joint_session(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    assert_eq!(report.result.plan.selected.len(), 2);
    assert_eq!(report.result.plan.expected_total_ms.get(), 62000);
    assert_eq!(
        report.clustering.selected_clusters[0].participant_utilities,
        vec![10, 10]
    );
    assert_eq!(report.clustering.selected_clusters[0].utility, 10);
    assert!(report.result.plan.selected.iter().all(|c| c.utility == 1));
}
#[test]
fn restricted_original_stage_never_becomes_ready_through_clustering() {
    for joint in [false, true] {
        let (app, request) = setup(joint, true);
        let now = Some(UtcMillis::new(NOW).unwrap());
        let selected = if joint {
            app.plan_joint_session(&request.to_string(), now)
                .unwrap()
                .result
                .plan
                .selected
        } else {
            app.plan_session(&request.to_string(), now)
                .unwrap()
                .result
                .selected
        };
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].candidate.as_str(), "synthetic-service");
    }
}
