//! Schema4 signed public bundle binding, separate from private hybrid receipts.
use super::*;
use azeroth_app::bundled_content::{MAX_FILES, MAX_MANIFEST_BYTES, Manifest};
use azeroth_local_data::{bundled_receipts as receipts, bundled_resources as resources};
fn checked_bytes(
    root: &Path,
    reference: &FileRef,
    maximum: usize,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    if reference.bytes == 0 || reference.bytes > maximum as u64 {
        return Err("Bundle document exceeds bounds");
    }
    let path = root.join(reference.path.as_str());
    budget.charge(&path, reference.bytes)?;
    let raw = resources::read(&path, maximum)?;
    if raw.len() as u64 != reference.bytes || hash(&raw) != reference.sha256.as_str() {
        return Err("Bundle document differs from source evidence");
    }
    budget
        .verified
        .insert(path, (reference.bytes, reference.sha256.as_str().into()));
    Ok(raw)
}
pub(super) fn verify(
    root: &Path,
    version: &str,
    inventory: &Inventory,
    evidence: &Evidence,
    tree: &Tree,
    budget: &mut Budget,
    selected_materials: &[materials::MaterialFile],
) -> Result<()> {
    verify_with_policy(
        &Context {
            root,
            version,
            inventory,
            evidence,
            tree,
            selected_materials,
        },
        budget,
        include_bytes!("../../../../content/bundled/signers-v1.json"),
    )
}
#[derive(Clone, Copy)]
pub(super) struct Context<'a> {
    pub(super) root: &'a Path,
    pub(super) version: &'a str,
    pub(super) inventory: &'a Inventory,
    pub(super) evidence: &'a Evidence,
    pub(super) tree: &'a Tree,
    pub(super) selected_materials: &'a [materials::MaterialFile],
}
pub(super) fn assemble(
    context: &Context<'_>,
    budget: &mut Budget,
    output: &Path,
    identity_path: &Path,
) -> Result<()> {
    assemble_with_policy(
        context,
        budget,
        output,
        identity_path,
        include_bytes!("../../../../content/bundled/signers-v1.json"),
    )
}
fn assemble_with_policy(
    context: &Context<'_>,
    budget: &mut Budget,
    output: &Path,
    identity_path: &Path,
    trusted_policy: &[u8],
) -> Result<()> {
    if context.inventory.schema_version != release::SCHEMA_STABLE {
        return Err("Bundle assembly requires schema4 source contracts");
    }
    verify_with_policy(context, budget, trusted_policy)?;
    let binding = context
        .evidence
        .bundled_content
        .as_ref()
        .ok_or("Bundle evidence is missing")?;
    let expected = receipts::Expected {
        app_version: context.version,
        target: context
            .inventory
            .target
            .as_ref()
            .ok_or("Bundle target is missing")?,
        bundle_manifest_sha256: binding.manifest.sha256.as_str(),
        production_sha256: &context.tree.fingerprint,
        tested_source_commit: context
            .evidence
            .tested_source_commit
            .as_deref()
            .ok_or("Bundle tested source is missing")?,
        review: &binding.review,
    };
    bundled_stage::assemble_verified(
        context.root,
        output,
        binding,
        identity_path,
        &expected,
        &receipts::policy(trusted_policy)?,
    )
}

