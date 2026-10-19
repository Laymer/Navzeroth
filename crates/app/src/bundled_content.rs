//! Verified installer resources. A resource hash alone never establishes admission.
use crate::snapshots::SnapshotApplication;
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    origin_policy::OriginPolicy,
    release::{FileRef, Target},
    strict_json,
    types::AssetPath,
};
use azeroth_local_data::bundled_receipts::{self as receipts, Expected, Policy};
use azeroth_storage::content::ContentRevisionKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_FILES: usize = 8192;
pub const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub directory: AssetPath,
    pub manifest: FileRef,
    pub key: ContentRevisionKey,
    pub conflicts: Option<FileRef>,
    pub normalization_conflicts: Option<FileRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub schema_version: u32,
    pub app_version: String,
    pub target: Target,
    pub accepted_origins: FileRef,
    pub review: FileRef,
    pub files: Vec<FileRef>,
    pub packs: Vec<Pack>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn bytes(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    azeroth_local_data::bundled_resources::read(path, maximum).map_err(invalid)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tree(root: &Path) -> Result<BTreeSet<String>> {
    let mut pending = vec![(root.to_owned(), 0)];
    let mut files = BTreeSet::new();
    let mut entries = 0;
    while let Some((directory, depth)) = pending.pop() {
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|_| invalid("Bundle directory unavailable"))?;
        if !metadata.file_type().is_dir() || depth > 16 {
            return Err(invalid("Invalid bundle directory"));
        }
        for entry in
            fs::read_dir(&directory).map_err(|_| invalid("Bundle directory unavailable"))?
        {
            entries += 1;
            if entries > MAX_FILES * 2 {
                return Err(invalid("Bundle tree exceeds bounds"));
            }
            let entry = entry.map_err(|_| invalid("Bundle entry unavailable"))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|_| invalid("Bundle entry unavailable"))?;
            if kind.is_dir() {
                pending.push((path, depth + 1));
            } else if kind.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| invalid("Invalid bundle path"))?;
                let name = relative
                    .to_str()
                    .ok_or_else(|| invalid("Invalid bundle path"))?
                    .replace('\\', "/");
                AssetPath::new(&name)?;
                if files.len() >= MAX_FILES || !files.insert(name) {
                    return Err(invalid("Bundle files exceed bounds"));
                }
            } else {
                return Err(invalid("Bundle links or special resources refused"));
            }
        }
    }
    Ok(files)
}
fn verify_file(root: &Path, reference: &FileRef) -> Result<()> {
    azeroth_local_data::bundled_resources::verify(
        &root.join(reference.path.as_str()),
        reference.bytes,
        reference.sha256.as_str(),
    )
    .map_err(invalid)
}
fn pinned_bytes(root: &Path, reference: &FileRef, maximum: usize) -> Result<Vec<u8>> {
    let raw = bytes(&root.join(reference.path.as_str()), maximum)?;
    if raw.len() as u64 != reference.bytes || digest(&raw) != reference.sha256.as_str() {
        return Err(invalid("Bundle document integrity differs"));
    }
    Ok(raw)
}
/// Expected pins and signer policy belong to the verified application build.
/// Reading the receipt to select any of them would remove the trust boundary.
pub fn verify(root: &Path, policy: &Policy, expected: &Expected<'_>) -> Result<Manifest> {
    let manifest_bytes = bytes(&root.join("manifest.json"), MAX_MANIFEST_BYTES)?;
    if digest(&manifest_bytes) != expected.bundle_manifest_sha256 {
        return Err(invalid("Bundle manifest differs from build"));
    }
    let value = strict_json::value(&manifest_bytes, MAX_MANIFEST_BYTES)
        .map_err(|_| invalid("Invalid bundle manifest"))?;
    let manifest: Manifest =
        serde_json::from_value(value).map_err(|_| invalid("Invalid bundle manifest"))?;
    if manifest.format != "navzeroth-installed-content"
        || manifest.schema_version != 1
        || manifest.app_version != expected.app_version
        || &manifest.target != expected.target
        || &manifest.review != expected.review
        || manifest.packs.is_empty()
        || manifest.packs.len() > 8
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES - 2
    {
        return Err(invalid("Bundle identity or resource bounds differ"));
    }
    let receipt = bytes(&root.join("receipt.json"), receipts::MAX_BYTES)?;
    receipts::verify(&receipt, policy, expected).map_err(invalid)?;
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    for file in &manifest.files {
        total = total
            .checked_add(file.bytes)
            .ok_or_else(|| invalid("Bundle size overflow"))?;
        if total > MAX_TOTAL_BYTES
            || files.insert(file.path.as_str(), file).is_some()
            || matches!(file.path.as_str(), "manifest.json" | "receipt.json")
        {
            return Err(invalid("Bundle repeats or exceeds resource bounds"));
        }
    }
    for reference in [&manifest.review, &manifest.accepted_origins]
        .into_iter()
        .chain(manifest.packs.iter().flat_map(|p| {
            std::iter::once(&p.manifest)
                .chain(&p.conflicts)
                .chain(&p.normalization_conflicts)
        }))
    {
        if files.get(reference.path.as_str()).copied() != Some(reference) {
            return Err(invalid("Bundle omitted required resource pin"));
        }
    }
    let mut declared: BTreeSet<String> = files.keys().map(|s| s.to_string()).collect();
    declared.extend(["manifest.json".into(), "receipt.json".into()]);
    if tree(root)? != declared {
        return Err(invalid("Bundle has missing or undeclared resources"));
    }
    for reference in files.values() {
        verify_file(root, reference)?;
    }
    Ok(manifest)
}
/// Stage all validated packs before the single-scope active pointer can move.
/// Staging is immutable; a failure retains the prior activation and preferences.
pub fn install(
    app: &SnapshotApplication,
    root: &Path,
    policy: &Policy,
    expected: &Expected<'_>,
) -> Result<ContentRevisionKey> {
    let manifest = verify(root, policy, expected)?;
    let origins = OriginPolicy::decode(&pinned_bytes(
        root,
        &manifest.accepted_origins,
        azeroth_core::origin_policy::MAX_POLICY_BYTES,
    )?)
    .map_err(invalid)?;
    let mut keys = Vec::new();
    for pack in &manifest.packs {
        if pack.manifest.path.as_str() != format!("{}/manifest.json", pack.directory.as_str())
            || pack.key.scope.fixture
            || pack.key.scope.context.build.edition.as_str() != manifest.target.edition
            || pack.key.scope.context.build.build.as_str()
                != format!("{}.{}", manifest.target.version, manifest.target.build)
            || pack.key.scope.context.build.interface.as_str()
                != manifest.target.interface.to_string()
            || keys.contains(&pack.key)
            || keys
                .first()
                .is_some_and(|key: &ContentRevisionKey| key.scope != pack.key.scope)
        {
            return Err(invalid("Bundle pack target or scope differs"));
        }
        let checked = azeroth_ingest::pack::v2::read_verified(&root.join(pack.directory.as_str()))
            .map_err(invalid)?;
        if checked.manifest_sha256 != pack.manifest.sha256 {
            return Err(invalid("Bundle pack manifest differs"));
        }
        let classification = azeroth_core::origin_policy::classify(
            &checked.content,
            &checked.sources,
            Some(&origins),
        )
        .map_err(invalid)?;
        if classification.remaining_gaps != 0 || classification.relabelled_evidence != 0 {
            return Err(invalid("Bundle content origin policy failed"));
        }
        let conflicts = match &pack.conflicts {
            Some(reference) => {
                let raw = pinned_bytes(root, reference, MAX_MANIFEST_BYTES)?;
                let value = strict_json::value(&raw, MAX_MANIFEST_BYTES)
                    .map_err(|_| invalid("Invalid bundle source conflicts"))?;
                serde_json::from_value(value)
                    .map_err(|_| invalid("Invalid bundle source conflicts"))?
            }
            None => vec![],
        };
        if let Some(reference) = &pack.normalization_conflicts {
            let raw = pinned_bytes(root, reference, MAX_MANIFEST_BYTES)?;
            let value = strict_json::value(&raw, MAX_MANIFEST_BYTES)
                .map_err(|_| invalid("Invalid normalization conflicts"))?;
            if value["format"] != "navzeroth-world-content-conflicts"
                || value["schema_version"] != 1
                || value["revision"].as_str() != Some(pack.key.revision.as_str())
                || !value["conflicts"].is_array()
                || value["precedence"]
                    != serde_json::to_value(&origins.precedence)
                        .map_err(|_| invalid("Invalid conflict precedence"))?
            {
                return Err(invalid(
                    "Normalization conflicts differ from catalog/policy",
                ));
            }
        }
        let content = azeroth_storage::content::ContentRevisionV2 {
            content: checked.content,
            sources: checked.sources,
            conflicts,
        };
        let key = content.key();
        if key != pack.key {
            return Err(invalid("Bundle catalog identity differs"));
        }
        let assets = checked
            .assets
            .into_iter()
            .map(|asset| azeroth_storage::catalog_assets::CatalogAsset {
                file: asset.path,
                bytes: asset.bytes,
            })
            .collect::<Vec<_>>();
        app.with_store_mut(|store| store.stage_content_v2_bundle(&content, &assets))?;
        keys.push(key);
    }
    let key = app.stage_content_set(&keys)?;
    app.configure_bundled_defaults(expected.app_version, std::slice::from_ref(&key))?;
    Ok(key)
}

