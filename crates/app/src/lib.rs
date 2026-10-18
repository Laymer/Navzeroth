//! Command handlers behind the Tauri boundary (plan section 13). They are plain
//! functions so they are tested without a webview; the desktop shell only
//! registers them. [`snapshots`] assembles bounded ingest and SQLite for the
//! desktop workflow. The free functions below remain explicit synthetic-demo
//! handlers for browser fixtures and compatibility tests.
pub mod atlas;
pub mod authored_content;
pub mod catalog;
pub mod catalog_pages;
pub mod clustering;
pub mod content;
pub mod content_dispositions;
pub mod dungeon_preparation;
pub mod gear;
pub mod local_data;
pub mod party;
pub mod plan_export;
pub mod planning;
pub mod routes;
pub mod snapshots;
pub mod stored_atlas;

use azeroth_core::contract::decode_envelope;
use azeroth_core::entities::CharacterSnapshot;
use azeroth_core::error::{AppError, ErrorCode};
use serde::Serialize;

const DEMO_SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");

/// Where displayed data comes from. Fixture status is independent of storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataMode {
    Demo,
    Stored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Unverified,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AppStatus {
    pub version: &'static str,
    pub data_mode: DataMode,
    pub active_profile: Option<String>,
    pub capabilities: std::collections::BTreeMap<&'static str, CapabilityState>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: &'static str,
    pub data_mode: DataMode,
}

pub(crate) fn demo_snapshot()
-> Result<azeroth_core::contract::Envelope<CharacterSnapshot>, AppError> {
    decode_envelope(DEMO_SNAPSHOT)
}

fn demo_profile_id() -> Result<String, AppError> {
    Ok(demo_snapshot()?.payload.character.as_str().to_owned())
}

pub fn get_app_status() -> Result<AppStatus, AppError> {
    Ok(AppStatus {
        version: env!("CARGO_PKG_VERSION"),
        data_mode: DataMode::Demo,
        active_profile: Some(demo_profile_id()?),
        capabilities: [
            ("addon_transport", CapabilityState::Unverified),
            ("live_position", CapabilityState::Unverified),
        ]
        .into(),
    })
}

pub fn list_profiles() -> Result<Vec<ProfileSummary>, AppError> {
    Ok(vec![ProfileSummary {
        id: demo_profile_id()?,
        display_name: "Demo paladin",
        data_mode: DataMode::Demo,
    }])
}

pub fn get_character_snapshot(
    profile_id: &str,
) -> Result<azeroth_core::contract::Envelope<CharacterSnapshot>, AppError> {
    if profile_id != demo_profile_id()? {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "No such profile",
            false,
        ));
    }
    demo_snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_demo_and_capabilities_are_unverified() {
        let status = get_app_status().unwrap();
        assert_eq!(status.data_mode, DataMode::Demo);
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["data_mode"], "demo");
        assert_eq!(json["capabilities"]["addon_transport"], "unverified");
        assert_eq!(json["active_profile"], "demo-character-paladin");
    }

    #[test]
    fn profiles_are_all_labelled_demo() {
        let profiles = list_profiles().unwrap();
        assert!(profiles.iter().all(|p| p.data_mode == DataMode::Demo));
    }

    #[test]
    fn snapshot_is_the_validated_fixture_and_serializes_to_the_contract() {
        let env = get_character_snapshot("demo-character-paladin").unwrap();
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["kind"], "character_snapshot");
        assert_eq!(json["fixture"], true);
    }

    #[test]
    fn unknown_profile_is_a_safe_invalid_input_error() {
        let err = get_character_snapshot("../etc/passwd").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
        assert!(!err.to_string().contains("passwd"));
    }
}

pub mod sessions;

pub mod calibration;
pub mod joint_planning;

pub mod party_proposals;
pub mod reminders;

pub mod diagnostics;
pub mod recovery;

pub mod planning_job_service;
pub mod planning_jobs;

pub mod compatibility;
pub mod recipient_export;

pub mod reference;
pub mod reference_controls;
pub mod route_acquisition;

pub mod local_submissions;
pub mod today;

mod local_changes;
