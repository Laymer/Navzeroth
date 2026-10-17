//! Private immutable research staging. Authoritative artifacts are independent of
//! filesystem timestamps, worker count and run IDs. No gameplay activation here.
use crate::{
    Result, Stamp, checked_path, hash, inspect, metadata,
    native::{Archive, ReadFailure},
    read_file, stamp, tables,
    worker::ExtractionRequest,
};
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InventoryDisposition},
    local_data_catalog::{DistributionClass, ExtractionSummary, ResearchCatalog, TableInventory},
};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const MAX_TABLE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_DECODED_BYTES: usize = 256 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;

struct SourceState {
    stamps: BTreeMap<String, Stamp>,
    hashes: BTreeMap<String, String>,
}
impl SourceState {
    fn capture(
        root: &Path,
        build: &azeroth_core::local_data::InstallationMetadata,
    ) -> Result<Self> {
        let mut names = vec![".build.info".to_owned()];
        for key in [&build.build_key, &build.cdn_key] {
            names.push(format!("Data/config/{}/{}/{key}", &key[..2], &key[2..4]));
        }
        let data = checked_path(&root.join("Data/data"))?;
        for entry in fs::read_dir(data).map_err(|_| "Archive directory inaccessible")? {
            let entry = entry.map_err(|_| "Archive directory inaccessible")?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "Archive filename unsupported")?;
            let lowercase = name.to_ascii_lowercase();
            if lowercase.ends_with(".idx") || lowercase.starts_with("data.") {
                if name.len() > 64
                    || !lowercase
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() || matches!(b, b'.' | b'i' | b'x' | b't'))
                {
                    return Err("Unexpected archive segment name");
                }
                names.push(format!("Data/data/{name}"));
            }
            if names.len() > 8192 {
                return Err("Archive directory exceeds capacity");
            }
        }
        names.sort();
        let mut stamps = BTreeMap::new();
        let mut hashes = BTreeMap::new();
        for name in names {
            let path = checked_path(&root.join(&name))?;
            let meta = fs::metadata(&path).map_err(|_| "Source metadata unavailable")?;
            if !meta.is_file() {
                return Err("Source input is not a regular file");
            }
            stamps.insert(name.clone(), stamp(&meta)?);
            if !name.to_ascii_lowercase().starts_with("data/data/data.") {
                let (bytes, _) = read_file(&path, 64 * 1024 * 1024)?;
                hashes.insert(name, hash(&bytes));
            }
        }
        Ok(Self { stamps, hashes })
    }
}

pub(crate) struct Store {
    root: PathBuf,
    quota: u64,
    used: u64,
    _lock: File,
}
fn total_size(directory: &Path, depth: usize, count: &mut usize) -> Result<u64> {
    if depth > 12 {
        return Err("Private store nesting exceeds capacity");
    }
    let mut total = 0u64;
    for entry in fs::read_dir(directory).map_err(|_| "Private store inaccessible")? {
        let entry = entry.map_err(|_| "Private store inaccessible")?;
        *count += 1;
        if *count > 200_000 {
            return Err("Private store file capacity exceeded");
        }
        let path = checked_path(&entry.path())?;
        let meta = fs::metadata(&path).map_err(|_| "Private store metadata unavailable")?;
        let size = if meta.is_dir() {
            total_size(&path, depth + 1, count)?
        } else if meta.is_file() {
            meta.len()
        } else {
            return Err("Private store contains a special file");
        };
        total = total.checked_add(size).ok_or("Storage size overflow")?;
    }
    Ok(total)
}

