//! Pure local-client-data contracts. No filesystem, network, native or UI access.
//!
//! Installation metadata identifies an input build, never playable availability.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_BUILD_INFO_BYTES: usize = 256 * 1024;
pub const MAX_BUILD_INFO_ROWS: usize = 64;
pub const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TABLES: usize = 16_384;
pub const CASCLIB_REVISION: &str = "38a34665624b8775bb875274b36191b21c38d97b";
pub const WOWDBDEFS_REVISION: &str = "b9a63096cdd99b26967f17f9515377ae62cadfd7";
pub const DBCD_ORACLE_REVISION: &str = "e732093f8864240fc5884bd1bba6b02f3dfc0d56";
const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformancePreset {
    Small,
    Balanced,
    Large,
    #[default]
    Max,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub workers: u8,
    /// Budget of the whole worker, not each decoding thread.
    pub worker_memory_bytes: u64,
    /// Includes originals, derivatives, indexes and staging, not just final output.
    pub total_storage_bytes: u64,
}

impl PerformancePreset {
    pub fn limits(self) -> ResourceLimits {
        let (workers, memory, storage) = match self {
            Self::Small => (1, 1, 8),
            Self::Balanced => (2, 2, 32),
            Self::Large => (4, 4, 64),
            Self::Max => (8, 8, 128),
        };
        ResourceLimits {
            workers,
            worker_memory_bytes: memory * GIB,
            total_storage_bytes: storage * GIB,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationMetadata {
    pub product: String,
    pub version: String,
    pub build: u32,
    pub build_key: String,
    pub cdn_key: String,
    /// Locale declarations in metadata, not proof of locally readable locale files.
    pub declared_locales: Vec<String>,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn hex_key(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parse only the selected active product. No guessed product or closest build.
/// Unknown columns are inert, but duplicate column names/ambiguous rows are invalid.
pub fn inspect_build_info(
    bytes: &[u8],
    selected_product: &str,
) -> Result<InstallationMetadata, &'static str> {
    if bytes.len() > MAX_BUILD_INFO_BYTES || !identifier(selected_product) {
        return Err("Invalid product or oversized installation metadata");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "Installation metadata is not UTF-8")?;
    if text.contains('\0') {
        return Err("Installation metadata contains NUL");
    }
    let mut lines = text.lines();
    let header = lines.next().ok_or("Installation metadata header missing")?;
    let mut names = BTreeSet::new();
    let columns: Vec<_> = header
        .split('|')
        .map(|s| s.split_once('!').map(|v| v.0))
        .collect::<Option<_>>()
        .ok_or("Invalid installation metadata header")?;
    if columns.len() > 64
        || columns
            .iter()
            .any(|name| name.is_empty() || !names.insert(*name))
    {
        return Err("Duplicate or excessive metadata columns");
    }
    let index = |key| {
        columns
            .iter()
            .position(|s| *s == key)
            .ok_or("Required metadata column missing")
    };
    for (name, kind) in [
        ("Product", "STRING:0"),
        ("Active", "DEC:1"),
        ("Version", "STRING:0"),
        ("Build Key", "HEX:16"),
        ("CDN Key", "HEX:16"),
        ("Tags", "STRING:0"),
    ] {
        let column = index(name)?;
        if header
            .split('|')
            .nth(column)
            .and_then(|s| s.split_once('!'))
            .map(|s| s.1)
            != Some(kind)
        {
            return Err("Unsupported installation metadata column type");
        }
    }
    let (product, active, version, build_key, cdn_key, tags) = (
        index("Product")?,
        index("Active")?,
        index("Version")?,
        index("Build Key")?,
        index("CDN Key")?,
        index("Tags")?,
    );
    let mut selected = None;
    for (number, line) in lines.enumerate() {
        if number >= MAX_BUILD_INFO_ROWS {
            return Err("Too many installation metadata rows");
        }
        let row: Vec<_> = line.split('|').collect();
        if row.len() != columns.len() || !matches!(row[active], "0" | "1") {
            return Err("Invalid installation metadata row");
        }
        if row[product] != selected_product || row[active] != "1" {
            continue;
        }
        if selected.is_some() {
            return Err("Selected active product is ambiguous");
        }
        let parts: Vec<_> = row[version].split('.').collect();
        if parts.len() != 4
            || parts
                .iter()
                .any(|p| p.is_empty() || p.len() > 10 || !p.bytes().all(|b| b.is_ascii_digit()))
            || parts.iter().any(|p| p.len() > 1 && p.starts_with('0'))
            || parts.iter().any(|p| p.parse::<u32>().is_err())
            || !hex_key(row[build_key])
            || !hex_key(row[cdn_key])
        {
            return Err("Invalid selected build identity");
        }
        let build = parts[3]
            .parse::<u32>()
            .map_err(|_| "Invalid build number")?;
        if build == 0 {
            return Err("Invalid build number");
        }
        let locales: BTreeSet<_> = row[tags]
            .split_ascii_whitespace()
            .filter(|tag| {
                matches!(
                    *tag,
                    "enUS"
                        | "enGB"
                        | "deDE"
                        | "esES"
                        | "esMX"
                        | "frFR"
                        | "itIT"
                        | "koKR"
                        | "ptBR"
                        | "ptPT"
                        | "ruRU"
                        | "zhCN"
                        | "zhTW"
                )
            })
            .map(str::to_owned)
            .collect();
        selected = Some(InstallationMetadata {
            product: selected_product.to_owned(),
            version: row[version].to_owned(),
            build,
            build_key: row[build_key].to_ascii_lowercase(),
            cdn_key: row[cdn_key].to_ascii_lowercase(),
            declared_locales: locales.into_iter().collect(),
        });
    }
    selected.ok_or("Selected active product is absent")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryDisposition {
    Decoded,
    RetainedUnsupported,
    NotLocallyPresent,
    UnavailableEncrypted,
    DefinitionMissing,
    LayoutMismatch,
    InvalidCorrupt,
    ExcludedByPolicy,
}

/// Metadata files are addressed by SHA-256, not upstream paths or URLs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataObject {
    pub sha256: String,
    pub bytes: u64,
    pub role: MetadataRole,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataRole {
    Definition,
    License,
    Notice,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TableDefinition {
    pub table: String,
    pub file_data_id: u32,
    pub product: String,
    pub build: u32,
    pub layout_hash: String,
    pub definition_sha256: String,
    pub license_sha256: String,
    pub transform: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataBundle {
    pub format_version: u32,
    pub wowdbdefs_revision: String,
    pub objects: Vec<MetadataObject>,
    pub tables: Vec<TableDefinition>,
}

/// Structural admission only; the I/O adapter must also verify every retained byte.
/// `implemented_transforms` is application-owned, never supplied by this bundle.
pub fn validate_metadata(
    bundle: &MetadataBundle,
    implemented_transforms: &[&str],
) -> Result<(), &'static str> {
    if bundle.format_version != FORMAT_VERSION || bundle.wowdbdefs_revision != WOWDBDEFS_REVISION {
        return Err("Unsupported metadata version or revision");
    }
    if bundle.tables.is_empty()
        || bundle.tables.len() > MAX_TABLES
        || bundle.objects.len() > MAX_TABLES * 2
    {
        return Err("Empty or excessive metadata inventory");
    }
    let mut objects = std::collections::BTreeMap::new();
    let mut total = 0u64;
    for object in &bundle.objects {
        if !is_sha256(&object.sha256)
            || object.bytes == 0
            || objects.insert(&object.sha256, object).is_some()
        {
            return Err("Invalid or duplicate metadata object");
        }
        total = total
            .checked_add(object.bytes)
            .ok_or("Metadata size overflow")?;
        if total > 64 * 1024 * 1024 {
            return Err("Metadata closure too large");
        }
    }
    let mut identities = BTreeSet::new();
    let mut file_ids = BTreeSet::new();
    for table in &bundle.tables {
        let has_role = |hash: &String, role| objects.get(hash).is_some_and(|o| o.role == role);
        if !identifier(&table.table)
            || !identifier(&table.product)
            || table.build == 0
            || table.file_data_id == 0
            || table.layout_hash.len() != 8
            || !table
                .layout_hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !has_role(&table.definition_sha256, MetadataRole::Definition)
            || !has_role(&table.license_sha256, MetadataRole::License)
            || !implemented_transforms.contains(&table.transform.as_str())
            || !identities.insert((
                &table.product,
                table.build,
                &table.table,
                &table.layout_hash,
            ))
            || !file_ids.insert((
                &table.product,
                table.build,
                table.file_data_id,
                &table.layout_hash,
            ))
        {
            return Err("Invalid, unlicensed, ambiguous or unsupported table definition");
        }
    }
    Ok(())
}

/// Byte/depth/duplicate checks precede typed decoding and structural admission.
pub fn decode_metadata(bytes: &[u8], transforms: &[&str]) -> Result<MetadataBundle, &'static str> {
    let value = crate::strict_json::value(bytes, MAX_METADATA_BYTES)?;
    let bundle: MetadataBundle =
        serde_json::from_value(value).map_err(|_| "Invalid metadata schema")?;
    validate_metadata(&bundle, transforms)?;
    Ok(bundle)
}
