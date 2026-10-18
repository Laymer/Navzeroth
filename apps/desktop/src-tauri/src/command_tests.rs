//! Official mock IPC runtime; real WebView2 evidence remains separate.
use super::*;
use serde_json::Value;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    command: String,
    args: Value,
    codes: Option<Vec<String>>,
}
fn command_handler() -> fn(tauri::ipc::Invoke<tauri::test::MockRuntime>) -> bool {
    tauri::generate_handler![
        start_planning_job,
        query_atlas,
        get_snapshot_markers,
        preview_diagnostics,
        stage_verified_content_bundle,
        pin_current_plan,
        clear_current_plan,
        finish_local_startup,
        get_reference_window_state,
        toggle_reference_window,
        set_reference_interaction,
        set_reference_window,
        forget_local_data_job,
        forget_local_material,
        get_calibration,
        get_joint_planning_catalog,
        project_positions,
        reminder_view,
        review_selected_local_guide,
        set_session_calibration,
        get_app_status,
        list_profiles,
        list_parties,
        get_app_settings,
        get_today_view,
        get_local_workflow,
        list_local_materials,
        get_local_change_detectors,
        list_local_installations,
        list_local_data_jobs,
        get_database_status,
        check_database,
        get_reference_view,
        get_party_plan_proposal,
        get_party,
        delete_party,
        get_planning_job,
        cancel_planning_job,
        take_planning_job_result,
        get_planning_catalog,
        get_content_dispositions,
        get_active_content,
        rollback_content,
        get_character_snapshot,
        get_stored_snapshot,
        select_profile,
        list_sessions,
        get_gear_catalog,
        get_compatibility_report,
        get_historical_snapshot,
        promote_snapshot,
        activate_content,
        get_party_quests,
        get_party_eligibility,
        save_party,
        get_snapshot_history,
        import_snapshot_text,
        import_snapshot_file,
        start_snapshot_watch,
        get_snapshot_watch,
        stop_snapshot_watch,
        import_session_text,
        import_session_file,
        get_session_detail,
        delete_session,
        create_database_backup,
        inspect_database_backup,
        restore_database_backup,
        export_diagnostics,
        list_local_submissions,
        submit_local_observation,
        get_local_data_job,
        cancel_local_data_job,
        update_app_settings,
        mutate_reminders,
        plan_joint_session,
        propose_party_plan,
        accept_party_plan,
        change_content_disposition,
        export_plan,
        export_agreed_party_plan,
        inspect_plan_route,
        plan_session,
        get_content_inspection_page,
        stage_content,
        get_catalog_page,
        get_authored_catalog,
        get_dungeon_preparation,
        compare_gear,
        preview_acquisition_route,
        assess_route_acquisition,
        change_local_submission,
        export_local_submission,
        preview_local_data,
        start_local_data,
        save_local_workflow_choices,
        acknowledge_local_build_change,
        stage_selected_local_input,
        get_local_input_page,
        discover_local_media,
        extract_local_media,
        get_local_media_page,
        get_local_media_manifest,
        get_local_artifact_page,
        get_research_delivery_manifest,
        build_local_inventory,
        get_local_inventory_summary,
        get_local_inventory_page,
        get_local_tooltip,
        restore_local_installation,
        resume_local_data,
        get_local_data_activation,
        activate_local_data,
        get_local_data_inventory,
        prepare_research_delivery,
        get_research_delivery_page,
        get_research_delivery_activation,
        activate_research_delivery,
        diff_research_delivery,
    ]
}
fn request(command: &str, args: Value) -> tauri::webview::InvokeRequest {
    tauri::webview::InvokeRequest {
        cmd: command.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: if cfg!(any(windows, target_os = "android")) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        }
        .parse()
        .unwrap(),
        body: tauri::ipc::InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.into(),
    }
}
#[test]
fn missing_managed_services_refuse_before_application_dispatch() {
    let app = tauri::test::mock_builder()
        .invoke_handler(command_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let view =
        tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::App("index.html".into()))
            .build()
            .unwrap();
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../fixtures/desktop/native-boundaries/requests.json"
    ))
    .unwrap();
    for case in cases {
        let error = tauri::test::get_ipc_response(&view, request(&case.command, case.args))
            .err()
            .unwrap();
        let text = error
            .as_str()
            .expect("Tauri dependency error is a transport string, not an application refusal");
        assert!(
            text.contains("state not managed for field `state`"),
            "{}: {text}",
            case.command
        );
        assert!(text.contains(&format!("on command `{}`", case.command)));
    }
    for command in [
        "get_app_status",
        "list_profiles",
        "list_parties",
        "get_app_settings",
        "get_today_view",
        "get_local_workflow",
        "list_local_materials",
        "get_local_change_detectors",
        "list_local_installations",
        "list_local_data_jobs",
        "get_database_status",
        "check_database",
        "get_reference_view",
        "get_party_plan_proposal",
        "get_party",
        "delete_party",
        "start_snapshot_watch",
        "get_snapshot_watch",
        "stop_snapshot_watch",
    ] {
        let error = tauri::test::get_ipc_response(&view, request(command, serde_json::json!({})))
            .err()
            .unwrap();
        assert!(
            error
                .as_str()
                .unwrap()
                .contains("state not managed for field `state`"),
            "{command}: {error}"
        );
    }
}
#[test]
fn invalid_native_ipc_preserves_current_reference_and_structured_errors() {
    let root = std::env::temp_dir().join(format!("nav-native-ipc-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let host = Arc::new(azeroth_app::recovery::DatabaseHost::open(
        &root.join("state.sqlite3"),
    ));
    let before = serde_json::to_value(
        host.with_application(|app| app.reference_view(None))
            .unwrap(),
    )
    .unwrap();
    let app = tauri::test::mock_builder()
        .manage(host.clone())
        .manage(reference_window::Controller::default())
        .manage(Arc::new(azeroth_app::local_data::LocalDataService::new(
            root.join("private"),
        )))
        .invoke_handler(command_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let view =
        tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::App("index.html".into()))
            .build()
            .unwrap();
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../fixtures/desktop/native-boundaries/requests.json"
    ))
    .unwrap();
    for case in cases {
        let result = tauri::test::get_ipc_response(
            &view,
            tauri::webview::InvokeRequest {
                cmd: case.command.clone(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(any(windows, target_os = "android")) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(case.args),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_owned(),
            },
        );
        assert!(
            result.is_err(),
            "{} unexpectedly accepted invalid input",
            case.command
        );
        let error = result.err().unwrap();
        let codes = case
            .codes
            .unwrap_or_else(|| vec!["INVALID_INPUT".into(), "STORAGE_ERROR".into()]);
        assert!(
            error["code"]
                .as_str()
                .is_some_and(|code| codes.iter().any(|allowed| allowed == code)),
            "{}: {}",
            case.command,
            error
        );
        assert!(error["message"].is_string(), "{}: {}", case.command, error);
        assert!(
            error["retryable"].is_boolean(),
            "{}: {}",
            case.command,
            error
        );
    }
    // A selected missing file is an accepted watcher startup, not a refusal.
    // Exercise its asynchronous status and explicit stop through real handlers.
    let invoke = |command: &str, args: Value| -> Value {
        tauri::test::get_ipc_response(
            &view,
            tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(any(windows, target_os = "android")) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .unwrap()
        .deserialize()
        .unwrap()
    };
    for command in [
        "list_profiles",
        "list_parties",
        "list_local_materials",
        "get_local_change_detectors",
        "list_local_installations",
        "list_local_data_jobs",
    ] {
        assert_eq!(
            invoke(command, serde_json::json!({})),
            serde_json::json!([]),
            "{command}"
        );
    }
    for command in [
        "get_app_status",
        "get_app_settings",
        "get_today_view",
        "get_local_workflow",
        "get_reference_view",
    ] {
        assert!(
            invoke(command, serde_json::json!({})).is_object(),
            "{command}"
        );
    }
    assert_eq!(
        invoke("get_database_status", serde_json::json!({}))["state"],
        "ready"
    );
    assert!(invoke("check_database", serde_json::json!({})).is_null());
    assert!(
        invoke(
            "get_party_plan_proposal",
            serde_json::json!({"party_id":"missing-synthetic-party"})
        )
        .is_null()
    );
    assert!(
        invoke(
            "get_party",
            serde_json::json!({"party_id":"missing-synthetic-party"})
        )
        .is_null()
    );
    assert_eq!(
        invoke(
            "delete_party",
            serde_json::json!({"party_id":"missing-synthetic-party"})
        ),
        false
    );
    let started = invoke(
        "start_snapshot_watch",
        serde_json::json!({"path":root.join("owned-missing.txt"),"format":{"kind":"anav1_text"}}),
    );
    assert_eq!(started["active"], true);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status = invoke("get_snapshot_watch", serde_json::json!({}));
        assert!(status["last_import"].is_null());
        if status["status"] == "missing" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "missing selected file status timed out: {status}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        invoke("stop_snapshot_watch", serde_json::json!({}))["active"],
        false
    );
    assert_eq!(
        invoke("get_reference_window_state", serde_json::json!({}))["exists"],
        false
    );
    assert_eq!(
        invoke(
            "set_reference_window",
            serde_json::json!({"visible":true,"always_on_top":false})
        )["exists"],
        true
    );
    assert_eq!(
        invoke(
            "set_reference_window",
            serde_json::json!({"visible":false,"always_on_top":false})
        )["exists"],
        true
    );
    for (command, args) in [
        (
            "toggle_reference_window",
            serde_json::json!({"always_on_top":false,"interaction_mode":"interactive"}),
        ),
        (
            "set_reference_interaction",
            serde_json::json!({"interaction_mode":"pass_through"}),
        ),
    ] {
        let receipt = invoke(command, args);
        // Official MockRuntime reports the main window unfocused. Preserve the
        // real guard/refusal receipt instead of asserting native focus success.
        assert_eq!(receipt["dispatch"], "refused");
        assert_eq!(receipt["reason"], "main_not_focused");
    }
    let reference = app.get_webview_window(reference_window::LABEL).unwrap();
    let controls = [
        ("get_reference_window_state", serde_json::json!({})),
        (
            "set_reference_window",
            serde_json::json!({"visible":false,"always_on_top":false}),
        ),
        (
            "toggle_reference_window",
            serde_json::json!({"always_on_top":false,"interaction_mode":"interactive"}),
        ),
        (
            "set_reference_interaction",
            serde_json::json!({"interaction_mode":"interactive"}),
        ),
    ];
    let refuse =
        |caller: &tauri::WebviewWindow<tauri::test::MockRuntime>, command: &str, args: Value| {
            let error = tauri::test::get_ipc_response(
                caller,
                tauri::webview::InvokeRequest {
                    cmd: command.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: if cfg!(any(windows, target_os = "android")) {
                        "http://tauri.localhost"
                    } else {
                        "tauri://localhost"
                    }
                    .parse()
                    .unwrap(),
                    body: tauri::ipc::InvokeBody::Json(args),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            )
            .err()
            .unwrap();
            assert_eq!(error["code"], "PROVIDER_UNAVAILABLE", "{command}: {error}");
            assert_eq!(error["retryable"], false);
            error
        };
    for (command, args) in &controls {
        assert!(
            refuse(&reference, command, args.clone())["message"]
                .as_str()
                .unwrap()
                .contains("main desktop")
        );
    }
    let errors = [
        tauri::async_runtime::block_on(local_data_blocking::<()>(app.state::<LocalData>(), |_| {
            panic!("SENTINEL synthetic local worker")
        })),
        tauri::async_runtime::block_on(blocking::<()>(app.state::<Application>(), |_| {
            panic!("SENTINEL synthetic snapshot worker")
        })),
        tauri::async_runtime::block_on(observed_blocking::<()>(
            app.state::<Application>(),
            DiagnosticOperation::Planning,
            |_| panic!("SENTINEL synthetic observed worker"),
        )),
        tauri::async_runtime::block_on(intent_foreground::<()>(app.state::<Application>(), |_| {
            panic!("SENTINEL synthetic foreground worker")
        })),
        tauri::async_runtime::block_on(recovery_blocking::<()>(app.state::<Application>(), |_| {
            panic!("SENTINEL synthetic recovery worker")
        })),
    ];
    for result in errors {
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderUnavailable);
        assert!(!error.retryable);
        let serialized = serde_json::to_value(error).unwrap();
        assert!(!serialized["message"].as_str().unwrap().contains("SENTINEL"));
    }
    assert_eq!(host.status().unwrap().state, "ready");
    let controller = app.state::<reference_window::Controller>();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = controller.inner().0.lock().unwrap();
        panic!("synthetic controller poison");
    }));
    assert!(poisoned.is_err());
    for (command, args) in controls {
        assert!(
            refuse(&view, command, args)["message"]
                .as_str()
                .unwrap()
                .contains("unavailable")
        );
    }
    let after = serde_json::to_value(
        host.with_application(|app| app.reference_view(None))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(before, after);
}

