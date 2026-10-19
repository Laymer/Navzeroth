//! `verify-world-data`: offline checks of generated `navzeroth-world-data` v1 shards
//! (docs/data/WORLD_DATA.md). Every shard is parsed and validated by
//! `azeroth_core::world_data`, shard sets are checked per folder and, when a run
//! manifest is present, every listed file is matched by hash, size and record count.
//! Nothing is fetched or executed; the report names relative paths, never content.
use azeroth_core::world_data::{self, Generator, MAX_SHARD_BYTES, Origin, RecordKind, Target};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const USAGE: &str = "verify-world-data --input <dir> [--manifest <file>]. \
    Validates every navzeroth-world-data shard below <dir> (skipping run-manifest.json, facts/ and dot-folders), \
    the shard set of each folder and the run manifest (default <dir>/run-manifest.json when present). \
    Prints a JSON summary; exit 0 only without errors. See docs/data/WORLD_DATA.md.";
pub const MANIFEST_FILE: &str = "run-manifest.json";
pub const MANIFEST_FORMAT: &str = "navzeroth-world-data-run";
pub const REPORT_FORMAT: &str = "navzeroth-world-data-verification";
const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILES: usize = 65_536;
const MAX_DEPTH: usize = 32;
/// Errors listed in the report; `error_count` keeps the full number.
pub const MAX_REPORTED_ERRORS: usize = 1000;

