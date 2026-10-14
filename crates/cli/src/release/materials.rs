//! Select inert release materials from verified pack and source contracts.
//! Permission/acceptance proofs are validated locally and never auto-published.
use super::*;
use azeroth_core::release::{Permission, SourceEvidence, Status, Target};
use std::{
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};

pub const MANIFEST: &str = "RELEASE_MATERIALS.json";
pub const MAX_MATERIAL_FILES: usize = 16_384;
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    ContentPack,
    SourceMaterial,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialFile {
    pub input: FileRef,
    pub archive_path: AssetPath,
    pub kind: Kind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMaterial {
    pub id: String,
    pub revision: String,
    pub origin_edition: String,
    pub origin_build: Option<String>,
    pub files: Vec<FileRef>,
    pub license_files: Vec<FileRef>,
    pub attribution_files: Vec<FileRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub version: String,
    pub candidate_source_commit: String,
    pub production_sha256: String,
    pub inventory_sha256: String,
    pub target: Target,
    pub packs: Vec<release::PackRef>,
    pub sources: Vec<SourceMaterial>,
    pub files: Vec<MaterialFile>,
}
impl Manifest {
    pub fn bytes(&self) -> Result<Vec<u8>> {
        let mut bytes =
            serde_json::to_vec_pretty(self).map_err(|_| "Cannot serialize material manifest")?;
        bytes.push(b'\n');
        if bytes.len() > release::MAX_RELEASE_BYTES {
            return Err("Material manifest exceeds its size limit");
        }
        Ok(bytes)
    }
}

pub struct Arguments {
    pub release: Options,
    pub output: Option<PathBuf>,
}
impl Arguments {
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let mut common = Vec::new();
        let mut output = None;
        let (pairs, remainder) = args.as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        for pair in pairs {
            if pair[0] == "--output" {
                if output.is_some() || pair[1].is_empty() || pair[1].to_str()?.starts_with("--") {
                    return None;
                }
                output = Some(PathBuf::from(&pair[1]));
            } else {
                common.extend_from_slice(pair);
            }
        }
        Some(Self {
            release: Options::parse(&common)?,
            output,
        })
    }
}

