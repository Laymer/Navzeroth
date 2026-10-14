use azeroth_app::{
    planning_jobs::{PlanningJobs, PlanningKind},
    snapshots::SnapshotApplication,
    today::PlanState,
};
use azeroth_core::{
    error::ErrorCode,
    types::{DatasetRevision, PartyId, UtcMillis},
};
use azeroth_storage::{
    ProfileKey, SnapshotStore, SqliteStore,
    today::{MAX_DOCUMENT, MAX_REVISION, Settings},
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
fn explicit_pin_survives_one_shot_result_and_restart_without_watch_or_window() {
    let path = Temp::new();
    let app = SnapshotApplication::open(&path.0).unwrap();
    let mut s = snapshot();
    s["x_secret"] = json!("planted-private-export-secret");
    let request = setup(&app, &s);
    let jobs = PlanningJobs::default();
    let id = run(&app, &jobs, PlanningKind::Solo, &request);
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Empty
    ));
    jobs.take_result(&id).unwrap();
    assert!(jobs.take_result(&id).is_err());
    assert_eq!(
        app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
            .unwrap(),
        2
    );
    let before = app.today_view(Some(time(NOW))).unwrap();
    assert!(matches!(before.current_plan.state, PlanState::Current));
    assert_eq!(
        before.current_plan.summary.unwrap().steps[0]
            .objective
            .as_str(),
        "synthetic-service"
    );
    assert!(!app.watch_status().unwrap().active);
    drop(app);
    let app = SnapshotApplication::open(&path.0).unwrap();
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Current
    ));
    assert_eq!(
        app.status().unwrap().active_profile.as_deref(),
        request["profile_id"].as_str()
    );
    assert!(!app.watch_status().unwrap().active);
    let store = SqliteStore::open(&path.0).unwrap();
    let descriptor = store.current_plan().unwrap().plan.unwrap();
    assert!(
        !format!(
            "{}{}{}",
            descriptor.request_json,
            serde_json::to_string(&descriptor.guard).unwrap(),
            serde_json::to_string(&descriptor.summary).unwrap()
        )
        .contains("planted-private-export-secret")
    );
    let reference = app.reference_view(Some(time(NOW))).unwrap();
    assert!(matches!(
        reference.current_plan.unwrap().state,
        PlanState::Current
    ));
}
#[test]
fn cas_clear_is_monotonic_and_failed_replace_preserves_existing_plan() {
    let app = SnapshotApplication::in_memory().unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let id = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
        .unwrap();
    assert!(app.clear_current_plan(1).is_err());
    assert_eq!(app.clear_current_plan(2).unwrap(), 3);
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Empty
    ));
    assert!(
        app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
            .is_err()
    );
    assert_eq!(
        app.pin_current_plan(&jobs, &id, 3, Some(time(NOW)))
            .unwrap(),
        4
    );
    assert!(
        app.pin_current_plan(&jobs, "foreign", 4, Some(time(NOW)))
            .is_err()
    );
    assert_eq!(
        app.today_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .revision,
        4
    );
}
#[test]
fn captures_field_expiry_rollback_future_fields_and_unknown_clock_invalidate_saved_advice() {
    for observed in [NOW - 1_790_000, NOW + 10000] {
        let app = SnapshotApplication::in_memory().unwrap();
        let mut s = snapshot();
        s["payload"]["level"]["clock_trust"] = json!("trusted");
        s["payload"]["level"]["observed_at_utc_ms"] = json!(observed);
        let request = setup(&app, &s);
        let jobs = PlanningJobs::default();
        let id = run(&app, &jobs, PlanningKind::Solo, &request);
        app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
            .unwrap();
        assert!(matches!(
            app.today_view(Some(time(NOW + 20000)))
                .unwrap()
                .current_plan
                .state,
            PlanState::Historical
        ));
        assert_eq!(
            app.pin_current_plan(&jobs, &id, 2, Some(time(NOW + 20000)))
                .unwrap_err()
                .code,
            ErrorCode::SnapshotOutdated
        );
        assert_eq!(
            app.today_view(Some(time(NOW + 20000)))
                .unwrap()
                .current_plan
                .revision,
            2
        );
    }
    let app = SnapshotApplication::in_memory().unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let id = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
        .unwrap();
    for now in [Some(time(NOW - 100000)), Some(time(NOW + 1_800_001)), None] {
        let p = app.today_view(now).unwrap().current_plan;
        assert!(matches!(p.state, PlanState::Historical));
        assert!(p.summary.is_some());
    }
}
#[test]
fn changed_snapshot_content_and_missing_subject_keep_explanatory_historical_slot() {
    let path = Temp::new();
    let app = SnapshotApplication::open(&path.0).unwrap();
    let mut s = snapshot();
    let request = setup(&app, &s);
    let jobs = PlanningJobs::default();
    let id = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
        .unwrap();
    s["payload"]["id"] = json!("new-current-snapshot");
    s["producer"]["sequence"] = json!(2000);
    s["payload"]["capture"]["sequence"] = json!(2000);
    s["payload"]["capture"]["session_elapsed_ms"] = json!(500000);
    app.import_text(&format!("ANAV1:{s}"), Some(time(NOW)))
        .unwrap();
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Historical
    ));
    let key = ProfileKey::of(
        &azeroth_core::contract::decode_envelope::<azeroth_core::entities::CharacterSnapshot>(
            &s.to_string(),
        )
        .unwrap(),
    );
    SqliteStore::open(&path.0)
        .unwrap()
        .delete_profile(&key)
        .unwrap();
    let view = app.today_view(Some(time(NOW))).unwrap();
    assert!(!view.selection_available);
    assert!(matches!(view.current_plan.state, PlanState::Historical));
    assert_eq!(view.current_plan.revision, 2);
}
#[test]
fn canceled_failed_foreign_and_pruned_jobs_cannot_replace_slot() {
    let app = SnapshotApplication::in_memory().unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let first = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &first, 1, Some(time(NOW)))
        .unwrap();
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let canceled = ticket.id().to_owned();
    jobs.cancel(&canceled).unwrap();
    jobs.run(ticket, |_| Ok(json!({"ignored":true})));
    assert!(
        app.pin_current_plan(&jobs, &canceled, 2, Some(time(NOW)))
            .is_err()
    );
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let failed = ticket.id().to_owned();
    jobs.run(ticket, |_| {
        Err::<Value, _>(azeroth_core::error::AppError::new(
            ErrorCode::InvalidInput,
            "failure",
            false,
        ))
    });
    assert!(
        app.pin_current_plan(&jobs, &failed, 2, Some(time(NOW)))
            .is_err()
    );
    let foreign = PlanningJobs::default();
    assert!(
        app.pin_current_plan(&foreign, &first, 2, Some(time(NOW)))
            .is_err()
    );
    for _ in 0..17 {
        let t = jobs.register(PlanningKind::Solo).unwrap();
        jobs.run(t, |_| Ok(json!({"empty":true})));
    }
    assert!(
        app.pin_current_plan(&jobs, &first, 2, Some(time(NOW)))
            .is_err()
    );
    assert_eq!(
        app.today_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .revision,
        2
    );
}
#[test]
fn joint_pin_is_local_intent_and_membership_changes_invalidate_it() {
    let app = SnapshotApplication::in_memory().unwrap();
    let first = setup(&app, &snapshot());
    let mut second = snapshot();
    second["payload"]["character"] = json!("second-member");
    second["payload"]["id"] = json!("second-snapshot");
    let id = app
        .import_text(&format!("ANAV1:{second}"), Some(time(NOW)))
        .unwrap()
        .profile_id;
    let party = PartyId::new("local-party").unwrap();
    app.save_party(
        &party,
        &[first["profile_id"].as_str().unwrap().into(), id.clone()],
    )
    .unwrap();
    let catalog = app.joint_planning_catalog(&party, Some(time(NOW))).unwrap();
    let mut profiles: Vec<_> = catalog
        .participants
        .iter()
        .map(|p| json!({"profile_id":p.profile_id,"snapshot_id":p.snapshot_id}))
        .collect();
    profiles.reverse(); // Request member order need not equal declared party order.
    let utilities: serde_json::Map<String, Value> = catalog
        .participants
        .iter()
        .map(|p| (p.profile_id.clone(), json!(10)))
        .collect();
    let request = json!({"party_id":"local-party","participants":profiles,"revision":"synthetic-planning-v1","goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utilities":utilities,"completion_ms":1000,"service_ms":0}],"excluded":[]});
    let record = app.app_settings().unwrap();
    let mut settings = record.settings;
    settings.selected_subject = Some(azeroth_storage::today::Subject::Party {
        party_id: party.clone(),
    });
    app.update_settings(
        &json!({"schema_version":1,"expected_revision":record.revision,"settings":settings})
            .to_string(),
    )
    .unwrap();
    let jobs = PlanningJobs::default();
    let job = run(&app, &jobs, PlanningKind::Joint, &request);
    app.pin_current_plan(&jobs, &job, 1, Some(time(NOW)))
        .unwrap();
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Current
    ));
    assert!(
        app.party_plan_proposal(&party, Some(time(NOW)))
            .unwrap()
            .is_none()
    );
    app.save_party(&party, &[id]).unwrap();
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Historical
    ));
}
#[test]
fn settings_are_bounded_explicit_and_cas_preserves_false_zero_and_unknown_goal() {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut settings = Settings {
        selected_installation: Some("/chosen/local/install".into()),
        reference_always_on_top: false,
        ..Settings::default()
    };
    settings.manual_planning_defaults.budget_ms = azeroth_core::types::Millis::new(0).unwrap();
    let update = json!({"schema_version":1,"expected_revision":0,"settings":settings});
    let saved = app.update_settings(&update.to_string()).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.settings.manual_planning_defaults.budget_ms.get(), 0);
    assert!(
        saved
            .settings
            .manual_planning_defaults
            .goal_profile
            .is_none()
    );
    assert!(!saved.settings.reference_always_on_top);
    assert!(!app.watch_status().unwrap().active);
    assert!(app.update_settings(&update.to_string()).is_err());
    assert_eq!(app.app_settings().unwrap().revision, 1);
    let mut invalid = serde_json::to_value(&saved.settings).unwrap();
    invalid["accessibility"]["text_scale_percent"] = json!(99);
    assert!(azeroth_storage::today::validate_settings_document(&invalid.to_string()).is_err());
    invalid = serde_json::to_value(&saved.settings).unwrap();
    invalid["selected_installation"] = json!("a".repeat(4097));
    assert!(azeroth_storage::today::validate_settings_document(&invalid.to_string()).is_err());
    for text in [
        "{\"schema_version\":1,\"schema_version\":1}",
        "{\"schema_version\":2}",
        &" ".repeat(MAX_DOCUMENT + 1),
    ] {
        assert!(azeroth_storage::today::validate_settings_document(text).is_err());
    }
}
#[test]
fn malformed_saved_documents_and_revision_exhaustion_fail_without_mutation() {
    let path = Temp::new();
    let app = SnapshotApplication::open(&path.0).unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let job = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &job, 1, Some(time(NOW)))
        .unwrap();
    let store = SqliteStore::open(&path.0).unwrap();
    let plan = store.current_plan().unwrap().plan.unwrap();
    let guard = serde_json::to_string(&plan.guard).unwrap();
    let summary = serde_json::to_string(&plan.summary).unwrap();
    azeroth_storage::today::validate_current_plan_documents(
        "solo",
        &plan.request_json,
        &guard,
        &summary,
    )
    .unwrap();
    let mut v: Value = serde_json::from_str(&guard).unwrap();
    v["profiles"][0]["key"]["unknown_nested"] = json!(true);
    assert!(
        azeroth_storage::today::validate_current_plan_documents(
            "solo",
            &plan.request_json,
            &v.to_string(),
            &summary
        )
        .is_err()
    );
    v = serde_json::from_str(&summary).unwrap();
    v["schema_version"] = json!(2);
    assert!(
        azeroth_storage::today::validate_current_plan_documents(
            "solo",
            &plan.request_json,
            &guard,
            &v.to_string()
        )
        .is_err()
    );
    assert!(app.clear_current_plan(MAX_REVISION).is_err());
    assert_eq!(
        app.today_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .revision,
        2
    );
}
#[test]
fn content_restrict_release_reactivate_aba_refuses_original_publication_and_pin() {
    let app = SnapshotApplication::in_memory().unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let job = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &job, 1, Some(time(NOW)))
        .unwrap();
    let (_, guard) = app
        .calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(time(NOW)),
            &mut azeroth_core::search_control::NoopSearchControl,
        )
        .unwrap();
    let profile = request["profile_id"].as_str().unwrap();
    for state in ["quarantined", "eligible"] {
        let g = app.content_governance(profile).unwrap();
        let change = json!({"schema_version":1,"profile_id":profile,"mutation":{"guard":g.revisions[0].guard(),"state":state,"reason_code":"operator_review"}});
        app.change_content_disposition(&change.to_string(), Some(time(NOW)))
            .unwrap();
    }
    app.activate_content(
        profile,
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    assert_eq!(
        guard
            .publish(&app, Some(time(NOW)), &mut || Ok(()))
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Historical
    ));
    assert_eq!(
        app.pin_current_plan(&jobs, &job, 2, Some(time(NOW)))
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    assert_eq!(
        app.today_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .revision,
        2
    );
}
#[test]
fn synthetic_today_wire_fixture_has_declared_clock_and_original_steps() {
    let app = SnapshotApplication::in_memory().unwrap();
    let source = snapshot();
    let request = setup(&app, &source);
    let jobs = PlanningJobs::default();
    let job = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &job, 1, Some(time(NOW)))
        .unwrap();
    let view = app.today_view(Some(time(NOW))).unwrap();
    let mut value = serde_json::to_value(view).unwrap();
    // The wire fixture uses the explicitly declared fixture clock, not measured
    // wall-clock duration. Production service ages retain actual elapsed UTC.
    let typed: azeroth_core::contract::Envelope<azeroth_core::entities::CharacterSnapshot> =
        azeroth_core::contract::decode_envelope(&source.to_string()).unwrap();
    let clock = azeroth_core::observation::FreshnessContext {
        now_utc_ms: Some(time(NOW)),
        clock_trust: azeroth_core::observation::ClockTrust::Trusted,
        session: None,
        max_age_ms: azeroth_core::types::Millis::new(1800000).unwrap(),
    };
    value["members"][0]["freshness"] = serde_json::to_value(azeroth_core::observation::freshness(
        &typed.payload.capture,
        &clock,
    ))
    .unwrap();
    let saved = value["current_plan"]["saved_at_utc_ms"].as_i64().unwrap();
    value["current_plan"]["age_ms"] = json!(NOW.checked_sub(saved).unwrap());
    let fixture = json!({"clock_semantics":"fixed_declared_synthetic_utc","now_utc_ms":NOW,"today":value,"settings":app.app_settings().unwrap(),"snapshot":source,"profiles":app.profiles().unwrap(),"reference":azeroth_app::reference::build(Some(&typed),&clock).unwrap()});
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/desktop/today/synthetic-view.json");
    if std::env::var_os("UPDATE_TODAY_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&fixture).unwrap()),
        )
        .unwrap();
    }
    let committed: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(fixture, committed);
}

