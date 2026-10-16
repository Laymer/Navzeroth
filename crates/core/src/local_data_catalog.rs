//! Versioned private research artifacts. These never establish playable availability.
use crate::local_data::{InstallationMetadata, InventoryDisposition, PerformancePreset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TableInventory {
    pub table: String,
    #[serde(default)]
    pub locale: Option<String>,
    pub file_data_id: Option<u32>,
    pub disposition: InventoryDisposition,
    pub reason: String,
    pub retained_sha256: Option<String>,
    pub decoded_sha256: Option<String>,
    pub encoded_bytes: Option<u64>,
    pub records: Option<u32>,
    pub layout_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchCatalog {
    pub format_version: u32,
    pub classification: DistributionClass,
    pub installation: InstallationMetadata,
    pub build_info_sha256: String,
    pub extractor_version: String,
    pub native_revision: String,
    pub metadata_sha256: String,
    pub parser_profile: String,
    #[serde(default)]
    pub extractor_profile_sha256: Option<String>,
    #[serde(default)]
    pub archive_statistics: Option<(u32, u32, u32)>,
    pub input_hashes: BTreeMap<String, String>,
    pub tables: Vec<TableInventory>,
    pub gaps: Vec<String>,
    pub gameplay_projection_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DistributionClass {
    LocalOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionSummary {
    pub format_version: u32,
    pub catalog_sha256: String,
    pub build: u32,
    pub tables_processed: u32,
    pub tables_decoded: u32,
    pub preset: PerformancePreset,
    pub actual_workers: u8,
    pub gameplay_projection_available: bool,
    pub activated: bool,
}