impl Store {
    pub(crate) fn open(destination: &Path, quota: u64) -> Result<Self> {
        let root = checked_path(destination)?;
        if !root.is_dir() {
            return Err("Private destination must be a directory");
        }
        for entry in fs::read_dir(&root).map_err(|_| "Private destination inaccessible")? {
            let name = entry
                .map_err(|_| "Private destination inaccessible")?
                .file_name();
            if !["writer.lock", "objects", "catalogs", "projections"]
                .iter()
                .any(|allowed| name == *allowed)
            {
                return Err("Destination must be empty or an existing local-data store");
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if fs::metadata(&root)
                .map_err(|_| "Private destination metadata unavailable")?
                .permissions()
                .mode()
                & 0o077
                != 0
            {
                return Err("Private destination must permit access only to its owner");
            }
        }
        let lock_path = root.join("writer.lock");
        if lock_path.exists() {
            checked_path(&lock_path)?;
            if !lock_path.is_file() {
                return Err("Writer lock must be a regular file");
            }
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|_| "Private writer lock unavailable")?;
        lock.try_lock()
            .map_err(|_| "Another extraction owns the destination")?;
        let used = total_size(&root, 0, &mut 0)?;
        if used > quota {
            return Err("Existing private store exceeds configured quota");
        }
        for name in ["objects", "catalogs", "projections"] {
            let path = root.join(name);
            if !path.exists() {
                fs::create_dir(&path).map_err(|_| "Cannot create private store")?;
            }
            checked_path(&path)?;
        }
        Ok(Self {
            root,
            quota,
            used,
            _lock: lock,
        })
    }
    pub(crate) fn put(&mut self, namespace: &str, bytes: &[u8]) -> Result<String> {
        let digest = hash(bytes);
        let path = self.root.join(namespace).join(&digest);
        if path.exists() {
            let (size, existing) = crate::hash_file(&path, bytes.len())?;
            if existing != digest || size != bytes.len() as u64 {
                return Err("Existing private object is corrupt");
            }
            return Ok(digest);
        }
        let after = self
            .used
            .checked_add(bytes.len() as u64)
            .ok_or("Storage quota overflow")?;
        if after > self.quota {
            return Err("Storage quota exhausted without truncation");
        }
        if crate::isolation::available_space(&self.root)? < bytes.len() as u64 {
            return Err("Insufficient free storage space");
        }
        // Readers accept only content-addressed, hash-verified objects. A killed
        // writer may leave an invalid object; it cannot become a sealed catalog.
        let temporary = self
            .root
            .join(namespace)
            .join(format!(".{digest}.{}.staging", std::process::id()));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| "Cannot create immutable private object")?;
        if file
            .write_all(bytes)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            let _ = fs::remove_file(&temporary);
            return Err("Private object write failed");
        }
        drop(file);
        fs::hard_link(&temporary, &path).map_err(|_| "Cannot atomically seal private object")?;
        fs::remove_file(temporary).map_err(|_| "Cannot remove sealed staging reference")?;
        self.used = after;
        Ok(digest)
    }
}

fn installation_lock(root: &Path, destination: &Path) -> Result<File> {
    // OS file locks release on crashes. The lock directory is caller-private;
    // source paths appear only inside the digest, never in events/manifests.
    // A common per-user temp location serializes alternate destinations too.
    let mut key = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .unwrap_or_default()
        .as_encoded_bytes()
        .to_vec();
    key.extend_from_slice(root.as_os_str().as_encoded_bytes());
    let name = format!("navzeroth-installation-{}.lock", hash(&key));
    let directory = checked_path(&std::env::temp_dir())?;
    if directory.starts_with(root) || root.starts_with(destination) {
        return Err("Unsafe installation lock location");
    }
    let path = directory.join(name);
    if path.exists() {
        checked_path(&path)?;
        if !path.is_file() {
            return Err("Installation lock must be a regular file");
        }
    }
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|_| "Installation lock unavailable")?;
    file.try_lock()
        .map_err(|_| "Another extraction owns this installation")?;
    Ok(file)
}

