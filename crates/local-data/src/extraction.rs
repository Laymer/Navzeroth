//! Private immutable research staging. Authoritative artifacts are independent of
//! filesystem timestamps, worker count and run IDs. No gameplay activation here.
use crate::{
    Result, Stamp, checked_path, hash, inspect, metadata,
    native::{Archive, Logical, ReadFailure},
    partial::PartialFile,
    read_file, stamp, tables,
    worker::ExtractionRequest,
};
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InventoryDisposition},
    local_data_catalog::{DistributionClass, ExtractionSummary, ResearchCatalog, TableInventory},
    local_data_lifecycle::{Checkpoint, CheckpointBinding, JobState},
};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const MAX_TABLE_BYTES: usize = 64 * 1024 * 1024;
const LIFECYCLE_RESERVE_BYTES: u64 = 130 * 1024 * 1024;
pub const MAX_DECODED_BYTES: usize = 256 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
/// A partially available file whose header, field, pallet, common or section
/// directory bytes lie in an encrypted frame: no section can be located.
const UNAVAILABLE_METADATA: &str = "unavailable_encrypted_frames_in_table_metadata";

struct SourceState {
    stamps: BTreeMap<String, Stamp>,
    hashes: BTreeMap<String, String>,
}
impl SourceState {
    #[cfg(test)]
    fn capture(
        root: &Path,
        build: &azeroth_core::local_data::InstallationMetadata,
    ) -> Result<Self> {
        Self::capture_checked(root, build, || Ok(()))
    }
    fn capture_checked(
        root: &Path,
        build: &azeroth_core::local_data::InstallationMetadata,
        mut check: impl FnMut() -> Result<()>,
    ) -> Result<Self> {
        check()?;
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
            check()?;
            let path = checked_path(&root.join(&name))?;
            let meta = fs::metadata(&path).map_err(|_| "Source metadata unavailable")?;
            if !meta.is_file() {
                return Err("Source input is not a regular file");
            }
            stamps.insert(name.clone(), stamp(&meta)?);
            let digest = if name.to_ascii_lowercase().starts_with("data/data/data.") {
                // Streaming content identity prevents same-size/mtime rewrites from
                // reusing a checkpoint, including on Windows. No segment buffering.
                crate::hash_file(&path, 4usize * 1024 * 1024 * 1024)?.1
            } else {
                hash(&read_file(&path, 64 * 1024 * 1024)?.0)
            };
            hashes.insert(name, digest);
        }
        Ok(Self { stamps, hashes })
    }
}

