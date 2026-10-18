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
        .manage(Arc::new(azeroth_app::local_data::LocalDataService::new(
            root.join("private"),
        )))
        .invoke_handler(tauri::generate_handler![
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
        ])
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
    let after = serde_json::to_value(
        host.with_application(|app| app.reference_view(None))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(before, after);
}
