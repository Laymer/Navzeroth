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

fn propose(
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
fn accept(
    db: &AppDb,
    report: &azeroth_app::party_proposals::PartyProposalReport,
    index: usize,
    accepted: bool,
) -> azeroth_core::error::Result<azeroth_app::party_proposals::PartyProposalReport> {
    db.app.accept_party_plan(&json!({"party_id":"duo","profile_id":report.participants[index].profile_id,"expected":report.guard,"accepted":accepted}).to_string(),Some(UtcMillis::new(NOW).unwrap()))
}
#[test]
fn proposal_requires_every_members_explicit_local_acceptance_and_withdrawal_survives_reopen() {
    use azeroth_app::party_proposals::AgreementState::*;
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let proposed = propose(&db, None);
    assert!(matches!(proposed.state, AwaitingAcceptance));
    assert!(proposed.participants.iter().all(|p| !p.accepted));
    let one = accept(&db, &proposed, 0, true).unwrap();
    assert!(matches!(one.state, AwaitingAcceptance));
    let agreed = accept(&db, &one, 1, true).unwrap();
    assert!(matches!(agreed.state, Agreed));
    let withdrawn = accept(&db, &agreed, 0, false).unwrap();
    assert!(matches!(withdrawn.state, AwaitingAcceptance));
    let reopened = SnapshotApplication::open(&db.path)
        .unwrap()
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(reopened.guard, withdrawn.guard);
    assert!(!reopened.participants[0].accepted);
    assert!(reopened.participants[1].accepted);
}
#[test]
fn replacement_uses_exact_guard_resets_acceptance_and_rejects_old_plan_acceptance() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let original = propose(&db, None);
    let accepted = accept(&db, &original, 0, true).unwrap();
    let replacement = propose(&db, Some(accepted.guard.clone()));
    assert_ne!(replacement.guard.proposal_id, original.guard.proposal_id);
    assert!(replacement.participants.iter().all(|p| !p.accepted));
    assert!(accept(&db, &accepted, 1, true).is_err());
    let planning = request(&db);
    assert!(db.app.propose_party_plan(&json!({"planning":planning,"proposer_profile_id":replacement.proposer_profile_id,"expected":null}).to_string(),Some(UtcMillis::new(NOW).unwrap())).is_err());
}
#[test]
fn concurrent_acceptances_cannot_overwrite_another_member_declaration() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let original = propose(&db, None);
    let first = accept(&db, &original, 0, true).unwrap();
    assert!(accept(&db, &original, 1, true).is_err());
    let current = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap()
        .unwrap();
    assert!(current.participants[0].accepted);
    assert!(!current.participants[1].accepted);
    assert_eq!(current.guard, first.guard);
    assert!(matches!(
        accept(&db, &current, 1, true).unwrap().state,
        azeroth_app::party_proposals::AgreementState::Agreed
    ));
}
#[test]
fn new_participant_snapshot_invalidates_agreement_and_exposes_quest_observation_changes() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let proposal = propose(&db, None);
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    value["payload"]["id"] = json!("second-refresh");
    value["payload"]["character"] = json!("two");
    value["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW + 1000);
    value["payload"]["capture"]["sequence"] = json!(13);
    value["producer"]["sequence"] = json!(13);
    value["payload"]["quests"]["demo-quest-1"]["completed"] = json!({"status":"restricted"});
    db.app.import_text(&format!("ANAV1:{value}"), None).unwrap();
    let invalid = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW + 1000).unwrap()))
        .unwrap()
        .unwrap();
    assert!(matches!(
        invalid.state,
        azeroth_app::party_proposals::AgreementState::Invalidated
    ));
    let changed = invalid
        .participants
        .iter()
        .find(|p| {
            p.current_snapshot_id
                .as_ref()
                .is_some_and(|id| id.as_str() == "second-refresh")
        })
        .unwrap();
    assert!(!changed.reasons.is_empty());
    assert!(!changed.progress_changes.is_empty());
    assert_eq!(
        changed.progress_changes[0]
            .current
            .as_ref()
            .unwrap()
            .completed
            .status(),
        azeroth_core::observation::ObservationStatus::Restricted
    );
    assert!(accept(&db, &proposal, 0, true).is_err());
}
#[test]
fn missing_member_capture_and_single_member_party_keep_invalidated_history_visible() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let proposal = propose(&db, None);
    let first = proposal.request.participants[0].profile_id.clone();
    db.app.save_party(&id(), &[first]).unwrap();
    let status = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap()
        .unwrap();
    assert!(matches!(
        status.state,
        azeroth_app::party_proposals::AgreementState::Invalidated
    ));
    assert!(
        status
            .participants
            .iter()
            .any(|p| p.current_snapshot_id.is_none())
    );
}
#[test]
fn stale_or_uncertain_clock_invalidates_even_previously_agreed_proposals() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let proposal = propose(&db, None);
    let one = accept(&db, &proposal, 0, true).unwrap();
    let agreed = accept(&db, &one, 1, true).unwrap();
    for now in [None, Some(UtcMillis::new(NOW + 1_800_001).unwrap())] {
        let status = db.app.party_plan_proposal(&id(), now).unwrap().unwrap();
        assert!(matches!(
            status.state,
            azeroth_app::party_proposals::AgreementState::Invalidated
        ));
        assert!(status.participants.iter().all(|p| p.accepted));
        assert!(db.app.accept_party_plan(&json!({"party_id":"duo","profile_id":agreed.participants[0].profile_id,"expected":agreed.guard,"accepted":true}).to_string(),now).is_err());
    }
}
#[test]
fn changed_active_content_revision_invalidates_plan_and_party_deletion_cascades_only_agreement() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let original = propose(&db, None);
    let mut store = SqliteStore::open(&db.path).unwrap();
    let mut revision: ContentRevision = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    let mut value = serde_json::to_value(&revision.envelope).unwrap();
    value["payload"]["revision"] = json!("demo-r2");
    revision.envelope = decode_envelope(&value.to_string()).unwrap();
    store.stage_content_revision(&revision).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    drop(store);
    let invalid = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap()
        .unwrap();
    assert!(matches!(
        invalid.state,
        azeroth_app::party_proposals::AgreementState::Invalidated
    ));
    assert!(accept(&db, &original, 0, true).is_err());
    db.app.delete_party(&id()).unwrap();
    assert!(
        db.app
            .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap()
            .is_none()
    );
    assert_eq!(db.app.profiles().unwrap().len(), 2);
}
#[test]
fn wrong_actor_malformed_and_oversized_agreement_requests_preserve_current_record() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let original = propose(&db, None);
    assert!(db.app.accept_party_plan(&json!({"party_id":"duo","profile_id":"outsider","expected":original.guard,"accepted":true}).to_string(),Some(UtcMillis::new(NOW).unwrap())).is_err());
    assert!(db.app.propose_party_plan("not-json", None).is_err());
    assert!(
        db.app
            .propose_party_plan(&"x".repeat(2 * 1024 * 1024 + 1), None)
            .is_err()
    );
    assert_eq!(
        db.app
            .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap()
            .unwrap()
            .guard,
        original.guard
    );
}

