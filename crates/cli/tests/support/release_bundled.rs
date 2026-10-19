use super::*;
use crate::release::bundle_test_fixture as shared;
use azeroth_core::release::{BundledContentEvidence, PackRef};
use materials::{Kind, MaterialFile};
use serde_json::json;
struct Fixture {
    bundle: shared::Fixture,
    inventory: Inventory,
    evidence: Evidence,
    tree: Tree,
    materials: Vec<MaterialFile>,
    policy: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let bundle = shared::Fixture::new();
        let target = serde_json::to_value(&bundle.manifest.target).unwrap();
        let inventory = serde_json::from_value(json!({"schema_version":4,"inventory_id":"synthetic-source-binding","target":target,"slice":null,"domains":[],"sources":[]})).unwrap();
        let evidence = serde_json::from_value(json!({"schema_version":4,"target":target,"inventory_sha256":"a".repeat(64),"admission_registry":null,
            "tested_source_commit":bundle.payload.tested_source_commit,"production_sha256":bundle.payload.production_sha256,
            "checks":[],"coverage":[],"sources":[],"packs":[],"optional_deferrals":[],"matrix":[],"origin_policy":bundle.manifest.accepted_origins})).unwrap();
        let materials = bundle
            .manifest
            .files
            .iter()
            .filter(|f| f.path.as_str().starts_with("authored/"))
            .map(|f| MaterialFile {
                input: f.clone(),
                archive_path: f.path.clone(),
                kind: Kind::ContentPack,
                additional_kinds: vec![],
            })
            .collect();
        let policy = receipts::encode(&bundle.policy).unwrap();
        fs::create_dir_all(bundle.root.join("content/bundled")).unwrap();
        fs::create_dir_all(bundle.root.join("release/v1")).unwrap();
        fs::write(bundle.root.join("content/bundled/signers-v1.json"), &policy).unwrap();
        let tree = Tree {
            fingerprint: bundle.payload.production_sha256.as_str().into(),
            files: BTreeSet::from(["content/bundled/signers-v1.json".into()]),
            all_files: BTreeSet::new(),
        };
        let mut f = Self {
            bundle,
            inventory,
            evidence,
            tree,
            materials,
            policy,
        };
        f.bind();
        f
    }
    fn bind(&mut self) {
        self.bundle.sign();
        fs::copy(
            self.bundle.root.join("manifest.json"),
            self.bundle.root.join("release/v1/bundle-manifest.json"),
        )
        .unwrap();
        fs::copy(
            self.bundle.root.join("receipt.json"),
            self.bundle.root.join("release/v1/bundle-receipt.json"),
        )
        .unwrap();
        self.evidence.bundled_content = Some(BundledContentEvidence {
            manifest: shared::reference(&self.bundle.root, "release/v1/bundle-manifest.json"),
            receipt: shared::reference(&self.bundle.root, "release/v1/bundle-receipt.json"),
            review: self.bundle.manifest.review.clone(),
        });
        self.evidence.packs = self
            .bundle
            .manifest
            .packs
            .iter()
            .map(|p| PackRef {
                manifest: p.manifest.clone(),
                dataset_revisions: vec![p.key.revision.as_str().into()],
            })
            .collect();
        self.tree.all_files = self
            .bundle
            .manifest
            .files
            .iter()
            .map(|f| f.path.as_str().into())
            .chain([
                "release/v1/bundle-manifest.json".into(),
                "release/v1/bundle-receipt.json".into(),
                "content/bundled/signers-v1.json".into(),
            ])
            .collect();
    }
    fn context(&self) -> Context<'_> {
        Context {
            root: &self.bundle.root,
            version: "1.0.0",
            inventory: &self.inventory,
            evidence: &self.evidence,
            tree: &self.tree,
            selected_materials: &self.materials,
        }
    }
    fn verify(&self) -> bool {
        verify_with_policy(&self.context(), &mut Budget::default(), &self.policy).is_ok()
    }
}
#[test]
fn source_bundle_binds_review_materials_production_and_caller_trust() {
    let f = Fixture::new();
    assert!(f.verify());
    assert!(
        verify(
            &f.bundle.root,
            "1.0.0",
            &f.inventory,
            &f.evidence,
            &f.tree,
            &mut Budget::default(),
            &f.materials
        )
        .is_err(),
        "production verifier cannot trust synthetic keys"
    );
    for change in [
        "missing",
        "target",
        "tested",
        "production",
        "untracked-review",
        "policy-untracked",
        "policy",
        "receipt",
        "review",
        "origin",
        "material",
        "duplicate",
        "pack",
        "conflicts",
        "size",
        "shape",
        "untracked-resource",
    ] {
        let mut f = Fixture::new();
        match change {
            "missing" => f.evidence.bundled_content = None,
            "target" => f.inventory.target = None,
            "tested" => f.evidence.tested_source_commit = None,
            "production" => f.tree.fingerprint = "b".repeat(64),
            "untracked-review" => {
                f.tree
                    .all_files
                    .remove(f.bundle.manifest.review.path.as_str());
            }
            "policy-untracked" => f.tree.files.clear(),
            "policy" => {
                fs::write(f.bundle.root.join("content/bundled/signers-v1.json"), b"{}").unwrap()
            }
            "receipt" => {
                fs::write(f.bundle.root.join("release/v1/bundle-receipt.json"), b"{}").unwrap()
            }
            "review" => fs::write(f.bundle.root.join("review.json"), b"altered").unwrap(),
            "origin" => f.evidence.origin_policy = None,
            "material" => {
                f.bundle
                    .manifest
                    .files
                    .retain(|r| r.path.as_str() != "authored/catalog.json");
                f.bind();
            }
            "duplicate" => {
                f.bundle
                    .manifest
                    .files
                    .push(f.bundle.manifest.files[0].clone());
                f.bind();
            }
            "pack" => f.evidence.packs[0].dataset_revisions = vec!["wrong".into()],
            "conflicts" => {
                f.bundle
                    .manifest
                    .files
                    .retain(|r| r.path.as_str() != "conflicts.json");
                f.bind();
            }
            "size" => {
                f.bundle.manifest.files[0].bytes =
                    azeroth_app::bundled_content::MAX_TOTAL_BYTES + 1;
                f.bind();
            }
            "shape" => {
                f.bundle.manifest.format = "wrong".into();
                f.bind();
            }
            _ => {
                f.tree.all_files.remove("authored/catalog.json");
            }
        }
        assert!(!f.verify(), "{change}");
    }
}
#[test]
fn private_validation_and_admission_bytes_cannot_be_smuggled_as_extra_resources() {
    for registry in [false, true] {
        let mut f = Fixture::new();
        fs::create_dir_all(f.bundle.root.join("docs")).unwrap();
        fs::write(
            f.bundle.root.join("docs/private.json"),
            b"Synthetic private review fixture, no secret",
        )
        .unwrap();
        let private = shared::reference(&f.bundle.root, "docs/private.json");
        f.bundle.manifest.files.push(private.clone());
        f.bind();
        if registry {
            f.evidence.admission_registry = Some(private);
        } else {
            f.inventory.domains=serde_json::from_value(json!([{"domain":"quests","completeness":"unknown","enumeration_evidence":[private],"units":[]}])).unwrap();
        }
        assert!(!f.verify());
    }
    let mut f = Fixture::new();
    f.evidence.admission_registry = Some(f.bundle.manifest.review.clone());
    assert!(
        !f.verify(),
        "explicit review cannot relabel the private registry"
    );
}

#[test]
fn source_bound_assembly_uses_exact_trusted_materials_and_preserves_existing_output() {
    let mut f = Fixture::new();
    let identity = f.bundle.root.with_extension("identity.json");
    fs::write(&identity, receipts::encode(&f.bundle.payload).unwrap()).unwrap();
    let output = f.bundle.root.join("assembled");
    assert!(
        assemble_with_policy(
            &f.context(),
            &mut Budget::default(),
            &output,
            &identity,
            &f.policy
        )
        .is_ok()
    );
    assert!(
        assemble_with_policy(
            &f.context(),
            &mut Budget::default(),
            &output,
            &identity,
            &f.policy
        )
        .is_err()
    );
    assert!(assemble(&f.context(), &mut Budget::default(), &output, &identity).is_err());
    f.inventory.schema_version = 3;
    assert!(
        assemble_with_policy(
            &f.context(),
            &mut Budget::default(),
            &output,
            &identity,
            &f.policy
        )
        .is_err()
    );
    fs::remove_file(identity).unwrap();
}
