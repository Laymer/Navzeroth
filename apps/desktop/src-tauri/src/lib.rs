//! Explicit command DTOs; snapshot normalization and SQLite execute on blocking
//! workers. No live addon bridge or generic filesystem command is exposed.
mod atlas_protocol;
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
                jobs.run_guarded(
                    ticket,
                    |control| {
                        let (report, input_guard) =
                            app.calculate_planning_job(kind, &text, system_now(), control)?;
                        *guard.borrow_mut() = Some(input_guard);
                        Ok(report)
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
        .invoke_handler(tauri::generate_handler![
            preview_acquisition_route,
            assess_route_acquisition,
            preview_diagnostics,
            export_diagnostics,
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
            get_compatibility_report,
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
            get_dungeon_preparation,
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
        ])
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
