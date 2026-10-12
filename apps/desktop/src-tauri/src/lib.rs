//! Thin Tauri shell. Each command is registered by name and delegates to a
//! framework-free handler in `azeroth-app`; there is no generic dispatcher,
//! shell access or filesystem command (plan section 13).
mod atlas_protocol;

use azeroth_app::atlas::{AtlasView, ProjectedPosition, SnapshotMarker};
use azeroth_app::{AppStatus, ProfileSummary};
use azeroth_core::contract::Envelope;
use azeroth_core::entities::CharacterSnapshot;
use azeroth_core::error::AppError;
use azeroth_core::types::{Position, UtcMillis};

#[tauri::command]
fn get_app_status() -> Result<AppStatus, AppError> {
    azeroth_app::get_app_status()
}

#[tauri::command]
fn list_profiles() -> Result<Vec<ProfileSummary>, AppError> {
    azeroth_app::list_profiles()
}

#[tauri::command]
fn get_character_snapshot(profile_id: String) -> Result<Envelope<CharacterSnapshot>, AppError> {
    azeroth_app::get_character_snapshot(&profile_id)
}

#[tauri::command]
fn query_atlas(image: Option<String>) -> Result<AtlasView, AppError> {
    azeroth_app::atlas::query_atlas(image.as_deref())
}

#[tauri::command]
fn project_positions(positions: Vec<Position>) -> Result<Vec<ProjectedPosition>, AppError> {
    azeroth_app::atlas::project_positions(&positions)
}

/// The only place the clock is read: once per call, as milliseconds since the
/// Unix epoch. An unreadable or out-of-range clock means "no trusted time".
#[tauri::command]
fn get_snapshot_markers() -> Result<Vec<SnapshotMarker>, AppError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .and_then(|ms| UtcMillis::new(ms).ok());
    azeroth_app::atlas::get_snapshot_markers(now)
}

pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol(atlas_protocol::SCHEME, |_ctx, request| {
            atlas_protocol::respond(&request)
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            list_profiles,
            get_character_snapshot,
            query_atlas,
            project_positions,
            get_snapshot_markers
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Azeroth Navigator shell");
}
