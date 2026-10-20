//! Deterministic local reference packs. Files are bounded data, never executable inputs.
mod bounded_file;
pub mod v2;
use azeroth_core::selection_path::{PrefixRefusal, check_prefix};
use azeroth_core::{
    contract::{Envelope, MAX_DATASET_BYTES, ReferenceDataset, decode_envelope},
    coverage::content_report,
    sources::{Redistribution, SourceRecord, SourceRegistry},
    types::{AssetPath, DataContext, DatasetRevision, Sha256Hex, SourceId},
};
use image::{ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_SOURCES_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_ASSET_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_DATASETS: usize = 128;
pub const MAX_ASSETS: usize = 2048;
const MAX_DIMENSION: u32 = 8192;
const MAX_DECODE_ALLOC: u64 = 256 * 1024 * 1024;
const MAX_TREE_ENTRIES: usize = 8192;
type Res<T> = Result<T, &'static str>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sources {
    schema_version: u32,
    sources: Vec<SourceRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    path: AssetPath,
    bytes: u64,
    sha256: Sha256Hex,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage {
    report_version: u32,
    report_sha256: Sha256Hex,
    gaps: u32,
    collections: Vec<Count>,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Count {
    collection: String,
    records: u32,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    context: DataContext,
    fixture: bool,
    revision: DatasetRevision,
    dataset: FileRecord,
    assets: Vec<FileRecord>,
    coverage: Coverage,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    generator_version: String,
    purpose: String,
    sources: FileRecord,
    entries: Vec<Entry>,
    distribution_eligible: bool,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub report_version: u32,
    pub datasets: usize,
    pub fixtures: usize,
    pub files: usize,
    pub total_bytes: usize,
    pub coverage_gaps: u64,
    pub distribution_eligible: bool,
}

fn hash(bytes: &[u8]) -> Sha256Hex {
    // A SHA256 encoder always produces exactly64 lowercase hexadecimal characters.
    Sha256Hex::new(format!("{:x}", Sha256::digest(bytes))).expect("SHA256 encoding")
}
fn canonical<T: Serialize>(value: &T) -> Res<Vec<u8>> {
    let value = serde_json::to_value(value).map_err(|_| "Pack data cannot be serialized")?;
    let mut bytes =
        serde_json::to_vec_pretty(&value).map_err(|_| "Pack data cannot be serialized")?;
    bytes.push(b'\n');
    Ok(bytes)
}
/// Existing exchange domain types intentionally ignore some unknown fields. The
/// pack subset refuses them rather than silently losing or redistributing data.
fn known_fields(input: &Value, typed: &Value) -> Res<()> {
    match (input, typed) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                let expected = b
                    .get(key)
                    .ok_or("Untyped or noncanonical optional fields are unsupported in packs")?;
                known_fields(value, expected)?;
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (a, b) in a.iter().zip(b) {
                known_fields(a, b)?;
            }
        }
        _ => {}
    }
    Ok(())
}
/// Refuse duplicate JSON members before typed decoding (including nested maps).
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique-key JSON data")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|v| Unique(Value::Number(v)))
                    .ok_or_else(|| E::custom("Invalid number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.to_owned())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<Unique>()? {
                    values.push(v.0);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("Duplicate JSON member"));
                    }
                    values.insert(key, a.next_value::<Unique>()?.0);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn json(bytes: &[u8]) -> Res<Value> {
    serde_json::from_slice::<Unique>(bytes)
        .map(|v| v.0)
        .map_err(|_| "Invalid or duplicate-key pack JSON")
}
fn typed<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Res<T> {
    let value = json(bytes)?;
    let parsed: T =
        serde_json::from_value(value.clone()).map_err(|_| "Invalid typed pack document")?;
    known_fields(
        &value,
        &serde_json::to_value(&parsed).map_err(|_| "Invalid typed pack document")?,
    )?;
    Ok(parsed)
}
fn path_checked(path: &Path, directory: bool) -> Res<()> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| "Current directory unavailable")?
            .join(path)
    };
    check_prefix(&absolute).map_err(|refusal| match refusal {
        PrefixRefusal::DeviceNamespace => "Device namespace paths are unsupported",
        PrefixRefusal::DriveRelative => "Drive-relative paths are unsupported",
    })?;
    let mut prefix = PathBuf::new();
    for part in absolute.components() {
        if matches!(part, Component::ParentDir) {
            return Err("Parent traversal is unsupported");
        }
        prefix.push(part.as_os_str());
        // A bare Windows prefix (`C:`, `\\?\C:`, `\\server\share`) names a drive
        // or volume, not a directory entry; a verbatim one cannot be inspected at
        // all. Its root is inspected at the next component, so skip only the prefix.
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        let meta =
            fs::symlink_metadata(&prefix).map_err(|_| "Selected pack path cannot be inspected")?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Symlink or reparse paths are unsupported");
            }
        }
        if meta.file_type().is_symlink() {
            return Err("Symlink paths are unsupported");
        }
        if prefix != absolute && !meta.is_dir() {
            return Err("Pack path ancestor must be a directory");
        }
    }
    let meta = fs::symlink_metadata(path).map_err(|_| "Selected pack path cannot be inspected")?;
    if (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
        return Err("Pack path has the wrong file kind");
    }
    Ok(())
}
fn root_join(root: &Path, path: &AssetPath) -> PathBuf {
    path.segments().fold(root.to_owned(), |p, s| p.join(s))
}
struct Budget {
    bytes: usize,
    limit: usize,
}
impl Budget {
    fn read(&mut self, path: &Path, limit: usize) -> Res<Vec<u8>> {
        let available = self.limit.saturating_sub(self.bytes);
        let bytes = bounded_file::read(path, limit.min(available))?;
        self.bytes += bytes.len();
        Ok(bytes)
    }
}
fn record(path: impl Into<String>, bytes: &[u8]) -> Res<FileRecord> {
    Ok(FileRecord {
        path: AssetPath::new(path.into()).map_err(|_| "Invalid pack asset path")?,
        bytes: bytes.len() as u64,
        sha256: hash(bytes),
    })
}
fn read_sources(bytes: &[u8]) -> Res<SourceRegistry> {
    let sources: Sources = typed(bytes)?;
    if sources.schema_version != 1 || sources.sources.len() > 1024 {
        return Err("Unsupported source registry version or size");
    }
    SourceRegistry::try_from(sources.sources)
        .map_err(|_| "Invalid or duplicate source registration")
}
fn source_bytes(registry: &SourceRegistry) -> Res<Vec<u8>> {
    canonical(&Sources {
        schema_version: 1,
        sources: registry.clone().into(),
    })
}
fn dataset(bytes: &[u8]) -> Res<Envelope<ReferenceDataset>> {
    let raw = json(bytes)?;
    let text = std::str::from_utf8(bytes).map_err(|_| "Dataset must be UTF-8")?;
    let envelope =
        decode_envelope::<ReferenceDataset>(text).map_err(|_| "Invalid reference dataset")?;
    if !envelope.extensions.is_empty() {
        return Err("Exchange extensions are unsupported in local packs");
    }
    let mut comparison = serde_json::to_value(&envelope).map_err(|_| "Invalid dataset")?;
    // Schema1 defines this additive typed default; serde omits its empty array.
    // Restore only its comparison shape. The input and canonical envelope stay
    // unchanged, and generic source/manifest/unknown-field checks stay strict.
    if envelope.payload.map_embeddings.is_empty()
        && raw
            .get("payload")
            .and_then(|payload| payload.get("map_embeddings"))
            .is_some_and(|value| matches!(value, Value::Array(entries) if entries.is_empty()))
    {
        comparison
            .get_mut("payload")
            .and_then(Value::as_object_mut)
            .ok_or("Invalid dataset comparison shape")?
            .insert("map_embeddings".into(), Value::Array(Vec::new()));
    }
    known_fields(&raw, &comparison)?;
    if !content_report(&envelope).passes {
        return Err("Reference dataset has validation errors");
    }
    Ok(envelope)
}
fn context(data: &Envelope<ReferenceDataset>) -> DataContext {
    DataContext {
        build: data.game.build_key(),
        namespace: data.namespace,
    }
}
fn identity(data: &Envelope<ReferenceDataset>) -> (DataContext, bool, DatasetRevision) {
    (context(data), data.fixture, data.payload.revision.clone())
}
fn coverage(data: &Envelope<ReferenceDataset>) -> Res<Coverage> {
    let report = content_report(data);
    let counts = report
        .coverage
        .as_ref()
        .ok_or("Missing dataset coverage")?
        .collections
        .iter()
        .map(|(kind, c)| Count {
            collection: serde_json::to_value(kind)
                .expect("collection name")
                .as_str()
                .expect("collection string")
                .to_owned(),
            records: c.total as u32,
        })
        .collect();
    Ok(Coverage {
        report_version: report.report_version,
        report_sha256: hash(&canonical(&report)?),
        gaps: report.gaps.len() as u32,
        collections: counts,
    })
}
fn cited(data: &Envelope<ReferenceDataset>) -> BTreeSet<SourceId> {
    data.payload
        .evidence
        .iter()
        .map(|e| e.source.clone())
        .chain(data.payload.map_images.iter().map(|im| im.source.clone()))
        .collect()
}
fn distribution(data: &Envelope<ReferenceDataset>, sources: &SourceRegistry) -> Res<bool> {
    let ids = cited(data);
    let mut permitted = !ids.is_empty();
    for id in ids {
        let source = sources
            .ensure_registered(&id)
            .map_err(|_| "Dataset cites an unregistered source")?;
        permitted &= source.redistribution.value() == Some(&Redistribution::Permitted)
            && source.license.value().is_some()
            && source.attribution.value().is_some()
            && source.allowed_fields.value().is_some()
            && source.transformations.value().is_some()
            && source
                .editions
                .value()
                .is_some_and(|editions| editions.contains(&data.game.edition));
    }
    Ok(permitted)
}
fn layers(data: &Envelope<ReferenceDataset>) -> Res<BTreeMap<AssetPath, (Sha256Hex, u32, u32)>> {
    layers_data(&data.payload)
}
fn layers_data(data: &ReferenceDataset) -> Res<BTreeMap<AssetPath, (Sha256Hex, u32, u32)>> {
    let mut layers = BTreeMap::new();
    for layer in data.map_images.iter().flat_map(|im| &im.layers) {
        let spec = (
            layer.sha256.clone(),
            layer.width_px.get(),
            layer.height_px.get(),
        );
        if let Some(previous) = layers.insert(layer.file.clone(), spec.clone())
            && previous != spec
        {
            return Err("Conflicting asset declarations");
        }
    }
    if layers.len() > MAX_ASSETS {
        return Err("Too many pack assets");
    }
    Ok(layers)
}
fn png(bytes: &[u8], spec: &(Sha256Hex, u32, u32)) -> Res<()> {
    if hash(bytes) != spec.0 {
        return Err("Image hash differs from dataset");
    }
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| "Image is not a PNG within decoding limits")?;
    if decoded.width() != spec.1 || decoded.height() != spec.2 {
        return Err("Image dimensions differ from dataset");
    }
    Ok(())
}
fn expected_tree(files: &BTreeSet<AssetPath>) -> Res<BTreeSet<AssetPath>> {
    let mut dirs = BTreeSet::new();
    for path in files {
        let parts: Vec<_> = path.segments().collect();
        for end in 1..parts.len() {
            dirs.insert(
                AssetPath::new(parts[..end].join("/")).map_err(|_| "Invalid pack directory")?,
            );
            if files.len() + dirs.len() > MAX_TREE_ENTRIES {
                return Err("Pack file tree exceeds limit");
            }
        }
    }
    if files.len() + dirs.len() > MAX_TREE_ENTRIES {
        return Err("Pack file tree exceeds limit");
    }
    Ok(dirs)
}
fn exact_tree(root: &Path, files: &BTreeSet<AssetPath>) -> Res<()> {
    let dirs = expected_tree(files)?;
    let mut pending = vec![(root.to_owned(), String::new())];
    let mut found = BTreeSet::new();
    let mut seen = 0;
    while let Some((dir, relative)) = pending.pop() {
        path_checked(&dir, true)?;
        for child in fs::read_dir(dir).map_err(|_| "Pack tree cannot be read")? {
            seen += 1;
            if seen > MAX_TREE_ENTRIES {
                return Err("Pack tree exceeds limit");
            }
            let child = child.map_err(|_| "Pack tree cannot be read")?;
            let name = child
                .file_name()
                .into_string()
                .map_err(|_| "Invalid pack file name")?;
            let rel = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            let path = AssetPath::new(rel.clone()).map_err(|_| "Invalid pack file name")?;
            let meta =
                fs::symlink_metadata(child.path()).map_err(|_| "Pack entry cannot be read")?;
            if meta.file_type().is_symlink() {
                return Err("Pack symlink rejected");
            }
            if meta.is_dir() && dirs.contains(&path) {
                pending.push((child.path(), rel));
            } else if meta.is_file() && files.contains(&path) {
                path_checked(&child.path(), false)?;
                found.insert(path);
            } else {
                return Err("Unexpected file, directory or file kind in pack");
            }
        }
    }
    if found != *files {
        return Err("Pack files are missing");
    }
    Ok(())
}
fn check_record(root: &Path, item: &FileRecord, limit: usize, budget: &mut Budget) -> Res<Vec<u8>> {
    if item.bytes > limit as u64 {
        return Err("Manifest file size exceeds limit");
    }
    let bytes = budget.read(&root_join(root, &item.path), limit)?;
    if bytes.len() as u64 != item.bytes || hash(&bytes) != item.sha256 {
        return Err("Pack size or digest mismatch");
    }
    Ok(bytes)
}
pub fn verify(root: &Path) -> Res<Report> {
    path_checked(root, true)?;
    let mut budget = Budget {
        bytes: 0,
        limit: MAX_TOTAL_BYTES,
    };
    let manifest_bytes = budget.read(&root.join("manifest.json"), MAX_MANIFEST_BYTES)?;
    if json(&manifest_bytes)?
        .get("schema_version")
        .and_then(Value::as_u64)
        == Some(2)
    {
        return v2::verify(root);
    }
    let manifest: Manifest = typed(&manifest_bytes)?;
    if manifest.schema_version != 1
        || manifest.purpose != "local_reference"
        || manifest.generator_version.is_empty()
        || manifest.generator_version.len() > 64
        || !manifest
            .generator_version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
    {
        return Err("Unsupported pack header");
    }
    if manifest.entries.is_empty()
        || manifest.entries.len() > MAX_DATASETS
        || manifest.sources.path.as_str() != "sources.json"
    {
        return Err("Invalid manifest entries or source path");
    }
    let mut files = BTreeSet::from([
        AssetPath::new("manifest.json").expect("constant"),
        manifest.sources.path.clone(),
    ]);
    let mut asset_count = 0;
    for entry in &manifest.entries {
        if entry.coverage.collections.len() != 11 {
            return Err("Invalid coverage collection count");
        }
        if !files.insert(entry.dataset.path.clone()) {
            return Err("Duplicate pack file");
        }
        asset_count += entry.assets.len();
        if asset_count > MAX_ASSETS {
            return Err("Too many pack assets");
        }
        for asset in &entry.assets {
            if !files.insert(asset.path.clone()) {
                return Err("Duplicate pack asset");
            }
        }
    }
    exact_tree(root, &files)?;
    let sources_data = check_record(root, &manifest.sources, MAX_SOURCES_BYTES, &mut budget)?;
    let sources = read_sources(&sources_data)?;
    if source_bytes(&sources)? != sources_data {
        return Err("Source registry is not canonical");
    }
    let mut previous = None;
    let mut permitted = true;
    let mut gaps = 0;
    let mut fixtures = 0;
    for (index, entry) in manifest.entries.iter().enumerate() {
        let prefix = format!("entries/{index:04}");
        if entry.dataset.path.as_str() != format!("{prefix}/dataset.json") {
            return Err("Dataset path differs from numbered identity");
        }
        let bytes = check_record(root, &entry.dataset, MAX_DATASET_BYTES, &mut budget)?;
        let data = dataset(&bytes)?;
        if canonical(&data)? != bytes {
            return Err("Dataset is not canonical");
        }
        let id = identity(&data);
        if previous.as_ref().is_some_and(|old| old >= &id)
            || id != (entry.context.clone(), entry.fixture, entry.revision.clone())
        {
            return Err("Dataset context, revision, order or duplicate identity mismatch");
        }
        previous = Some(id);
        if coverage(&data)? != entry.coverage {
            return Err("Coverage differs from dataset");
        }
        permitted &= distribution(&data, &sources)?;
        gaps += entry.coverage.gaps as u64;
        fixtures += usize::from(data.fixture);
        let declared = layers(&data)?;
        if declared.len() != entry.assets.len() {
            return Err("Asset metadata differs from dataset");
        }
        for ((path, spec), asset) in declared.into_iter().zip(&entry.assets) {
            if asset.path.as_str() != format!("{prefix}/{}", path.as_str()) {
                return Err("Asset path or ordering differs from dataset");
            }
            png(
                &check_record(root, asset, MAX_ASSET_BYTES, &mut budget)?,
                &spec,
            )?;
        }
    }
    if permitted != manifest.distribution_eligible {
        return Err("Distribution declaration differs from source policy");
    }
    Ok(Report {
        report_version: 1,
        datasets: manifest.entries.len(),
        fixtures,
        files: files.len(),
        total_bytes: budget.bytes,
        coverage_gaps: gaps,
        distribution_eligible: permitted,
    })
}
fn input_datasets(input: &Path, budget: &mut Budget) -> Res<Vec<Envelope<ReferenceDataset>>> {
    path_checked(input, true)?;
    let root = input.join("datasets");
    path_checked(&root, true)?;
    let mut datasets = BTreeMap::new();
    let mut count = 0;
    for child in fs::read_dir(root).map_err(|_| "Dataset directory cannot be read")? {
        count += 1;
        if count > MAX_DATASETS {
            return Err("Too many input dataset entries");
        }
        let path = child
            .map_err(|_| "Dataset directory cannot be read")?
            .path();
        if path.extension().is_none_or(|extension| extension != "json") {
            return Err("Only dataset JSON files are allowed in datasets");
        }
        let data = dataset(&budget.read(&path, MAX_DATASET_BYTES)?)?;
        if datasets.insert(identity(&data), data).is_some() {
            return Err("Duplicate dataset identity");
        }
    }
    if datasets.is_empty() {
        return Err("No input datasets");
    }
    Ok(datasets.into_values().collect())
}
static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);
struct Staging {
    root: PathBuf,
    limit: usize,
    bytes: Cell<usize>,
    files: RefCell<Vec<PathBuf>>,
    directories: RefCell<Vec<PathBuf>>,
}
impl Staging {
    fn directories(&self, path: &Path) -> Res<()> {
        let suffix = path
            .strip_prefix(&self.root)
            .map_err(|_| "Invalid staging directory")?;
        let mut next = self.root.clone();
        for segment in suffix.components() {
            if !matches!(segment, Component::Normal(_)) {
                return Err("Invalid staging directory");
            }
            next.push(segment.as_os_str());
            if self.directories.borrow().contains(&next) {
                path_checked(&next, true)?;
            } else {
                if self.directories.borrow().len() + self.files.borrow().len() >= MAX_TREE_ENTRIES {
                    return Err("Staging file tree exceeds limit");
                }
                fs::create_dir(&next).map_err(|_| "Staging directory collision")?;
                self.directories.borrow_mut().push(next.clone());
            }
        }
        Ok(())
    }
    fn write(&self, path: &Path, bytes: &[u8]) -> Res<()> {
        // Canonical serialization can expand inputs. Bound actual planned staging
        // bytes independently of the input/read budget before creating any file.
        if bytes.len() > self.limit.saturating_sub(self.bytes.get()) {
            return Err("Staging byte budget exceeded");
        }
        if self.directories.borrow().len() + self.files.borrow().len() >= MAX_TREE_ENTRIES {
            return Err("Staging file tree exceeds limit");
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| "Pack output cannot be created without overwrite")?;
        self.files.borrow_mut().push(path.to_owned());
        self.bytes.set(self.bytes.get() + bytes.len());
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Pack output cannot be written")
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        // Remove only paths exclusively created by this builder. Unknown concurrent
        // files survive; remove_dir refuses a nonempty directory rather than recurse.
        for file in self.files.get_mut().iter().rev() {
            let _ = fs::remove_file(file);
        }
        for directory in self.directories.get_mut().iter().rev() {
            let _ = fs::remove_dir(directory);
        }
    }
}
fn stage(output: &Path) -> Res<Staging> {
    stage_with_limit(output, MAX_TOTAL_BYTES)
}
fn stage_with_limit(output: &Path, limit: usize) -> Res<Staging> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    path_checked(parent, true)?;
    if output.file_name().is_none()
        || output
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err("Invalid output directory");
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err("Output already exists; no overwrite is permitted");
    }
    let nonce = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = parent.join(format!(".anav-pack-stage-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).map_err(|_| "Create-new pack staging failed")?;
    Ok(Staging {
        root: root.clone(),
        limit,
        bytes: Cell::new(0),
        files: RefCell::new(Vec::new()),
        directories: RefCell::new(vec![root]),
    })
}
/// Portable no-overwrite publication reserves a new output directory, creates
/// owned children exclusively, and writes manifest last. It is not an atomic
/// directory rename. A partially published directory cannot verify as a pack.
fn publish(stage: &Path, output: &Path, files: &BTreeSet<AssetPath>) -> Res<()> {
    publish_with_limit(stage, output, files, MAX_TOTAL_BYTES, MAX_ASSET_BYTES)
}
fn publish_with_limit(
    stage: &Path,
    output: &Path,
    files: &BTreeSet<AssetPath>,
    total: usize,
    file_limit: usize,
) -> Res<()> {
    fs::create_dir(output).map_err(|_| "Output already exists or cannot be reserved")?;
    let mut created_files = Vec::new();
    let mut created_dirs = vec![output.to_owned()];
    let result = (|| {
        let dirs = expected_tree(files)?;
        for dir in dirs {
            let path = root_join(output, &dir);
            fs::create_dir(&path).map_err(|_| "Output directory collision")?;
            created_dirs.push(path);
        }
        let mut budget = Budget {
            bytes: 0,
            limit: total,
        };
        for file in files
            .iter()
            .filter(|p| p.as_str() != "manifest.json")
            .chain(files.iter().filter(|p| p.as_str() == "manifest.json"))
        {
            let path = root_join(output, file);
            let mut target = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|_| "Output file collision")?;
            created_files.push(path);
            let bytes = budget.read(&root_join(stage, file), file_limit)?;
            target
                .write_all(&bytes)
                .and_then(|_| target.sync_all())
                .map_err(|_| "Output write failed")?;
        }
        Ok(())
    })();
    if result.is_err() {
        for path in created_files.iter().rev() {
            let _ = fs::remove_file(path);
        }
        for path in created_dirs.iter().rev() {
            let _ = fs::remove_dir(path);
        }
    }
    result
}
pub fn build(input: &Path, output: &Path) -> Res<Report> {
    let stage = stage(output)?;
    let mut budget = Budget {
        bytes: 0,
        limit: MAX_TOTAL_BYTES,
    };
    let datasets = input_datasets(input, &mut budget)?;
    let registry = read_sources(&budget.read(&input.join("sources.json"), MAX_SOURCES_BYTES)?)?;
    let source_bytes = source_bytes(&registry)?;
    if source_bytes.len() > MAX_SOURCES_BYTES {
        return Err("Canonical sources exceed size limit");
    }
    stage.write(&stage.root.join("sources.json"), &source_bytes)?;
    let mut entries = Vec::new();
    let mut permitted = true;
    let mut asset_count = 0;
    for (index, data) in datasets.into_iter().enumerate() {
        permitted &= distribution(&data, &registry)?;
        let prefix = format!("entries/{index:04}");
        let root = stage.root.join("entries").join(format!("{index:04}"));
        stage.directories(&root)?;
        let bytes = canonical(&data)?;
        if bytes.len() > MAX_DATASET_BYTES {
            return Err("Canonical dataset exceeds size limit");
        }
        stage.write(&root.join("dataset.json"), &bytes)?;
        let mut assets = Vec::new();
        for (path, spec) in layers(&data)? {
            asset_count += 1;
            if asset_count > MAX_ASSETS {
                return Err("Too many pack assets");
            }
            // Dataset metadata itself reserves dataset.json; assets cannot overwrite it.
            if path.as_str() == "dataset.json" {
                return Err("Asset conflicts with dataset metadata");
            }
            let bytes = budget.read(&root_join(input, &path), MAX_ASSET_BYTES)?;
            png(&bytes, &spec)?;
            let target = root_join(&root, &path);
            stage.directories(target.parent().ok_or("Invalid asset parent")?)?;
            stage.write(&target, &bytes)?;
            assets.push(record(format!("{prefix}/{}", path.as_str()), &bytes)?);
        }
        entries.push(Entry {
            context: context(&data),
            fixture: data.fixture,
            revision: data.payload.revision.clone(),
            dataset: record(format!("{prefix}/dataset.json"), &bytes)?,
            assets,
            coverage: coverage(&data)?,
        });
    }
    let manifest = Manifest {
        schema_version: 1,
        generator_version: env!("CARGO_PKG_VERSION").to_owned(),
        purpose: "local_reference".to_owned(),
        sources: record("sources.json", &source_bytes)?,
        entries,
        distribution_eligible: permitted,
    };
    let bytes = canonical(&manifest)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("Manifest exceeds size limit");
    }
    stage.write(&stage.root.join("manifest.json"), &bytes)?;
    let report = verify(&stage.root)?;
    let mut files = BTreeSet::from([
        AssetPath::new("manifest.json").expect("constant"),
        manifest.sources.path.clone(),
    ]);
    for entry in manifest.entries {
        files.insert(entry.dataset.path);
        files.extend(entry.assets.into_iter().map(|a| a.path));
    }
    publish(&stage.root, output, &files)?;
    Ok(report)
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    fn temporary() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "anav-pack-publication-{}-{}",
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    #[test]
    fn failed_publication_leaves_no_valid_pack_and_never_overwrites_existing_target() {
        let root = temporary();
        let stage = root.join("stage");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("a.json"), b"synthetic").unwrap();
        let files = BTreeSet::from([
            AssetPath::new("a.json").unwrap(),
            AssetPath::new("missing.json").unwrap(),
            AssetPath::new("manifest.json").unwrap(),
        ]);
        let output = root.join("output");
        assert!(publish(&stage, &output, &files).is_err());
        assert!(verify(&output).is_err());
        assert!(!output.exists());
        fs::create_dir(&output).unwrap();
        fs::write(output.join("unrelated-sentinel"), b"preserved").unwrap();
        assert!(publish(&stage, &output, &files).is_err());
        assert_eq!(
            fs::read(output.join("unrelated-sentinel")).unwrap(),
            b"preserved"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn manifest_directory_prefixes_are_bounded_before_filesystem_traversal() {
        let files = (0..500)
            .map(|n| AssetPath::new(format!("path{n}/{}image.png", "nested/".repeat(20))).unwrap())
            .collect();
        assert!(expected_tree(&files).is_err());
    }
    #[test]
    fn canonical_expansion_is_refused_before_staging_file_creation() {
        let root = temporary();
        let staging = stage(&root.join("output")).unwrap();
        let raw = br#"{"x":0}"#;
        let canonical_bytes = canonical(&json(raw).unwrap()).unwrap();
        assert!(canonical_bytes.len() > raw.len());
        staging.bytes.set(MAX_TOTAL_BYTES - raw.len());
        let target = staging.root.join("dataset.json");
        assert!(staging.write(&target, &canonical_bytes).is_err());
        assert!(!target.exists());
        assert!(staging.files.borrow().is_empty());
        assert_eq!(staging.bytes.get(), MAX_TOTAL_BYTES - raw.len());
        // At the exact boundary a smaller file may be created, but no subsequent
        // manifest/source/asset write may create another byte or even an empty file.
        staging.write(&target, raw).unwrap();
        assert_eq!(staging.bytes.get(), MAX_TOTAL_BYTES);
        let manifest = staging.root.join("manifest.json");
        assert!(staging.write(&manifest, b"x").is_err());
        assert!(!manifest.exists());
        drop(staging);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn staging_cleanup_preserves_unrelated_files_and_nonempty_directory() {
        let root = temporary();
        let output = root.join("output");
        let staging = stage(&output).unwrap();
        let path = staging.root.clone();
        staging.directories(&path.join("entries")).unwrap();
        staging
            .write(&path.join("entries/owned.json"), b"synthetic")
            .unwrap();
        fs::write(path.join("entries/unrelated-sentinel"), b"preserved").unwrap();
        drop(staging);
        assert!(!path.join("entries/owned.json").exists());
        assert_eq!(
            fs::read(path.join("entries/unrelated-sentinel")).unwrap(),
            b"preserved"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    fn temporary(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("anav-pack-path-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("pack")).unwrap();
        fs::write(root.join("pack/a.json"), b"{}").unwrap();
        root
    }
    #[test]
    fn absolute_paths_pass_and_parent_traversal_is_refused() {
        let root = temporary("absolute");
        path_checked(&root.join("pack"), true).unwrap();
        path_checked(&root.join("pack/a.json"), false).unwrap();
        assert_eq!(
            path_checked(&root.join("pack/a.json"), true),
            Err("Pack path has the wrong file kind")
        );
        assert_eq!(
            path_checked(&root.join("pack/../pack"), true),
            Err("Parent traversal is unsupported")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn symlinked_ancestors_stay_refused() {
        // The temp directory itself may sit under a symlink (macOS `/var`).
        let root = fs::canonicalize(temporary("symlink")).unwrap();
        std::os::unix::fs::symlink(root.join("pack"), root.join("link")).unwrap();
        assert_eq!(
            path_checked(&root.join("link/a.json"), false),
            Err("Symlink paths are unsupported")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn verbatim_prefixed_paths_are_inspected_from_their_root() {
        let root = temporary("verbatim");
        // `canonicalize` returns a `\\?\C:\...` verbatim path on Windows.
        let verbatim = fs::canonicalize(root.join("pack")).unwrap();
        assert!(matches!(
            verbatim.components().next(),
            Some(Component::Prefix(p)) if p.kind().is_verbatim()
        ));
        path_checked(&verbatim, true).unwrap();
        path_checked(&verbatim.join("a.json"), false).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn device_namespace_and_drive_relative_paths_are_refused() {
        for device in [r"\\.\pipe\x", r"\\.\C:\Windows", r"\\?\GLOBALROOT\Device\x"] {
            assert_eq!(
                path_checked(Path::new(device), true),
                Err("Device namespace paths are unsupported"),
                "{device}"
            );
        }
        let drive = std::env::current_dir().unwrap();
        // Drive-relative forms exist only for a drive-letter working directory.
        let Some(Component::Prefix(prefix)) = drive.components().next() else {
            panic!("current directory has no prefix");
        };
        if !matches!(prefix.kind(), std::path::Prefix::Disk(_)) {
            return;
        }
        let letter = prefix.as_os_str().to_str().unwrap().to_owned();
        for relative in [format!("{letter}Windows"), letter] {
            assert_eq!(
                path_checked(Path::new(&relative), true),
                Err("Drive-relative paths are unsupported"),
                "{relative}"
            );
        }
    }
}

#[cfg(test)]
mod dataset_default_tests {
    use super::*;

    #[test]
    fn actual_forever_reference_with_explicit_empty_embeddings_keeps_original_identity() {
        let bytes = include_bytes!("../../../content/forever/datasets/forever-atlas-r1.json");
        let parsed = dataset(bytes).expect("existing typed empty default must parse");
        let original =
            decode_envelope::<ReferenceDataset>(std::str::from_utf8(bytes).unwrap()).unwrap();
        assert_eq!(canonical(&parsed).unwrap(), canonical(&original).unwrap());
        assert_eq!(parsed.game.build.as_str(), "1.60.1.70205");
        assert_eq!(parsed.game.interface.as_str(), "16001");
        assert_eq!(
            parsed.namespace,
            azeroth_core::types::DatasetNamespace::Reference
        );
        assert!(!parsed.fixture);
        assert_eq!(parsed.payload.revision.as_str(), "forever-atlas-r1");
        assert!(parsed.payload.map_embeddings.is_empty());
        let report = content_report(&parsed);
        assert!(report.passes);
        assert!(report.errors.is_empty());
        assert_eq!(report.gaps.len(), 3);
    }
}
