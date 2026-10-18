//! Pure domain models, validation, observations and rules for Azeroth Navigator.
//! I/O, native UI, storage and live game access belong outside this crate.
pub mod analytics;
pub mod atlas;
pub mod contract;
pub mod coverage;
pub mod eligibility;
pub mod entities;
pub mod error;
pub mod evaluation;
pub mod gear;
pub mod json;
pub mod objective_clustering;
pub mod observation;
pub mod planner;
pub mod predicate;
pub mod release;
pub mod reminders;
pub mod selection_path;
pub mod sessions;
pub mod sources;
pub mod travel;
pub mod types;

pub mod calibration;
pub mod joint_planner;

pub mod search_control;

pub mod submissions;

pub mod catalog;
pub mod content_v2;
pub mod release_inventory;
pub mod release_party;
pub mod source_admission;

pub mod local_data;
pub mod local_data_catalog;
pub mod local_data_lifecycle;
pub mod local_data_projection;
pub mod local_data_release;
pub mod source_pipeline;
pub mod strict_json;

pub mod local_data_inputs;

pub mod local_data_gameplay;

pub mod local_data_guide;

pub mod tooltip;

pub mod local_data_media;

pub mod local_data_delivery;
