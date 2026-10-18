//! Versioned private extraction lifecycle. Research activation never grants gameplay trust.
use crate::local_data::{PerformancePreset, is_sha256};
use crate::local_data_catalog::{ExtractionSummary, TableInventory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const VERSION: u32 = 1;
pub const MAX_CHECKPOINT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_JOBS: usize = 8;
pub const MAX_SELECTIONS: usize = 16;
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointBinding {
    pub format_version: u32,
    pub source_sha256: String,
    pub extractor_sha256: String,
    pub selection_sha256: String,
    pub preset: PerformancePreset,
    pub total: u32,
}
impl CheckpointBinding {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != VERSION
            || self.total == 0
            || self.total > 32768
            || !is_sha256(&self.source_sha256)
            || !is_sha256(&self.extractor_sha256)
            || !is_sha256(&self.selection_sha256)
        {
            return Err("Unsupported checkpoint binding");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub format_version: u32,
    pub binding: CheckpointBinding,
    pub tables: Vec<TableInventory>,
    pub gaps: Vec<String>,
}
impl Checkpoint {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.binding.validate()?;
        if self.format_version != VERSION
            || self.tables.len() > self.binding.total as usize
            || self.gaps.len() > 131072
            || self.gaps.iter().any(|s| s.len() > 512)
        {
            return Err("Invalid checkpoint capacity or version");
        }
        let mut keys = BTreeSet::new();
        for row in &self.tables {
            if row.table.is_empty()
                || row.table.len() > 80
                || !row
                    .table
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || row.reason.len() > 512
                || row
                    .layout_hash
                    .as_ref()
                    .is_some_and(|s| s.len() != 8 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
                || (row.disposition == crate::local_data::InventoryDisposition::Decoded
                    && (row.retained_sha256.is_none() || row.decoded_sha256.is_none()))
                || !keys.insert((&row.table, &row.locale))
                || row.locale.as_ref().is_none_or(|s| s.len() != 4)
                || [&row.retained_sha256, &row.decoded_sha256]
                    .into_iter()
                    .flatten()
                    .any(|s| !is_sha256(s))
            {
                return Err("Invalid checkpoint inventory");
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Running,
    Cancelled,
    Interrupted,
    Failed,
    Complete,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobStatus {
    pub format_version: u32,
    pub job_id: String,
    pub state: JobState,
    pub completed: u32,
    pub total: u32,
    pub summary: Option<ExtractionSummary>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub format_version: u32,
    pub scope: String,
    pub generation: u64,
    pub catalog_sha256: Option<String>,
    /// Research selection only. Consumers must independently validate availability.
    pub verified_for_planning: bool,
}
