use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{
    contract::decode_envelope,
    types::{PartyId, UtcMillis},
};
use azeroth_storage::{SqliteStore, content::ContentRevision};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);
pub const NOW: i64 = 1791792000000;

pub struct AppDb {
    pub app: SnapshotApplication,
    pub path: std::path::PathBuf,
}
impl Drop for AppDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
pub fn setup(edit: impl FnOnce(&mut Value)) -> AppDb {
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
            "../../../../fixtures/desktop/content/demo-revision.json"
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
pub fn import(app: &SnapshotApplication, who: &str, level: u16, capture: i64) -> String {
    import_edit(app, who, level, capture, |_| {})
}
pub fn import_edit(
    app: &SnapshotApplication,
    who: &str,
    level: u16,
    capture: i64,
    edit: impl FnOnce(&mut Value),
) -> String {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/v1/character-snapshot.json"
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
    edit(&mut value["payload"]);
    app.import_text(&format!("ANAV1:{value}"), None)
        .unwrap()
        .profile_id
}
pub fn id() -> PartyId {
    PartyId::new("duo").unwrap()
}

pub fn members(db: &AppDb, second_level: u16, second_capture: i64) {
    let one = import(&db.app, "one", 22, NOW);
    let two = import(&db.app, "two", second_level, second_capture);
    db.app.save_party(&id(), &[one, two]).unwrap();
}
pub fn request(db: &AppDb) -> Value {
    let catalog = db
        .app
        .joint_planning_catalog(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    json!({"party_id":"duo", "participants": catalog.participants.iter().map(|p| json!({"profile_id":p.profile_id,"snapshot_id":p.snapshot_id})).collect::<Vec<_>>(),
        "revision":"demo-r1", "goal_profile":"manual-same-unit-v1", "origin":"demo-node-1", "stopping_node":"demo-node-1", "budget_ms":600000,"stopping_service_ms":0,
        "excluded":[], "candidates":[{"objective":"demo-objective-1","node":"demo-node-1","utilities":catalog.participants.iter().map(|p|(p.profile_id.clone(),json!(10))).collect::<serde_json::Map<_,_>>(), "completion_ms":60000,"service_ms":0}]})
}

pub fn propose(
    db: &AppDb,
    expected: Option<azeroth_storage::party_proposals::ProposalGuard>,
) -> azeroth_app::party_proposals::PartyProposalReport {
    let planning = request(db);
    let proposer = planning["participants"][0]["profile_id"].as_str().unwrap();
    db.app
        .propose_party_plan(
            &json!({"planning":planning,"proposer_profile_id":proposer,"expected":expected})
                .to_string(),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap()
}
pub fn accept(
    db: &AppDb,
    report: &azeroth_app::party_proposals::PartyProposalReport,
    index: usize,
    accepted: bool,
) -> azeroth_core::error::Result<azeroth_app::party_proposals::PartyProposalReport> {
    db.app.accept_party_plan(&json!({"party_id":"duo","profile_id":report.participants[index].profile_id,"expected":report.guard,"accepted":accepted}).to_string(),Some(UtcMillis::new(NOW).unwrap()))
}
pub fn agree(db: &AppDb) -> azeroth_app::party_proposals::PartyProposalReport {
    let p = propose(db, None);
    let p = accept(db, &p, 0, true).unwrap();
    accept(db, &p, 1, true).unwrap()
}
pub fn export_request(p: &azeroth_app::party_proposals::PartyProposalReport, index: usize) -> String {
    json!({"party_id":p.party_id,"expected":p.guard,"recipient_profile_id":p.participants[index].profile_id}).to_string()
}
pub fn export(
    db: &AppDb,
    p: &azeroth_app::party_proposals::PartyProposalReport,
    index: usize,
) -> azeroth_core::error::Result<azeroth_app::plan_export::PlanExport> {
    db.app.export_agreed_party_plan(
        &export_request(p, index),
        Some(UtcMillis::new(NOW).unwrap()),
    )
}