/// Native builds supply an independently recorded identity and compiled signer
/// policy. Stable installations fail closed when that build identity is absent.
pub fn start(
    database: &Path,
    resources: &Path,
    app_version: &str,
    identity: Option<&[u8]>,
    signer_policy: &[u8],
) -> Result<std::sync::Arc<crate::recovery::DatabaseHost>> {
    let Some(raw) = identity else {
        return if azeroth_core::release::is_stable(app_version).map_err(invalid)? {
            Err(invalid(
                "Stable installation is missing its bundled-content build identity",
            ))
        } else {
            Ok(crate::recovery::DatabaseHost::start(database))
        };
    };
    let identity = receipts::payload(raw).map_err(invalid)?;
    if identity.app_version != app_version {
        return Err(invalid("Bundled content version differs from application"));
    }
    let policy = receipts::policy(signer_policy).map_err(invalid)?;
    let root = resources.to_owned();
    Ok(crate::recovery::DatabaseHost::start_initialized(
        database,
        move |application| {
            let expected = Expected {
                app_version: &identity.app_version,
                target: &identity.target,
                bundle_manifest_sha256: identity.bundle_manifest_sha256.as_str(),
                production_sha256: identity.production_sha256.as_str(),
                tested_source_commit: &identity.tested_source_commit,
                review: &identity.review,
            };
            install(application, &root, &policy, &expected).map(|_| ())
        },
    ))
}
