//! Release gate I/O: bounded evidence, exact Git/source binding and verified packs.
//! All referenced game/source material is data; no Lua or SQL is executed.
mod admission;
pub mod materials;
use azeroth_core::{
    contract::{Envelope, ReferenceDataset, decode_envelope},
    local_data_release::{DeliveryMode, DeliveryPolicy},
    origin_policy::{GateClassification, OriginPolicy},
    release::{self, Bindings, Evidence, FileRef, Inventory, Readiness, ReleasePolicy},
    sources::SourceRecord,
    types::{AssetPath, DatasetNamespace},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

type Result<T> = std::result::Result<T, &'static str>;
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_GIT_BYTES: u64 = 32 * 1024 * 1024;
const INVENTORY_PATH: &str = "release/v1/coverage-inventory.json";
const EVIDENCE_PATH: &str = "release/v1/acceptance-evidence.json";
/// Hybrid delivery evidence (ADR 0006 section 5); outside the production fingerprint.
const RECEIPT_PATH: &str = "release/v1/local-catalog-receipt.json";
/// ADR 0007 §6 and §3: schema 3 release inputs, bound by hash and equality.
pub const WORLD_DATA_TARGET_PATH: &str = "release/v1/world-data-target.json";
pub const ORIGIN_POLICY_PATH: &str = "release/v1/accepted-origins.json";

#[derive(Debug)]
pub struct Options {
    pub root: PathBuf,
    pub version: String,
    pub evidence: Option<PathBuf>,
}
impl Options {
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let (mut root, mut version, mut evidence) = (None, None, None);
        let (pairs, remainder) = args.as_chunks::<2>();
        for pair in pairs {
            let flag = pair[0].to_str()?;
            let value = pair[1].to_str()?;
            if value.is_empty() || value.starts_with("--") {
                return None;
            }
            match flag {
                "--root" if root.is_none() => root = Some(PathBuf::from(value)),
                "--version" if version.is_none() => version = Some(value.to_owned()),
                "--evidence" if evidence.is_none() => evidence = Some(PathBuf::from(value)),
                _ => return None,
            }
        }
        if !remainder.is_empty() {
            return None;
        }
        Some(Self {
            root: root?,
            version: version?,
            evidence,
        })
    }
}
#[derive(Debug, Serialize)]
pub struct Report {
    #[serde(flatten)]
    pub readiness: Readiness,
    pub candidate_source_commit: String,
    pub production_sha256: String,
    pub inventory_sha256: String,
    pub verified_evidence_files: usize,
    pub verified_content_packs: usize,
    /// Schema 3 only: the applied world-data target and accepted-origin policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_data: Option<WorldDataReport>,
}
/// A preparation verdict retains every checked I/O result but cannot publish.
#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub phase: release::BuildPhase,
    pub build_ready: bool,
    pub publication_allowed: bool,
    pub report: Report,
}
impl BuildReport {
    pub fn new(mut report: Report, phase: release::BuildPhase) -> Self {
        let build_ready = phase.ready(&report.readiness);
        report.readiness.publication_allowed = false;
        Self {
            phase,
            build_ready,
            publication_allowed: false,
            report,
        }
    }
}
/// What the accepted-origin policy accepted (ADR 0007 §3 and §5). Accepted
/// unknowns stay unknown; they are counted here, never written as zero.
#[derive(Debug, Default, Serialize)]
pub struct WorldDataReport {
    pub target: Option<release::Target>,
    pub origin_policy_sha256: Option<String>,
    /// Accepted origin editions per domain, as applied.
    pub accepted_origins_by_domain: BTreeMap<String, Vec<String>>,
    pub accept_inferred: Option<bool>,
    pub accept_unknowns: Option<bool>,
    /// Release sources whose target validation the policy satisfies.
    pub origin_accepted_sources: Vec<String>,
    pub accepted_origins: u64,
    pub accepted_unknowns: u64,
    pub remaining_gaps: u64,
    pub packs: Vec<PackClassification>,
}
#[derive(Debug, Serialize)]
pub struct PackClassification {
    pub manifest: String,
    #[serde(flatten)]
    pub classification: GateClassification,
}
/// Schema 3 release inputs read from the repository.
#[derive(Default)]
pub(crate) struct WorldInputs {
    pub(crate) target: Option<release::Target>,
    pub(crate) origins: Option<OriginPolicy>,
    pub(crate) origin_sha256: Option<String>,
}
impl WorldInputs {
    pub(crate) fn policy(&self) -> ReleasePolicy<'_> {
        ReleasePolicy {
            target: self.target.as_ref(),
            origins: self.origins.as_ref(),
            origin_policy_sha256: self.origin_sha256.as_deref(),
        }
    }
}
fn world_inputs(root: &Path, inventory: &Inventory, budget: &mut Budget) -> Result<WorldInputs> {
    if !release::is_world_schema(inventory.schema_version) {
        return Ok(WorldInputs::default());
    }
    let mut read = |path: &str| -> Result<Option<Vec<u8>>> {
        match fs::symlink_metadata(root.join(path)) {
            Ok(_) => budget.read(&root.join(path), 64 * 1024).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("Release policy input cannot be inspected"),
        }
    };
    let target = read(WORLD_DATA_TARGET_PATH)?;
    let origins = read(ORIGIN_POLICY_PATH)?;
    // A missing or invalid input is left absent; the assessment blocks on it.
    Ok(WorldInputs {
        target: target.and_then(|b| release::decode_world_data_target(&b).ok()),
        origin_sha256: origins.as_ref().map(|b| hash(b)),
        origins: origins.and_then(|b| OriginPolicy::decode(&b).ok()),
    })
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn production(path: &[u8]) -> bool {
    !path.starts_with(b"docs/") && !path.starts_with(b"release/")
}
fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    false
}
fn checked_path(path: &Path, directory: bool) -> Result<()> {
    let mut current = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => return Err("Parent traversal is not allowed"),
            Component::CurDir => continue,
            Component::Prefix(_) => {
                current.push(part.as_os_str());
                continue;
            }
            _ => current.push(part.as_os_str()),
        }
        let metadata = fs::symlink_metadata(&current).map_err(|_| "Release path is unavailable")?;
        if is_link(&metadata) {
            return Err("Symlink or reparse release path is not allowed");
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "Release path is unavailable")?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err("Release path has the wrong file kind");
    }
    Ok(())
}
fn root_path(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| "Current directory is unavailable")?
            .join(path)
    };
    checked_path(&path, true)?;
    // Parent and symlink components have already been refused. Lexical cleanup
    // preserves the caller's Windows path form for the existing pack verifier.
    Ok(path
        .components()
        .filter(|part| !matches!(part, Component::CurDir))
        .collect())
}
fn relative(root: &Path, path: &Path) -> Result<AssetPath> {
    let path = if path.is_absolute() {
        path.strip_prefix(root)
            .map_err(|_| "Evidence must be beneath the selected repository")?
    } else {
        path
    };
    let text = path.to_str().ok_or("Release paths must be UTF-8")?;
    // Public FileRef uses portable slash paths even on Windows.
    AssetPath::new(text.replace('\\', "/")).map_err(|_| "Invalid relative release path")
}
#[derive(Default)]
struct Budget {
    bytes: u64,
    verified: BTreeMap<PathBuf, (u64, String)>,
    charged: BTreeMap<PathBuf, u64>,
}
impl Budget {
    fn add(&mut self, bytes: u64) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or("Release byte budget exceeded")?;
        if self.bytes > MAX_TOTAL_BYTES {
            return Err("Release byte budget exceeded");
        }
        Ok(())
    }
    fn charge(&mut self, path: &Path, bytes: u64) -> Result<()> {
        if let Some(old) = self.charged.get(path) {
            return if *old == bytes {
                Ok(())
            } else {
                Err("Release input size changed")
            };
        }
        self.add(bytes)?;
        self.charged.insert(path.to_owned(), bytes);
        Ok(())
    }
    fn read(&mut self, path: &Path, limit: usize) -> Result<Vec<u8>> {
        checked_path(path, false)?;
        let file = File::open(path).map_err(|_| "Release file cannot be opened")?;
        let size = file
            .metadata()
            .map_err(|_| "Release file metadata is unavailable")?
            .len();
        if size > limit as u64 {
            return Err("Release file exceeds its size limit");
        }
        self.charge(path, size)?;
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Release file cannot be read")?;
        if bytes.len() as u64 != size {
            return Err("Release file changed or exceeds its limit");
        }
        Ok(bytes)
    }
    fn verify(&mut self, root: &Path, reference: &FileRef) -> Result<()> {
        let path = root.join(reference.path.as_str());
        let expected = (reference.bytes, reference.sha256.as_str().to_owned());
        if let Some(previous) = self.verified.get(&path) {
            return if *previous == expected {
                Ok(())
            } else {
                Err("Conflicting evidence file references")
            };
        }
        if reference.bytes == 0 || reference.bytes > MAX_FILE_BYTES {
            return Err("Evidence file exceeds its size limit");
        }
        checked_path(&path, false)?;
        let mut file = File::open(&path).map_err(|_| "Evidence file cannot be opened")?;
        let size = file
            .metadata()
            .map_err(|_| "Evidence metadata is unavailable")?
            .len();
        if size != reference.bytes {
            return Err("Evidence file size does not match");
        }
        self.charge(&path, size)?;
        let mut digest = Sha256::new();
        let mut read = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| "Evidence file cannot be read")?;
            if count == 0 {
                break;
            }
            read += count as u64;
            if read > size {
                return Err("Evidence file changed while reading");
            }
            digest.update(&buffer[..count]);
        }
        if read != size
            || format!("{digest:x}", digest = digest.finalize()) != reference.sha256.as_str()
        {
            return Err("Evidence file hash or size does not match");
        }
        self.verified.insert(path, expected);
        Ok(())
    }
}
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Git is unavailable for release verification")?;
    let mut bytes = Vec::new();
    let result = child
        .stdout
        .take()
        .ok_or("Git output is unavailable")?
        .take(MAX_GIT_BYTES + 1)
        .read_to_end(&mut bytes);
    if result.is_err() || bytes.len() as u64 > MAX_GIT_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Git output exceeds release verification bounds");
    }
    if !child
        .wait()
        .map_err(|_| "Git verification did not complete")?
        .success()
    {
        return Err("Git revision or repository verification failed");
    }
    Ok(bytes)
}
struct Tree {
    fingerprint: String,
    files: BTreeSet<String>,
}
fn tree(root: &Path, revision: &str) -> Result<Tree> {
    let bytes = git(root, &["ls-tree", "-r", "-z", "--full-tree", revision])?;
    let mut digest = Sha256::new();
    let mut files = BTreeSet::new();
    for entry in bytes.split_inclusive(|b| *b == 0) {
        if entry.last() != Some(&0) {
            return Err("Invalid Git tree output");
        }
        let tab = entry
            .iter()
            .position(|b| *b == b'\t')
            .ok_or("Invalid Git tree entry")?;
        let path = &entry[tab + 1..entry.len() - 1];
        if production(path) {
            digest.update(entry);
            let path = std::str::from_utf8(path).map_err(|_| "Production paths must be UTF-8")?;
            files.insert(path.to_owned());
        }
    }
    Ok(Tree {
        fingerprint: format!("{:x}", digest.finalize()),
        files,
    })
}
fn addon_inputs(root: &Path, candidate: &Tree) -> Result<()> {
    // Both the ZIP and Tauri resource mapping recursively package this tree.
    // Git status omits ignored files, so inspect the actual input directory too.
    let mut pending = vec![root.join("addons/AzerothNavigator")];
    let mut count = 0usize;
    while let Some(directory) = pending.pop() {
        checked_path(&directory, true)?;
        for entry in fs::read_dir(directory).map_err(|_| "Addon inputs cannot be inspected")? {
            let entry = entry.map_err(|_| "Addon inputs cannot be inspected")?;
            count += 1;
            if count > release::MAX_RECORDS {
                return Err("Addon packaging inputs exceed verification bounds");
            }
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| "Addon input metadata is unavailable")?;
            if is_link(&metadata) {
                return Err("Addon packaging inputs contain a symlink or reparse path");
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                let path = path
                    .strip_prefix(root)
                    .map_err(|_| "Addon input is outside the repository")?
                    .to_str()
                    .ok_or("Addon input paths must be UTF-8")?
                    .replace('\\', "/");
                if !candidate.files.contains(&path) {
                    return Err("Addon packaging inputs contain an untracked or ignored file");
                }
            } else {
                return Err("Addon packaging inputs contain an unsupported file kind");
            }
        }
    }
    Ok(())
}
fn string_value(value: &str) -> Result<String> {
    serde_json::from_str(value.trim().trim_end_matches(','))
        .map_err(|_| "Invalid source version declaration")
}
fn assignment(text: &str, key: &str) -> Result<String> {
    let mut values = text.lines().filter_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then_some(value)
    });
    let value = string_value(
        values
            .next()
            .ok_or("Source version declaration is missing")?,
    )?;
    if values.next().is_some() {
        return Err("Duplicate source version declaration");
    }
    Ok(value)
}
fn versions(root: &Path, version: &str, budget: &mut Budget) -> Result<()> {
    let read_text = |budget: &mut Budget, path: &str| -> Result<String> {
        String::from_utf8(budget.read(&root.join(path), 1024 * 1024)?)
            .map_err(|_| "Source version file must be UTF-8")
    };
    let cargo = read_text(budget, "Cargo.toml")?;
    let section = cargo
        .split_once("[workspace.package]")
        .ok_or("Workspace version is missing")?
        .1
        .split('[')
        .next()
        .ok_or("Workspace version is missing")?;
    let mut found = vec![assignment(section, "version")?];
    for path in [
        "package.json",
        "apps/desktop/package.json",
        "apps/desktop/src-tauri/tauri.conf.json",
    ] {
        let value: Value = serde_json::from_str(&read_text(budget, path)?)
            .map_err(|_| "Invalid source version JSON")?;
        found.push(
            value
                .get("version")
                .and_then(Value::as_str)
                .ok_or("Source JSON version is missing")?
                .to_owned(),
        );
    }
    let toc = read_text(budget, "addons/AzerothNavigator/AzerothNavigator.toc")?;
    let toc_versions: Vec<_> = toc
        .lines()
        .filter_map(|line| line.strip_prefix("## Version:").map(str::trim))
        .collect();
    if toc_versions.len() != 1 {
        return Err("Addon version is missing or duplicated");
    }
    found.push(toc_versions[0].to_owned());
    found.push(assignment(
        &read_text(budget, "addons/AzerothNavigator/Core/Version.lua")?,
        "addon",
    )?);
    let lock = read_text(budget, "Cargo.lock")?;
    let expected = BTreeSet::from([
        "azeroth-app",
        "azeroth-cli",
        "azeroth-core",
        "azeroth-desktop",
        "azeroth-ingest",
        "azeroth-lua-harness",
        "azeroth-local-data",
        "azeroth-storage",
    ]);
    let mut packages = BTreeSet::new();
    for entry in lock.split("[[package]]").skip(1) {
        let name = assignment(entry, "name")?;
        if name.starts_with("azeroth-") {
            if !packages.insert(name.clone()) {
                return Err("Duplicate internal lockfile package");
            }
            found.push(assignment(entry, "version")?);
        }
    }
    if packages.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected
        || found.iter().any(|v| v != version)
    {
        return Err("Requested release version differs from application, addon or lockfile");
    }
    Ok(())
}

