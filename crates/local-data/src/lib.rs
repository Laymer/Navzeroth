//! Read-only local client-data adapters. No game processes, network or implicit discovery.
//!
//! This initial adapter inspects build metadata only. Archive consistency, readable
//! locales, edition applicability and gameplay availability remain unverified.
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

mod extraction;
#[allow(unsafe_code)]
mod isolation;
pub mod metadata;
#[allow(unsafe_code)]
mod native;
pub mod receipts;
pub mod tables;
pub mod worker;
pub use extraction::verify_catalog;

use azeroth_core::local_data::{
    FORMAT_VERSION, InstallationMetadata, MAX_BUILD_INFO_BYTES, MAX_METADATA_BYTES, MetadataBundle,
    PerformancePreset, ResourceLimits, decode_metadata, inspect_build_info,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, Metadata},
    io::Read,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};

/// Stable diagnostics intentionally omit paths, imported values and OS error text.
pub type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Inspection {
    pub format_version: u32,
    pub metadata: InstallationMetadata,
    pub build_info_sha256: String,
    pub consistency: InspectionConsistency,
    pub limitations: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionConsistency {
    /// Only the selected .build.info was compared over the stable interval.
    BuildInfoStable,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn bounded_json<T: serde::Serialize>(value: &T, maximum: usize) -> Result<Vec<u8>> {
    struct Output {
        bytes: Vec<u8>,
        maximum: usize,
    }
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self
                .bytes
                .len()
                .checked_add(bytes.len())
                .is_none_or(|v| v > self.maximum)
            {
                return Err(std::io::ErrorKind::OutOfMemory.into());
            }
            self.bytes
                .try_reserve(bytes.len())
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut output, value)
        .map_err(|_| "Encoded output exceeds capacity or is invalid")?;
    Ok(output.bytes)
}

fn linked(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Reject all reparse points, including junctions, not merely symlinks.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Refuse link traversal for the selected path and existing ancestors. This is
/// an inspection preflight, not a race-proof extraction sandbox. A future native
/// worker must hold/revalidate handles throughout archive and output operations.
pub fn checked_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() || path.components().any(|p| p == Component::ParentDir) {
        return Err("Empty or parent-traversing selection");
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "Cannot resolve selection")?
            .join(path)
    };
    let mut cursor = PathBuf::new();
    for part in absolute.components() {
        if part == Component::CurDir {
            continue;
        }
        cursor.push(part.as_os_str());
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&cursor).map_err(|_| "Selection is inaccessible")?;
        if linked(&metadata) {
            return Err("Linked or reparse-point selections are unsupported");
        }
    }
    fs::canonicalize(absolute).map_err(|_| "Cannot resolve selection")
}

#[derive(Debug, PartialEq)]
struct Stamp {
    bytes: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}

fn stamp(metadata: &Metadata) -> Result<Stamp> {
    Ok(Stamp {
        bytes: metadata.len(),
        modified: metadata
            .modified()
            .map_err(|_| "File modification identity unavailable")?,
        #[cfg(unix)]
        identity: {
            use std::os::unix::fs::MetadataExt;
            (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            )
        },
    })
}

/// Bounded read; refuses ordinary changes during read and doesn't expose content
/// in errors. The filename and all ancestors must be selected private inputs.
fn read_file(path: &Path, maximum: usize) -> Result<(Vec<u8>, Stamp)> {
    let canonical = checked_path(path)?;
    let selected = fs::metadata(&canonical).map_err(|_| "Cannot inspect selected input")?;
    // Check before open as well: opening a FIFO/device can block or have effects.
    if !selected.is_file() || selected.len() > maximum as u64 {
        return Err("Selected input is not a bounded regular file");
    }
    let selected = stamp(&selected)?;
    let file = File::open(&canonical).map_err(|_| "Selected input cannot be opened")?;
    let before = file
        .metadata()
        .map_err(|_| "Cannot inspect selected input")?;
    if !before.is_file() || before.len() > maximum as u64 {
        return Err("Selected input is not a bounded regular file");
    }
    let before = stamp(&before)?;
    if selected != before {
        return Err("Selected input changed before opening");
    }
    let mut bytes = Vec::new();
    (&file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read selected input")?;
    let after = stamp(
        &file
            .metadata()
            .map_err(|_| "Cannot recheck selected input")?,
    )?;
    let current_path = checked_path(path)?;
    let current = stamp(&fs::metadata(&current_path).map_err(|_| "Selected input disappeared")?)?;
    if bytes.len() > maximum
        || before != after
        || before != current
        || canonical != current_path
        || bytes.len() as u64 != before.bytes
    {
        return Err("Selected input changed during reading");
    }
    Ok((bytes, before))
}

fn hash_file(path: &Path, maximum: usize) -> Result<(u64, String)> {
    let canonical = checked_path(path)?;
    let metadata = fs::metadata(&canonical).map_err(|_| "Object metadata unavailable")?;
    if !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err("Object exceeds size or file-kind bounds");
    }
    let before = stamp(&metadata)?;
    let mut file = File::open(&canonical).map_err(|_| "Cannot open retained object")?;
    if stamp(&file.metadata().map_err(|_| "Object metadata unavailable")?)? != before {
        return Err("Retained object changed");
    }
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut total = 0u64;
    loop {
        let size = file
            .read(&mut buffer)
            .map_err(|_| "Cannot read retained object")?;
        if size == 0 {
            break;
        }
        total += size as u64;
        if total > maximum as u64 {
            return Err("Retained object exceeds capacity");
        }
        digest.update(&buffer[..size]);
    }
    if total != before.bytes
        || stamp(&file.metadata().map_err(|_| "Object metadata unavailable")?)? != before
        || checked_path(path)? != canonical
        || stamp(&fs::metadata(&canonical).map_err(|_| "Object disappeared")?)? != before
    {
        return Err("Retained object changed");
    }
    Ok((total, format!("{:x}", digest.finalize())))
}