fn verify_with_policy(
    context: &Context<'_>,
    budget: &mut Budget,
    trusted_policy: &[u8],
) -> Result<()> {
    let Context {
        root,
        version,
        inventory,
        evidence,
        tree,
        selected_materials,
    } = *context;
    let bundle = evidence
        .bundled_content
        .as_ref()
        .ok_or("Signed bundled content evidence is missing")?;
    for reference in [&bundle.manifest, &bundle.receipt, &bundle.review] {
        if !tree.all_files.contains(reference.path.as_str()) {
            return Err("Bundle metadata must be tracked source evidence");
        }
    }
    let target = inventory
        .target
        .as_ref()
        .ok_or("Bundle target is missing")?;
    let tested = evidence
        .tested_source_commit
        .as_deref()
        .ok_or("Bundle tested source is missing")?;
    if evidence.production_sha256.as_ref().map(|p| p.as_str()) != Some(tree.fingerprint.as_str()) {
        return Err("Bundle production differs from candidate");
    }
    let manifest_bytes = checked_bytes(root, &bundle.manifest, MAX_MANIFEST_BYTES, budget)?;
    let value = azeroth_core::strict_json::value(&manifest_bytes, MAX_MANIFEST_BYTES)
        .map_err(|_| "Invalid signed bundle manifest")?;
    let manifest: Manifest =
        serde_json::from_value(value).map_err(|_| "Invalid signed bundle manifest")?;
    if manifest.format != "navzeroth-installed-content"
        || manifest.schema_version != 1
        || manifest.app_version != version
        || &manifest.target != target
        || manifest.review != bundle.review
        || evidence.origin_policy.as_ref() != Some(&manifest.accepted_origins)
        || manifest.packs.len() != evidence.packs.len()
        || manifest.packs.is_empty()
        || manifest.packs.len() > 8
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES - 2
    {
        return Err("Signed bundle differs from release contracts");
    }
    let receipt = checked_bytes(root, &bundle.receipt, receipts::MAX_BYTES, budget)?;
    let policy = receipts::policy(trusted_policy)?;
    if !tree.files.contains("content/bundled/signers-v1.json") {
        return Err("Bundle signer policy is not production source");
    }
    // Compile-time policy must equal the checked source tree's identity bytes.
    let policy_bytes = resources::read(
        &root.join("content/bundled/signers-v1.json"),
        receipts::MAX_BYTES,
    )?;
    if policy_bytes != trusted_policy {
        return Err("Bundle signer policy differs from verifier build");
    }
    receipts::verify(
        &receipt,
        &policy,
        &receipts::Expected {
            app_version: version,
            target,
            bundle_manifest_sha256: bundle.manifest.sha256.as_str(),
            production_sha256: &tree.fingerprint,
            tested_source_commit: tested,
            review: &bundle.review,
        },
    )?;
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    let private = materials::private_refs(inventory, evidence);
    for reference in &manifest.files {
        if !tree.all_files.contains(reference.path.as_str()) {
            return Err("Bundle resources must be tracked source files");
        }
        if evidence.admission_registry.as_ref().is_some_and(|p| {
            p.path == reference.path || (p.bytes == reference.bytes && p.sha256 == reference.sha256)
        }) {
            return Err("Private admission registry cannot ship in the bundle");
        }
        let protected = private.iter().any(|p| {
            p.path == reference.path || (p.bytes == reference.bytes && p.sha256 == reference.sha256)
        });
        let public_material = selected_materials
            .iter()
            .any(|m| m.input.bytes == reference.bytes && m.input.sha256 == reference.sha256);
        if protected
            && !public_material
            && reference != &bundle.review
            && reference != &manifest.accepted_origins
        {
            return Err("Private validation or permission evidence cannot ship in the bundle");
        }
        total = total
            .checked_add(reference.bytes)
            .ok_or("Bundle resource size overflow")?;
        if total > azeroth_app::bundled_content::MAX_TOTAL_BYTES
            || files.insert(reference.path.as_str(), reference).is_some()
            || matches!(reference.path.as_str(), "manifest.json" | "receipt.json")
        {
            return Err("Bundle resource set is invalid");
        }
        budget.charge(&root.join(reference.path.as_str()), reference.bytes)?;
        resources::verify(
            &root.join(reference.path.as_str()),
            reference.bytes,
            reference.sha256.as_str(),
        )?;
    }
    for required in [&bundle.review, &manifest.accepted_origins] {
        if files.get(required.path.as_str()).copied() != Some(required) {
            return Err("Bundle omits pinned review or origin policy");
        }
    }
    let mut seen = BTreeSet::new();
    for pack in &manifest.packs {
        let Some(accepted) = evidence.packs.iter().find(|p| p.manifest == pack.manifest) else {
            return Err("Bundle pack is not in accepted release inventory");
        };
        if !seen.insert(pack.manifest.path.as_str())
            || pack.manifest.path.as_str() != format!("{}/manifest.json", pack.directory.as_str())
            || pack.key.scope.fixture
            || pack.key.scope.context.build.edition.as_str() != target.edition
            || pack.key.scope.context.build.build.as_str()
                != format!("{}.{}", target.version, target.build)
            || pack.key.scope.context.build.interface.as_str() != target.interface.to_string()
            || accepted.dataset_revisions != [pack.key.revision.as_str()]
            || files.get(pack.manifest.path.as_str()).copied() != Some(&pack.manifest)
        {
            return Err("Bundle pack identity differs from accepted release inventory");
        }
        for reference in pack.conflicts.iter().chain(&pack.normalization_conflicts) {
            if files.get(reference.path.as_str()).copied() != Some(reference) {
                return Err("Bundle omits declared conflict material");
            }
        }
    }
    // Every verified public material must actually ship in the installer bundle.
    for material in selected_materials {
        if files.get(material.input.path.as_str()).copied() != Some(&material.input) {
            return Err("Bundle omits verified release material");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/support/release_bundled.rs"]
mod tests;