pub(crate) fn extract(
    request: &ExtractionRequest,
    mut progress: impl FnMut(u32, u32) -> Result<()>,
) -> Result<ExtractionSummary> {
    request.validate()?;
    let root = checked_path(&request.installation)?;
    let destination = checked_path(&request.destination)?;
    if root.starts_with(&destination) || destination.starts_with(&root) {
        return Err("Source and destination must be separate");
    }
    let _installation_lock = installation_lock(&root, &destination)?;
    let inspection = inspect(&root, &request.product)?;
    if !metadata::ADMITTED_BUILDS.contains(&inspection.metadata.build) {
        return Err("Build unsupported; no nearest-build fallback");
    }
    let candidates = metadata::candidates();
    if request
        .tables
        .iter()
        .any(|name| !candidates.iter().any(|t| &t.name == name))
    {
        return Err("Requested table is absent from reviewed metadata");
    }
    let tables: Vec<_> = candidates
        .into_iter()
        .filter(|t| request.tables.is_empty() || request.tables.contains(&t.name))
        .collect();
    let mut selected = Vec::new();
    for locale in &inspection.metadata.declared_locales {
        let mask = match locale.as_str() {
            "enUS" => 2,
            "koKR" => 4,
            "frFR" => 0x10,
            "deDE" => 0x20,
            "zhCN" => 0x40,
            "esES" => 0x80,
            "zhTW" => 0x100,
            "enGB" => 0x200,
            "esMX" => 0x1000,
            "ruRU" => 0x2000,
            "ptBR" => 0x4000,
            "itIT" => 0x8000,
            "ptPT" => 0x10000,
            _ => return Err("Installed locale unsupported"),
        };
        selected.extend(tables.iter().cloned().map(|t| (locale.clone(), mask, t)));
    }
    if selected.is_empty() || selected.len() > 32_768 {
        return Err("Installed locales or table inventory unavailable or excessive");
    }
    let source = SourceState::capture(&root, &inspection.metadata)?;
    if source.hashes.get(".build.info") != Some(&inspection.build_info_sha256) {
        return Err("Installation changed after inspection; candidate refused");
    }
    let mut store = Store::open(&destination, request.preset.limits().total_storage_bytes)?;
    let mut current_locale = selected[0].0.clone();
    let mut archive = Some(Archive::open(
        &root,
        &request.product,
        selected[0].1,
        inspection.metadata.build,
    )?);
    let archive_statistics = Some(archive.as_ref().unwrap().statistics()?);
    let mut inventory = Vec::new();
    let total = selected.len() as u32;
    progress(0, total)?;
    let mut gaps = vec![
        "research_only_no_playable_availability_evidence".into(),
        "hotfix_state_unknown".into(),
        "media_not_extracted".into(),
        "no_gameplay_projection".into(),
    ];
    if !request.tables.is_empty() {
        gaps.push("explicit_table_subset".into());
    }
    for (index, (locale, mask, candidate)) in selected.into_iter().enumerate() {
        if locale != current_locale {
            drop(archive.take());
            archive = Some(Archive::open(
                &root,
                &request.product,
                mask,
                inspection.metadata.build,
            )?);
            current_locale = locale.clone();
        }
        let id = candidate.file_data_id;
        let mut entry = TableInventory {
            table: candidate.name.clone(),
            locale: Some(locale),
            file_data_id: id,
            disposition: InventoryDisposition::RetainedUnsupported,
            reason: "unsupported_table_format".into(),
            retained_sha256: None,
            decoded_sha256: None,
            encoded_bytes: None,
            records: None,
            layout_hash: None,
        };
        match archive.as_ref().unwrap().read(
            id,
            &candidate.name,
            candidate.extension,
            MAX_TABLE_BYTES,
        ) {
            Ok(bytes) => {
                entry.encoded_bytes = Some(bytes.len() as u64);
                entry.retained_sha256 = Some(store.put("objects", &bytes)?);
                match tables::header(&bytes) {
                    Err(reason) => entry.reason = reason.into(),
                    Ok(header) => {
                        entry.layout_hash = Some(format!("{:08x}", header.layout_hash));
                        entry.records = Some(header.records);
                        if format!("{:08X}", header.table_hash)
                            != candidate.table_hash.to_ascii_uppercase()
                        {
                            entry.disposition = InventoryDisposition::InvalidCorrupt;
                            entry.reason = "table_hash_mismatch".into();
                        } else {
                            match metadata::definition(
                                &candidate.name,
                                inspection.metadata.build,
                                header.layout_hash,
                            ) {
                                Err(reason) => {
                                    entry.reason = reason.into();
                                    entry.disposition = if reason == "layout_mismatch" {
                                        InventoryDisposition::LayoutMismatch
                                    } else if reason == "definition_missing" {
                                        InventoryDisposition::DefinitionMissing
                                    } else {
                                        InventoryDisposition::RetainedUnsupported
                                    };
                                }
                                Ok(definition) => match tables::decode(&bytes, &definition) {
                                    Err(reason) => entry.reason = reason.into(),
                                    Ok(decoded) => {
                                        let output =
                                            crate::bounded_json(&decoded, MAX_DECODED_BYTES)?;
                                        if output.len() > MAX_DECODED_BYTES {
                                            return Err("Decoded table encoding exceeds capacity");
                                        }
                                        entry.decoded_sha256 = Some(store.put("objects", &output)?);
                                        entry.disposition = InventoryDisposition::Decoded;
                                        entry.reason = if decoded.gaps.is_empty() {
                                            "research_decoded"
                                        } else {
                                            "research_decoded_with_encrypted_gaps"
                                        }
                                        .into();
                                        gaps.extend(
                                            decoded
                                                .gaps
                                                .iter()
                                                .map(|g| format!("{}:{g}", candidate.name)),
                                        );
                                    }
                                },
                            }
                        }
                    }
                }
            }
            Err(error) => {
                let (disposition, reason) = match error {
                    ReadFailure::Missing => (
                        InventoryDisposition::NotLocallyPresent,
                        "not_locally_present",
                    ),
                    ReadFailure::Encrypted => (
                        InventoryDisposition::UnavailableEncrypted,
                        "unavailable_encrypted",
                    ),
                    ReadFailure::Unsupported => (
                        InventoryDisposition::UnavailableEncrypted,
                        "unsupported_archive_encoding_not_retained",
                    ),
                    ReadFailure::Unavailable => (
                        InventoryDisposition::UnavailableEncrypted,
                        "local_archive_read_unavailable",
                    ),
                    ReadFailure::Capacity => (
                        InventoryDisposition::ExcludedByPolicy,
                        "logical_file_or_allocation_exceeds_capacity_not_retained",
                    ),
                    ReadFailure::Invalid => (
                        InventoryDisposition::InvalidCorrupt,
                        "archive_verification_failed",
                    ),
                };
                entry.disposition = disposition;
                entry.reason = reason.into();
            }
        }
        inventory.push(entry);
        progress(index as u32 + 1, total)?;
    }
    drop(archive);
    let after = SourceState::capture(&root, &inspection.metadata)?;
    if source.stamps != after.stamps || source.hashes != after.hashes {
        return Err("Source changed; candidate was not sealed");
    }
    let decoded = inventory
        .iter()
        .filter(|t| t.disposition == InventoryDisposition::Decoded)
        .count() as u32;
    gaps.sort();
    gaps.dedup();
    let manifest = ResearchCatalog {
        format_version: 1,
        classification: DistributionClass::LocalOnly,
        installation: inspection.metadata,
        build_info_sha256: inspection.build_info_sha256,
        extractor_version: env!("CARGO_PKG_VERSION").into(),
        native_revision: CASCLIB_REVISION.into(),
        metadata_sha256: metadata::METADATA_SHA256.into(),
        parser_profile: "wdc3-5-research-v1_dbd-0.1.0-navzeroth-v1".into(),
        extractor_profile_sha256: Some(env!("NAVZEROTH_LOCAL_DATA_PROFILE_SHA256").into()),
        input_hashes: source.hashes,
        archive_statistics,
        tables: inventory,
        gaps,
        gameplay_projection_sha256: None,
    };
    let bytes = crate::bounded_json(&manifest, MAX_MANIFEST_BYTES)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("Catalog manifest exceeds capacity");
    }
    let digest = store.put("catalogs", &bytes)?;
    Ok(ExtractionSummary {
        format_version: 1,
        catalog_sha256: digest,
        build: manifest.installation.build,
        tables_processed: total,
        tables_decoded: decoded,
        preset: request.preset,
        actual_workers: 1,
        gameplay_projection_available: false,
        activated: false,
    })
}

