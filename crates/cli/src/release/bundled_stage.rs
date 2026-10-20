//! Atomic assembly of already signed, source-bound installer resources.
use super::*;
use azeroth_local_data::{bundled_receipts as receipts, bundled_resources as resources};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) fn assemble_verified(
    root: &Path,
    output: &Path,
    binding: &release::BundledContentEvidence,
    identity_path: &Path,
    expected: &receipts::Expected<'_>,
    policy: &receipts::Policy,
) -> Result<()> {
    let identity = receipts::payload(&receipts::read_input(identity_path, receipts::MAX_BYTES)?)?;
    if identity.app_version != expected.app_version
        || &identity.target != expected.target
        || identity.bundle_manifest_sha256.as_str() != expected.bundle_manifest_sha256
        || identity.production_sha256.as_str() != expected.production_sha256
        || identity.tested_source_commit != expected.tested_source_commit
        || &identity.review != expected.review
    {
        return Err("Build identity differs from source-bound bundle");
    }
    let raw = resources::read(
        &root.join(binding.manifest.path.as_str()),
        azeroth_app::bundled_content::MAX_MANIFEST_BYTES,
    )?;
    if raw.len() as u64 != binding.manifest.bytes || hash(&raw) != binding.manifest.sha256.as_str()
    {
        return Err("Bundle manifest changed during assembly");
    }
    let manifest: azeroth_app::bundled_content::Manifest =
        serde_json::from_slice(&raw).map_err(|_| "Invalid bundle manifest")?;
    stage_verified(root, output, binding, &identity, &manifest, policy)
}
pub(super) fn stage_verified(
    root: &Path,
    output: &Path,
    binding: &release::BundledContentEvidence,
    identity: &receipts::Payload,
    manifest: &azeroth_app::bundled_content::Manifest,
    policy: &receipts::Policy,
) -> Result<()> {
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
        return Err("Invalid bundle output path");
    }
    let parent = output.parent().ok_or("Bundle output must have a parent")?;
    checked_path(parent, true)?;
    if fs::symlink_metadata(&output).is_ok() {
        return Err("Bundle output already exists");
    }
    let temporary = parent.join(format!(
        ".bundle-stage-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary).map_err(|_| "Cannot create bundle staging directory")?;
    let result = (|| {
        for reference in &manifest.files {
            let destination = temporary.join(reference.path.as_str());
            fs::create_dir_all(destination.parent().ok_or("Invalid bundle destination")?)
                .map_err(|_| "Cannot create bundle resource directories")?;
            resources::copy(
                &root.join(reference.path.as_str()),
                &destination,
                reference.bytes,
                reference.sha256.as_str(),
            )?;
        }
        for (reference, name) in [
            (&binding.manifest, "manifest.json"),
            (&binding.receipt, "receipt.json"),
        ] {
            resources::copy(
                &root.join(reference.path.as_str()),
                &temporary.join(name),
                reference.bytes,
                reference.sha256.as_str(),
            )?;
        }
        azeroth_app::bundled_content::verify(
            &temporary,
            policy,
            &receipts::Expected {
                app_version: &identity.app_version,
                target: &identity.target,
                bundle_manifest_sha256: identity.bundle_manifest_sha256.as_str(),
                production_sha256: identity.production_sha256.as_str(),
                tested_source_commit: &identity.tested_source_commit,
                review: &identity.review,
            },
        )
        .map_err(|_| "Assembled bundle failed exact verification")?;
        if fs::symlink_metadata(&output).is_ok() {
            return Err("Bundle output appeared during assembly");
        }
        fs::rename(&temporary, &output).map_err(|_| "Cannot install assembled bundle")
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}

#[cfg(test)]
#[path = "../../tests/support/release_bundled_stage.rs"]
mod tests;