fn sorted(files: &[FileRef]) -> Vec<FileRef> {
    let mut result = files.to_vec();
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}
fn private_refs<'a>(inventory: &'a Inventory, evidence: &'a Evidence) -> Vec<&'a FileRef> {
    inventory
        .domains
        .iter()
        .flat_map(|d| &d.enumeration_evidence)
        .chain(evidence.checks.iter().flat_map(|c| &c.artifacts))
        .chain(
            evidence
                .coverage
                .iter()
                .flat_map(|c| &c.validation_evidence),
        )
        .chain(
            evidence
                .sources
                .iter()
                .flat_map(|s| s.permission_evidence.iter().chain(&s.target_evidence)),
        )
        .collect()
}
fn source_valid(source: &SourceEvidence) -> bool {
    let text =
        |v: &str| !v.trim().is_empty() && v.len() <= 4096 && !v.chars().any(char::is_control);
    source.redistribution == Permission::Permitted
        && source.target_validation == Status::Pass
        && text(&source.id)
        && text(&source.origin_edition)
        && source.origin_build.as_deref().is_none_or(text)
        && [
            &source.files,
            &source.license_files,
            &source.attribution_files,
            &source.permission_evidence,
            &source.target_evidence,
        ]
        .into_iter()
        .all(|files| {
            !files.is_empty()
                && files.len() <= release::MAX_UNITS
                && files.iter().map(|f| &f.path).collect::<BTreeSet<_>>().len() == files.len()
        })
}
pub(super) fn selected_files(
    root: &Path,
    inventory: &Inventory,
    evidence: &Evidence,
    tree: &Tree,
    catalog: &Catalog,
    budget: &mut Budget,
) -> Result<(Vec<MaterialFile>, Vec<SourceMaterial>)> {
    if evidence.sources.len() != inventory.sources.len()
        || inventory.sources.len() > release::MAX_UNITS
    {
        return Err("Material source inventory differs from accepted sources");
    }
    let accepted: BTreeMap<_, _> = inventory
        .sources
        .iter()
        .map(|s| (s.id.as_str(), s.revision.as_str()))
        .collect();
    if accepted.len() != inventory.sources.len()
        || accepted.values().any(|r| {
            r.len() != 40
                || !r
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    {
        return Err("Material source revisions are missing, duplicate or invalid");
    }
    let private = private_refs(inventory, evidence);
    let private_paths: BTreeSet<_> = private.iter().map(|f| f.path.as_str()).collect();
    let private_hashes: BTreeSet<_> = private
        .iter()
        .map(|f| (f.bytes, f.sha256.as_str()))
        .collect();
    let mut selected: BTreeMap<String, MaterialFile> = BTreeMap::new();
    let mut add = |reference: &FileRef, kind: Kind, budget: &mut Budget| -> Result<()> {
        if private_paths.contains(reference.path.as_str())
            || private_hashes.contains(&(reference.bytes, reference.sha256.as_str()))
        {
            return Err("Private validation or permission evidence cannot be shipped as material");
        }
        if !tree.files.contains(reference.path.as_str()) {
            return Err(
                "Release materials must be tracked production files outside docs and release",
            );
        }
        budget.verify(root, reference)?;
        let prefix = if kind == Kind::ContentPack {
            "content-packs"
        } else {
            "source-materials"
        };
        let archive_path = AssetPath::new(format!("{prefix}/{}", reference.path.as_str()))
            .map_err(|_| "Material archive path exceeds portable bounds")?;
        let material = MaterialFile {
            input: reference.clone(),
            archive_path,
            kind,
        };
        if let Some(previous) =
            selected.insert(material.archive_path.as_str().into(), material.clone())
            && previous != material
        {
            return Err("Conflicting material file references");
        }
        if selected.len() > MAX_MATERIAL_FILES {
            return Err("Too many release material files");
        }
        Ok(())
    };
    for reference in catalog.files.values() {
        add(reference, Kind::ContentPack, budget)?;
    }
    let mut sources = BTreeMap::new();
    for source in &evidence.sources {
        if accepted.get(source.id.as_str()) != Some(&source.revision.as_str())
            || !source_valid(source)
        {
            return Err(
                "Material source permission, provenance or target validation is incomplete",
            );
        }
        for proof in source
            .permission_evidence
            .iter()
            .chain(&source.target_evidence)
        {
            budget.verify(root, proof)?;
        }
        for reference in source
            .files
            .iter()
            .chain(&source.license_files)
            .chain(&source.attribution_files)
        {
            add(reference, Kind::SourceMaterial, budget)?;
        }
        let public = SourceMaterial {
            id: source.id.clone(),
            revision: source.revision.clone(),
            origin_edition: source.origin_edition.clone(),
            origin_build: source.origin_build.clone(),
            files: sorted(&source.files),
            license_files: sorted(&source.license_files),
            attribution_files: sorted(&source.attribution_files),
        };
        if sources.insert(source.id.clone(), public).is_some() {
            return Err("Duplicate material source");
        }
    }
    Ok((
        selected.into_values().collect(),
        sources.into_values().collect(),
    ))
}

/// Materials may be prepared for a preview without pretending live acceptance
/// passed. Source rights, target binding and every selected byte are mandatory.
pub fn plan(options: &Options) -> Result<Manifest> {
    let Inputs {
        root,
        mut budget,
        inventory_bytes,
        inventory,
        evidence,
        head,
        candidate,
    } = inputs(options)?;
    let evidence = evidence.ok_or("Material selection requires an evidence manifest")?;
    // Reuse the core's early schema/collection limits without treating missing
    // unrelated live checks as permission to skip source/material verification.
    release::assess(
        &options.version,
        &inventory,
        Some(&evidence),
        &Bindings {
            inventory_sha256: &hash(&inventory_bytes),
            production_sha256: &candidate.fingerprint,
            tested_source_commit: evidence.tested_source_commit.as_deref().unwrap_or(""),
        },
    )?;
    if inventory.target != Target::forever_70374() || evidence.target != inventory.target {
        return Err("Release material target differs from frozen build");
    }
    let empty =
        inventory.sources.is_empty() && evidence.sources.is_empty() && evidence.packs.is_empty();
    if !empty && evidence.inventory_sha256.as_str() != hash(&inventory_bytes) {
        return Err("Material evidence refers to a different inventory revision");
    }
    if evidence.packs.len() > release::MAX_UNITS
        || evidence
            .packs
            .iter()
            .map(|p| &p.manifest.path)
            .collect::<BTreeSet<_>>()
            .len()
            != evidence.packs.len()
        || evidence.packs.iter().any(|p| {
            p.dataset_revisions.is_empty()
                || p.dataset_revisions.iter().collect::<BTreeSet<_>>().len()
                    != p.dataset_revisions.len()
        })
    {
        return Err("Material pack inventory is duplicate or incomplete");
    }
    if !production_status(&root)?.is_empty() {
        return Err("Material source working tree is dirty or has untracked product files");
    }
    let (_, catalog) = packs(&root, &inventory, &evidence, &candidate, &mut budget)?;
    let (files, sources) = selected_files(
        &root,
        &inventory,
        &evidence,
        &candidate,
        &catalog,
        &mut budget,
    )?;
    let mut packs = evidence.packs.clone();
    for pack in &mut packs {
        pack.dataset_revisions.sort();
    }
    packs.sort_by(|a, b| a.manifest.path.cmp(&b.manifest.path));
    let manifest = Manifest {
        schema_version: 1,
        version: options.version.clone(),
        candidate_source_commit: head,
        production_sha256: candidate.fingerprint,
        inventory_sha256: hash(&inventory_bytes),
        target: inventory.target,
        packs,
        sources,
        files,
    };
    manifest.bytes()?;
    Ok(manifest)
}

fn copy_checked(source: &Path, destination: &Path, reference: &FileRef) -> Result<()> {
    checked_path(source, false)?;
    let mut input = File::open(source).map_err(|_| "Cannot open material input")?;
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|_| "Cannot create staged material")?;
    let mut digest = Sha256::new();
    let mut count = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|_| "Cannot read material input")?;
        if read == 0 {
            break;
        }
        count += read as u64;
        if count > reference.bytes {
            return Err("Material input changed during staging");
        }
        digest.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|_| "Cannot write staged material")?;
    }
    if count != reference.bytes || format!("{:x}", digest.finalize()) != reference.sha256.as_str() {
        return Err("Material input changed during staging");
    }
    output
        .sync_all()
        .map_err(|_| "Cannot flush staged material")
}

