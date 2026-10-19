//! Explicit network acquisition and deterministic offline source-knowledge builds.
//! Imported files are data. No upstream program, Lua runtime or SQL is invoked.
use azeroth_core::{
    source_admission::{self, Method, Origin, Registry, Status},
    source_pipeline::{self as knowledge, Candidate, Document, Fact},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub const USAGE: &str = "source-sync --registry <file> --cache <dir> [--fetch [--check-updates <policy-file>]]; source-build --registry <file> --plan <file> --cache <dir> --output <new-dir>";
type Result<T> = std::result::Result<T, &'static str>;
#[derive(Debug)]
pub struct Options {
    pub registry: PathBuf,
    pub cache: PathBuf,
    pub plan: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub fetch: bool,
    pub updates_policy: Option<PathBuf>,
}
impl Options {
    pub fn parse(args: &[OsString], build: bool) -> Option<Self> {
        let mut pairs = BTreeMap::new();
        let mut fetch = false;
        let mut i = 0;
        while i < args.len() {
            let key = args[i].to_str()?;
            if key == "--fetch" && !build && !fetch {
                fetch = true;
                i += 1;
                continue;
            }
            if !["--registry", "--cache"].contains(&key)
                && !(build && ["--plan", "--output"].contains(&key))
                && !(!build && key == "--check-updates")
            {
                return None;
            }
            let val = args.get(i + 1)?;
            if val.is_empty()
                || val.as_encoded_bytes().len() > 4096
                || val.as_encoded_bytes().starts_with(b"--")
                || pairs.insert(key, PathBuf::from(val)).is_some()
            {
                return None;
            }
            i += 2;
        }
        let updates_policy = pairs.remove("--check-updates");
        if updates_policy.is_some() && !fetch {
            return None;
        }
        Some(Self {
            registry: pairs.remove("--registry")?,
            cache: pairs.remove("--cache")?,
            plan: if build {
                Some(pairs.remove("--plan")?)
            } else {
                None
            },
            output: if build {
                Some(pairs.remove("--output")?)
            } else {
                None
            },
            fetch,
            updates_policy,
        })
    }
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    crate::questiedb::read_local_source(path).map_err(
        |_| "Source material must be bounded, unchanged and free of symlinks/reparse points",
    )
}
fn directory(path: &Path, create: bool) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| "Cannot resolve source directory")?
            .join(path)
    };
    if path.as_os_str().as_encoded_bytes().len() > 4096 {
        return Err("Source directory path exceeds bound");
    }
    let mut prefix = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir) {
            return Err("Parent traversal is unsupported");
        }
        prefix.push(part.as_os_str());
        if !prefix.exists() && create {
            fs::create_dir(&prefix).map_err(|_| "Cannot create source directory")?;
        }
        let meta = fs::symlink_metadata(&prefix).map_err(|_| "Source directory is unavailable")?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("Source directory must not contain links");
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Source directory must not contain reparse points");
            }
        }
    }
    Ok(path)
}
fn registry(path: &Path) -> Result<(Registry, Vec<u8>)> {
    let bytes = read(path)?;
    let registry = Registry::parse(&bytes).map_err(|_| "Invalid admission registry")?;
    Ok((registry, bytes))
}
#[derive(Clone)]
struct Material {
    url: String,
    sha256: String,
    bytes: Option<u64>,
}
fn closure(source: &source_admission::Source) -> Result<Vec<Material>> {
    let mut materials: BTreeMap<String, Material> = BTreeMap::new();
    for file in &source.files {
        if let Some(existing) = materials.get(&file.sha256)
            && (existing.url != file.url || existing.bytes != Some(file.bytes))
        {
            return Err("Repeated material hash has inconsistent declarations");
        }
        materials.insert(
            file.sha256.clone(),
            Material {
                url: file.url.clone(),
                sha256: file.sha256.clone(),
                bytes: Some(file.bytes),
            },
        );
    }
    for proof in source
        .rights
        .evidence
        .iter()
        .chain(source.retrieval.access_evidence.iter())
    {
        if let Some(old) = materials.get(&proof.sha256) {
            if old.url != proof.url {
                return Err("Material hash has ambiguous origin URLs");
            }
        } else {
            materials.insert(
                proof.sha256.clone(),
                Material {
                    url: proof.url.clone(),
                    sha256: proof.sha256.clone(),
                    bytes: None,
                },
            );
        }
    }
    if materials.len() > source.retrieval.max_requests as usize {
        return Err("Material closure exceeds declared request limit");
    }
    Ok(materials.into_values().collect())
}
fn verify(bytes: &[u8], material: &Material) -> Result<()> {
    if bytes.is_empty()
        || bytes.len() as u64 > source_admission::MAX_FILE_BYTES
        || material.bytes.is_some_and(|n| n != bytes.len() as u64)
        || digest(bytes) != material.sha256
    {
        return Err("Cached or retrieved source differs from admitted bytes");
    }
    Ok(())
}
fn cache_path(cache: &Path, hash: &str) -> PathBuf {
    cache.join(hash)
}
fn fetch(material: &Material, revision: &str, remaining_bytes: u64) -> Result<Vec<u8>> {
    // Automatic retrieval currently supports only publisher GitHub raw paths
    // pinned to this exact immutable source revision. Other hosts stay local.
    let path = material
        .url
        .strip_prefix("https://raw.githubusercontent.com/")
        .ok_or("Automatic source retrieval supports reviewed GitHub raw files only")?;
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() < 4
        || parts[2] != revision
        || parts
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return Err("Source retrieval URL is not pinned to the admitted revision");
    }
    let maximum = material
        .bytes
        .unwrap_or(source_admission::MAX_FILE_BYTES)
        .min(remaining_bytes);
    if maximum == 0 || material.bytes.is_some_and(|n| n > maximum) {
        return Err("Source retrieval would exceed remaining byte budget");
    }
    let bytes = https_get(&material.url, maximum)?;
    verify(&bytes, material)?;
    Ok(bytes)
}
fn https_get(url: &str, maximum: u64) -> Result<Vec<u8>> {
    // curl is a transport dependency, not an imported executable. Disable user
    // config, redirects and non-HTTPS. Check status, size and hash independently.
    let mut child = Command::new("curl")
        .args([
            "--disable",
            "--silent",
            "--globoff",
            "--fail",
            "--no-location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--max-filesize",
            &maximum.to_string(),
            "--write-out",
            "%{http_code}",
            "--url",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "HTTPS source fetch requires curl")?;
    let mut bytes = Vec::new();
    let outcome = child
        .stdout
        .take()
        .ok_or("Source transport pipe unavailable")?
        .take(maximum + 4)
        .read_to_end(&mut bytes);
    if outcome.is_err() || bytes.len() as u64 > maximum + 3 {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Source response exceeds declared bound");
    }
    let status = child.wait().map_err(|_| "Cannot finish source fetch")?;
    if !status.success() || bytes.len() < 3 || &bytes[bytes.len() - 3..] != b"200" {
        return Err("Source fetch failed or redirected; no fallback is permitted");
    }
    bytes.truncate(bytes.len() - 3);
    Ok(bytes)
}
fn publish_blob(cache: &Path, material: &Material, bytes: &[u8]) -> Result<()> {
    verify(bytes, material)?;
    let destination = cache_path(cache, &material.sha256);
    if destination.symlink_metadata().is_ok() {
        return verify(&read(&destination)?, material);
    }
    let temporary = cache.join(format!(
        ".pending-{}-{}",
        std::process::id(),
        material.sha256
    ));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "Cannot reserve source staging file")?;
    let result = (|| {
        f.write_all(bytes)
            .map_err(|_| "Cannot write source material")?;
        f.sync_all().map_err(|_| "Cannot flush source material")?;
        drop(f);
        // hard_link publishes without overwriting an existing immutable object.
        if fs::hard_link(&temporary, &destination).is_err() {
            verify(&read(&destination)?, material)?;
        }
        Ok(())
    })();
    let _ = fs::remove_file(temporary);
    result
}
#[derive(Serialize)]
pub struct SourceResult {
    pub source_id: String,
    pub revision: String,
    pub status: &'static str,
    pub files: usize,
    pub material_bytes: u64,
}
#[derive(Serialize)]
pub struct SyncReport {
    pub schema_version: u32,
    pub registry_sha256: String,
    pub sources: Vec<SourceResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_updates: Option<Vec<Update>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updates_policy_sha256: Option<String>,
}
pub fn sync(options: &Options) -> Result<SyncReport> {
    let (registry, raw) = registry(&options.registry)?;
    sync_registry(options, &registry, &raw)
}
fn sync_registry(options: &Options, registry: &Registry, raw: &[u8]) -> Result<SyncReport> {
    sync_registry_using(options, registry, raw, fetch)
}
fn sync_registry_using(
    options: &Options,
    registry: &Registry,
    raw: &[u8],
    mut fetch: impl FnMut(&Material, &str, u64) -> Result<Vec<u8>>,
) -> Result<SyncReport> {
    if options.updates_policy.is_some() && !options.fetch {
        return Err("Metadata requests require explicit network acquisition");
    }
    let metadata = options
        .updates_policy
        .as_ref()
        .map(|path| load_updates_policy(path, registry, raw))
        .transpose()?;
    let cache = directory(&options.cache, options.fetch)?;
    let mut reports = Vec::new();
    let mut last_request: Option<Instant> = None;
    let mut retained = BTreeSet::new();
    let mut retained_bytes = 0u64;
    let mut sources: Vec<_> = registry.sources.iter().collect();
    sources.sort_by_key(|s| &s.id);
    for source in sources {
        if source.rights.status != Status::Admitted {
            reports.push(SourceResult {
                source_id: source.id.clone(),
                revision: source.revision.clone(),
                status: if source.rights.status == Status::Rejected {
                    "excluded_rejected"
                } else {
                    "excluded_unreviewed"
                },
                files: 0,
                material_bytes: 0,
            });
            continue;
        }
        if source.retrieval.minimum_interval_ms > 60_000 {
            return Err("Source request interval exceeds this adapter's supported bound");
        }
        let mut total = 0u64;
        let materials = closure(source)?;
        for material in &materials {
            let path = cache_path(&cache, &material.sha256);
            let bytes = if path.symlink_metadata().is_ok() {
                read(&path)?
            } else {
                if !options.fetch {
                    return Err(
                        "Offline source closure is incomplete; run explicit source-sync --fetch",
                    );
                }
                match source.retrieval.method {
                    Method::HttpsPinned => {}
                    Method::ManualLocal => {
                        return Err("Source allows manual local acquisition only");
                    }
                    // Only the opt-in generators under data-generators/ retrieve
                    // these sources; their retained output is registered as files.
                    Method::HttpsRateLimited => {
                        return Err(
                            "Generator-retrieved source material is retained from data-generators; source-sync never fetches it",
                        );
                    }
                }
                if let Some(last) = last_request {
                    let interval = Duration::from_millis(source.retrieval.minimum_interval_ms);
                    if last.elapsed() < interval {
                        std::thread::sleep(interval - last.elapsed());
                    }
                }
                last_request = Some(Instant::now());
                let remaining = source
                    .retrieval
                    .max_total_bytes
                    .saturating_sub(total)
                    .min(source_admission::MAX_RETRIEVAL_BYTES.saturating_sub(retained_bytes));
                let bytes = fetch(material, &source.revision, remaining)?;
                // Verify aggregate bound before publishing the downloaded object.
                if total + bytes.len() as u64 > source.retrieval.max_total_bytes {
                    return Err("Source closure exceeds aggregate byte budget");
                }
                if !retained.contains(&material.sha256)
                    && retained_bytes + bytes.len() as u64 > source_admission::MAX_RETRIEVAL_BYTES
                {
                    return Err("Complete material closure exceeds two GiB");
                }
                publish_blob(&cache, material, &bytes)?;
                bytes
            };
            verify(&bytes, material)?;
            if retained.insert(material.sha256.clone()) {
                retained_bytes += bytes.len() as u64;
                if retained_bytes > source_admission::MAX_RETRIEVAL_BYTES {
                    return Err("Complete material closure exceeds two GiB");
                }
            }
            total = total
                .checked_add(bytes.len() as u64)
                .ok_or("Source byte count overflow")?;
            if total > source.retrieval.max_total_bytes {
                return Err("Source closure exceeds aggregate byte budget");
            }
        }
        reports.push(SourceResult {
            source_id: source.id.clone(),
            revision: source.revision.clone(),
            status: "verified_local_closure",
            files: materials.len(),
            material_bytes: total,
        });
    }
    Ok(SyncReport {
        schema_version: 1,
        registry_sha256: digest(raw),
        sources: reports,
        upstream_updates: metadata
            .as_ref()
            .map(|(policy, _)| updates(registry, policy)),
        updates_policy_sha256: metadata.map(|(_, hash)| hash),
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub target: Origin,
    pub as_of_utc_ms: i64,
    pub inputs: Vec<Input>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub source: String,
    pub file: String,
    pub adapter: Adapter,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Adapter {
    KnowledgeV1,
    AtlaslootAreasV1,
    MapzerothNodesV1,
    ModelSourceV1,
    AttFlightPointV1,
}
impl Adapter {
    pub fn transform(self) -> &'static str {
        match self {
            Self::KnowledgeV1 => "knowledge_v1",
            Self::AtlaslootAreasV1 => "atlasloot_areas_v1",
            Self::MapzerothNodesV1 => "mapzeroth_nodes_v1",
            Self::ModelSourceV1 => "model_source_v1",
            Self::AttFlightPointV1 => "att_flight_point_v1",
        }
    }
}
#[derive(Serialize)]
pub struct BuildReport {
    pub generator_version: &'static str,
    pub generator_sha256: String,
    pub format: &'static str,
    pub schema_version: u32,
    pub registry_sha256: String,
    pub plan_sha256: String,
    pub catalog_sha256: String,
    pub source_closure_sha256: String,
    pub publication_authorized: bool,
    pub facts: usize,
}
/// Counts and hashes the bytes it forwards and refuses to pass the catalog bound.
struct CatalogWriter<W> {
    inner: W,
    hasher: Sha256,
    bytes: u64,
    limit: u64,
    exceeded: bool,
}
impl<W: Write> Write for CatalogWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.bytes + buf.len() as u64 > self.limit {
            self.exceeded = true;
            return Err(std::io::Error::other("catalog byte bound"));
        }
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
/// Writes the pretty catalog to a new file and returns its SHA-256.
fn write_catalog<T: Serialize>(path: &Path, catalog: &T) -> Result<String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "Cannot reserve source output file")?;
    stream_catalog(file, catalog, knowledge::MAX_CATALOG_BYTES).and_then(|(hash, file)| {
        file.sync_all().map_err(|_| "Cannot flush source output")?;
        Ok(hash)
    })
}
fn stream_catalog<T: Serialize, W: Write>(
    inner: W,
    catalog: &T,
    limit: u64,
) -> Result<(String, W)> {
    let mut writer = std::io::BufWriter::with_capacity(
        1 << 20,
        CatalogWriter {
            inner,
            hasher: Sha256::new(),
            bytes: 0,
            limit,
            exceeded: false,
        },
    );
    let encoded = serde_json::to_writer_pretty(&mut writer, catalog).map_err(|_| ());
    let flushed = encoded.and_then(|_| writer.flush().map_err(|_| ()));
    // Unflushed bytes are discarded on failure; the staging directory is removed.
    let (counted, _) = writer.into_parts();
    if counted.exceeded {
        return Err("Source catalog exceeds encoded byte bound");
    }
    flushed.map_err(|_| "Cannot write source catalog")?;
    Ok((format!("{:x}", counted.hasher.finalize()), counted.inner))
}
fn output_file(root: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .map_err(|_| "Cannot reserve source output file")?;
    f.write_all(bytes)
        .map_err(|_| "Cannot write source output")?;
    f.sync_all().map_err(|_| "Cannot flush source output")
}
pub fn build(options: &Options) -> Result<BuildReport> {
    // Offline means no transport path is reachable, even when a blob is missing.
    if options.fetch || options.updates_policy.is_some() {
        return Err("Source builds never permit network acquisition");
    }
    let (registry, registry_bytes) = registry(&options.registry)?;
    let plan_bytes = read(
        options
            .plan
            .as_ref()
            .ok_or("Source build requires a plan")?,
    )?;
    let raw = azeroth_core::strict_json::value(&plan_bytes, 1_048_576)?;
    let plan: Plan = serde_json::from_value(raw).map_err(|_| "Invalid source build plan")?;
    if plan.schema_version != 1
        || plan.inputs.is_empty()
        || plan.inputs.len() > 1024
        || plan.as_of_utc_ms < 0
    {
        return Err("Unsupported source build plan or capacity");
    }
    let closure_report = sync_registry(options, &registry, &registry_bytes)?;
    let cache = directory(&options.cache, false)?;
    let mut facts = Vec::new();
    let mut seen = BTreeSet::new();
    let mut inputs = plan.inputs.clone();
    inputs.sort_by(|a, b| (&a.source, &a.file).cmp(&(&b.source, &b.file)));
    for input in &inputs {
        if !seen.insert((&input.source, &input.file)) {
            return Err("Duplicate source build input");
        }
        let source = registry
            .sources
            .iter()
            .find(|s| s.id == input.source)
            .ok_or("Build source is not registered")?;
        let file = source
            .files
            .iter()
            .find(|f| f.path == input.file)
            .ok_or("Build file is not admitted")?;
        let fields: Vec<_> = file.fields.iter().map(String::as_str).collect();
        registry
            .authorize(&input.source, &input.file, &fields, source.retrieval.method)
            .map_err(|_| "Build source rights or fields are not admitted")?;
        if file.transformation != input.adapter.transform() {
            return Err("Build transformation differs from source admission");
        }
        let bytes = read(&cache_path(&cache, &file.sha256))?;
        verify(
            &bytes,
            &Material {
                url: file.url.clone(),
                sha256: file.sha256.clone(),
                bytes: Some(file.bytes),
            },
        )?;
        let rows: Vec<Fact> = match input.adapter {
            Adapter::AttFlightPointV1 => azeroth_ingest::source_adapters::att_flight_point(&bytes)?,
            Adapter::KnowledgeV1 => Document::parse(&bytes, plan.as_of_utc_ms)?.facts,
            Adapter::AtlaslootAreasV1 => azeroth_ingest::source_adapters::atlasloot_areas(&bytes)?,
            Adapter::MapzerothNodesV1 => azeroth_ingest::source_adapters::mapzeroth_nodes(&bytes)?,
            Adapter::ModelSourceV1 => vec![azeroth_ingest::source_adapters::model_reference(
                &input.file,
                &file.sha256,
            )],
        };
        for fact in rows {
            if !file
                .fields
                .iter()
                .any(|f| f == &format!("{}.{}", fact.domain, fact.field))
            {
                return Err("Adapter fact exceeds admitted fields");
            }
            knowledge::check_fact_capacity(facts.len() + 1)?;
            facts.push(Candidate {
                source_id: source.id.clone(),
                source_revision: source.revision.clone(),
                source_file: file.path.clone(),
                source_sha256: file.sha256.clone(),
                origin: source.origin.clone(),
                fact,
            });
        }
    }
    let facts_count = facts.len();
    let catalog = knowledge::converge(plan.target, plan.as_of_utc_ms, facts)?;
    let closure_bytes = serde_json::to_vec_pretty(&closure_report)
        .map_err(|_| "Cannot encode source closure report")?;
    let mut report = BuildReport {
        generator_version: env!("CARGO_PKG_VERSION"),
        generator_sha256: digest(
            concat!(
                include_str!("source_pipeline.rs"),
                include_str!("../../core/src/source_pipeline.rs"),
                include_str!("../../ingest/src/source_adapters.rs"),
                include_str!("../../core/src/strict_json.rs"),
                include_str!("../../core/src/source_admission.rs"),
                include_str!("../../ingest/src/savedvars.rs")
            )
            .as_bytes(),
        ),
        format: "navzeroth-source-build",
        schema_version: 1,
        registry_sha256: digest(&registry_bytes),
        plan_sha256: digest(&plan_bytes),
        // Set once the catalog is streamed into the staging directory.
        catalog_sha256: String::new(),
        source_closure_sha256: digest(&closure_bytes),
        publication_authorized: false,
        facts: facts_count,
    };
    let output = options
        .output
        .as_ref()
        .ok_or("Source build requires an output directory")?;
    if output.symlink_metadata().is_ok() {
        return Err("Source build output must not already exist");
    }
    let parent = directory(
        output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
        true,
    )?;
    let name = output.file_name().ok_or("Invalid source output path")?;
    if name == "." || name == ".." {
        return Err("Invalid source output path");
    }
    let output = parent.join(name);
    let staging = parent.join(format!(".source-build-{}", std::process::id()));
    fs::create_dir(&staging).map_err(|_| "Cannot reserve source build staging")?;
    let outcome = (|| {
        // A whole-world catalog is gigabytes; stream it instead of holding a copy.
        report.catalog_sha256 = write_catalog(&staging.join("catalog.json"), &catalog)?;
        let report_bytes =
            serde_json::to_vec_pretty(&report).map_err(|_| "Cannot encode source build report")?;
        output_file(&staging, "closure.json", &closure_bytes)?;
        output_file(&staging, "registry.json", &registry_bytes)?;
        output_file(&staging, "plan.json", &plan_bytes)?;
        output_file(&staging, "build.json", &report_bytes)?;
        fs::create_dir(staging.join("materials"))
            .map_err(|_| "Cannot create retained source closure")?;
        fs::create_dir(staging.join("materials/objects"))
            .map_err(|_| "Cannot create retained source objects")?;
        let mut copied = BTreeSet::new();
        let mut notices = String::from(
            "Navzeroth source knowledge: third-party notices\n\nTransformation notice: Navzeroth contributors modified/extracted this source-derived catalog on 2026-10-15 (v1 adapter authorship date, not gameplay observation time). AtlasLoot-derived dungeon/NPC/item declarations remain under GPL-2.0-only; the exact original source and GPLv2 text accompany this artifact. Unsupported expressions were not evaluated. Independent MIT portions retain their notices.\n\nThis artifact does not authorize a gameplay release. Original source and licence bytes are retained in materials/objects, keyed by SHA256 and mapped by registry.json. The repository BSD licence does not relicense third-party works.\n",
        );
        for source in registry
            .sources
            .iter()
            .filter(|s| s.rights.status == Status::Admitted)
        {
            notices.push_str(&format!("\nSource: {}\nRevision: {}\nPublisher: {}\nLicence: {}\nAttribution: {}\nObligations: {}\n", source.id, source.revision, source.publisher, source.rights.license_identifier.as_deref().unwrap_or("unknown"), source.rights.attribution.as_deref().unwrap_or("unknown"), source.rights.obligations.as_deref().unwrap_or("unknown")));
            for material in closure(source)? {
                if copied.insert(material.sha256.clone()) {
                    let bytes = read(&cache_path(&cache, &material.sha256))?;
                    verify(&bytes, &material)?;
                    output_file(
                        &staging,
                        &format!("materials/objects/{}", material.sha256),
                        &bytes,
                    )?;
                }
            }
        }
        output_file(&staging, "NOTICES.txt", notices.as_bytes())?;

        // Destination is rechecked before publication; never replace prior output.
        if output.symlink_metadata().is_ok() {
            return Err("Source build output appeared during build");
        }
        fs::rename(&staging, &output).map_err(|_| "Cannot atomically publish source build")?;
        Ok(())
    })();
    if outcome.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    outcome?;
    Ok(report)
}

