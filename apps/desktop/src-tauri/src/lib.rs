//! Explicit command DTOs; snapshot normalization and SQLite execute on blocking
//! workers. No live addon bridge or generic filesystem command is exposed.
mod atlas_protocol;
use azeroth_app::AppStatus;
use azeroth_app::atlas::{AtlasView, ProjectedPosition, SnapshotMarker};
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
type Application = Arc<SnapshotApplication>;
async fn blocking<T: Send + 'static>(
    state: State<'_, Application>,
    call: impl FnOnce(Application) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || call(app))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::ProviderUnavailable,
                "Snapshot worker is unavailable",
                false,
            )
        })?
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
#[tauri::command]
async fn export_plan(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::plan_export::PlanExport, AppError> {
    blocking(state, move |app| app.export_plan(&text, system_now())).await
}
#[tauri::command]
async fn plan_session(
    state: State<'_, Application>,
    text: String,
) -> Result<azeroth_app::planning::PlanningReport, AppError> {
    blocking(state, move |app| app.plan_session(&text, system_now())).await
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
    blocking(state, move |app| app.stage_content(&text)).await
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
    blocking(state, move |app| app.import_text(&text, system_now())).await
}
#[tauri::command]
async fn import_snapshot_file(
    state: State<'_, Application>,
    path: String,
    format: FileInputFormat,
) -> Result<ImportReceipt, AppError> {
    blocking(state, move |app| {
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
    blocking(state, move |app| {
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
    blocking(state, move |app| {
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
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let database = directory.join("snapshots.sqlite3");
            let service = std::thread::spawn(move || SnapshotApplication::open(&database))
                .join()
                .map_err(|_| std::io::Error::other("Snapshot initialization worker failed"))??;
            app.manage(Arc::new(service));
            Ok(())
        })
        .register_uri_scheme_protocol(atlas_protocol::SCHEME, |_ctx, request| {
            atlas_protocol::respond(&request)
        })
        .invoke_handler(tauri::generate_handler![
            import_session_text,
            import_session_file,
            list_sessions,
            get_session_detail,
            delete_session,
            get_historical_snapshot,
            promote_snapshot,
            get_planning_catalog,
            plan_session,
            export_plan,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Azeroth Navigator shell");
}
