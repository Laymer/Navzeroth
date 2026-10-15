//! Release gate I/O: bounded evidence, exact Git/source binding and verified packs.
//! All referenced game/source material is data; no Lua or SQL is executed.
pub mod materials;
use azeroth_core::{
    contract::{Envelope, ReferenceDataset, decode_envelope},
    release::{self, Bindings, Evidence, FileRef, Inventory, Readiness},
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
    pub verified_evidence_files: usize,
    pub verified_content_packs: usize,
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
    records: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    revisions: BTreeSet<String>,
    sources: BTreeSet<String>,
    files: BTreeMap<String, FileRef>,
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
fn bind_v2_pack(
    root: &Path,
    pack: &release::PackRef,
    inventory: &Inventory,
    tree: &Tree,
    budget: &mut Budget,
    catalog: &mut Catalog,
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
    for source in sources.sources {
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
    if data.fixture
        || data.namespace != DatasetNamespace::Reference
        || data.game.edition.as_str() != inventory.target.edition
        || data.game.build.as_str()
            != format!("{}.{}", inventory.target.version, inventory.target.build)
        || data.game.interface.as_str() != inventory.target.interface.to_string()
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
    // Full target coverage requires a fact declaration per base record, even
    // when the record cannot currently supply a known fact. Unknowns are gaps.
    if content.equipment.len() != data.payload.items.len()
        || content.quest_facts.len() != data.payload.quests.len()
        || content.acquisition_facts.len() != data.payload.acquisitions.len()
    {
        return Err("V2 catalog lacks mandatory equipment, quest or acquisition facts");
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

fn packs(
    root: &Path,
    inventory: &Inventory,
    evidence: &Evidence,
    tree: &Tree,
    budget: &mut Budget,
) -> Result<(usize, Catalog)> {
    let accepted: BTreeMap<_, _> = inventory
        .sources
        .iter()
        .map(|s| (s.id.as_str(), s.revision.as_str()))
        .collect();
    let mut catalog = Catalog::default();
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

        if report.fixtures != 0 || report.coverage_gaps != 0 || !report.distribution_eligible {
            return Err(
                "Release pack has fixture content, coverage gaps or ineligible distribution",
            );
        }
        if serde_json::from_slice::<Value>(&manifest_bytes)
            .map_err(|_| "Invalid pack manifest")?
            .get("schema_version")
            .and_then(Value::as_u64)
            == Some(2)
        {
            bind_v2_pack(
                root,
                pack,
                inventory,
                tree,
                budget,
                &mut catalog,
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
            if dataset.fixture
                || dataset.namespace != DatasetNamespace::Reference
                || dataset.game.edition.as_str() != inventory.target.edition
                || dataset.game.build.as_str()
                    != format!("{}.{}", inventory.target.version, inventory.target.build)
                || dataset.game.interface.as_str() != inventory.target.interface.to_string()
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
                && !sources.is_empty()
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
}
fn inputs(options: &Options) -> Result<Inputs> {
    release::is_stable(&options.version)?;
    let root = root_path(&options.root)?;
    let mut budget = Budget::default();
    versions(&root, &options.version, &mut budget)?;
    let inventory_bytes = budget.read(&root.join(INVENTORY_PATH), release::MAX_RELEASE_BYTES)?;
    let inventory = release::decode_inventory(&inventory_bytes)?;
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
    })
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
    } = inputs(options)?;
    let tested = evidence
        .as_ref()
        .and_then(|e| e.tested_source_commit.as_deref())
        .unwrap_or("");
    let mut readiness = release::assess(
        &options.version,
        &inventory,
        evidence.as_ref(),
        &Bindings {
            inventory_sha256: &hash(&inventory_bytes),
            production_sha256: &candidate.fingerprint,
            tested_source_commit: tested,
        },
    )?;
    let status = production_status(&root)?;
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
    } else {
        readiness.block("Tested source commit is missing or invalid");
    }
    let mut verified_packs = 0;
    if let Some(evidence) = &evidence {
        for reference in release::evidence_files(&inventory, evidence) {
            if let Err(message) = budget.verify(&root, reference) {
                readiness.block(message);
                break;
            }
        }
        match packs(&root, &inventory, evidence, &candidate, &mut budget) {
            Ok((count, catalog)) => {
                verified_packs = count;
                bind_records(&mut readiness, &inventory, evidence, &catalog);
                if let Err(message) = materials::selected_files(
                    &root,
                    &inventory,
                    evidence,
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
    Ok(Report {
        readiness,
        candidate_source_commit: head,
        production_sha256: candidate.fingerprint,
        verified_evidence_files: budget.verified.len(),
        verified_content_packs: verified_packs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_core::release::{Completeness, DomainInventory, Status, Unit, UnitCoverage};
    fn sample() -> (Catalog, Inventory, Evidence) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text =
            fs::read_to_string(root.join("fixtures/contracts/v1/reference-dataset.json")).unwrap();
        let dataset = decode_envelope::<ReferenceDataset>(&text).unwrap();
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
            publication_allowed: true,
            blockers: vec![],
        }
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
        assert!(result.blockers.is_empty());
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