pub fn verify_catalog(destination: &Path, digest: &str) -> Result<ResearchCatalog> {
    if !azeroth_core::local_data::is_sha256(digest) {
        return Err("Invalid catalog identity");
    }
    let root = checked_path(destination)?;
    let (bytes, _) = read_file(&root.join("catalogs").join(digest), MAX_MANIFEST_BYTES)?;
    if hash(&bytes) != digest {
        return Err("Catalog hash mismatch");
    }
    let manifest: ResearchCatalog = serde_json::from_value(azeroth_core::strict_json::value(
        &bytes,
        MAX_MANIFEST_BYTES,
    )?)
    .map_err(|_| "Invalid catalog manifest")?;
    if manifest.format_version != 1
        || manifest.tables.len() > 32768
        || manifest.native_revision != CASCLIB_REVISION
        || manifest.metadata_sha256 != metadata::METADATA_SHA256
        || manifest.gameplay_projection_sha256.is_some()
    {
        return Err("Unsupported catalog version or projection");
    }
    let mut names = std::collections::BTreeSet::new();
    for entry in &manifest.tables {
        if !names.insert((&entry.table, &entry.locale)) {
            return Err("Duplicate table inventory");
        }
        for digest in [&entry.retained_sha256, &entry.decoded_sha256]
            .into_iter()
            .flatten()
        {
            if !azeroth_core::local_data::is_sha256(digest) {
                return Err("Invalid object identity");
            }
            let (_, actual) =
                crate::hash_file(&root.join("objects").join(digest), MAX_DECODED_BYTES)?;
            if actual != *digest {
                return Err("Retained object hash mismatch");
            }
        }
        if entry.disposition == InventoryDisposition::Decoded
            && (entry.retained_sha256.is_none() || entry.decoded_sha256.is_none())
        {
            return Err("Decoded table closure missing");
        }
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quota_refusal_and_duplicate_writes_preserve_sealed_objects() {
        let root =
            std::env::temp_dir().join(format!("navzeroth-store-quota-{}", std::process::id()));
        let mut builder = fs::DirBuilder::new();
        builder.recursive(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&root).unwrap();
        let mut store = Store::open(&root, 8).unwrap();
        let object = store.put("objects", b"retained").unwrap();
        assert_eq!(store.put("objects", b"retained").unwrap(), object);
        assert!(store.put("objects", b"extra").is_err());
        assert_eq!(
            fs::read(root.join("objects").join(object)).unwrap(),
            b"retained"
        );
        assert_eq!(fs::read_dir(root.join("catalogs")).unwrap().count(), 0);
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
}
