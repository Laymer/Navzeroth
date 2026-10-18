//! Selected build metadata staging. No cache reads, archive scans or activation.
use crate::{Result, bounded_json, extraction::Store, hash, read_file};
use azeroth_core::local_data::{
    InstallationMetadata, MAX_BUILD_INFO_BYTES, PerformancePreset, is_sha256,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub format_version: u32,
    pub selection_id: String,
    pub previous_sha256: String,
    pub current_sha256: String,
    pub metadata: InstallationMetadata,
    pub profile_sha256: String,
    pub local_only: bool,
    pub activated: bool,
}
/// Byte identity plus opaque size/time identity for cross-poll stability.
/// Unix additionally binds file identity; Windows file-ID detection is unverified.
/// The latter is transient and never enters persisted candidates or UI responses.
pub fn read_build_observation(installation: &Path) -> Result<(String, String)> {
    let (bytes, stamp) = read_file(&installation.join(".build.info"), MAX_BUILD_INFO_BYTES)?;
    let identity = serde_json::to_vec(&stamp).map_err(|_| "Cannot encode build file identity")?;
    Ok((hash(&bytes), hash(&identity)))
}
pub fn read_build_identity(installation: &Path) -> Result<String> {
    read_build_observation(installation).map(|(hash, _)| hash)
}
pub fn stage(
    root: &Path,
    installation: &Path,
    product: &str,
    selection: &str,
    previous: &str,
    current: &str,
    preset: PerformancePreset,
) -> Result<String> {
    if ![selection, previous, current].into_iter().all(is_sha256) || previous == current {
        return Err("Invalid changed-build identities");
    }
    let bytes = read_file(&installation.join(".build.info"), MAX_BUILD_INFO_BYTES)?.0;
    if hash(&bytes) != current {
        return Err("Build metadata changed before staging");
    }
    let inspected = crate::inspect(installation, product)?;
    if inspected.build_info_sha256 != current || read_build_identity(installation)? != current {
        return Err("Build metadata changed while staging");
    }
    let mut store = Store::open(root, preset.limits().total_storage_bytes)?;
    if store.put("objects", &bytes)? != current {
        return Err("Changed build object identity differs");
    }
    let candidate = Candidate {
        format_version: 1,
        selection_id: selection.into(),
        previous_sha256: previous.into(),
        current_sha256: current.into(),
        metadata: inspected.metadata,
        profile_sha256: crate::inputs::INPUT_PROFILE_SHA256.into(),
        local_only: true,
        activated: false,
    };
    let digest = store.put("changes", &bounded_json(&candidate, 64 * 1024)?)?;
    crate::lifecycle::index(root)?
        .set_change_candidate(selection, &digest)
        .map_err(|_| "Cannot persist changed-build candidate")?;
    Ok(digest)
}
pub fn verify(root: &Path, digest: &str) -> Result<Candidate> {
    if !is_sha256(digest) {
        return Err("Invalid changed-build candidate identity");
    }
    let bytes = read_file(&root.join("changes").join(digest), 64 * 1024)?.0;
    if hash(&bytes) != digest {
        return Err("Changed-build manifest hash mismatch");
    }
    let candidate: Candidate =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid changed-build manifest")?;
    if candidate.format_version != 1
        || !candidate.local_only
        || candidate.activated
        || candidate.previous_sha256 == candidate.current_sha256
        || ![
            candidate.selection_id.as_str(),
            candidate.previous_sha256.as_str(),
            candidate.current_sha256.as_str(),
        ]
        .into_iter()
        .all(is_sha256)
        || candidate.profile_sha256 != crate::inputs::INPUT_PROFILE_SHA256
        || bounded_json(&candidate, 64 * 1024)? != bytes
    {
        return Err("Unsupported or unbound changed-build candidate");
    }
    let source = read_file(
        &root.join("objects").join(&candidate.current_sha256),
        MAX_BUILD_INFO_BYTES,
    )?
    .0;
    if hash(&source) != candidate.current_sha256
        || azeroth_core::local_data::inspect_build_info(&source, &candidate.metadata.product)?
            != candidate.metadata
    {
        return Err("Changed-build source binding mismatch");
    }
    Ok(candidate)
}
#[derive(Debug, Serialize)]
pub struct ArtifactPage {
    pub format_version: u32,
    pub revision: String,
    pub namespace: String,
    pub total: u32,
    pub offset: u32,
    pub rows: Vec<String>,
    pub next_offset: Option<u32>,
    pub integrity_checked: bool,
    pub staging_present: bool,
}
pub fn artifacts(root: &Path, namespace: &str, offset: u32, limit: u32) -> Result<ArtifactPage> {
    if !matches!(
        namespace,
        "catalogs"
            | "projections"
            | "inputs"
            | "guide-facts"
            | "media"
            | "deliveries"
            | "inventories"
            | "changes"
    ) || limit == 0
        || limit > 200
    {
        return Err("Unsupported artifact page");
    }
    let root = crate::checked_path(root)?;
    if !root.is_dir() {
        return Err("Private artifact store unavailable");
    }
    let path = root.join(namespace);
    let mut ids = Vec::new();
    let mut staging = false;
    let mut count = 0usize;
    if path.exists() {
        let path = crate::checked_path(&path)?;
        for entry in std::fs::read_dir(path).map_err(|_| "Artifact directory unavailable")? {
            count += 1;
            if count > 200_000 {
                return Err("Artifact inventory capacity exceeded");
            }
            let entry = entry.map_err(|_| "Artifact entry unavailable")?;
            if !entry
                .file_type()
                .map_err(|_| "Artifact type unavailable")?
                .is_file()
            {
                return Err("Artifact store contains an unsupported entry");
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "Invalid artifact identity")?;
            if is_sha256(&name) {
                ids.push(name);
            } else if name.starts_with('.') && name.ends_with(".staging") {
                staging = true;
            } else {
                return Err("Unexpected artifact entry");
            }
            if ids.len() > 200_000 {
                return Err("Artifact inventory capacity exceeded");
            }
        }
    }
    ids.sort();
    if offset as usize > ids.len() {
        return Err("Artifact page offset out of range");
    }
    let revision = hash(&bounded_json(&(namespace, &ids), 16 * 1024 * 1024)?);
    let end = (offset as usize + limit as usize).min(ids.len());
    Ok(ArtifactPage {
        format_version: 1,
        revision,
        namespace: namespace.into(),
        total: ids.len() as u32,
        offset,
        rows: ids[offset as usize..end].to_vec(),
        next_offset: (end < ids.len()).then_some(end as u32),
        integrity_checked: false,
        staging_present: staging,
    })
}