#[derive(Serialize)]
pub struct Update {
    pub source_id: String,
    pub retained_revision: String,
    pub upstream_revision: Option<String>,
    pub status: &'static str,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdatesPolicy {
    schema_version: u32,
    registry_sha256: String,
    method: String,
    reviewer: String,
    reviewed_on: String,
    access_evidence_url: String,
    scope: String,
    max_requests: u32,
    max_response_bytes: u64,
    max_total_bytes: u64,
    minimum_interval_ms: u64,
    repositories: Vec<RepositoryMetadata>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryMetadata {
    source_id: String,
    repository: String,
}
fn load_updates_policy(
    path: &Path,
    registry: &Registry,
    registry_bytes: &[u8],
) -> Result<(UpdatesPolicy, String)> {
    let bytes = read(path)?;
    let value = azeroth_core::strict_json::value(&bytes, 1_048_576)?;
    let policy: UpdatesPolicy =
        serde_json::from_value(value).map_err(|_| "Invalid metadata retrieval policy")?;
    let bounded =
        |s: &str| !s.trim().is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control);
    if policy.schema_version != 1
        || policy.registry_sha256 != digest(registry_bytes)
        || policy.method != "github_commit_metadata"
        || !bounded(&policy.reviewer)
        || !bounded(&policy.reviewed_on)
        || !bounded(&policy.scope)
        || policy.access_evidence_url
            != "https://docs.github.com/en/rest/commits/commits#list-commits"
        || policy.max_requests == 0
        || policy.max_requests > 128
        || policy.repositories.len() > policy.max_requests as usize
        || policy.max_response_bytes == 0
        || policy.max_response_bytes > 1_048_576
        || policy.max_total_bytes > 128 * 1_048_576
        || policy.max_total_bytes < policy.max_response_bytes * policy.repositories.len() as u64
        || !(1000..=60_000).contains(&policy.minimum_interval_ms)
    {
        return Err("Metadata request permission, registry binding or bounds are invalid");
    }
    let mut sources = BTreeSet::new();
    let mut repos = BTreeSet::new();
    for row in &policy.repositories {
        let source = registry
            .sources
            .iter()
            .find(|s| s.id == row.source_id)
            .ok_or("Metadata source is not registered")?;
        if source.rights.status != Status::Admitted
            || source.publisher != row.repository
            || !sources.insert(&row.source_id)
            || !repos.insert(&row.repository)
            || row.repository.split('/').count() != 2
            || !row.repository.split('/').all(|p| {
                !p.is_empty()
                    && p != "."
                    && p != ".."
                    && p.len() <= 100
                    && p.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            })
        {
            return Err("Metadata repository is not independently permitted");
        }
    }
    Ok((policy, digest(&bytes)))
}
fn updates(registry: &Registry, policy: &UpdatesPolicy) -> Vec<Update> {
    updates_using(registry, policy, https_get)
}
fn updates_using(
    registry: &Registry,
    policy: &UpdatesPolicy,
    mut retrieve: impl FnMut(&str, u64) -> Result<Vec<u8>>,
) -> Vec<Update> {
    let mut rows = Vec::new();
    let mut last_request: Option<Instant> = None;
    for permission in &policy.repositories {
        let source = registry
            .sources
            .iter()
            .find(|s| s.id == permission.source_id)
            .expect("validated metadata source");
        if let Some(last) = last_request {
            let interval = Duration::from_millis(policy.minimum_interval_ms);
            if last.elapsed() < interval {
                std::thread::sleep(interval - last.elapsed());
            }
        }
        last_request = Some(Instant::now());
        // Independent metadata-only permission and worst-case aggregate budget.
        // Never download or admit content from a newly observed revision here.
        let url = format!(
            "https://api.github.com/repos/{}/commits?per_page=1",
            permission.repository
        );
        let revision = retrieve(&url, policy.max_response_bytes)
            .ok()
            .and_then(|bytes| {
                azeroth_core::strict_json::value(&bytes, policy.max_response_bytes as usize).ok()
            })
            .and_then(|v| {
                v.pointer("/0/sha")
                    .and_then(|s| s.as_str())
                    .map(str::to_owned)
            })
            .filter(|s| {
                s.len() == 40
                    && s.bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            });
        let status = match revision.as_deref() {
            Some(r) if r == source.revision => "retained_revision_is_current",
            Some(_) => "new_upstream_revision_requires_validation",
            None => "upstream_freshness_unknown",
        };
        rows.push(Update {
            source_id: source.id.clone(),
            retained_revision: source.revision.clone(),
            upstream_revision: revision,
            status,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admitted_retrieval_publishes_verified_blobs_and_never_overwrites_on_refusal() {
        use serde_json::json;
        let root = std::env::temp_dir().join(format!(
            "nav-source-retrieval-boundary-{}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
        let _cleanup = Cleanup(root.clone());
        let data = b"synthetic data fixture";
        let license = b"synthetic review evidence; no real rights grant";
        let revision = "a".repeat(40);
        let evidence = json!({"url":format!("https://raw.githubusercontent.com/synthetic/fixture/{revision}/LICENSE"),"revision":revision,"sha256":digest(license),"reviewed_scope":"synthetic fixture"});
        let value = json!({"schema_version":1,"sources":[{"id":"synthetic","revision":revision,"publisher":"synthetic/fixture","origin":{"edition":"wow_forever","build":"1.60.1.70374"},"rights":{"status":"admitted","license":"permissive","license_identifier":"synthetic-test-only","reviewer":"fixture","attribution":"fixture","obligations":"fixture","evidence":[evidence.clone()]},"retrieval":{"method":"https_pinned","max_total_bytes":data.len()+license.len(),"minimum_interval_ms":1000,"max_requests":2,"access_evidence":evidence},"files":[{"path":"facts.json","url":format!("https://raw.githubusercontent.com/synthetic/fixture/{revision}/facts.json"),"sha256":digest(data),"bytes":data.len(),"fields":["quests.shareable"],"transformation":"knowledge_v1"}]}]});
        let raw = serde_json::to_vec(&value).unwrap();
        let mut registry = Registry::parse(&raw).unwrap();
        let policy: UpdatesPolicy = serde_json::from_value(json!({
            "schema_version":1,"registry_sha256":digest(&raw),"method":"github_commit_metadata",
            "reviewer":"synthetic","reviewed_on":"synthetic","access_evidence_url":"https://docs.github.com/en/rest/commits/commits#list-commits",
            "scope":"metadata only","max_requests":1,"max_response_bytes":1024,"max_total_bytes":1024,"minimum_interval_ms":1000,
            "repositories":[{"source_id":"synthetic","repository":"synthetic/fixture"}]
        })).unwrap();
        for (response, expected, observed) in [
            (
                Ok(json!([{"sha":revision}]).to_string().into_bytes()),
                "retained_revision_is_current",
                Some(revision.clone()),
            ),
            (
                Ok(json!([{"sha":"b".repeat(40)}]).to_string().into_bytes()),
                "new_upstream_revision_requires_validation",
                Some("b".repeat(40)),
            ),
            (
                Ok(json!([{"sha":"A".repeat(40)}]).to_string().into_bytes()),
                "upstream_freshness_unknown",
                None,
            ),
            (
                Ok(b"truncated{".to_vec()),
                "upstream_freshness_unknown",
                None,
            ),
            (
                Err("synthetic unavailable transport"),
                "upstream_freshness_unknown",
                None,
            ),
        ] {
            let mut calls = 0;
            let rows = updates_using(&registry, &policy, |url, bound| {
                calls += 1;
                assert_eq!(
                    url,
                    "https://api.github.com/repos/synthetic/fixture/commits?per_page=1"
                );
                assert_eq!(bound, 1024);
                response.clone()
            });
            assert_eq!(calls, 1);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].status, expected);
            assert_eq!(rows[0].retained_revision, revision);
            assert_eq!(rows[0].upstream_revision, observed);
            assert_eq!(registry.sources[0].revision, revision);
        }
        let options = Options {
            registry: root.join("unused.json"),
            cache: root.join("cache"),
            plan: None,
            output: None,
            fetch: true,
            updates_policy: None,
        };
        let mut requests = 0;
        sync_registry_using(&options, &registry, &raw, |material, pinned, bound| {
            assert_eq!(pinned, revision);
            requests += 1;
            let bytes = if material.sha256 == digest(data) {
                data.as_slice()
            } else {
                license.as_slice()
            };
            assert!(bytes.len() as u64 <= bound);
            Ok(bytes.to_vec())
        })
        .unwrap();
        assert_eq!(requests, 2);
        assert_eq!(fs::read(options.cache.join(digest(data))).unwrap(), data);
        assert_eq!(
            fs::read(options.cache.join(digest(license))).unwrap(),
            license
        );
        sync_registry_using(&options, &registry, &raw, |_, _, _| {
            panic!("Verified cached bytes must not be retrieved again")
        })
        .unwrap();
        let material = Material {
            url: "unused".into(),
            sha256: digest(data),
            bytes: Some(data.len() as u64),
        };
        publish_blob(&options.cache, &material, data).unwrap();
        fs::remove_file(options.cache.join(digest(data))).unwrap();
        assert!(
            sync_registry_using(&options, &registry, &raw, |_, _, _| Ok(
                b"altered download".to_vec()
            ))
            .is_err()
        );
        assert!(!options.cache.join(digest(data)).exists());
        assert_eq!(
            fs::read(options.cache.join(digest(license))).unwrap(),
            license
        );
        registry.sources[0].retrieval.method = Method::ManualLocal;
        assert_eq!(
            sync_registry_using(&options, &registry, &raw, |_, _, _| panic!(
                "Manual-only source must never be fetched"
            ))
            .err(),
            Some("Source allows manual local acquisition only")
        );
        registry.sources[0].retrieval.method = Method::HttpsRateLimited;
        assert_eq!(
            sync_registry_using(&options, &registry, &raw, |_, _, _| panic!(
                "Generator-retrieved source must never be fetched by source-sync"
            ))
            .err(),
            Some(
                "Generator-retrieved source material is retained from data-generators; source-sync never fetches it"
            )
        );
        registry.sources[0].retrieval.minimum_interval_ms = 60_001;
        assert_eq!(
            sync_registry_using(&options, &registry, &raw, |_, _, _| panic!(
                "Unsupported interval must be refused before transport"
            ))
            .err(),
            Some("Source request interval exceeds this adapter's supported bound")
        );
    }
    #[test]
    fn exhausted_or_smaller_transport_budget_is_refused_before_fetch() {
        let mut material = Material {
            url: format!(
                "https://raw.githubusercontent.com/fixture/repo/{}/data",
                "a".repeat(40)
            ),
            sha256: "0".repeat(64),
            bytes: Some(1024),
        };
        assert_eq!(
            fetch(&material, &"a".repeat(40), 1023).err(),
            Some("Source retrieval would exceed remaining byte budget")
        );
        material.bytes = None;
        assert_eq!(
            fetch(&material, &"a".repeat(40), 0).err(),
            Some("Source retrieval would exceed remaining byte budget")
        );
    }
    #[test]
    fn streamed_catalog_is_hashed_and_bounded_at_the_exact_limit() {
        let catalog = serde_json::json!({"entries": [1, 2, 3]});
        let pretty = serde_json::to_vec_pretty(&catalog).unwrap();
        let limit = pretty.len() as u64;
        let (hash, bytes) = stream_catalog(Vec::new(), &catalog, limit).unwrap();
        assert_eq!(bytes, pretty);
        assert_eq!(hash, digest(&pretty));
        assert_eq!(
            stream_catalog(Vec::new(), &catalog, limit - 1).err(),
            Some("Source catalog exceeds encoded byte bound")
        );
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("synthetic disk failure"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        Broken.flush().unwrap();
        assert_eq!(
            stream_catalog(Broken, &catalog, limit).err(),
            Some("Cannot write source catalog")
        );
        let mut counted = CatalogWriter {
            inner: Vec::new(),
            hasher: Sha256::new(),
            bytes: 0,
            limit: 4,
            exceeded: false,
        };
        assert_eq!(counted.write(b"1234").unwrap(), 4);
        counted.flush().unwrap();
        assert!(counted.write(b"5").is_err());
        assert!(counted.exceeded);
        assert_eq!(knowledge::MAX_CATALOG_BYTES, 8 * 1024 * 1024 * 1024);
    }
}
