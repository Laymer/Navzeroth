//! Pinned, byte-verified metadata. CC-BY-SA definitions remain separate from code.
use serde::Deserialize;
use std::collections::BTreeMap;
use wowdbdefs_rs::{Layout, RawDbdFile, RawDefinition, Version};

include!(concat!(env!("OUT_DIR"), "/metadata.rs"));

#[derive(Clone, Debug, Deserialize)]
pub struct TableCandidate {
    #[serde(rename = "tableName")]
    pub name: String,
    #[serde(rename = "db2FileDataID")]
    pub file_data_id: Option<u32>,
    #[serde(rename = "dbcFileDataID")]
    pub dbc_file_data_id: Option<u32>,
    #[serde(skip)]
    pub extension: &'static str,
    #[serde(rename = "tableHash")]
    pub table_hash: String,
}

pub fn candidates() -> Vec<TableCandidate> {
    let mut tables: Vec<TableCandidate> =
        serde_json::from_str(include_str!("../vendor/wowdbdefs/manifest.json"))
            .expect("pinned manifest");
    for table in &mut tables {
        table.extension = if table.file_data_id.is_some() {
            "db2"
        } else if table.dbc_file_data_id.is_some() {
            "dbc"
        } else {
            "db2"
        };
        table.file_data_id = table.file_data_id.or(table.dbc_file_data_id);
    }
    tables.sort_by(|a, b| a.name.cmp(&b.name));
    tables
}

pub struct Definition {
    pub file: RawDbdFile,
    pub layout: RawDefinition,
}

/// Exact explicit build plus layout selection. Ambiguous matches fail closed.
pub fn definition(name: &str, build: u32, layout: u32) -> Result<Definition, &'static str> {
    if !matches!(build, 70374 | 70425) {
        return Err("unsupported_build");
    }
    let text = DEFINITIONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, t)| *t)
        .ok_or("definition_missing")?;
    if text.len() > 1024 * 1024 || text.lines().count() > 50_000 {
        return Err("definition_capacity");
    }
    // The older parser may reject new DBD syntax. Do not retry with a different
    // build, remove tokens, or promote partial parse results.
    let file = std::panic::catch_unwind(|| {
        wowdbdefs_rs::load_file_from_string(text, format!("{name}.dbd"))
    })
    .map_err(|_| "definition_unsupported")?
    .map_err(|_| "definition_unsupported")?;
    let version = Version::new(1, 60, 1, build);
    let matching: Vec<_> = file
        .definitions
        .iter()
        .filter(|d| d.versions.contains(&version) && d.layouts.contains(&Layout::new(layout)))
        .collect();
    if matching.len() != 1 {
        return Err("layout_mismatch");
    }
    let layout = matching[0].clone();
    if layout.entries.len() > 512 {
        return Err("definition_capacity");
    }
    Ok(Definition { file, layout })
}

pub fn definition_hashes() -> BTreeMap<&'static str, String> {
    use sha2::{Digest, Sha256};
    DEFINITIONS
        .iter()
        .map(|(name, text)| (*name, format!("{:x}", Sha256::digest(text.as_bytes()))))
        .collect()
}