#[derive(Debug)]
pub struct Options {
    input: PathBuf,
    manifest: Option<PathBuf>,
}
impl Options {
    /// Exact option/value pairs, each at most once; values never start with `--`.
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let (pairs, remainder) = args.as_chunks::<2>();
        if !remainder.is_empty() || pairs.len() > 2 {
            return None;
        }
        let (mut input, mut manifest) = (None, None);
        for [flag, value] in pairs {
            if value.is_empty() || value.as_encoded_bytes().starts_with(b"--") {
                return None;
            }
            match flag.to_str()? {
                "--input" if input.is_none() => input = Some(PathBuf::from(value)),
                "--manifest" if manifest.is_none() => manifest = Some(PathBuf::from(value)),
                _ => return None,
            }
        }
        Some(Self {
            input: input?,
            manifest,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Folder {
    pub path: String,
    pub dataset: String,
    pub record_kind: RecordKind,
    pub origin_edition: String,
    pub origin_build: Option<String>,
    pub shards: usize,
    pub records: usize,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub format: &'static str,
    pub schema_version: u32,
    pub passes: bool,
    pub files: usize,
    pub records: usize,
    /// Records per dataset, summed over origins.
    pub datasets: BTreeMap<String, usize>,
    pub folders: Vec<Folder>,
    pub manifest_checked: bool,
    pub error_count: usize,
    pub errors: Vec<String>,
}

/// The envelope of one valid shard; records are counted, not retained.
struct Header {
    dataset: String,
    record_kind: RecordKind,
    origin: Origin,
    target: Target,
    confidence: azeroth_core::entities::Confidence,
    generator: Generator,
    index: u32,
    count: u32,
    records: usize,
}
struct Shard {
    path: String,
    folder: Vec<String>,
    name: String,
    sha256: String,
    bytes: u64,
    header: Option<Header>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    schema_version: u32,
    generators: Vec<GeneratorRun>,
    sources: Vec<SourceRun>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratorRun {
    name: String,
    status: RunStatus,
    files: Vec<FileEntry>,
}
#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RunStatus {
    Ok,
    Failed,
    Skipped,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileEntry {
    path: String,
    sha256: String,
    bytes: u64,
    records: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRun {
    url: String,
    status: SourceStatus,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    bytes: Option<u64>,
}
/// A word (`ok`, `cached`, `failed`, ...) or an HTTP status code.
#[derive(Deserialize)]
#[serde(untagged)]
enum SourceStatus {
    Word(String),
    Code(u16),
}

fn bounded(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}
/// Relative, `/`-separated, no empty, `.` or `..` component, no drive or backslash.
fn relative_path(s: &str) -> bool {
    bounded(s)
        && !s.contains(['\\', ':'])
        && s.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Walk<'a> {
    files: &'a mut Vec<(String, Vec<String>, String, PathBuf)>,
    errors: &'a mut Vec<String>,
}
impl Walk<'_> {
    fn dir(&mut self, path: &Path, rel: &mut Vec<String>) -> Result<(), &'static str> {
        if rel.len() > MAX_DEPTH {
            return Err("World-data folders nest too deeply");
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(path).map_err(|_| "Cannot list a world-data folder")? {
            entries.push(entry.map_err(|_| "Cannot list a world-data folder")?);
        }
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let shown = |rel: &[String], name: &str| {
                rel.iter()
                    .map(String::as_str)
                    .chain([name])
                    .collect::<Vec<_>>()
                    .join("/")
            };
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                self.errors.push(format!(
                    "{}: a file name is not UTF-8",
                    if rel.is_empty() {
                        ".".to_owned()
                    } else {
                        rel.join("/")
                    }
                ));
                continue;
            };
            let kind = entry
                .file_type()
                .map_err(|_| "Cannot inspect a world-data entry")?;
            if kind.is_symlink() {
                self.errors
                    .push(format!("{}: symbolic links are refused", shown(rel, &name)));
            } else if kind.is_dir() {
                if name.starts_with('.') || name == "facts" {
                    continue;
                }
                rel.push(name);
                self.dir(&entry.path(), rel)?;
                rel.pop();
            } else if name != MANIFEST_FILE && name.ends_with(".json") {
                if self.files.len() >= MAX_FILES {
                    return Err("More than 65536 world-data files");
                }
                self.files
                    .push((shown(rel, &name), rel.clone(), name, entry.path()));
            }
        }
        Ok(())
    }
}

fn read_shard(path: &str, file: &Path, errors: &mut Vec<String>) -> Option<(Vec<u8>, u64)> {
    let size = fs::metadata(file).map(|m| m.len()).unwrap_or(u64::MAX);
    if size > MAX_SHARD_BYTES as u64 {
        errors.push(format!("{path}: larger than 16 MiB; not read"));
        return None;
    }
    let mut bytes = Vec::new();
    let read = fs::File::open(file)
        .and_then(|f| f.take(MAX_SHARD_BYTES as u64 + 1).read_to_end(&mut bytes));
    if read.is_err() {
        errors.push(format!("{path}: cannot be read"));
        return None;
    }
    let len = bytes.len() as u64;
    Some((bytes, len))
}

fn load(input: &Path, errors: &mut Vec<String>) -> Result<Vec<Shard>, &'static str> {
    if !fs::symlink_metadata(input).is_ok_and(|m| m.is_dir()) {
        return Err("Input must be an existing directory, not a link");
    }
    let mut files = Vec::new();
    Walk {
        files: &mut files,
        errors,
    }
    .dir(input, &mut Vec::new())?;
    let mut shards = Vec::new();
    for (path, folder, name, file) in files {
        let Some((bytes, size)) = read_shard(&path, &file, errors) else {
            continue;
        };
        let header = match world_data::parse(&bytes) {
            Ok(d) => Some(Header {
                dataset: d.dataset,
                record_kind: d.record_kind,
                origin: d.origin,
                target: d.target,
                confidence: d.confidence,
                generator: d.generator,
                index: d.shard.index,
                count: d.shard.count,
                records: d.records.len(),
            }),
            Err(e) => {
                errors.push(format!("{path}: {e}"));
                None
            }
        };
        shards.push(Shard {
            path,
            folder,
            name,
            sha256: digest(&bytes),
            bytes: size,
            header,
        });
    }
    Ok(shards)
}

fn check_folder(path: &str, shards: &[&Shard], errors: &mut Vec<String>) -> Option<Folder> {
    let valid: Vec<(&Shard, &Header)> = shards
        .iter()
        .filter_map(|s| s.header.as_ref().map(|h| (*s, h)))
        .collect();
    let (first_shard, first) = *valid.first()?;
    for (shard, h) in &valid[1..] {
        let differs: Vec<&str> = [
            ("dataset", h.dataset != first.dataset),
            ("record_kind", h.record_kind != first.record_kind),
            ("origin", h.origin != first.origin),
            ("target", h.target != first.target),
            ("confidence", h.confidence != first.confidence),
            ("generator", h.generator != first.generator),
        ]
        .into_iter()
        .filter_map(|(field, d)| d.then_some(field))
        .collect();
        if !differs.is_empty() {
            errors.push(format!(
                "{}: {} differ from {}",
                shard.path,
                differs.join(", "),
                first_shard.path
            ));
        }
    }
    let counts: BTreeSet<u32> = valid.iter().map(|(_, h)| h.count).collect();
    if counts.len() > 1 {
        errors.push(format!("{path}: shards disagree on the shard count"));
    } else {
        let mut seen = BTreeSet::new();
        for (shard, h) in &valid {
            if !seen.insert(h.index) {
                errors.push(format!("{}: duplicate shard index {}", shard.path, h.index));
            }
        }
        if seen.len() != first.count as usize || shards.len() != valid.len() {
            errors.push(format!(
                "{path}: shard indexes are not exactly 0..{}",
                first.count - 1
            ));
        }
    }
    for (shard, h) in &valid {
        if shard.name != format!("part-{:04}.json", h.index) {
            errors.push(format!(
                "{}: file name must be part-{:04}.json",
                shard.path, h.index
            ));
        }
    }
    let build = first.origin.build.value().map(|b| b.as_str().to_owned());
    let mut expected: Vec<String> = first.dataset.split('.').map(str::to_owned).collect();
    expected.push(first.origin.edition.as_str().to_owned());
    expected.push(build.clone().unwrap_or_else(|| "unknown-build".to_owned()));
    if first_shard.folder != expected {
        errors.push(format!(
            "{}: folder must be {}/",
            if path.is_empty() { "." } else { path },
            expected.join("/")
        ));
    }
    Some(Folder {
        path: path.to_owned(),
        dataset: first.dataset.clone(),
        record_kind: first.record_kind,
        origin_edition: first.origin.edition.as_str().to_owned(),
        origin_build: build,
        shards: valid.len(),
        records: valid.iter().map(|(_, h)| h.records).sum(),
    })
}

fn check_manifest(bytes: &[u8], shards: &[Shard], errors: &mut Vec<String>) {
    let manifest = match azeroth_core::strict_json::value(bytes, MAX_MANIFEST_BYTES)
        .map_err(|_| ())
        .and_then(|v| serde_json::from_value::<Manifest>(v).map_err(|_| ()))
    {
        Ok(m) => m,
        Err(()) => {
            errors.push("run manifest: not a valid navzeroth-world-data-run document".into());
            return;
        }
    };
    if manifest.format != MANIFEST_FORMAT || manifest.schema_version != 1 {
        errors.push("run manifest: format must be navzeroth-world-data-run v1".into());
        return;
    }
    let by_path: BTreeMap<&str, &Shard> = shards.iter().map(|s| (s.path.as_str(), s)).collect();
    let mut names = BTreeSet::new();
    let mut listed = BTreeSet::new();
    for generator in &manifest.generators {
        if !bounded(&generator.name) || !names.insert(generator.name.as_str()) {
            errors.push("run manifest: generator names must be unique, non-empty text".into());
            continue;
        }
        if generator.status == RunStatus::Failed {
            errors.push(format!("run manifest: generator {} failed", generator.name));
        }
        for file in &generator.files {
            if !relative_path(&file.path) {
                errors.push(format!(
                    "run manifest: generator {} lists a path that is not relative with / separators",
                    generator.name
                ));
                continue;
            }
            if !listed.insert(file.path.as_str()) {
                errors.push(format!("run manifest: {} is listed twice", file.path));
                continue;
            }
            let Some(shard) = by_path.get(file.path.as_str()) else {
                errors.push(format!(
                    "run manifest: {} is not a world-data shard on disk",
                    file.path
                ));
                continue;
            };
            if !sha256_hex(&file.sha256) || file.sha256 != shard.sha256 {
                errors.push(format!(
                    "{}: SHA-256 differs from the run manifest",
                    file.path
                ));
            }
            if file.bytes != shard.bytes {
                errors.push(format!(
                    "{}: byte count differs from the run manifest",
                    file.path
                ));
            }
            if let Some(h) = &shard.header
                && file.records != h.records as u64
            {
                errors.push(format!(
                    "{}: record count differs from the run manifest",
                    file.path
                ));
            }
        }
    }
    for shard in shards {
        if !listed.contains(shard.path.as_str()) {
            errors.push(format!("{}: not listed in the run manifest", shard.path));
        }
    }
    for source in &manifest.sources {
        let status = match &source.status {
            SourceStatus::Word(w) => bounded(w),
            SourceStatus::Code(c) => (100..=599).contains(c),
        };
        if !source.url.starts_with("https://")
            || !bounded(&source.url)
            || !status
            || source.sha256.as_deref().is_some_and(|h| !sha256_hex(h))
        {
            errors.push(
                "run manifest: a source needs an https URL, a status and a lowercase SHA-256 when hashed"
                    .into(),
            );
        }
    }
}

/// Verify the shards below `options.input`. `Err` means the input itself cannot be
/// inspected; every finding about its content is listed in the report.
pub fn verify(options: &Options) -> Result<Report, &'static str> {
    let mut errors = Vec::new();
    let shards = load(&options.input, &mut errors)?;
    let mut by_folder: BTreeMap<String, Vec<&Shard>> = BTreeMap::new();
    for shard in &shards {
        by_folder
            .entry(shard.folder.join("/"))
            .or_default()
            .push(shard);
    }
    let mut folders = Vec::new();
    for (path, group) in &by_folder {
        folders.extend(check_folder(path, group, &mut errors));
    }
    let default = options.input.join(MANIFEST_FILE);
    let manifest = match &options.manifest {
        Some(path) => Some(path.clone()),
        None => fs::symlink_metadata(&default).is_ok().then_some(default),
    };
    if let Some(path) = &manifest {
        match fs::symlink_metadata(path) {
            Ok(m) if m.is_file() && m.len() <= MAX_MANIFEST_BYTES as u64 => match fs::read(path) {
                Ok(bytes) => check_manifest(&bytes, &shards, &mut errors),
                Err(_) => errors.push("run manifest: cannot be read".into()),
            },
            _ => errors
                .push("run manifest: must be an existing regular file of at most 16 MiB".into()),
        }
    }
    let mut datasets = BTreeMap::new();
    for folder in &folders {
        *datasets.entry(folder.dataset.clone()).or_default() += folder.records;
    }
    let error_count = errors.len();
    errors.truncate(MAX_REPORTED_ERRORS);
    Ok(Report {
        format: REPORT_FORMAT,
        schema_version: 1,
        passes: error_count == 0,
        files: shards.len(),
        records: folders.iter().map(|f| f.records).sum(),
        datasets,
        folders,
        manifest_checked: manifest.is_some(),
        error_count,
        errors,
    })
}