pub fn stage(options: &Options, output: &Path) -> Result<Manifest> {
    let manifest = plan(options)?; // No mutation before all inputs are validated.
    let output = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| "Current directory unavailable")?
            .join(output)
    };
    if output
        .components()
        .any(|p| matches!(p, Component::ParentDir))
    {
        return Err("Invalid material output path");
    }
    let parent = output
        .parent()
        .ok_or("Material output must have a parent")?;
    checked_path(parent, true)?;
    if fs::symlink_metadata(&output).is_ok() {
        return Err("Material output already exists");
    }
    if manifest.files.is_empty() {
        return Ok(manifest);
    }
    let temporary = parent.join(format!(
        ".release-materials-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary).map_err(|_| "Cannot create material staging directory")?;
    let result = (|| {
        let root = root_path(&options.root)?;
        for material in &manifest.files {
            let destination = temporary.join(material.archive_path.as_str());
            fs::create_dir_all(destination.parent().ok_or("Invalid material destination")?)
                .map_err(|_| "Cannot create material directories")?;
            copy_checked(
                &root.join(material.input.path.as_str()),
                &destination,
                &material.input,
            )?;
        }
        fs::write(temporary.join(MANIFEST), manifest.bytes()?)
            .map_err(|_| "Cannot write material manifest")?;
        if fs::symlink_metadata(&output).is_ok() {
            return Err("Material output appeared during staging");
        }
        fs::rename(&temporary, &output).map_err(|_| "Cannot install material staging directory")?;
        Ok(manifest)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}
