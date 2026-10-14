use azeroth_app::{
    planning_jobs::{PlanningJobs, PlanningKind},
    snapshots::SnapshotApplication,
};
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    search_control::{NoopSearchControl, SearchControl, SearchPhase},
    types::{DatasetRevision, PartyId, UtcMillis},
};
use serde_json::{Value, json};
use std::sync::{Arc, Barrier, mpsc};
use std::time::Duration;
const BUNDLE: &str = include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
fn setup() -> (Arc<SnapshotApplication>, Value, Value, UtcMillis) {
    let app = Arc::new(SnapshotApplication::in_memory().unwrap());
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let now = UtcMillis::new(
        snapshot["payload"]["capture"]["observed_at_utc_ms"]
            .as_i64()
            .unwrap()
            + 100,
    )
    .unwrap();
    let id = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    app.stage_content(BUNDLE).unwrap();
    app.activate_content(&id, &DatasetRevision::new("synthetic-planning-v1").unwrap())
        .unwrap();
    let catalog = app.planning_catalog(&id).unwrap();
    let request = json!({"profile_id":id,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
    (app, request, snapshot, now)
}
struct Pause {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    paused: bool,
}
impl SearchControl for Pause {
    fn checkpoint(&mut self, phase: SearchPhase) -> Result<()> {
        if !self.paused && phase == SearchPhase::CandidateAssessment {
            self.paused = true;
            self.entered.wait();
            self.release.wait();
        }
        Ok(())
    }
}
#[test]
fn calculation_releases_store_lock_and_changed_imported_snapshot_rejects_success() {
    let (app, request, mut snapshot, now) = setup();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let worker_app = app.clone();
    let worker_entered = entered.clone();
    let worker_release = release.clone();
    let worker = std::thread::spawn(move || {
        worker_app.calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(now),
            &mut Pause {
                entered: worker_entered,
                release: worker_release,
                paused: false,
            },
        )
    });
    entered.wait();
    let (tx, rx) = mpsc::channel();
    let read_app = app.clone();
    let reader = std::thread::spawn(move || {
        tx.send(read_app.profiles()).unwrap();
    });
    let responsive = rx.recv_timeout(Duration::from_secs(2));
    // Always release the worker if the regression returns; avoid hanging tests.
    if responsive.is_err() {
        release.wait();
        let _ = worker.join();
        reader.join().unwrap();
        panic!("Profile lookup held behind paused calculation");
    }
    assert!(!responsive.unwrap().unwrap().is_empty());
    reader.join().unwrap();
    snapshot["payload"]["capture"]["observed_at_utc_ms"] = json!(now.get());
    snapshot["payload"]["level"]["value"] = json!(23);
    snapshot["payload"]["capture"]["sequence"] =
        json!(snapshot["payload"]["capture"]["sequence"].as_u64().unwrap() + 1);
    snapshot["producer"]["sequence"] = snapshot["payload"]["capture"]["sequence"].clone();
    // Existing import identity rules refuse ID reuse with changed content.
    let reused = app.import_text(&format!("ANAV1:{snapshot}"), Some(now));
    snapshot["payload"]["id"] = json!("planning-job-new-snapshot");
    let imported = app.import_text(&format!("ANAV1:{snapshot}"), Some(now));
    release.wait();
    assert_eq!(reused.unwrap_err().code, ErrorCode::InvalidInput);
    imported.unwrap();
    assert_eq!(
        worker.join().unwrap().err().unwrap().code,
        ErrorCode::SnapshotOutdated
    );
}
#[test]
fn final_serialized_publication_checks_revision_and_freshness_before_handoff() {
    let (app, request, _, now) = setup();
    let (report, guard) = app
        .calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(now),
            &mut NoopSearchControl,
        )
        .unwrap();
    let mut published = false;
    assert_eq!(
        guard
            .publish(
                &app,
                Some(UtcMillis::new(now.get() + 1_800_001).unwrap()),
                &mut || {
                    published = true;
                    Ok(())
                }
            )
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    assert!(!published);
    let jobs = PlanningJobs::default();
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    let mut changed: Value = serde_json::from_str(BUNDLE).unwrap();
    changed["envelope"]["payload"]["revision"] = json!("synthetic-planning-v2");
    jobs.run_guarded(
        ticket,
        |_| Ok(report),
        |publish| {
            app.stage_content(&changed.to_string())?;
            app.activate_content(
                request["profile_id"].as_str().unwrap(),
                &DatasetRevision::new("synthetic-planning-v2").unwrap(),
            )?;
            guard.publish(&app, Some(now), publish)
        },
    );
    assert_eq!(
        jobs.status(&id).unwrap().error.unwrap().code,
        ErrorCode::SnapshotOutdated
    );
    assert!(!jobs.status(&id).unwrap().result_available);
}
#[test]
fn cooperative_cancel_during_application_candidate_preparation_preserves_storage() {
    let (app, request, _, now) = setup();
    struct Cancel;
    impl SearchControl for Cancel {
        fn checkpoint(&mut self, phase: SearchPhase) -> Result<()> {
            if phase == SearchPhase::CandidateAssessment {
                Err(AppError::new(
                    ErrorCode::OperationCanceled,
                    "Synthetic cancellation",
                    false,
                ))
            } else {
                Ok(())
            }
        }
    }
    let before = app
        .planning_catalog(request["profile_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        app.calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(now),
            &mut Cancel
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::OperationCanceled
    );
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(
            app.planning_catalog(request["profile_id"].as_str().unwrap())
                .unwrap()
        )
        .unwrap()
    );
}

#[test]
fn joint_publication_rechecks_membership_and_each_participant_snapshot() {
    let (app, solo, mut second, now) = setup();
    second["payload"]["character"] = json!("second-planning-member");
    second["payload"]["id"] = json!("second-planning-snapshot");
    let second_id = app
        .import_text(&format!("ANAV1:{second}"), Some(now))
        .unwrap()
        .profile_id;
    let first_id = solo["profile_id"].as_str().unwrap().to_owned();
    let members = vec![first_id, second_id];
    let party = PartyId::new("planning-publication-duo").unwrap();
    app.save_party(&party, &members).unwrap();
    let catalog = app.joint_planning_catalog(&party, Some(now)).unwrap();
    let request = json!({"party_id":party,"participants":catalog.participants.iter().map(|p| json!({"profile_id":p.profile_id,"snapshot_id":p.snapshot_id})).collect::<Vec<_>>(),"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[],"excluded":[]});
    let (_, guard) = app
        .calculate_planning_job(
            PlanningKind::Joint,
            &request.to_string(),
            Some(now),
            &mut NoopSearchControl,
        )
        .unwrap();
    let mut published = false;
    let mut publish = || {
        published = true;
        Ok(())
    };
    app.save_party(&party, &[members[1].clone(), members[0].clone()])
        .unwrap();
    assert_eq!(
        guard
            .publish(&app, Some(now), &mut publish)
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    app.save_party(&party, &members).unwrap();
    second["payload"]["id"] = json!("second-planning-snapshot-new");
    second["payload"]["capture"]["sequence"] = json!(13);
    second["producer"]["sequence"] = json!(13);
    second["payload"]["capture"]["observed_at_utc_ms"] = json!(now.get());
    app.import_text(&format!("ANAV1:{second}"), Some(now))
        .unwrap();
    assert_eq!(
        guard
            .publish(&app, Some(now), &mut publish)
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    assert!(!published);
}
