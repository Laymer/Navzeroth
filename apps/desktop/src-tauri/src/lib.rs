//! Explicit command DTOs; snapshot normalization and SQLite execute on blocking
//! workers. No live addon bridge or generic filesystem command is exposed.
mod atlas_protocol;
mod reference_window;
use azeroth_app::AppStatus;
use azeroth_app::atlas::{AtlasView, ProjectedPosition, SnapshotMarker};
use azeroth_app::diagnostics::DiagnosticOperation;
use azeroth_app::snapshots::{
    FileInputFormat, HistoryEntry, ImportReceipt, ProfileDto, SnapshotApplication, StoredParty,
    StoredSnapshot, WatchState, system_now,
};
use azeroth_core::{
    contract::Envelope,
    entities::CharacterSnapshot,
    error::{AppError, ErrorCode},
    types::{DatasetRevision, PartyId, Position, QuestId},
};
use std::sync::Arc;
use tauri::{Manager, State};
type Application = Arc<azeroth_app::recovery::DatabaseHost>;
async fn blocking<T: Send + 'static>(
    state: State<'_, Application>,
    call: impl FnOnce(&SnapshotApplication) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.with_application(call))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::ProviderUnavailable,
                "Snapshot worker is unavailable",
                false,
            )
        })?
}
async fn observed_blocking<T: Send + 'static>(
    state: State<'_, Application>,
    operation: DiagnosticOperation,
    call: impl FnOnce(&SnapshotApplication) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.observe(operation, || host.with_application(call))
    })
    .await
    .map_err(|_| {
        AppError::new(
            ErrorCode::ProviderUnavailable,
            "Foreground worker unavailable",
            false,
        )
    })?
}

#[tauri::command(rename_all = "snake_case")]
async fn start_planning_job(
    state: State<'_, Application>,
    kind: azeroth_app::planning_jobs::PlanningKind,
    text: String,
) -> Result<azeroth_app::planning_jobs::JobAccepted, AppError> {
    if text.len() > azeroth_app::planning::MAX_REQUEST_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Planning request exceeds size limit",
            false,
        ));
    }
    let host = state.inner().clone();
    let ticket = host.planning_jobs().register(kind)?;
    let job_id = ticket.id().to_owned();
    let _worker = tauri::async_runtime::spawn_blocking(move || {
        let terminal_ticket = ticket.clone();
        let operation_kind = match kind {
            azeroth_app::planning_jobs::PlanningKind::Solo => DiagnosticOperation::Planning,
            azeroth_app::planning_jobs::PlanningKind::Joint => DiagnosticOperation::JointPlanning,
        };
        let _observed = host.observe(operation_kind, || {
            let failed_ticket = ticket.clone();
            let jobs = host.planning_jobs();
            let operation = host.with_application(|app| {
                let guard = std::cell::RefCell::new(
                    None::<azeroth_app::planning_job_service::PublicationGuard>,
                );
                jobs.run_guarded_with_pin(
                    ticket,
                    |control| {
                        let (report, input_guard) =
                            app.calculate_planning_job(kind, &text, system_now(), control)?;
                        let pin = input_guard.pin_descriptor(&report, system_now()).ok();
                        *guard.borrow_mut() = Some(input_guard);
                        Ok((report, pin))
                    },
                    |publish| {
                        guard
                            .borrow()
                            .as_ref()
                            .ok_or_else(|| {
                                AppError::new(
                                    ErrorCode::SnapshotOutdated,
                                    "Planning input guard unavailable",
                                    false,
                                )
                            })?
                            .publish(app, system_now(), publish)
                    },
                );
                Ok(())
            });
            if let Err(error) = operation {
                jobs.run(failed_ticket, |_| Err::<(), _>(error));
            }
            terminal_ticket.completion_result()
        });
    });
    Ok(azeroth_app::planning_jobs::JobAccepted {
        job_schema_version: 1,
        job_id,
    })
}
#[tauri::command(rename_all = "snake_case")]
async fn get_planning_job(
    state: State<'_, Application>,
    job_id: String,
) -> Result<azeroth_app::planning_jobs::JobStatus, AppError> {
    state.planning_jobs().status(&job_id)
}
#[tauri::command(rename_all = "snake_case")]
async fn cancel_planning_job(
    state: State<'_, Application>,
    job_id: String,
) -> Result<(), AppError> {
    state.planning_jobs().cancel(&job_id)
}
#[tauri::command(rename_all = "snake_case")]
async fn take_planning_job_result(
    state: State<'_, Application>,
    job_id: String,
) -> Result<azeroth_app::planning_jobs::JobResult, AppError> {
    state.planning_jobs().take_result(&job_id)
}

