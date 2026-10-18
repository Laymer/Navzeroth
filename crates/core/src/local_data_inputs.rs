//! Versioned research input lineage. Cache envelopes never establish availability.
use crate::local_data::is_sha256;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const INPUT_VERSION: u32 = 1;
pub const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CACHE_RECORDS: usize = 100_000;
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputTuple {
    pub product: String,
    pub version: String,
    pub build: u32,
    pub locale: String,
}
impl InputTuple {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.product.is_empty()
            || self.product.len() > 80
            || !self
                .product
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || self.version.is_empty()
            || self.version.len() > 80
            || !self
                .version
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'.')
            || self.build == 0
            || !matches!(
                self.locale.as_str(),
                "enUS"
                    | "enGB"
                    | "deDE"
                    | "esES"
                    | "esMX"
                    | "frFR"
                    | "itIT"
                    | "koKR"
                    | "ptBR"
                    | "ruRU"
                    | "zhCN"
                    | "zhTW"
            )
        {
            return Err("Invalid input tuple");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    HotfixCache,
    QuestCache,
    CreatureCache,
    GameObjectCache,
    ItemCache,
    CharacterSnapshot,
    SessionEvents,
}
impl InputKind {
    pub fn cache_filename(self) -> Option<&'static str> {
        match self {
            Self::HotfixCache => Some("DBCache.bin"),
            Self::QuestCache => Some("questcache.wdb"),
            Self::CreatureCache => Some("creaturecache.wdb"),
            Self::GameObjectCache => Some("gameobjectcache.wdb"),
            Self::ItemCache => Some("itemcache.wdb"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheOperation {
    /// A payload candidate; table layout and add-versus-replace remain unverified.
    UpsertCandidate,
    /// Invalidating flags are retained, not guessed to be a universal deletion.
    InvalidationUnknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CacheRecord {
    pub ordinal: u32,
    pub push_id: i32,
    pub unique_id: Option<i32>,
    pub region_id: Option<i32>,
    pub table_hash: u32,
    pub record_id: u32,
    pub opcode: u8,
    pub operation: CacheOperation,
    pub payload_offset: u32,
    pub payload_size: u32,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CacheEnvelope {
    pub format_version: u32,
    pub source_version: Option<u32>,
    pub source_build: Option<u32>,
    pub unsupported: Option<String>,
    /// Original file order is authoritative. Equal push IDs may conflict.
    pub records: Vec<CacheRecord>,
    pub conflicting_records: Vec<(u32, u32, i32)>,
}

/// Parse only reviewed XFTH envelopes. Payloads stay bounded opaque byte ranges.
/// HTFX version 8 has two historical layouts and is deliberately unsupported.
pub fn parse_cache(
    bytes: &[u8],
    kind: InputKind,
    expected_build: u32,
) -> Result<CacheEnvelope, &'static str> {
    if bytes.len() > MAX_CACHE_BYTES || expected_build == 0 || kind.cache_filename().is_none() {
        return Err("Invalid cache request or cache exceeds capacity");
    }
    let mut result = CacheEnvelope {
        format_version: INPUT_VERSION,
        source_version: None,
        source_build: None,
        unsupported: None,
        records: vec![],
        conflicting_records: vec![],
    };
    if bytes.len() < 4 {
        return Err("Truncated cache signature");
    }
    if kind != InputKind::HotfixCache || &bytes[..4] != b"XFTH" {
        result.unsupported = Some("unsupported_cache_format".into());
        return Ok(result);
    }
    let u32_at = |offset: usize| -> Result<u32, &'static str> {
        let end = offset.checked_add(4).ok_or("Cache offset overflow")?;
        Ok(u32::from_le_bytes(
            bytes
                .get(offset..end)
                .ok_or("Truncated cache header")?
                .try_into()
                .map_err(|_| "Truncated cache header")?,
        ))
    };
    let version = u32_at(4)?;
    let build = u32_at(8)?;
    result.source_version = Some(version);
    result.source_build = Some(build);
    if build != expected_build {
        return Err("Cache build does not match selected catalog");
    }
    if !matches!(version, 1..=7 | 9) {
        result.unsupported = Some(
            if version == 8 {
                "ambiguous_htfx_version_8"
            } else {
                "unsupported_htfx_version"
            }
            .into(),
        );
        return Ok(result);
    }
    let mut offset = if version >= 5 { 44 } else { 12 };
    if bytes.len() < offset {
        return Err("Truncated cache extended header");
    }
    let mut seen = BTreeSet::new();
    let mut conflicts = BTreeSet::new();
    while offset < bytes.len() {
        if result.records.len() >= MAX_CACHE_RECORDS {
            return Err("Cache record capacity exceeded");
        }
        if u32_at(offset)? != u32::from_le_bytes(*b"XFTH") {
            return Err("Invalid cache record signature");
        }
        let start = offset + 4;
        let (header_size, push, unique, region, table, id, size, flag) = match version {
            1 => (
                20,
                start,
                None,
                None,
                start + 8,
                start + 12,
                start + 4,
                start + 16,
            ),
            2..=6 => {
                // Retain only envelopes whose per-record version agrees.
                if u32_at(start)? != version {
                    return Err("Cache record version mismatch");
                }
                (
                    24,
                    start + 4,
                    None,
                    None,
                    start + 12,
                    start + 16,
                    start + 8,
                    start + 20,
                )
            }
            7 => (
                20,
                start,
                None,
                None,
                start + 4,
                start + 8,
                start + 12,
                start + 16,
            ),
            9 => (
                28,
                start + 4,
                Some(start + 8),
                Some(start),
                start + 12,
                start + 16,
                start + 20,
                start + 24,
            ),
            _ => unreachable!(),
        };
        let end_header = start + header_size;
        let header = bytes
            .get(start..end_header)
            .ok_or("Truncated cache record header")?;
        let opcode = header[flag - start];
        if header[flag - start + 1..].iter().any(|b| *b != 0) {
            return Err("Unsupported cache record flags");
        }
        // Signed sizes and record IDs cannot be reinterpreted as huge unsigned values.
        let payload_size = u32_at(size)?;
        let record_id = u32_at(id)?;
        if payload_size > MAX_RECORD_BYTES as u32 || record_id > i32::MAX as u32 || record_id == 0 {
            return Err("Cache record exceeds limits");
        }
        let end = end_header
            .checked_add(payload_size as usize)
            .ok_or("Cache payload overflow")?;
        if end > bytes.len() {
            return Err("Truncated cache payload");
        }
        let table_hash = u32_at(table)?;
        let push_id = u32_at(push)? as i32;
        if table_hash == 0 {
            return Err("Invalid cache table hash");
        }
        let key = (table_hash, record_id, push_id);
        if !seen.insert(key) {
            conflicts.insert(key);
        }
        result.records.push(CacheRecord {
            ordinal: result.records.len() as u32,
            push_id,
            unique_id: unique.map(u32_at).transpose()?.map(|v| v as i32),
            region_id: region.map(u32_at).transpose()?.map(|v| v as i32),
            table_hash,
            record_id,
            opcode,
            operation: if opcode == 1 && payload_size > 0 {
                CacheOperation::UpsertCandidate
            } else {
                CacheOperation::InvalidationUnknown
            },
            payload_offset: end_header as u32,
            payload_size,
        });
        offset = end;
    }
    result.conflicting_records = conflicts.into_iter().collect();
    Ok(result)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchInput {
    pub classification: crate::local_data_catalog::DistributionClass,
    pub format_version: u32,
    pub catalog_sha256: String,
    /// Selected catalog context, not a claim that an unsupported source records it.
    pub catalog_tuple: InputTuple,
    pub kind: InputKind,
    pub source_sha256: String,
    pub analysis_sha256: String,
    pub analyzer_profile_sha256: String,
    pub fixture: bool,
    pub unsupported: Option<String>,
    pub record_count: Option<u32>,
    pub conflict_count: Option<u32>,
    /// Cache locale comes from the selected directory, never from the XFTH header.
    pub locale_evidence: String,
    pub gaps: Vec<String>,
}
impl ResearchInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.catalog_tuple.validate()?;
        if self.kind.cache_filename().is_some() {
            if self.locale_evidence != "selected_cache_directory_only"
                || self.unsupported.as_ref().is_some_and(|v| {
                    !matches!(
                        v.as_str(),
                        "unsupported_cache_format"
                            | "ambiguous_htfx_version_8"
                            | "unsupported_htfx_version"
                    )
                })
                || self.unsupported.is_some() != self.record_count.is_none()
                || self.record_count.is_none() != self.conflict_count.is_none()
                || self
                    .record_count
                    .is_some_and(|v| v as usize > MAX_CACHE_RECORDS)
                || self
                    .conflict_count
                    .is_some_and(|v| v > self.record_count.unwrap_or(0))
            {
                return Err("Invalid cache input disposition");
            }
        } else if self.unsupported.is_some()
            || self.record_count.is_some()
            || self.conflict_count.is_some()
            || !matches!(
                (self.kind, self.locale_evidence.as_str()),
                (InputKind::CharacterSnapshot, "recorded_addon_context")
                    | (
                        InputKind::SessionEvents,
                        "unrecorded_addon_locale" | "recorded_addon_context"
                    )
            )
        {
            return Err("Invalid observation input disposition");
        }
        if self.classification != crate::local_data_catalog::DistributionClass::LocalOnly
            || self.format_version != INPUT_VERSION
            || ![
                &self.catalog_sha256,
                &self.source_sha256,
                &self.analysis_sha256,
                &self.analyzer_profile_sha256,
            ]
            .iter()
            .all(|s| is_sha256(s))
            || !matches!(
                self.locale_evidence.as_str(),
                "selected_cache_directory_only"
                    | "recorded_addon_context"
                    | "unrecorded_addon_locale"
            )
            || self.gaps.len() > 32
            || self.gaps.iter().any(|s| {
                s.is_empty()
                    || s.len() > 100
                    || !s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            })
        {
            return Err("Invalid research input manifest");
        }
        Ok(())
    }
}