struct AttemptGuard {
    root: PathBuf,
    id: String,
}
impl Drop for AttemptGuard {
    fn drop(&mut self) {
        // Created only AFTER this worker owns the destination and began a job.
        // A competing/refused worker must never terminalize another attempt.
        if let Ok(mut db) = crate::lifecycle::index(&self.root) {
            let _ = db.finish(&self.id, JobState::Failed, None);
        }
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
            if ![
                "writer.lock",
                "objects",
                "catalogs",
                "projections",
                "lifecycle.sqlite3",
                "lifecycle.sqlite3-journal",
            ]
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
        if lock_path.symlink_metadata().is_ok() {
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
        // Reserve the bounded 64 MiB lifecycle database and rollback-journal overhead.
        // Counting existing bytes as well is conservative and avoids quota races.
        let used = total_size(&root, 0, &mut 0)?
            .checked_add(LIFECYCLE_RESERVE_BYTES)
            .ok_or("Storage quota overflow")?;
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
    if path.symlink_metadata().is_ok() {
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
    let mut store = Store::open(&destination, request.preset.limits().total_storage_bytes)?;
    let mut db = crate::lifecycle::index(&destination)?;
    let preparing = db
        .prepare(
            &request.request_id,
            &serde_json::to_string(request).map_err(|_| "Cannot encode private request")?,
            request.resume,
        )
        .map_err(|_| "Job exists or preparation/resume refused")?;
    let _attempt = AttemptGuard {
        root: destination.clone(),
        id: request.request_id.clone(),
    };
    progress(preparing.completed, preparing.total)?;
    let source = SourceState::capture_checked(&root, &inspection.metadata, || {
        if db
            .status(&request.request_id)
            .map_err(|_| "Cannot read job state")?
            .state
            != JobState::Preparing
        {
            return Err("Extraction cancelled during fingerprinting");
        }
        Ok(())
    })?;
    if source.hashes.get(".build.info") != Some(&inspection.build_info_sha256) {
        return Err("Installation changed after inspection; candidate refused");
    }
    let source_sha256 = hash(&crate::bounded_json(
        &(&root, &source.stamps, &source.hashes),
        MAX_MANIFEST_BYTES,
    )?);
    let selection_sha256 = hash(&crate::bounded_json(
        &(
            &request.product,
            &request.tables,
            selected
                .iter()
                .map(|(locale, _, table)| (locale, &table.name, table.file_data_id))
                .collect::<Vec<_>>(),
        ),
        MAX_MANIFEST_BYTES,
    )?);
    let binding = CheckpointBinding {
        format_version: 1,
        source_sha256,
        extractor_sha256: env!("NAVZEROTH_LOCAL_DATA_PROFILE_SHA256").into(),
        selection_sha256,
        preset: request.preset,
        total: selected.len() as u32,
    };
    let empty = Checkpoint {
        format_version: 1,
        binding,
        tables: Vec::new(),
        gaps: Vec::new(),
    };
    let checkpoint = db
        .begin(
            &request.request_id,
            &serde_json::to_string(request).map_err(|_| "Cannot encode private request")?,
            &empty,
            request.resume,
        )
        .map_err(|_| "Job exists, capacity reached or source/profile changed; resume refused")?;
    checkpoint.validate()?;
    for (index, row) in checkpoint.tables.iter().enumerate() {
        let (locale, _, candidate) = &selected[index];
        if row.table != candidate.name
            || row.locale.as_ref() != Some(locale)
            || row.file_data_id != candidate.file_data_id
        {
            return Err("Checkpoint does not match selected table order");
        }
    }
    verify_checkpoint_objects(&destination, &checkpoint, || {
        if db
            .status(&request.request_id)
            .map_err(|_| "Cannot read job state")?
            .state
            != JobState::Running
        {
            return Err("Extraction cancelled while verifying checkpoint");
        }
        Ok(())
    })?;
    let binding = checkpoint.binding.clone();
    let resumed = checkpoint.tables.len();
    let mut current_locale = selected[0].0.clone();
    let mut archive = Some(Archive::open(
        &root,
        &request.product,
        selected[0].1,
        inspection.metadata.build,
    )?);
    let archive_statistics = Some(archive.as_ref().unwrap().statistics()?);
    let mut inventory = checkpoint.tables;
    let total = selected.len() as u32;
    progress(resumed as u32, total)?;
    let mut gaps = vec![
        "research_only_no_playable_availability_evidence".into(),
        "hotfix_state_unknown".into(),
        "media_not_extracted".into(),
        "no_gameplay_projection".into(),
    ];
    if !request.tables.is_empty() {
        gaps.push("explicit_table_subset".into());
    }
    gaps.extend(checkpoint.gaps);
    for (index, (locale, mask, candidate)) in selected.into_iter().enumerate().skip(resumed) {
        if db
            .status(&request.request_id)
            .map_err(|_| "Cannot read job state")?
            .state
            != JobState::Running
        {
            return Err("Extraction cancelled");
        }
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
            Ok(logical) => {
                // A partial file keeps only its verified plain frames; its retained
                // form lists the encrypted ranges as gaps and holds no bytes for them.
                let (input, partial) = match &logical {
                    Logical::Whole(bytes) => {
                        entry.retained_sha256 = Some(store.put("objects", bytes)?);
                        (tables::Input::whole(bytes), None)
                    }
                    Logical::Partial(file) => {
                        entry.retained_sha256 = Some(store.put("objects", &file.retained_bytes())?);
                        (file.input(), Some(file))
                    }
                };
                entry.encoded_bytes = Some(input.len() as u64);
                match tables::header_of(&input) {
                    Err(tables::UNAVAILABLE_RANGE) => {
                        entry.disposition = InventoryDisposition::UnavailableEncrypted;
                        entry.reason = UNAVAILABLE_METADATA.into();
                    }
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
                                Ok(definition) => match tables::decode_input(&input, &definition) {
                                    Err(tables::UNAVAILABLE_RANGE) => {
                                        entry.disposition =
                                            InventoryDisposition::UnavailableEncrypted;
                                        entry.reason = UNAVAILABLE_METADATA.into();
                                    }
                                    Err(
                                        reason @ (tables::STRING_IN_UNAVAILABLE_SECTION
                                        | tables::COPY_SOURCE_IN_UNAVAILABLE_SECTION),
                                    ) => {
                                        entry.disposition =
                                            InventoryDisposition::UnavailableEncrypted;
                                        entry.reason = reason.into();
                                    }
                                    Err(reason) => entry.reason = reason.into(),
                                    Ok(mut decoded) => {
                                        if let Some(file) = partial {
                                            decoded
                                                .gaps
                                                .extend(file.unavailable().iter().map(|r| r.gap()));
                                            decoded.gaps.sort();
                                        }
                                        let output =
                                            crate::bounded_json(&decoded, MAX_DECODED_BYTES)?;
                                        if output.len() > MAX_DECODED_BYTES {
                                            return Err("Decoded table encoding exceeds capacity");
                                        }
                                        entry.decoded_sha256 = Some(store.put("objects", &output)?);
                                        entry.disposition = InventoryDisposition::Decoded;
                                        entry.reason = if partial.is_some() {
                                            "research_decoded_from_plain_frames"
                                        } else if decoded.gaps.is_empty() {
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
                if let Some(file) = partial
                    && entry.disposition != InventoryDisposition::Decoded
                {
                    record_undecoded_partial(&mut entry, file, &mut gaps);
                }
            }
            Err(error) => {
                let (disposition, reason) = read_failure_disposition(error);
                entry.disposition = disposition;
                entry.reason = reason.into();
            }
        }
        inventory.push(entry);
        gaps.sort();
        gaps.dedup();
        db.checkpoint(
            &request.request_id,
            &Checkpoint {
                format_version: 1,
                binding: binding.clone(),
                tables: inventory.clone(),
                gaps: gaps.clone(),
            },
        )
        .map_err(|_| "Checkpoint write refused or extraction cancelled")?;
        progress(index as u32 + 1, total)?;
    }
    drop(archive);
    let after = SourceState::capture_checked(&root, &inspection.metadata, || {
        if db
            .status(&request.request_id)
            .map_err(|_| "Cannot read job state")?
            .state
            != JobState::Running
        {
            return Err("Extraction cancelled during fingerprinting");
        }
        Ok(())
    })?;
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
    if db
        .status(&request.request_id)
        .map_err(|_| "Cannot read job state")?
        .state
        != JobState::Running
    {
        return Err("Extraction cancelled");
    }
    let digest = store.put("catalogs", &bytes)?;
    let summary = ExtractionSummary {
        format_version: 1,
        catalog_sha256: digest,
        build: manifest.installation.build,
        tables_processed: total,
        tables_decoded: decoded,
        preset: request.preset,
        actual_workers: 1,
        gameplay_projection_available: false,
        activated: false,
    };
    let status = db
        .finish(
            &request.request_id,
            JobState::Complete,
            Some(summary.clone()),
        )
        .map_err(|_| "Cannot complete extraction job")?;
    if status.state != JobState::Complete {
        return Err("Extraction cancelled before completion");
    }
    Ok(summary)
}

fn verify_checkpoint_objects(
    destination: &Path,
    checkpoint: &Checkpoint,
    mut check: impl FnMut() -> Result<()>,
) -> Result<()> {
    for row in &checkpoint.tables {
        check()?;
        for digest in [&row.retained_sha256, &row.decoded_sha256]
            .into_iter()
            .flatten()
        {
            check()?;
            let (_, actual) =
                crate::hash_file(&destination.join("objects").join(digest), MAX_DECODED_BYTES)?;
            if actual != *digest {
                return Err("Checkpoint object hash mismatch");
            }
        }
    }
    Ok(())
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

/// Reason suffix of a partially read table that was not decoded: its retained
/// object is a partial container, not the client file.
const PARTIAL_SUFFIX: &str = "partial_plain_frames";

/// A partially read table that was not decoded keeps its encrypted ranges as
/// catalog gaps, and its reason says that only plain frames were read.
fn record_undecoded_partial(
    entry: &mut TableInventory,
    file: &PartialFile,
    gaps: &mut Vec<String>,
) {
    if entry.reason != UNAVAILABLE_METADATA {
        entry.reason = format!("{}:{PARTIAL_SUFFIX}", entry.reason);
    }
    gaps.extend(
        file.unavailable()
            .iter()
            .map(|r| format!("{}:{}", entry.table, r.gap())),
    );
}

/// Catalog disposition and reason for a failed strict logical read. Causes stay
/// distinct: encryption is never reported as corruption and an unexplained short
/// read is never asserted to be corruption.
fn read_failure_disposition(error: ReadFailure) -> (InventoryDisposition, &'static str) {
    match error {
        ReadFailure::Missing => (
            InventoryDisposition::NotLocallyPresent,
            "not_locally_present",
        ),
        ReadFailure::Encrypted => (
            InventoryDisposition::UnavailableEncrypted,
            "unavailable_encrypted",
        ),
        ReadFailure::EncryptedFrames => (
            InventoryDisposition::UnavailableEncrypted,
            "unavailable_encrypted_frames",
        ),
        ReadFailure::Unsupported => (
            InventoryDisposition::UnavailableEncrypted,
            "unsupported_archive_encoding_not_retained",
        ),
        ReadFailure::Unavailable => (
            InventoryDisposition::UnavailableEncrypted,
            "local_archive_read_unavailable",
        ),
        ReadFailure::ShortRead => (
            InventoryDisposition::UnavailableEncrypted,
            "archive_short_read_unexplained",
        ),
        ReadFailure::Truncated => (
            InventoryDisposition::NotLocallyPresent,
            "archive_span_incomplete",
        ),
        ReadFailure::Capacity => (
            InventoryDisposition::ExcludedByPolicy,
            "logical_file_or_allocation_exceeds_capacity_not_retained",
        ),
        ReadFailure::Invalid => (
            InventoryDisposition::InvalidCorrupt,
            "archive_verification_failed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_failures_keep_distinct_dispositions_and_reasons() {
        use InventoryDisposition::*;
        let cases = [
            (
                ReadFailure::Missing,
                NotLocallyPresent,
                "not_locally_present",
            ),
            (
                ReadFailure::Encrypted,
                UnavailableEncrypted,
                "unavailable_encrypted",
            ),
            (
                ReadFailure::EncryptedFrames,
                UnavailableEncrypted,
                "unavailable_encrypted_frames",
            ),
            (
                ReadFailure::Unsupported,
                UnavailableEncrypted,
                "unsupported_archive_encoding_not_retained",
            ),
            (
                ReadFailure::Unavailable,
                UnavailableEncrypted,
                "local_archive_read_unavailable",
            ),
            (
                ReadFailure::ShortRead,
                UnavailableEncrypted,
                "archive_short_read_unexplained",
            ),
            (
                ReadFailure::Truncated,
                NotLocallyPresent,
                "archive_span_incomplete",
            ),
            (
                ReadFailure::Capacity,
                ExcludedByPolicy,
                "logical_file_or_allocation_exceeds_capacity_not_retained",
            ),
            (
                ReadFailure::Invalid,
                InvalidCorrupt,
                "archive_verification_failed",
            ),
        ];
        let mut reasons = std::collections::BTreeSet::new();
        for (failure, disposition, reason) in cases {
            assert_eq!(read_failure_disposition(failure), (disposition, reason));
            assert!(reasons.insert(reason), "{reason} reused");
        }
        // Encryption, partial or whole, and incomplete or unexplained short reads
        // are never reported as corruption.
        for failure in [
            ReadFailure::Encrypted,
            ReadFailure::EncryptedFrames,
            ReadFailure::Truncated,
            ReadFailure::ShortRead,
        ] {
            assert_ne!(read_failure_disposition(failure).0, InvalidCorrupt);
        }
    }
    #[test]
    fn undecoded_partial_tables_keep_encrypted_ranges_and_say_partial() {
        use crate::partial::FrameLayout;
        let layout = [
            FrameLayout {
                start: 0,
                length: 8,
                encrypted: false,
            },
            FrameLayout {
                start: 8,
                length: 4,
                encrypted: true,
            },
        ];
        let file = PartialFile::assemble(12, &layout, |offset, length| {
            assert!(offset + length <= 8, "reader asked for encrypted bytes");
            Ok::<_, ()>(vec![0; length])
        })
        .unwrap();
        let entry = |disposition, reason: &str| TableInventory {
            table: "Synthetic".into(),
            locale: None,
            file_data_id: Some(1),
            disposition,
            reason: reason.into(),
            retained_sha256: None,
            decoded_sha256: None,
            encoded_bytes: None,
            records: None,
            layout_hash: None,
        };
        for (disposition, reason, expected) in [
            (
                InventoryDisposition::InvalidCorrupt,
                "table_hash_mismatch",
                "table_hash_mismatch:partial_plain_frames",
            ),
            (
                InventoryDisposition::DefinitionMissing,
                "definition_missing",
                "definition_missing:partial_plain_frames",
            ),
            (
                InventoryDisposition::RetainedUnsupported,
                "cyclic_or_missing_copy_source",
                "cyclic_or_missing_copy_source:partial_plain_frames",
            ),
            (
                InventoryDisposition::UnavailableEncrypted,
                UNAVAILABLE_METADATA,
                UNAVAILABLE_METADATA,
            ),
        ] {
            let mut table = entry(disposition, reason);
            let mut gaps = Vec::new();
            record_undecoded_partial(&mut table, &file, &mut gaps);
            assert_eq!(table.reason, expected);
            assert_eq!(table.disposition, disposition);
            assert_eq!(
                gaps,
                vec!["Synthetic:unavailable_encrypted_frames:1-1:bytes:8+4".to_string()]
            );
        }
    }
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
        let mut store = Store::open(&root, LIFECYCLE_RESERVE_BYTES + 8).unwrap();
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

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn reused_objects_refuse_corruption_and_honor_cancellation_before_reading() {
        let root =
            std::env::temp_dir().join(format!("nav-checkpoint-object-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("objects")).unwrap();
        let digest = hash(b"original");
        let checkpoint = Checkpoint {
            format_version: 1,
            binding: CheckpointBinding {
                format_version: 1,
                source_sha256: "a".repeat(64),
                extractor_sha256: "b".repeat(64),
                selection_sha256: "c".repeat(64),
                preset: azeroth_core::local_data::PerformancePreset::Small,
                total: 1,
            },
            tables: vec![TableInventory {
                table: "Map".into(),
                locale: Some("enUS".into()),
                file_data_id: Some(1),
                disposition: InventoryDisposition::RetainedUnsupported,
                reason: "synthetic".into(),
                retained_sha256: Some(digest.clone()),
                decoded_sha256: None,
                encoded_bytes: Some(8),
                records: None,
                layout_hash: None,
            }],
            gaps: vec![],
        };
        fs::write(root.join("objects").join(&digest), b"tampered").unwrap();
        assert_eq!(
            verify_checkpoint_objects(&root, &checkpoint, || Ok(())),
            Err("Checkpoint object hash mismatch")
        );
        fs::write(root.join("objects").join(&digest), b"original").unwrap();
        verify_checkpoint_objects(&root, &checkpoint, || Ok(())).unwrap();
        fs::remove_file(root.join("objects").join(&digest)).unwrap();
        assert_eq!(
            verify_checkpoint_objects(&root, &checkpoint, || Err("cancelled")),
            Err("cancelled")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn segment_fingerprint_detects_same_length_bytes_with_restored_mtime() {
        let root =
            std::env::temp_dir().join(format!("nav-source-fingerprint-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join("Data/data")).unwrap();
        let metadata = azeroth_core::local_data::InstallationMetadata {
            product: "wow_classic_beta".into(),
            version: "1.60.1.70450".into(),
            build: 70450,
            build_key: "a".repeat(32),
            cdn_key: "b".repeat(32),
            declared_locales: vec!["enUS".into()],
        };
        fs::write(root.join(".build.info"), b"synthetic").unwrap();
        for key in [&metadata.build_key, &metadata.cdn_key] {
            let dir = root.join("Data/config").join(&key[..2]).join(&key[2..4]);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(key), b"synthetic").unwrap();
        }
        let segment = root.join("Data/data/data.000");
        fs::write(&segment, b"first").unwrap();
        let modified = fs::metadata(&segment).unwrap().modified().unwrap();
        let before = SourceState::capture(&root, &metadata).unwrap();
        fs::write(&segment, b"other").unwrap();
        File::options()
            .write(true)
            .open(&segment)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        let after = SourceState::capture(&root, &metadata).unwrap();
        assert_ne!(before.hashes, after.hashes);
        fs::remove_dir_all(root).unwrap();
    }
}