async fn intent_foreground<T: Send + 'static>(
    state: State<'_, Application>,
    call: impl FnOnce(&azeroth_app::snapshots::SnapshotApplication) -> Result<T, AppError>
    + Send
    + 'static,
) -> Result<T, AppError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.with_application(call))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::ProviderUnavailable,
                "Foreground worker unavailable",
                false,
            )
        })?
}
#[tauri::command(rename_all = "snake_case")]
async fn get_app_settings(
    state: State<'_, Application>,
) -> Result<azeroth_app::today::SettingsRecord, AppError> {
    intent_foreground(state, |app| app.app_settings()).await
}
#[tauri::command(rename_all = "snake_case")]
async fn update_app_settings(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::today::SettingsRecord, AppError> {
    intent_foreground(state, move |app| app.update_settings(&text)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_today_view(
    state: State<'_, Application>,
) -> Result<azeroth_app::today::TodayView, AppError> {
    intent_foreground(state, |app| app.today_view(system_now())).await
}
#[tauri::command(rename_all = "snake_case")]
async fn clear_current_plan(
    state: State<'_, Application>,
    expected_revision: i64,
) -> Result<i64, AppError> {
    intent_foreground(state, move |app| app.clear_current_plan(expected_revision)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn pin_current_plan(
    state: State<'_, Application>,
    job_id: String,
    expected_revision: i64,
) -> Result<i64, AppError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.with_application(|app| {
            app.pin_current_plan(
                host.planning_jobs(),
                &job_id,
                expected_revision,
                system_now(),
            )
        })
    })
    .await
    .map_err(|_| {
        AppError::new(
            ErrorCode::ProviderUnavailable,
            "Foreground worker unavailable",
            false,
        )
    })?
}

#[tauri::command(rename_all = "snake_case")]
async fn set_session_calibration(
    state: State<'_, Application>,
    profile_id: String,
    session_id: azeroth_core::types::SessionId,
    annotation: azeroth_core::calibration::SessionCalibrationAnnotation,
) -> Result<(), AppError> {
    blocking(state, move |app| {
        app.set_session_calibration(&profile_id, &session_id, &annotation)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_calibration(
    state: State<'_, Application>,
    profile_id: String,
    request: azeroth_core::calibration::CalibrationRequest,
) -> Result<azeroth_core::calibration::CalibrationReport, AppError> {
    blocking(state, move |app| app.get_calibration(&profile_id, &request)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn reminder_view(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<azeroth_app::reminders::ReminderView, AppError> {
    blocking(state, move |app| {
        app.reminder_view(&profile_id, system_now())
    })
    .await
}
#[tauri::command]
async fn mutate_reminders(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::reminders::ReminderView, AppError> {
    blocking(state, move |app| app.mutate_reminders(&text, system_now())).await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_joint_planning_catalog(
    state: State<'_, Application>,
    party_id: PartyId,
) -> Result<azeroth_app::joint_planning::JointPlanningCatalog, AppError> {
    blocking(state, move |app| {
        app.joint_planning_catalog(&party_id, system_now())
    })
    .await
}
#[tauri::command]
async fn plan_joint_session(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::joint_planning::JointPlanningReport, AppError> {
    observed_blocking(state, DiagnosticOperation::JointPlanning, move |app| {
        app.plan_joint_session(&text, system_now())
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_party_plan_proposal(
    state: State<'_, Application>,
    party_id: PartyId,
) -> Result<Option<azeroth_app::party_proposals::PartyProposalReport>, AppError> {
    blocking(state, move |app| {
        app.party_plan_proposal(&party_id, system_now())
    })
    .await
}
#[tauri::command]
async fn propose_party_plan(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::party_proposals::PartyProposalReport, AppError> {
    blocking(state, move |app| {
        app.propose_party_plan(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn accept_party_plan(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::party_proposals::PartyProposalReport, AppError> {
    blocking(state, move |app| app.accept_party_plan(&text, system_now())).await
}

#[tauri::command(rename_all = "snake_case")]
async fn get_historical_snapshot(
    state: State<'_, Application>,
    profile_id: String,
    snapshot_id: azeroth_core::types::SnapshotId,
) -> Result<Option<StoredSnapshot>, AppError> {
    blocking(state, move |app| {
        app.historical(&profile_id, &snapshot_id, system_now())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn promote_snapshot(
    state: State<'_, Application>,
    profile_id: String,
    snapshot_id: azeroth_core::types::SnapshotId,
    expected_current: azeroth_core::types::SnapshotId,
    reason: String,
) -> Result<(), AppError> {
    blocking(state, move |app| {
        app.promote(
            &profile_id,
            &snapshot_id,
            &expected_current,
            &reason,
            system_now(),
        )
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_planning_catalog(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<azeroth_app::planning::PlanningCatalog, AppError> {
    blocking(state, move |app| app.planning_catalog(&profile_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_compatibility_report(
    state: State<'_, Application>,
    profile_id: Option<String>,
) -> Result<azeroth_app::compatibility::CompatibilityReport, AppError> {
    blocking(state, move |app| {
        app.compatibility_report(profile_id.as_deref())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_content_dispositions(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<azeroth_app::content_dispositions::ContentGovernance, AppError> {
    blocking(state, move |app| app.content_governance(&profile_id)).await
}
#[tauri::command]
async fn change_content_disposition(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::content_dispositions::ContentDecision, AppError> {
    blocking(state, move |app| {
        app.change_content_disposition(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn export_plan(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::plan_export::PlanExport, AppError> {
    blocking(state, move |app| app.export_plan(&text, system_now())).await
}
#[tauri::command]
async fn export_agreed_party_plan(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::plan_export::PlanExport, AppError> {
    blocking(state, move |app| {
        app.export_agreed_party_plan(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn inspect_plan_route(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::routes::RouteInspection, AppError> {
    blocking(state, move |app| {
        app.inspect_plan_route(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn plan_session(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::planning::PlanningReport, AppError> {
    observed_blocking(state, DiagnosticOperation::Planning, move |app| {
        app.plan_session(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn get_app_status(state: State<'_, Application>) -> Result<AppStatus, AppError> {
    blocking(state, |app| app.status()).await
}
#[tauri::command]
async fn list_profiles(state: State<'_, Application>) -> Result<Vec<ProfileDto>, AppError> {
    blocking(state, |app| app.profiles()).await
}
#[tauri::command]
async fn list_parties(state: State<'_, Application>) -> Result<Vec<PartyId>, AppError> {
    blocking(state, |app| app.party_ids()).await
}
#[tauri::command]
async fn stage_content(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::content::ContentInspection, AppError> {
    observed_blocking(state, DiagnosticOperation::ContentImport, move |app| {
        app.stage_content(&text)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_active_content(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<Option<azeroth_app::content::ContentInspection>, AppError> {
    blocking(state, move |app| app.active_content(&profile_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn activate_content(
    state: State<'_, Application>,
    profile_id: String,
    revision: DatasetRevision,
) -> Result<azeroth_app::content::ContentInspection, AppError> {
    blocking(state, move |app| {
        app.activate_content(&profile_id, &revision)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn rollback_content(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<azeroth_app::content::ContentRollback, AppError> {
    blocking(state, move |app| app.rollback_content(&profile_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_party_quests(
    state: State<'_, Application>,
    party_id: PartyId,
) -> Result<azeroth_app::party::PartyQuestCatalog, AppError> {
    blocking(state, move |app| app.party_quests(&party_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_dungeon_preparation(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::dungeon_preparation::PreparationReport, AppError> {
    blocking(state, move |app| {
        app.dungeon_preparation(&text, system_now())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_party_eligibility(
    state: State<'_, Application>,
    party_id: PartyId,
    revision: DatasetRevision,
    quest_id: QuestId,
) -> Result<azeroth_app::party::PartyQuestReport, AppError> {
    blocking(state, move |app| {
        app.party_eligibility(&party_id, &revision, &quest_id, system_now())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_party(
    state: State<'_, Application>,
    party_id: PartyId,
) -> Result<Option<StoredParty>, AppError> {
    blocking(state, move |app| app.party(&party_id, system_now())).await
}
#[tauri::command(rename_all = "snake_case")]
async fn save_party(
    state: State<'_, Application>,
    party_id: PartyId,
    profile_ids: Vec<String>,
) -> Result<(), AppError> {
    blocking(state, move |app| app.save_party(&party_id, &profile_ids)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn delete_party(state: State<'_, Application>, party_id: PartyId) -> Result<bool, AppError> {
    blocking(state, move |app| app.delete_party(&party_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_character_snapshot(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<Envelope<CharacterSnapshot>, AppError> {
    blocking(state, move |app| {
        app.current(&profile_id, system_now())?
            .map(|s| s.envelope)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Profile has no current snapshot",
                    false,
                )
            })
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_stored_snapshot(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<Option<StoredSnapshot>, AppError> {
    blocking(state, move |app| app.current(&profile_id, system_now())).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_snapshot_history(
    state: State<'_, Application>,
    profile_id: String,
    limit: usize,
) -> Result<Vec<HistoryEntry>, AppError> {
    blocking(state, move |app| {
        app.history(&profile_id, limit, system_now())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn select_profile(state: State<'_, Application>, profile_id: String) -> Result<(), AppError> {
    blocking(state, move |app| app.select_profile(&profile_id)).await
}
#[tauri::command]
async fn import_snapshot_text(
    state: State<'_, Application>,
    text: String,
) -> Result<ImportReceipt, AppError> {
    observed_blocking(state, DiagnosticOperation::SnapshotImport, move |app| {
        app.import_text(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn import_snapshot_file(
    state: State<'_, Application>,
    path: String,
    format: FileInputFormat,
) -> Result<ImportReceipt, AppError> {
    observed_blocking(state, DiagnosticOperation::SnapshotImport, move |app| {
        app.import_file(std::path::Path::new(&path), &format, system_now())
    })
    .await
}
#[tauri::command]
async fn start_snapshot_watch(
    state: State<'_, Application>,
    path: String,
    format: FileInputFormat,
) -> Result<WatchState, AppError> {
    blocking(state, move |app| {
        app.start_watch(std::path::Path::new(&path), &format)
    })
    .await
}
#[tauri::command]
async fn stop_snapshot_watch(state: State<'_, Application>) -> Result<WatchState, AppError> {
    blocking(state, |app| app.stop_watch()).await
}
#[tauri::command]
async fn get_snapshot_watch(state: State<'_, Application>) -> Result<WatchState, AppError> {
    blocking(state, |app| app.watch_status()).await
}
#[tauri::command]
fn query_atlas(image: Option<String>) -> Result<AtlasView, AppError> {
    azeroth_app::atlas::query_atlas(image.as_deref())
}
#[tauri::command]
fn project_positions(positions: Vec<Position>) -> Result<Vec<ProjectedPosition>, AppError> {
    azeroth_app::atlas::project_positions(&positions)
}
#[tauri::command]
async fn get_snapshot_markers(
    state: State<'_, Application>,
) -> Result<Vec<SnapshotMarker>, AppError> {
    blocking(state, |app| app.markers(system_now())).await
}
#[tauri::command(rename_all = "snake_case")]
async fn import_session_text(
    state: State<'_, Application>,
    profile_id: String,
    text: String,
) -> Result<azeroth_app::sessions::SessionImportReceipt, AppError> {
    observed_blocking(state, DiagnosticOperation::SessionImport, move |app| {
        app.import_session_text(&profile_id, &text)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn import_session_file(
    state: State<'_, Application>,
    profile_id: String,
    path: String,
    format: FileInputFormat,
) -> Result<azeroth_app::sessions::SessionImportReceipt, AppError> {
    observed_blocking(state, DiagnosticOperation::SessionImport, move |app| {
        app.import_session_file(&profile_id, std::path::Path::new(&path), format)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn list_sessions(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<Vec<azeroth_app::sessions::SessionListEntry>, AppError> {
    blocking(state, move |app| app.list_sessions(&profile_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_session_detail(
    state: State<'_, Application>,
    profile_id: String,
    session_id: azeroth_core::types::SessionId,
) -> Result<azeroth_app::sessions::SessionDetail, AppError> {
    blocking(state, move |app| {
        app.get_session_detail(&profile_id, &session_id)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn delete_session(
    state: State<'_, Application>,
    profile_id: String,
    session_id: azeroth_core::types::SessionId,
) -> Result<bool, AppError> {
    blocking(state, move |app| {
        app.delete_session(&profile_id, &session_id)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn get_gear_catalog(
    state: State<'_, Application>,
    profile_id: String,
) -> Result<azeroth_app::gear::GearCatalog, AppError> {
    blocking(state, move |app| app.gear_catalog(&profile_id)).await
}
#[tauri::command]
async fn compare_gear(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::gear::GearReport, AppError> {
    blocking(state, move |app| app.compare_gear(&text, system_now())).await
}
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let database = directory.join("snapshots.sqlite3");
            let service = azeroth_app::recovery::DatabaseHost::start(&database);
            app.manage(service);
            Ok(())
        })
        .register_uri_scheme_protocol(atlas_protocol::SCHEME, |_ctx, request| {
            atlas_protocol::respond(&request)
        })
        .manage(reference_window::Controller::default())
        .invoke_handler({
            let generated: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
                list_local_submissions,
                submit_local_observation,
                change_local_submission,
                export_local_submission,
                get_content_dispositions,
                change_content_disposition,
                get_dungeon_preparation,
                preview_acquisition_route,
                assess_route_acquisition,
                preview_diagnostics,
                export_diagnostics,
                get_reference_view,
                get_app_settings,
                update_app_settings,
                get_today_view,
                pin_current_plan,
                clear_current_plan,
                get_compatibility_report,
                get_reference_window_state,
                set_reference_window,
                toggle_reference_window,
                set_reference_interaction,
                start_planning_job,
                get_planning_job,
                cancel_planning_job,
                take_planning_job_result,
                get_database_status,
                check_database,
                create_database_backup,
                inspect_database_backup,
                restore_database_backup,
                import_session_text,
                import_session_file,
                list_sessions,
                get_session_detail,
                delete_session,
                get_gear_catalog,
                compare_gear,
                get_historical_snapshot,
                promote_snapshot,
                get_planning_catalog,
                plan_session,
                get_party_plan_proposal,
                propose_party_plan,
                accept_party_plan,
                get_joint_planning_catalog,
                plan_joint_session,
                set_session_calibration,
                get_calibration,
                reminder_view,
                mutate_reminders,
                inspect_plan_route,
                export_plan,
                export_agreed_party_plan,
                get_app_status,
                list_profiles,
                stage_content,
                get_active_content,
                activate_content,
                rollback_content,
                list_parties,
                get_party_quests,
                get_party_eligibility,
                get_party,
                save_party,
                delete_party,
                get_character_snapshot,
                get_stored_snapshot,
                get_snapshot_history,
                select_profile,
                import_snapshot_text,
                import_snapshot_file,
                start_snapshot_watch,
                stop_snapshot_watch,
                get_snapshot_watch,
                query_atlas,
                project_positions,
                get_snapshot_markers
            ];
            move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
                if !reference_window::permits(
                    invoke.message.webview_ref().label(),
                    invoke.message.command(),
                ) {
                    invoke.resolver.reject(AppError::new(
                        ErrorCode::ProviderUnavailable,
                        "This application window permits imported reference reads only",
                        false,
                    ));
                    true
                } else {
                    generated(invoke)
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running the Azeroth Navigator shell");
}

async fn recovery_blocking<T: Send + 'static>(
    state: State<'_, Application>,
    call: impl FnOnce(Application) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || call(host))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::ProviderUnavailable,
                "Recovery worker unavailable",
                false,
            )
        })?
}
#[tauri::command]
async fn get_database_status(
    state: State<'_, Application>,
) -> Result<azeroth_app::recovery::DatabaseStatus, AppError> {
    recovery_blocking(state, |host| host.status()).await
}
#[tauri::command]
async fn check_database(state: State<'_, Application>) -> Result<(), AppError> {
    recovery_blocking(state, |host| host.check()).await
}
#[tauri::command]
async fn create_database_backup(
    state: State<'_, Application>,
    path: String,
) -> Result<(), AppError> {
    recovery_blocking(state, move |host| {
        host.observe(DiagnosticOperation::Backup, || {
            host.backup(std::path::Path::new(&path))
        })
    })
    .await
}
#[tauri::command]
async fn inspect_database_backup(
    state: State<'_, Application>,
    path: String,
) -> Result<azeroth_app::recovery::BackupPreview, AppError> {
    recovery_blocking(state, move |host| host.inspect(std::path::Path::new(&path))).await
}
#[tauri::command(rename_all = "snake_case")]
async fn restore_database_backup(
    state: State<'_, Application>,
    candidate_id: String,
) -> Result<azeroth_app::recovery::RestoreReceipt, AppError> {
    recovery_blocking(state, move |host| {
        host.observe(DiagnosticOperation::Restore, || host.restore(&candidate_id))
    })
    .await
}

#[tauri::command]
async fn preview_diagnostics(
    state: State<'_, Application>,
) -> Result<azeroth_app::diagnostics::DiagnosticReport, AppError> {
    recovery_blocking(state, |host| host.diagnostic_report(system_now())).await
}
#[tauri::command]
async fn export_diagnostics(
    state: State<'_, Application>,
    path: String,
) -> Result<azeroth_app::diagnostics::DiagnosticReceipt, AppError> {
    if path.len() > 4096 || path.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Select a bounded new diagnostic file path",
            false,
        ));
    }
    recovery_blocking(state, move |host| {
        host.export_diagnostics(std::path::Path::new(&path), system_now())
    })
    .await
}

#[tauri::command]
async fn get_reference_view(
    state: State<'_, Application>,
) -> Result<azeroth_app::reference::ReferenceView, AppError> {
    blocking(state, |app| app.reference_view(system_now())).await
}
#[tauri::command]
async fn get_reference_window_state(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    controller: State<'_, reference_window::Controller>,
) -> Result<azeroth_app::reference::ReferenceWindowState, AppError> {
    reference_window::main_caller(window.label())?;
    let _serial = controller.inner().0.lock().map_err(|_| {
        AppError::new(
            ErrorCode::ProviderUnavailable,
            "Reference window controls unavailable",
            false,
        )
    })?;
    reference_window::state(&app)
}
#[tauri::command(rename_all = "snake_case")]
async fn toggle_reference_window(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    controller: State<'_, reference_window::Controller>,
    always_on_top: bool,
    interaction_mode: azeroth_app::reference_controls::InteractionMode,
) -> Result<azeroth_app::reference_controls::ControlReceipt, AppError> {
    reference_window::main_caller(window.label())?;
    let _serial = controller.inner().0.lock().map_err(|_| {
        AppError::new(
            ErrorCode::ProviderUnavailable,
            "Reference controls unavailable",
            false,
        )
    })?;
    Ok(azeroth_app::reference_controls::toggle(
        &mut reference_window::Operations::new(app, window),
        interaction_mode,
        always_on_top,
    ))
}
#[tauri::command(rename_all = "snake_case")]
async fn set_reference_interaction(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    controller: State<'_, reference_window::Controller>,
    interaction_mode: azeroth_app::reference_controls::InteractionMode,
) -> Result<azeroth_app::reference_controls::ControlReceipt, AppError> {
    reference_window::main_caller(window.label())?;
    let _serial = controller.inner().0.lock().map_err(|_| {
        AppError::new(
            ErrorCode::ProviderUnavailable,
            "Reference controls unavailable",
            false,
        )
    })?;
    Ok(azeroth_app::reference_controls::set_interaction(
        &mut reference_window::Operations::new(app, window),
        interaction_mode,
    ))
}
#[tauri::command(rename_all = "snake_case")]
async fn set_reference_window(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    controller: State<'_, reference_window::Controller>,
    visible: bool,
    always_on_top: bool,
) -> Result<azeroth_app::reference::ReferenceWindowState, AppError> {
    reference_window::main_caller(window.label())?;
    let _serial = controller.inner().0.lock().map_err(|_| {
        AppError::new(
            ErrorCode::ProviderUnavailable,
            "Reference window controls unavailable",
            false,
        )
    })?;
    reference_window::set(&app, visible, always_on_top)
}

#[tauri::command]
async fn preview_acquisition_route(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::route_acquisition::RouteAcquisitionBaseline, AppError> {
    observed_blocking(state, DiagnosticOperation::Planning, move |app| {
        app.preview_acquisition_route(&text, system_now())
    })
    .await
}
#[tauri::command]
async fn assess_route_acquisition(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::route_acquisition::RouteAcquisitionReport, AppError> {
    blocking(state, move |app| {
        app.assess_route_acquisition(&text, system_now())
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn list_local_submissions(
    state: State<'_, Application>,
    after: Option<azeroth_core::types::Sha256Hex>,
    limit: usize,
) -> Result<azeroth_app::local_submissions::SubmissionPage, AppError> {
    blocking(state, move |app| {
        app.local_submission_page(after.as_ref(), limit)
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn submit_local_observation(
    state: State<'_, Application>,
    text: String,
    opt_in: bool,
) -> Result<azeroth_app::local_submissions::SubmitReceipt, AppError> {
    blocking(state, move |app| {
        app.submit_local_observation(&text, opt_in, system_now())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn change_local_submission(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::local_submissions::SubmissionRecord, AppError> {
    blocking(state, move |app| {
        app.change_local_submission(&text, system_now())
    })
    .await
}
#[tauri::command(rename_all = "snake_case")]
async fn export_local_submission(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::local_submissions::LocalSubmissionExport, AppError> {
    blocking(state, move |app| {
        app.export_local_submission(&text, system_now())
    })
    .await
}
