use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{
    contract::decode_envelope,
    types::{PartyId, UtcMillis},
};
use azeroth_storage::{SqliteStore, content::ContentRevision};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);
const NOW: i64 = 1791792000000;

struct AppDb {
    app: SnapshotApplication,
    path: std::path::PathBuf,
}
impl Drop for AppDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn setup(edit: impl FnOnce(&mut Value)) -> AppDb {
    let path = std::env::temp_dir().join(format!(
        "anav-eligibility-{}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_DATABASE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut store = SqliteStore::open(&path).unwrap();
    {
        let mut revision: ContentRevision = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        let mut value = serde_json::to_value(&revision.envelope).unwrap();
        value["payload"]["objectives"][0]["participants"] = json!(["one", "two"]);
        value["payload"]["objectives"][0]["targets"] = json!([{"kind":"service", "node":{"id":"demo-node-1", "map":value["payload"]["nodes"][0]["position"]["map"]}}]);
        value["payload"]["objectives"][0]["requirements"] = json!({"status":"known","value":{"op":"level_at_least","level":22},"source":"reference_data"});
        edit(&mut value);
        revision.envelope = decode_envelope(&value.to_string()).unwrap();
        store.stage_content_revision(&revision).unwrap();
        store.activate_content_revision(&revision.key()).unwrap();
    }
    drop(store);
    AppDb {
        app: SnapshotApplication::open(&path).unwrap(),
        path,
    }
}
fn import(app: &SnapshotApplication, who: &str, level: u16, capture: i64) -> String {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    value["payload"]["id"] = json!(format!("snapshot-{who}"));
    value["payload"]["character"] = json!(who);
    value["payload"]["level"]["value"] = json!(level);
    value["payload"]["capture"]["observed_at_utc_ms"] = json!(capture);
    fn date(value: &mut Value) {
        match value {
            Value::Object(map) => {
                if map.get("status") == Some(&json!("known")) {
                    map.insert("clock_trust".into(), json!("trusted"));
                    map.insert("observed_at_utc_ms".into(), json!(NOW));
                }
                map.values_mut().for_each(date);
            }
            Value::Array(v) => v.iter_mut().for_each(date),
            _ => {}
        }
    }
    date(&mut value["payload"]);
    app.import_text(&format!("ANAV1:{value}"), None)
        .unwrap()
        .profile_id
}
fn id() -> PartyId {
    PartyId::new("duo").unwrap()
}

fn members(db: &AppDb, second_level: u16, second_capture: i64) {
    let one = import(&db.app, "one", 22, NOW);
    let two = import(&db.app, "two", second_level, second_capture);
    db.app.save_party(&id(), &[one, two]).unwrap();
}
fn request(db: &AppDb) -> Value {
    let catalog = db
        .app
        .joint_planning_catalog(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    json!({"party_id":"duo", "participants": catalog.participants.iter().map(|p| json!({"profile_id":p.profile_id,"snapshot_id":p.snapshot_id})).collect::<Vec<_>>(),
        "revision":"demo-r1", "goal_profile":"manual-same-unit-v1", "origin":"demo-node-1", "stopping_node":"demo-node-1", "budget_ms":600000,"stopping_service_ms":0,
        "excluded":[], "candidates":[{"objective":"demo-objective-1","node":"demo-node-1","utilities":catalog.participants.iter().map(|p|(p.profile_id.clone(),json!(10))).collect::<serde_json::Map<_,_>>(), "completion_ms":60000,"service_ms":0}]})
}
fn plan(
    db: &AppDb,
    req: &Value,
) -> azeroth_core::error::Result<azeroth_app::joint_planning::JointPlanningReport> {
    db.app
        .plan_joint_session(&req.to_string(), Some(UtcMillis::new(NOW).unwrap()))
}
#[test]
fn minimum_member_utility_is_group_score_and_round_trip_inputs_remain_explicit() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let mut req = request(&db);
    let member = req["participants"][1]["profile_id"]
        .as_str()
        .unwrap()
        .to_string();
    req["candidates"][0]["utilities"][member] = json!(3);
    let result = plan(&db, &req).unwrap();
    assert_eq!(result.result.plan.selected.len(), 1);
    assert_eq!(result.result.plan.selected[0].utility, 3);
    assert_eq!(result.participants.len(), 2);
    assert_eq!(result.result.plan.expected_total_ms.get(), 60000);
}
#[test]
fn one_members_unknown_or_false_readiness_never_becomes_joint_ready() {
    let db = setup(|_| {});
    members(&db, 21, NOW);
    let result = plan(&db, &request(&db)).unwrap();
    assert!(result.result.plan.selected.is_empty());
    assert!(!result.result.assessments[0].ready);
    assert_eq!(
        result.result.assessments[0].participants[1].readiness.truth,
        azeroth_core::evaluation::Truth::False
    );
}
#[test]
fn zero_member_contribution_is_excluded_even_when_every_member_is_ready() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let mut req = request(&db);
    let member = req["participants"][1]["profile_id"]
        .as_str()
        .unwrap()
        .to_string();
    req["candidates"][0]["utilities"][member] = json!(0);
    let result = plan(&db, &req).unwrap();
    assert!(result.result.plan.selected.is_empty());
}
#[test]
fn stale_or_uncertain_capture_blocks_joint_planning() {
    let db = setup(|_| {});
    members(&db, 23, NOW - 1_800_001);
    let req = request(&db);
    assert!(plan(&db, &req).is_err());
    assert!(db.app.plan_joint_session(&req.to_string(), None).is_err());
}
#[test]
fn snapshot_revision_and_membership_changes_reject_requests() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let req = request(&db);
    let mut modified = req.clone();
    modified["participants"][0]["snapshot_id"] = json!("wrong");
    assert!(plan(&db, &modified).is_err());
    modified = req.clone();
    modified["revision"] = json!("old-r0");
    assert!(plan(&db, &modified).is_err());
    modified = req.clone();
    modified["participants"].as_array_mut().unwrap().pop();
    assert!(plan(&db, &modified).is_err());
}
#[test]
fn duplicate_missing_utility_and_over_budget_inputs_are_conservative() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let req = request(&db);
    let mut modified = req.clone();
    modified["candidates"][0]["utilities"] = json!({});
    assert!(plan(&db, &modified).is_err());
    modified = req.clone();
    modified["candidates"]
        .as_array_mut()
        .unwrap()
        .push(req["candidates"][0].clone());
    assert!(plan(&db, &modified).is_err());
    modified = req.clone();
    modified["budget_ms"] = json!(1000);
    assert!(
        plan(&db, &modified)
            .unwrap()
            .result
            .plan
            .selected
            .is_empty()
    );
}
#[test]
fn travel_requirement_of_second_member_is_required_for_joint_stopping_route() {
    let db = setup(|value| {
        value["payload"]["edges"][0]["requirements"] = json!({"status":"known","value":{"op":"level_at_least","level":23},"source":"reference_data"});
    });
    let one = import(&db.app, "one", 23, NOW);
    let two = import(&db.app, "two", 22, NOW);
    db.app.save_party(&id(), &[one, two]).unwrap();
    let mut req = request(&db);
    req["stopping_node"] = json!("demo-node-2");
    // First member is eligible but the second cannot use the edge.
    assert!(plan(&db, &req).is_err());
}
#[test]
fn authored_participants_and_unknown_requirements_remain_explicit_exclusions() {
    for mismatch in [true, false] {
        let db = setup(|value| {
            if mismatch {
                value["payload"]["objectives"][0]["participants"] = json!(["someone-else"]);
            } else {
                value["payload"]["objectives"][0]["requirements"] = json!({"status":"restricted"});
            }
        });
        members(&db, 23, NOW);
        let result = plan(&db, &request(&db)).unwrap();
        assert!(result.result.plan.selected.is_empty());
        assert_eq!(result.filtered.len(), 1);
    }
}

#[test]
fn hard_exclusions_override_ready_positive_contributions_and_validate_ids() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let mut req = request(&db);
    req["excluded"] = json!(["demo-objective-1"]);
    let result = plan(&db, &req).unwrap();
    assert!(result.result.plan.selected.is_empty());
    assert!(
        result
            .result
            .plan
            .trace
            .iter()
            .any(|t| t.decision == azeroth_core::planner::Decision::Excluded)
    );
    req["excluded"] = json!(["missing-objective"]);
    assert!(plan(&db, &req).is_err());
}