#[test]
fn same_quest_values_recaptured_at_new_time_do_not_invent_progress() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    propose(&db, None);
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    value["payload"]["id"] = json!("recapture-only");
    value["payload"]["character"] = json!("two");
    value["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW + 1000);
    value["payload"]["capture"]["sequence"] = json!(13);
    value["producer"]["sequence"] = json!(13);
    db.app.import_text(&format!("ANAV1:{value}"), None).unwrap();
    let status = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW + 1000).unwrap()))
        .unwrap()
        .unwrap();
    assert!(matches!(
        status.state,
        azeroth_app::party_proposals::AgreementState::Invalidated
    ));
    assert!(
        status
            .participants
            .iter()
            .all(|p| p.progress_changes.is_empty())
    );
}
#[test]
fn proposal_recalculation_refuses_over_count_before_candidate_evaluation() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let mut planning = request(&db);
    let proposer = planning["participants"][0]["profile_id"]
        .as_str()
        .unwrap()
        .to_string();
    planning["candidates"] = json!(vec![planning["candidates"][0].clone(); 129]);
    assert!(
        db.app
            .propose_party_plan(
                &json!({"planning":planning,"proposer_profile_id":proposer,"expected":null})
                    .to_string(),
                Some(UtcMillis::new(NOW).unwrap())
            )
            .is_err()
    );
    assert!(
        db.app
            .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap()
            .is_none()
    );
}