/// Reads exactly .build.info under the explicitly selected installation.
pub fn inspect(installation: &Path, product: &str) -> Result<Inspection> {
    inspect_with_stability_check(installation, product, || {
        std::thread::sleep(Duration::from_millis(200));
    })
}

fn inspect_with_stability_check(
    installation: &Path,
    product: &str,
    stable_interval: impl FnOnce(),
) -> Result<Inspection> {
    let root = checked_path(installation)?;
    if !root.is_dir() {
        return Err("Installation selection is not a directory");
    }
    let path = root.join(".build.info");
    let (bytes, before) = read_file(&path, MAX_BUILD_INFO_BYTES)?;
    let metadata = inspect_build_info(&bytes, product)?;
    stable_interval();
    let (again, after) = read_file(&path, MAX_BUILD_INFO_BYTES)?;
    if bytes != again || before != after || checked_path(installation)? != root {
        return Err("Installation metadata is changing; retry after the update finishes");
    }
    Ok(Inspection {
        format_version: FORMAT_VERSION,
        metadata,
        build_info_sha256: hash(&bytes),
        consistency: InspectionConsistency::BuildInfoStable,
        limitations: vec![
            "archive_storage_unverified",
            "edition_unverified",
            "installed_locale_files_unverified",
            "playable_availability_unknown",
            "hotfix_state_unknown",
        ],
    })
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionPlan {
    pub format_version: u32,
    pub installation: Inspection,
    pub preset: PerformancePreset,
    pub limits: ResourceLimits,
    pub required_storage_bytes: Option<u64>,
    pub available_storage_bytes: Option<u64>,
    pub actual_workers: u8,
    pub extraction_ready: bool,
    pub activation_automatic: bool,
    pub blockers: Vec<&'static str>,
}

/// Read-only preliminary plan. No output creation, archive opening or activation.
/// Destination must already exist; descendants and ancestors of source are refused.
pub fn plan(
    installation: &Path,
    product: &str,
    destination: &Path,
    preset: PerformancePreset,
) -> Result<ExtractionPlan> {
    let source = checked_path(installation)?;
    let output = checked_path(destination)?;
    if !output.is_dir() || source.starts_with(&output) || output.starts_with(&source) {
        return Err("Destination must be a separate existing directory outside the installation");
    }
    Ok(ExtractionPlan {
        format_version: FORMAT_VERSION,
        installation: inspect(&source, product)?,
        preset,
        limits: preset.limits(),
        required_storage_bytes: None,
        available_storage_bytes: isolation::available_space(&output).ok(),
        actual_workers: 0,
        extraction_ready: false,
        activation_automatic: false,
        blockers: vec![
            "archive_inventory_estimate_pending",
            "research_tables_only_no_gameplay_projection",
            "media_extraction_pending",
            "storage_estimate_unknown",
            "destination_write_permissions_unverified",
        ],
    })
}

/// Application-owned review policy. Neither a bundle nor its retained objects
/// can establish their own trust or grant new executable transforms.
pub struct MetadataPolicy<'a> {
    pub reviewed_bundle_sha256: &'a str,
    pub implemented_transforms: &'a [&'a str],
}

/// Verify a reviewed metadata bundle against its trusted digest and every retained
/// object. The production caller must supply policy from reviewed configuration.
/// Hash verification establishes bytes, not legal sufficiency or parser support.
pub fn verify_metadata(
    bundle_file: &Path,
    object_directory: &Path,
    policy: &MetadataPolicy<'_>,
) -> Result<MetadataBundle> {
    let (bytes, _) = read_file(bundle_file, MAX_METADATA_BYTES)?;
    if !azeroth_core::local_data::is_sha256(policy.reviewed_bundle_sha256)
        || hash(&bytes) != policy.reviewed_bundle_sha256
    {
        return Err("Metadata bundle is not bound to the reviewed policy");
    }
    let bundle = decode_metadata(&bytes, policy.implemented_transforms)?;
    let objects = checked_path(object_directory)?;
    if !objects.is_dir() {
        return Err("Metadata object selection is not a directory");
    }
    for object in &bundle.objects {
        let bound = usize::try_from(object.bytes)
            .map_err(|_| "Metadata object exceeds platform capacity")?;
        let (bytes, _) = read_file(&objects.join(&object.sha256), bound)?;
        if bytes.len() as u64 != object.bytes || hash(&bytes) != object.sha256 {
            return Err("Metadata closure hash or size mismatch");
        }
    }
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_source_is_refused_even_with_same_file_length() {
        // Deterministic mutation between the two reads, without a timing race.
        let root = std::env::temp_dir().join(format!(
            "navzeroth-changing-build-info-{}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let source = format!(
            "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70425|{}|{}|enUS text\n",
            "a".repeat(32),
            "b".repeat(32)
        );
        let file = root.join(".build.info");
        fs::write(&file, &source).unwrap();
        let result = inspect_with_stability_check(&root, "wow_classic_beta", || {
            fs::write(&file, source.replace("70425", "70374")).unwrap();
        });
        assert_eq!(
            result.unwrap_err(),
            "Installation metadata is changing; retry after the update finishes"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