#[derive(Deserialize)]
struct PackManifest {
    sources: FileRef,
    entries: Vec<PackEntry>,
}
#[derive(Deserialize)]
struct PackEntry {
    dataset: FileRef,
    assets: Vec<FileRef>,
}
#[derive(Deserialize)]
struct PackSources {
    sources: Vec<SourceRecord>,
}
#[derive(Default)]
struct Catalog {
    admitted: BTreeMap<String, azeroth_core::source_admission::Source>,
    records: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    revisions: BTreeSet<String>,
    sources: BTreeSet<String>,
    files: BTreeMap<String, FileRef>,
    /// Release source id to its evidenced origin edition.
    origins: BTreeMap<String, String>,
}
fn catalog_dataset(catalog: &mut Catalog, dataset: &Envelope<ReferenceDataset>) -> Result<()> {
    if !catalog
        .revisions
        .insert(dataset.payload.revision.as_str().to_owned())
    {
        return Err("Duplicate dataset revision across release packs");
    }
    for (domain, rows) in azeroth_core::content_v2::records::reference_records(&dataset.payload)? {
        for (identity, sources) in rows {
            if catalog
                .records
                .entry(domain.clone())
                .or_default()
                .insert(identity, sources)
                .is_some()
            {
                return Err("Duplicate record identity across release packs");
            }
        }
    }
    if catalog.records.values().map(BTreeMap::len).sum::<usize>() > release::MAX_RECORDS {
        return Err("Release catalog exceeds record limit");
    }
    Ok(())
}
fn checked_ref_bytes(
    budget: &mut Budget,
    root: &Path,
    reference: &FileRef,
    limit: usize,
) -> Result<Vec<u8>> {
    let bytes = budget.read(&root.join(reference.path.as_str()), limit)?;
    if bytes.len() as u64 != reference.bytes || hash(&bytes) != reference.sha256.as_str() {
        return Err("Pack bytes changed after verification");
    }
    Ok(bytes)
}
fn production_file(root: &Path, pack_root: &Path, reference: &FileRef, tree: &Tree) -> Result<()> {
    let path = pack_root.join(reference.path.as_str());
    let relative = relative(root, &path)?;
    if !tree.files.contains(relative.as_str()) {
        return Err("Pack files must be tracked production files outside docs and release");
    }
    checked_path(&path, false)
}
pub(crate) fn v2_catalog_records(
    content: &azeroth_core::content_v2::ReferenceContent,
) -> Result<azeroth_core::release_party::ConcreteCatalog> {
    azeroth_core::content_v2::records::content_records(content)
}
fn pack_file(
    root: &Path,
    pack_root: &Path,
    reference: &FileRef,
    tree: &Tree,
    catalog: &mut Catalog,
) -> Result<()> {
    production_file(root, pack_root, reference, tree)?;
    let mut reference = reference.clone();
    reference.path = relative(root, &pack_root.join(reference.path.as_str()))?;
    if let Some(previous) = catalog
        .files
        .insert(reference.path.as_str().to_owned(), reference.clone())
        && previous != reference
    {
        return Err("Conflicting release material file references");
    }
    Ok(())
}
#[derive(Deserialize)]
struct PackV2 {
    sources: FileRef,
    catalog: FileRef,
    assets: Vec<FileRef>,
    party_support: Option<FileRef>,
    supporting_files: Vec<FileRef>,
}
/// Content binds only to the code-side frozen target of schema 1 and 2, or the
/// world-data target file of schema 3; a pending (null) or self-chosen target
/// blocks it.
fn release_target<'a>(
    inventory: &'a Inventory,
    policy: &ReleasePolicy<'_>,
) -> Result<&'a release::Target> {
    let target = inventory
        .target
        .as_ref()
        .ok_or("Release target is pending; content cannot be bound to it")?;
    if Some(target) != release::expected_target_with(inventory.schema_version, policy).as_ref() {
        return Err("Release target differs from the frozen target");
    }
    Ok(target)
}
/// Schema 3 v2 packs: the strict coverage count is classified under the
/// accepted-origin policy, and only findings it does not accept block.
fn classify_v2(
    pack: &release::PackRef,
    content: &azeroth_core::content_v2::ReferenceContent,
    sources: &[SourceRecord],
    policy: &ReleasePolicy<'_>,
    strict_gaps: u64,
    classified: &mut Vec<PackClassification>,
) -> Result<()> {
    let classification = azeroth_core::origin_policy::classify(content, sources, policy.origins)?;
    if classification.total() != strict_gaps {
        return Err("Pack coverage classification differs from its verified gap count");
    }
    let remaining = classification.remaining_gaps;
    let relabelled = classification.relabelled_evidence;
    classified.push(PackClassification {
        manifest: pack.manifest.path.as_str().to_owned(),
        classification,
    });
    if relabelled != 0 {
        return Err("Release pack evidence claims more than its source origin allows");
    }
    if remaining != 0 {
        return Err("Release pack has coverage gaps the accepted-origin policy does not accept");
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn bind_v2_pack(
    root: &Path,
    pack: &release::PackRef,
    inventory: &Inventory,
    policy: &ReleasePolicy<'_>,
    strict_gaps: u64,
    tree: &Tree,
    budget: &mut Budget,
    catalog: &mut Catalog,
    classified: &mut Vec<PackClassification>,
    bytes: &[u8],
) -> Result<()> {
    let manifest_path = root.join(pack.manifest.path.as_str());
    let pack_root = manifest_path.parent().ok_or("Invalid v2 pack location")?;
    let manifest: PackV2 = serde_json::from_slice(bytes).map_err(|_| "Invalid v2 pack manifest")?;
    for file in std::iter::once(&manifest.sources)
        .chain(std::iter::once(&manifest.catalog))
        .chain(&manifest.assets)
        .chain(&manifest.party_support)
        .chain(&manifest.supporting_files)
    {
        pack_file(root, pack_root, file, tree, catalog)?;
    }
    let sources: PackSources = serde_json::from_slice(&checked_ref_bytes(
        budget,
        pack_root,
        &manifest.sources,
        crate::pack::MAX_SOURCES_BYTES,
    )?)
    .map_err(|_| "Invalid v2 sources")?;
    for source in &sources.sources {
        admission::pack_source(source, &catalog.admitted)?;
        let accepted = inventory
            .sources
            .iter()
            .find(|s| s.id == source.id.as_str())
            .ok_or("V2 source is not accepted")?;
        if source.verified_revision.value().map(|r| r.as_str()) != Some(accepted.revision.as_str())
            || source.offline_usable.value() != Some(&true)
        {
            return Err("V2 source revision or offline permission differs");
        }
        // Schema 3 accepts facts by the pack source's editions, so they must name
        // the origin the release evidence records; neither can be relabelled alone.
        if release::is_world_schema(inventory.schema_version)
            && !catalog
                .origins
                .get(source.id.as_str())
                .is_some_and(|origin| {
                    source
                        .editions
                        .value()
                        .is_some_and(|e| e.iter().any(|e| e.as_str() == origin))
                })
        {
            return Err("V2 source editions differ from the release source origin");
        }
        catalog.sources.insert(source.id.as_str().to_owned());
    }
    let content = azeroth_core::content_v2::decode(&checked_ref_bytes(
        budget,
        pack_root,
        &manifest.catalog,
        crate::pack::v2::MAX_CATALOG_BYTES,
    )?)
    .map_err(|_| "Invalid release v2 catalog")?;
    let data = &content.reference;
    if data.fixture || data.namespace != DatasetNamespace::Reference {
        return Err("V2 catalog target differs or contains fixtures");
    }
    // Bound after integrity and fixture refusals, so those still report first.
    let target = release_target(inventory, policy)?;
    if data.game.edition.as_str() != target.edition
        || data.game.build.as_str() != format!("{}.{}", target.version, target.build)
        || data.game.interface.as_str() != target.interface.to_string()
    {
        return Err("V2 catalog target differs or contains fixtures");
    }
    if pack.dataset_revisions != [data.payload.revision.as_str()]
        || !catalog
            .revisions
            .insert(data.payload.revision.as_str().to_owned())
    {
        return Err("V2 catalog revisions differ or duplicate another pack");
    }
    require_complete_v2_facts(&content)?;
    if release::is_world_schema(inventory.schema_version) {
        classify_v2(
            pack,
            &content,
            &sources.sources,
            policy,
            strict_gaps,
            classified,
        )?;
    }
    let mut records = v2_catalog_records(&content)?;
    if let Some(support) = &manifest.party_support {
        let matrix = azeroth_core::release_party::SupportMatrix::decode(&checked_ref_bytes(
            budget,
            pack_root,
            support,
            release::MAX_RELEASE_BYTES,
        )?)
        .map_err(|_| "Invalid release party support")?;
        if matrix.target.as_ref() != Some(target) {
            return Err("Party support target differs from the frozen target");
        }
        let rows = matrix
            .bind(manifest.catalog.sha256.as_str(), &records)
            .map_err(|_| "Party support does not bind actual catalog dependencies")?;
        records.insert("party".into(), rows);
    }
    for (domain, rows) in records {
        for (identity, sources) in rows {
            if catalog
                .records
                .entry(domain.clone())
                .or_default()
                .insert(identity, sources)
                .is_some()
            {
                return Err("Duplicate record across v2 release packs");
            }
        }
    }
    if catalog.records.values().map(BTreeMap::len).sum::<usize>() > release::MAX_RECORDS {
        return Err("Release catalog exceeds record limit");
    }
    Ok(())
}

/// Verify and bind every release pack. Schema 3 pack classifications are
/// appended to `classified` even when a later check refuses the pack.
fn packs(
    root: &Path,
    inventory: &Inventory,
    evidence: &Evidence,
    policy: &ReleasePolicy<'_>,
    tree: &Tree,
    budget: &mut Budget,
    classified: &mut Vec<PackClassification>,
) -> Result<(usize, Catalog)> {
    let accepted: BTreeMap<_, _> = inventory
        .sources
        .iter()
        .map(|s| (s.id.as_str(), s.revision.as_str()))
        .collect();
    let mut catalog = Catalog {
        admitted: admission::load(root, inventory, evidence, budget)?,
        origins: evidence
            .sources
            .iter()
            .map(|s| (s.id.clone(), s.origin_edition.clone()))
            .collect(),
        ..Catalog::default()
    };
    let mut count = 0;
    for pack in &evidence.packs {
        let manifest_path = root.join(pack.manifest.path.as_str());
        if manifest_path
            .file_name()
            .is_none_or(|n| n != "manifest.json")
        {
            return Err("Release pack must name manifest.json");
        }
        if !tree.files.contains(pack.manifest.path.as_str()) {
            return Err("Pack manifest must be tracked production data");
        }
        let pack_root = manifest_path.parent().ok_or("Invalid pack location")?;
        pack_file(root, root, &pack.manifest, tree, &mut catalog)?;
        budget.verify(root, &pack.manifest)?;
        let manifest_bytes = checked_ref_bytes(
            budget,
            root,
            &pack.manifest,
            crate::pack::MAX_MANIFEST_BYTES,
        )?;
        // Charge each distinct material before invoking the full verifier. Multiple
        // passes over the same bounded bytes do not shrink the 1 GiB closure limit.
        let version = serde_json::from_slice::<Value>(&manifest_bytes)
            .map_err(|_| "Invalid pack manifest")?
            .get("schema_version")
            .and_then(Value::as_u64);
        if version == Some(2) {
            let manifest: PackV2 =
                serde_json::from_slice(&manifest_bytes).map_err(|_| "Invalid v2 manifest")?;
            for file in std::iter::once(&manifest.sources)
                .chain(std::iter::once(&manifest.catalog))
                .chain(&manifest.assets)
                .chain(&manifest.party_support)
                .chain(&manifest.supporting_files)
            {
                budget.verify(pack_root, file)?;
            }
        } else {
            let manifest: PackManifest =
                serde_json::from_slice(&manifest_bytes).map_err(|_| "Invalid manifest")?;
            budget.verify(pack_root, &manifest.sources)?;
            for entry in &manifest.entries {
                for file in std::iter::once(&entry.dataset).chain(&entry.assets) {
                    budget.verify(pack_root, file)?;
                }
            }
        }
        // Existing verifier is authoritative for strict fields, canonical bytes,
        // exact trees, distribution, coordinates/references and PNG verification.
        let report = crate::pack::verify(pack_root)?;
        // Schema 3 classifies v2 gaps under the accepted-origin policy in
        // bind_v2_pack; every other combination keeps the strict zero.
        let policy_classified =
            version == Some(2) && release::is_world_schema(inventory.schema_version);
        if report.fixtures != 0
            || (!policy_classified && report.coverage_gaps != 0)
            || !report.distribution_eligible
        {
            return Err(
                "Release pack has fixture content, coverage gaps or ineligible distribution",
            );
        }
        if version == Some(2) {
            bind_v2_pack(
                root,
                pack,
                inventory,
                policy,
                report.coverage_gaps,
                tree,
                budget,
                &mut catalog,
                classified,
                &manifest_bytes,
            )?;
            count += 1;
            continue;
        }
        let manifest: PackManifest =
            serde_json::from_slice(&manifest_bytes).map_err(|_| "Invalid pack manifest")?;
        pack_file(root, pack_root, &manifest.sources, tree, &mut catalog)?;
        let sources: PackSources = serde_json::from_slice(&checked_ref_bytes(
            budget,
            pack_root,
            &manifest.sources,
            crate::pack::MAX_SOURCES_BYTES,
        )?)
        .map_err(|_| "Invalid pack sources")?;
        for source in sources.sources {
            admission::pack_source(&source, &catalog.admitted)?;
            let Some(revision) = accepted.get(source.id.as_str()) else {
                return Err("Pack source is not in the accepted inventory");
            };
            if source.verified_revision.value().map(|r| r.as_str()) != Some(*revision) {
                return Err("Pack source revision differs from accepted source");
            }
            if source.offline_usable.value() != Some(&true) {
                return Err("Release pack source does not permit offline use");
            }
            catalog.sources.insert(source.id.as_str().to_owned());
        }
        let mut revisions = BTreeSet::new();
        for entry in manifest.entries {
            pack_file(root, pack_root, &entry.dataset, tree, &mut catalog)?;
            for asset in &entry.assets {
                pack_file(root, pack_root, asset, tree, &mut catalog)?;
            }
            let bytes = checked_ref_bytes(
                budget,
                pack_root,
                &entry.dataset,
                azeroth_core::contract::MAX_DATASET_BYTES,
            )?;
            let dataset = decode_envelope::<ReferenceDataset>(
                std::str::from_utf8(&bytes).map_err(|_| "Dataset is not UTF-8")?,
            )
            .map_err(|_| "Invalid release dataset")?;
            if dataset.fixture || dataset.namespace != DatasetNamespace::Reference {
                return Err("Release dataset differs from frozen target or contains fixtures");
            }
            let target = release_target(inventory, policy)?;
            if dataset.game.edition.as_str() != target.edition
                || dataset.game.build.as_str() != format!("{}.{}", target.version, target.build)
                || dataset.game.interface.as_str() != target.interface.to_string()
            {
                return Err("Release dataset differs from frozen target or contains fixtures");
            }
            revisions.insert(dataset.payload.revision.as_str().to_owned());
            catalog_dataset(&mut catalog, &dataset)?;
        }
        if revisions != pack.dataset_revisions.iter().cloned().collect() {
            return Err("Declared pack revisions differ from verified content");
        }
        count += 1;
    }
    if catalog.sources != accepted.keys().map(|s| (*s).to_owned()).collect() {
        return Err("Accepted source set differs from shipped pack sources");
    }
    Ok((count, catalog))
}
fn bind_records(
    report: &mut Readiness,
    inventory: &Inventory,
    evidence: &Evidence,
    catalog: &Catalog,
) {
    let coverage: BTreeMap<_, _> = evidence
        .coverage
        .iter()
        .map(|c| (c.unit_id.as_str(), c))
        .collect();
    for domain in &inventory.domains {
        let Some(records) = catalog.records.get(&domain.domain) else {
            report.block(format!(
                "No validated pack record binding for mandatory domain: {}",
                domain.domain
            ));
            continue;
        };
        let mut required = BTreeSet::new();
        for unit in &domain.units {
            let mut sources = BTreeSet::new();
            if let Some(ids) = &unit.required_record_ids {
                for id in ids {
                    required.insert(id.as_str());
                    if let Some(record_sources) = records.get(id) {
                        if record_sources.is_empty() {
                            report.block(format!(
                                "Required record lacks source provenance: {}",
                                unit.id
                            ));
                        }
                        sources.extend(record_sources.iter().map(String::as_str));
                    } else {
                        report.block(format!(
                            "Required record is absent from verified pack content: {}",
                            unit.id
                        ));
                    }
                }
            }
            if let Some(row) = coverage.get(unit.id.as_str())
                && sources != row.source_ids.iter().map(String::as_str).collect()
            {
                report.block(format!(
                    "Coverage sources differ from verified dataset sources: {}",
                    unit.id
                ));
            }
        }
        if required != records.keys().map(String::as_str).collect() {
            report.block(format!(
                "Inventory does not exactly enumerate shipped records: {}",
                domain.domain
            ));
        }
    }
}

struct Inputs {
    root: PathBuf,
    budget: Budget,
    inventory_bytes: Vec<u8>,
    inventory: Inventory,
    evidence: Option<Evidence>,
    head: String,
    candidate: Tree,
    world: WorldInputs,
}
fn inputs(options: &Options) -> Result<Inputs> {
    release::is_stable(&options.version)?;
    let root = root_path(&options.root)?;
    let mut budget = Budget::default();
    versions(&root, &options.version, &mut budget)?;
    let inventory_bytes = budget.read(&root.join(INVENTORY_PATH), release::MAX_RELEASE_BYTES)?;
    let inventory = release::decode_inventory(&inventory_bytes)?;
    let world = world_inputs(&root, &inventory, &mut budget)?;
    let selected = options
        .evidence
        .as_deref()
        .unwrap_or(Path::new(EVIDENCE_PATH));
    let selected = relative(&root, selected)?;
    let evidence_path = root.join(selected.as_str());
    let evidence = match fs::symlink_metadata(&evidence_path) {
        Ok(_) => Some(release::decode_evidence(
            &budget.read(&evidence_path, release::MAX_RELEASE_BYTES)?,
        )?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("Acceptance evidence cannot be inspected"),
    };
    let head = String::from_utf8(git(&root, &["rev-parse", "--verify", "HEAD"])?)
        .map_err(|_| "Invalid Git source identity")?;
    let head = head.trim().to_owned();
    if head.len() != 40 || !head.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Unsupported Git source identity");
    }
    let candidate = tree(&root, "HEAD")?;
    Ok(Inputs {
        root,
        budget,
        inventory_bytes,
        inventory,
        evidence,
        head,
        candidate,
        world,
    })
}
/// The receipt attests the tested candidate, like the acceptance evidence, so a
/// later documentation-only commit keeps it valid.
fn hybrid_receipt(
    root: &Path,
    budget: &mut Budget,
    policy: &DeliveryPolicy,
    tested: &str,
    production_sha256: &str,
) -> Result<()> {
    if policy
        .local_content
        .as_ref()
        .is_none_or(|l| l.expected_catalogs.is_empty())
    {
        return Err("Hybrid delivery has no declared expected local catalog digests");
    }
    let path = root.join(RECEIPT_PATH);
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("Hybrid delivery requires a signed local-catalog receipt");
        }
        Err(_) => return Err("Local-catalog receipt cannot be inspected"),
    }
    if tested.is_empty() {
        return Err("Hybrid receipt requires a tested source binding");
    }
    let bytes = budget.read(&path, 128 * 1024)?;
    azeroth_local_data::receipts::verify_hybrid(&bytes, policy, tested, production_sha256)?;
    Ok(())
}
fn production_status(root: &Path) -> Result<Vec<u8>> {
    git(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
            ":(exclude)docs/**",
            ":(exclude)release/**",
        ],
    )
}
pub fn report(options: &Options) -> Result<Report> {
    let Inputs {
        root,
        mut budget,
        inventory_bytes,
        inventory,
        evidence,
        head,
        candidate,
        world,
    } = inputs(options)?;
    let policy = world.policy();
    let tested = evidence
        .as_ref()
        .and_then(|e| e.tested_source_commit.as_deref())
        .unwrap_or("");
    let mut readiness = release::assess_with(
        &options.version,
        &inventory,
        evidence.as_ref(),
        &Bindings {
            inventory_sha256: &hash(&inventory_bytes),
            production_sha256: &candidate.fingerprint,
            tested_source_commit: tested,
        },
        &policy,
    )?;
    let status = production_status(&root)?;
    // Historical repositories without a delivery policy retain their original
    // bundled rules. An explicit policy is strict and production-fingerprinted.
    if root.join("config/release-delivery-v1.json").exists() {
        let bytes = budget.read(&root.join("config/release-delivery-v1.json"), 128 * 1024)?;
        let policy = azeroth_local_data::receipts::policy(&bytes)?;
        match policy.delivery_mode {
            DeliveryMode::LocalCatalog => readiness.block("Local-catalog delivery requires private inventory, receipt and packaging integration; unavailable in this release-policy version"),
            DeliveryMode::Hybrid => {
                if let Err(message) =
                    hybrid_receipt(&root, &mut budget, &policy, tested, &candidate.fingerprint)
                {
                    readiness.block(message);
                }
            }
            DeliveryMode::BundledContent => {}
        }
    }
    if !status.is_empty() {
        readiness.block("Production working tree contains tracked or untracked changes");
    }
    if let Err(message) = addon_inputs(&root, &candidate) {
        readiness.block(message);
    }
    if tested.len() == 40 && tested.bytes().all(|b| b.is_ascii_hexdigit()) {
        if git(&root, &["merge-base", "--is-ancestor", tested, "HEAD"]).is_err()
            || tree(&root, tested).map_or(true, |t| t.fingerprint != candidate.fingerprint)
        {
            readiness
                .block("Tested source is missing, unrelated or has different production bytes");
        }
    } else if !evidence
        .as_ref()
        .is_some_and(|e| e.tested_source_commit.is_none() && e.production_sha256.is_none())
    {
        readiness.block("Tested source commit is missing or invalid");
    }
    let mut verified_packs = 0;
    let mut classifications = vec![];
    if let Some(evidence) = &evidence {
        for reference in release::evidence_files(&inventory, evidence) {
            if let Err(message) = budget.verify(&root, reference) {
                readiness.block(message);
                break;
            }
        }
        match packs(
            &root,
            &inventory,
            evidence,
            &policy,
            &candidate,
            &mut budget,
            &mut classifications,
        ) {
            Ok((count, catalog)) => {
                verified_packs = count;
                bind_records(&mut readiness, &inventory, evidence, &catalog);
                if let Err(message) = materials::selected_files(
                    &root,
                    &inventory,
                    evidence,
                    &policy,
                    &candidate,
                    &catalog,
                    &mut budget,
                ) {
                    readiness.block(message);
                }
            }
            Err(message) => readiness.block(message),
        }
    }
    let world_data = (release::is_world_schema(inventory.schema_version))
        .then(|| world_report(&world, &inventory, evidence.as_ref(), classifications));
    Ok(Report {
        readiness,
        candidate_source_commit: head,
        production_sha256: candidate.fingerprint,
        inventory_sha256: hash(&inventory_bytes),
        verified_evidence_files: budget.verified.len(),
        verified_content_packs: verified_packs,
        world_data,
    })
}
fn world_report(
    world: &WorldInputs,
    inventory: &Inventory,
    evidence: Option<&Evidence>,
    packs: Vec<PackClassification>,
) -> WorldDataReport {
    let origins = world.origins.as_ref();
    WorldDataReport {
        target: world.target.clone(),
        origin_policy_sha256: world.origin_sha256.clone(),
        accepted_origins_by_domain: origins
            .map(|o| {
                o.domains
                    .iter()
                    .map(|(d, e)| (d.clone(), e.iter().map(|e| e.as_str().to_owned()).collect()))
                    .collect()
            })
            .unwrap_or_default(),
        accept_inferred: origins.map(|o| o.accept_inferred),
        accept_unknowns: origins.map(|o| o.accept_unknowns),
        origin_accepted_sources: evidence
            .map(|e| {
                release::origin_accepted_sources(inventory, e, &world.policy())
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default(),
        accepted_origins: packs
            .iter()
            .map(|p| p.classification.accepted_origins)
            .sum(),
        accepted_unknowns: packs
            .iter()
            .map(|p| p.classification.accepted_unknowns)
            .sum(),
        remaining_gaps: packs.iter().map(|p| p.classification.remaining_gaps).sum(),
        packs,
    }
}

fn require_complete_v2_facts(
    content: &azeroth_core::content_v2::ReferenceContent,
) -> std::result::Result<(), &'static str> {
    // Full target coverage requires a fact declaration per base record, even
    // when the record cannot currently supply a known fact. Unknowns are gaps.
    if content.node_facts.len() != content.reference.payload.nodes.len()
        || content.equipment.len() != content.reference.payload.items.len()
        || content.quest_facts.len() != content.reference.payload.quests.len()
        || content.acquisition_facts.len() != content.reference.payload.acquisitions.len()
    {
        return Err("V2 catalog lacks mandatory node, equipment, quest or acquisition facts");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_core::release::{Completeness, DomainInventory, Status, Unit, UnitCoverage};
    use serde_json::json;
    fn sample() -> (Catalog, Inventory, Evidence) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text =
            fs::read_to_string(root.join("fixtures/contracts/v1/reference-dataset.json")).unwrap();
        // This isolated mapping fixture explicitly attributes quest/objective
        // facts; the shared v1 example deliberately leaves their provenance empty.
        let mut value: Value = serde_json::from_str(&text).unwrap();
        for collection in ["quests", "objectives"] {
            for record in value["payload"][collection].as_array_mut().unwrap() {
                record["evidence"] = json!(["demo-evidence-1"]);
            }
        }
        let dataset = decode_envelope::<ReferenceDataset>(&value.to_string()).unwrap();
        let mut catalog = Catalog::default();
        catalog_dataset(&mut catalog, &dataset).unwrap();
        let records = &catalog.records["quests"];
        let mut inventory =
            release::decode_inventory(&fs::read(root.join(INVENTORY_PATH)).unwrap()).unwrap();
        inventory.domains = vec![DomainInventory {
            domain: "quests".into(),
            completeness: Completeness::Complete,
            enumeration_evidence: vec![],
            units: vec![Unit {
                id: "synthetic-only".into(),
                scope: "Isolated mapping test".into(),
                required_record_ids: Some(records.keys().cloned().collect()),
            }],
            out_of_scope: vec![],
        }];
        let mut evidence =
            release::decode_evidence(&fs::read(root.join(EVIDENCE_PATH)).unwrap()).unwrap();
        evidence.coverage = vec![UnitCoverage {
            unit_id: "synthetic-only".into(),
            status: Status::Pass,
            included_record_ids: records.keys().cloned().collect(),
            source_ids: records
                .values()
                .flat_map(|s| s.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            unresolved_conflicts: 0,
            unsupported_corrections: 0,
            validation_evidence: vec![],
        }];
        (catalog, inventory, evidence)
    }
    fn mapping_report() -> Readiness {
        Readiness {
            report_version: 1,
            version: "1.0.0".into(),
            stable: true,
            stable_ready: true,
            candidate_ready: true,
            pending: vec![],
            publication_allowed: true,
            blockers: vec![],
        }
    }
    #[test]
    fn preparation_wrapper_preserves_source_evidence_but_cannot_publish() {
        for phase in [release::BuildPhase::Prebuild, release::BuildPhase::Draft] {
            let mut readiness = mapping_report();
            readiness.report_version = 4;
            let prepared = BuildReport::new(
                Report {
                    readiness,
                    candidate_source_commit: "a".repeat(40),
                    production_sha256: "b".repeat(64),
                    inventory_sha256: "c".repeat(64),
                    verified_evidence_files: 7,
                    verified_content_packs: 2,
                    world_data: None,
                },
                phase,
            );
            assert!(prepared.build_ready);
            assert!(
                !prepared.publication_allowed && !prepared.report.readiness.publication_allowed
            );
            let json = serde_json::to_value(prepared).unwrap();
            assert_eq!(json["report"]["candidate_source_commit"], "a".repeat(40));
            assert_eq!(json["report"]["verified_evidence_files"], 7);
            assert_eq!(json["report"]["verified_content_packs"], 2);
            assert_eq!(json["publication_allowed"], false);
            assert_eq!(json["report"]["publication_allowed"], false);
        }
    }
    #[test]
    fn full_v2_requires_node_facts_and_source_less_claims_cannot_borrow_other_records() {
        let mut raw: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v2/reference-content.json"
        ))
        .unwrap();
        let content = azeroth_core::content_v2::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
        assert!(require_complete_v2_facts(&content).is_err());
        raw["node_facts"] = Value::Array(
            raw["reference"]["payload"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| json!({"node":n["id"],"evidence":["demo-evidence-1"]}))
                .collect(),
        );
        let content = azeroth_core::content_v2::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
        require_complete_v2_facts(&content).unwrap();
        let (mut catalog, inventory, evidence) = sample();
        catalog
            .records
            .get_mut("quests")
            .unwrap()
            .values_mut()
            .next()
            .unwrap()
            .clear();
        let mut result = mapping_report();
        bind_records(&mut result, &inventory, &evidence, &catalog);
        assert!(
            result
                .blockers
                .iter()
                .any(|v| v.contains("lacks source provenance"))
        );
    }
    #[test]
    fn repeated_material_reads_charge_distinct_closure_bytes_once() {
        let mut budget = Budget::default();
        let path = Path::new("/synthetic/catalog.json");
        budget.charge(path, MAX_TOTAL_BYTES).unwrap();
        budget.charge(path, MAX_TOTAL_BYTES).unwrap();
        assert_eq!(budget.bytes, MAX_TOTAL_BYTES);
        assert!(budget.charge(path, MAX_TOTAL_BYTES - 1).is_err());
        assert!(
            budget
                .charge(Path::new("/synthetic/extra.json"), 1)
                .is_err()
        );
    }
    #[test]
    fn canonical_record_mapping_matches_actual_typed_identity_and_source_sets() {
        let (catalog, inventory, evidence) = sample();
        let mut result = mapping_report();
        bind_records(&mut result, &inventory, &evidence, &catalog);
        assert!(result.blockers.is_empty(), "{:?}", result.blockers);
        assert!(
            catalog.records["equipment"]
                .keys()
                .any(|id| id == "[\"demo-r1\",\"items\",\"demo-item-1\",\"base\"]")
        );
        assert!(
            catalog.records["quests"]
                .keys()
                .all(|key| serde_json::from_str::<Value>(key).unwrap().is_array())
        );
    }
    #[test]
    fn claimed_missing_records_omissions_and_wrong_sources_are_rejected() {
        let (catalog, mut inventory, mut evidence) = sample();
        inventory.domains[0].units[0]
            .required_record_ids
            .as_mut()
            .unwrap()
            .push("[\"demo-r1\",\"quests\",\"missing\"]".into());
        let mut result = mapping_report();
        bind_records(&mut result, &inventory, &evidence, &catalog);
        assert!(!result.publication_allowed);
        let (_, mut inventory, _) = sample();
        inventory.domains[0].units[0]
            .required_record_ids
            .as_mut()
            .unwrap()
            .pop();
        let mut result = mapping_report();
        bind_records(&mut result, &inventory, &evidence, &catalog);
        assert!(!result.publication_allowed);
        let (_, inventory, _) = sample();
        evidence.coverage[0].source_ids = vec!["unrelated".into()];
        let mut result = mapping_report();
        bind_records(&mut result, &inventory, &evidence, &catalog);
        assert!(!result.publication_allowed);
    }
    #[test]
    fn absent_role_party_dungeon_contracts_never_become_automatic_passes() {
        let (catalog, mut inventory, evidence) = sample();
        for domain in ["class_roles", "party", "dungeons"] {
            inventory.domains[0].domain = domain.into();
            let mut result = mapping_report();
            bind_records(&mut result, &inventory, &evidence, &catalog);
            assert!(result.blockers[0].contains("No validated pack record binding"));
            assert!(!result.publication_allowed);
        }
    }
}
