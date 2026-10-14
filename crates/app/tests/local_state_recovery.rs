use azeroth_app::{
    planning_jobs::{PlanningJobs, PlanningKind},
    snapshots::SnapshotApplication,
    today::PlanState,
};
use azeroth_core::types::{DatasetRevision, UtcMillis};
use azeroth_storage::{
    SqliteStore,
    local_submissions::SubmissionAction,
    recovery::reconstruct_backup,
    today::{PlanProvenance, Settings},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
const NOW: i64 = 1791792000000;
const BUNDLE: &str = include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
fn time(n: i64) -> UtcMillis {
    UtcMillis::new(n).unwrap()
}
fn snapshot() -> Value {
    let mut s: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    s["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW);
    s
}
fn setup(app: &SnapshotApplication, snapshot: &Value) -> Value {
    let id = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(time(NOW)))
        .unwrap()
        .profile_id;
    app.stage_content(BUNDLE).unwrap();
    app.activate_content(&id, &DatasetRevision::new("synthetic-planning-v1").unwrap())
        .unwrap();
    app.select_profile(&id).unwrap();
    let c = app.planning_catalog(&id).unwrap();
    json!({"profile_id":id,"snapshot_id":c.snapshot_id,"revision":c.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]})
}
fn run(
    app: &SnapshotApplication,
    jobs: &PlanningJobs,
    kind: PlanningKind,
    request: &Value,
) -> String {
    let ticket = jobs.register(kind).unwrap();
    let id = ticket.id().to_owned();
    let guard = RefCell::new(None);
    jobs.run_guarded_with_pin(
        ticket,
        |control| {
            let (report, g) =
                app.calculate_planning_job(kind, &request.to_string(), Some(time(NOW)), control)?;
            let pin = g.pin_descriptor(&report, Some(time(NOW)))?;
            *guard.borrow_mut() = Some(g);
            Ok((report, Some(pin)))
        },
        |publish| {
            guard
                .borrow()
                .as_ref()
                .unwrap()
                .publish(app, Some(time(NOW)), publish)
        },
    );
    assert_eq!(jobs.status(&id).unwrap().state, "succeeded");
    id
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self(std::env::temp_dir().join(format!(
            "navzeroth-today-{}-{}.sqlite",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}
#[test]
fn populated_backup_preserves_intent_queue_tombstones_and_marks_advice_historical() {
    let source = Temp::new();
    let destination = Temp::new();
    let app = SnapshotApplication::open(&source.0).unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let id = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
        .unwrap();
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Current
    ));
    drop(app);
    let mut store = SqliteStore::open(&source.0).unwrap();
    let mut settings = Settings::default();
    settings.accessibility.text_scale_percent = 200;
    settings.selected_installation = Some("explicit-local-installation".into());
    settings.reference_always_on_top = true;
    let settings_revision = store.settings().unwrap().revision;
    let settings_before = store
        .with_intent_transaction(|s| s.replace_settings(settings_revision, &settings))
        .unwrap();
    let plan_before = store.current_plan().unwrap();
    assert!(matches!(
        plan_before.plan.as_ref().unwrap().guard.provenance,
        PlanProvenance::LocalGuardedSuccess
    ));
    let demo = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    store.stage_content_revision(&demo).unwrap();
    let observation = azeroth_core::submissions::decode_local_observation(include_str!(
        "../../../fixtures/desktop/submissions/synthetic-price.json"
    ))
    .unwrap();
    let pending = store
        .submit_local_observation(&observation, true, Some(time(0)))
        .unwrap();
    let accepted = store
        .change_local_submission(
            &pending.record.guard,
            SubmissionAction::Accept,
            Some(time(NOW)),
        )
        .unwrap();
    let mut erased_observation = observation.clone();
    erased_observation.observed_at_utc_ms = Some(time(0));
    let erased = store
        .submit_local_observation(&erased_observation, true, None)
        .unwrap();
    let erased = store
        .change_local_submission(&erased.record.guard, SubmissionAction::Withdraw, None)
        .unwrap();
    let dispositions_before = store.list_content_dispositions(&demo.key().scope).unwrap();
    drop(store);
    let bytes = std::fs::read(&source.0).unwrap();
    let inspection = reconstruct_backup(&source.0, &destination.0).unwrap();
    assert_eq!(inspection.rows["local_submissions"], 2);
    assert_eq!(std::fs::read(&source.0).unwrap(), bytes);
    let store = SqliteStore::open(&destination.0).unwrap();
    assert_eq!(
        serde_json::to_value(store.settings().unwrap()).unwrap(),
        serde_json::to_value(settings_before).unwrap()
    );
    assert_eq!(
        store.local_submission(&accepted.guard.id).unwrap().unwrap(),
        accepted
    );
    assert_eq!(
        store.local_submission(&erased.guard.id).unwrap().unwrap(),
        erased
    );
    assert_eq!(
        store.list_content_dispositions(&demo.key().scope).unwrap(),
        dispositions_before
    );
    let restored = store.current_plan().unwrap();
    assert_eq!(restored.revision, plan_before.revision);
    let mut expected = plan_before.plan.unwrap();
    expected.guard.provenance = PlanProvenance::RecoveredReference;
    let restored = restored.plan.unwrap();
    assert_eq!(
        serde_json::to_value(restored.kind).unwrap(),
        serde_json::to_value(expected.kind).unwrap()
    );
    assert_eq!(restored.request_json, expected.request_json);
    assert_eq!(
        serde_json::to_value(restored.guard).unwrap(),
        serde_json::to_value(expected.guard).unwrap()
    );
    assert_eq!(
        serde_json::to_value(restored.summary).unwrap(),
        serde_json::to_value(expected.summary).unwrap()
    );
    drop(store);
    let recovered = SnapshotApplication::open(&destination.0).unwrap();
    assert!(matches!(
        recovered
            .today_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .state,
        PlanState::Historical
    ));
    assert!(matches!(
        recovered
            .reference_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .unwrap()
            .state,
        PlanState::Historical
    ));
    assert!(!recovered.watch_status().unwrap().active);
    drop(recovered);
    let repeated = Temp::new();
    reconstruct_backup(&destination.0, &repeated.0).unwrap();
    let recovered = SnapshotApplication::open(&repeated.0).unwrap();
    assert!(matches!(
        recovered
            .today_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .state,
        PlanState::Historical
    ));
}