#[test]
fn native_planning_jobs_publish_then_pin_only_the_verified_synthetic_result() {
    let root = std::env::temp_dir().join(format!("nav-native-planning-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let host = Arc::new(azeroth_app::recovery::DatabaseHost::open(
        &root.join("state.sqlite3"),
    ));
    let app = tauri::test::mock_builder()
        .manage(host.clone())
        .invoke_handler(command_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let view =
        tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::App("index.html".into()))
            .build()
            .unwrap();
    let invoke = |command: &str, args: Value| -> Result<Value, Value> {
        tauri::test::get_ipc_response(&view, request(command, args))
            .map(|body| body.deserialize().unwrap())
    };
    let wait = |id: &Value| -> Value {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let status = invoke("get_planning_job", serde_json::json!({"job_id":id})).unwrap();
            if status["state"] != "queued" && status["state"] != "running" {
                return status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "planning job timed out: {status}"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    for kind in ["solo", "joint"] {
        let job = invoke(
            "start_planning_job",
            serde_json::json!({"kind":kind,"text":"{}"}),
        )
        .unwrap();
        let status = wait(&job["job_id"]);
        assert_eq!(status["state"], "failed");
        assert_eq!(status["kind"], kind);
        assert_eq!(status["error"]["code"], "INVALID_INPUT");
        assert_eq!(status["result_available"], false);
        let error = invoke(
            "take_planning_job_result",
            serde_json::json!({"job_id":job["job_id"]}),
        )
        .unwrap_err();
        assert_eq!(error["code"], "INVALID_INPUT");
    }
    let oversized = " ".repeat(azeroth_app::planning::MAX_REQUEST_BYTES + 1);
    assert_eq!(
        invoke(
            "start_planning_job",
            serde_json::json!({"kind":"solo","text":oversized})
        )
        .unwrap_err()["code"],
        "IMPORT_TOO_LARGE"
    );
    assert_eq!(
        invoke("get_snapshot_markers", serde_json::json!({})).unwrap(),
        serde_json::json!([])
    );
    let atlas = invoke("query_atlas", serde_json::json!({})).unwrap_err();
    assert_eq!(atlas["code"], "INVALID_INPUT");
    assert!(
        invoke("preview_diagnostics", serde_json::json!({}))
            .unwrap()
            .is_object()
    );
    let diagnostic_path = root.join("diagnostic.json");
    assert!(
        invoke(
            "export_diagnostics",
            serde_json::json!({"path":diagnostic_path})
        )
        .unwrap()
        .is_object()
    );
    assert!(diagnostic_path.is_file());
    assert_eq!(
        invoke(
            "stage_verified_content_bundle",
            serde_json::json!({"path":root.join("missing-bundle")})
        )
        .unwrap_err()["code"],
        "INVALID_INPUT"
    );
    let now = system_now().unwrap();
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    fn freshen(value: &mut Value, timestamp: i64) {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    if key.ends_with("_utc_ms") && value.is_number() {
                        *value = serde_json::json!(timestamp);
                    } else {
                        freshen(value, timestamp);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    freshen(value, timestamp);
                }
            }
            _ => {}
        }
    }
    freshen(&mut snapshot, now.get() - 100);
    let plan = host.with_application(|application| {
        let profile = application.import_text(&format!("ANAV1:{snapshot}"), Some(now))?.profile_id;
        application.stage_content(include_str!("../../../../fixtures/desktop/planning/synthetic-revision.json"))?;
        application.activate_content(&profile, &DatasetRevision::new("synthetic-planning-v1")?)?;
        let catalog = application.planning_catalog(&profile)?;
        Ok(serde_json::json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]}))
    }).unwrap();
    let job = invoke(
        "start_planning_job",
        serde_json::json!({"kind":"solo","text":plan.to_string()}),
    )
    .unwrap();
    let status = wait(&job["job_id"]);
    assert_eq!(status["state"], "succeeded", "{status}");
    assert_eq!(status["result_available"], true);
    assert_eq!(
        invoke(
            "pin_current_plan",
            serde_json::json!({"job_id":job["job_id"],"expected_revision":0})
        )
        .unwrap(),
        1
    );
    let result = invoke(
        "take_planning_job_result",
        serde_json::json!({"job_id":job["job_id"]}),
    )
    .unwrap();
    let report: Value = serde_json::from_str(result["report_json"].as_str().unwrap()).unwrap();
    assert_eq!(report["result"]["selected"].as_array().unwrap().len(), 1);
    assert_eq!(report["result"]["expected_total_ms"], 61000);
    assert_eq!(
        invoke(
            "clear_current_plan",
            serde_json::json!({"expected_revision":1})
        )
        .unwrap(),
        2
    );
}