#[test]
fn recovered_reference_remains_historical_even_with_identical_current_inputs() {
    let path = Temp::new();
    let app = SnapshotApplication::open(&path.0).unwrap();
    let request = setup(&app, &snapshot());
    let jobs = PlanningJobs::default();
    let id = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &id, 1, Some(time(NOW)))
        .unwrap();
    let store = SqliteStore::open(&path.0).unwrap();
    let mut plan = store.current_plan().unwrap().plan.unwrap();
    let original = serde_json::to_string(&plan.guard).unwrap();
    let changed = azeroth_storage::today::mark_recovered_plan_guard(&original).unwrap();
    plan.guard = serde_json::from_str(&changed).unwrap();
    store
        .with_intent_transaction(|s| s.replace_current_plan(2, Some(&plan)))
        .unwrap();
    let view = app.today_view(Some(time(NOW))).unwrap();
    assert!(matches!(view.current_plan.state, PlanState::Historical));
    assert!(view.current_plan.summary.is_some());
    assert_eq!(view.current_plan.revision, 3);
    assert!(matches!(
        app.reference_view(Some(time(NOW)))
            .unwrap()
            .current_plan
            .unwrap()
            .state,
        PlanState::Historical
    ));
    let next = run(&app, &jobs, PlanningKind::Solo, &request);
    app.pin_current_plan(&jobs, &next, 3, Some(time(NOW)))
        .unwrap();
    assert!(matches!(
        app.today_view(Some(time(NOW))).unwrap().current_plan.state,
        PlanState::Current
    ));
}
