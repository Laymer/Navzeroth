//! Thin Tauri shell. Each command is registered by name and delegates to a
//! framework-free handler in `azeroth-app`; there is no generic dispatcher,
//! shell access or filesystem command (plan section 13).
use azeroth_app::{AppStatus, ProfileSummary};
use azeroth_core::contract::Envelope;
use azeroth_core::entities::CharacterSnapshot;
use azeroth_core::error::AppError;

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

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            list_profiles,
            get_character_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Azeroth Navigator shell");
}
