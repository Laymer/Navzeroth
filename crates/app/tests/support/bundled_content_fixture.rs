//! Installer-resource mutation tests use only deterministic synthetic signing seeds.
use azeroth_app::{
    bundled_content::{self as bundle, Manifest, Pack},
    snapshots::SnapshotApplication,
};
use azeroth_core::{
    release::{FileRef, Target},
    types::{AssetPath, Sha256Hex},
};
use azeroth_local_data::{
    bundled_receipts::{self as receipts, Expected, Payload, Policy, Role, TrustedSigner},
    receipts::public_key_hex,
};

use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub fn hash(bytes: &[u8]) -> Sha256Hex {
    Sha256Hex::new(format!("{:x}", Sha256::digest(bytes))).unwrap()
}
pub fn reference(root: &Path, path: &str) -> FileRef {
    let raw = fs::read(root.join(path)).unwrap();
    FileRef {
        path: AssetPath::new(path).unwrap(),
        bytes: raw.len() as u64,
        sha256: hash(&raw),
    }
}
fn files(root: &Path, here: &Path, out: &mut Vec<FileRef>) {
    for entry in fs::read_dir(here).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(root, &path, out);
        } else {
            out.push(reference(
                root,
                &path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/"),
            ));
        }
    }
}
pub struct Fixture {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub payload: Payload,
    pub policy: Policy,
}
impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "nav-bundled-resources-{}-{}-{}",
            std::process::id(),
            &hash(module_path!().as_bytes()).as_str()[..12],
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/authored/slice-v1")
            .canonicalize()
            .unwrap();
        azeroth_ingest::pack::v2::build(&input, &root.join("authored")).unwrap();
        let checked = azeroth_ingest::pack::v2::read_verified(&root.join("authored")).unwrap();
        let key = azeroth_storage::content::ContentRevisionV2 {
            content: checked.content,
            sources: checked.sources,
            conflicts: vec![],
        }
        .key();
        fs::write(
            root.join("accepted-origins.json"),
            include_bytes!("../../../../release/v1/accepted-origins.json"),
        )
        .unwrap();
        fs::write(
            root.join("review.json"),
            b"Synthetic review evidence, not release approval.",
        )
        .unwrap();
        fs::write(root.join("conflicts.json"), b"[]").unwrap();
        let mut resources = Vec::new();
        files(&root, &root, &mut resources);
        let manifest = Manifest {
            format: "navzeroth-installed-content".into(),
            schema_version: 1,
            app_version: "1.0.0".into(),
            target: Target {
                edition: "wow_forever".into(),
                version: "1.60.1".into(),
                build: "70450".into(),
                interface: 16001,
            },
            accepted_origins: reference(&root, "accepted-origins.json"),
            review: reference(&root, "review.json"),
            files: resources,
            packs: vec![Pack {
                directory: AssetPath::new("authored").unwrap(),
                manifest: reference(&root, "authored/manifest.json"),
                key,
                conflicts: Some(reference(&root, "conflicts.json")),
                normalization_conflicts: None,
            }],
        };
        let policy = Policy {
            format: receipts::POLICY_FORMAT.into(),
            schema_version: 1,
            operator_controls_both_keys: true,
            signers: [
                (Role::Operator, "operator", [11; 32]),
                (Role::ReviewEvidence, "review", [22; 32]),
            ]
            .map(|(role, id, seed)| TrustedSigner {
                role,
                key_id: id.into(),
                public_key_hex: public_key_hex(&seed),
                identity: "Same synthetic operator".into(),
                revoked: false,
            }),
        };
        let payload = Payload {
            schema_version: 1,
            app_version: manifest.app_version.clone(),
            target: manifest.target.clone(),
            bundle_manifest_sha256: hash(b"pending"),
            production_sha256: hash(b"synthetic production"),
            tested_source_commit: "a".repeat(40),
            review: manifest.review.clone(),
        };
        let mut fixture = Self {
            root,
            manifest,
            payload,
            policy,
        };
        fixture.sign();
        fixture
    }
    pub fn sign(&mut self) {
        let raw = serde_json::to_vec(&self.manifest).unwrap();
        self.payload.bundle_manifest_sha256 = hash(&raw);
        fs::write(self.root.join("manifest.json"), raw).unwrap();
        let operator = receipts::sign(&self.payload, &self.policy, "operator", &[11; 32]).unwrap();
        let review = receipts::sign(&self.payload, &self.policy, "review", &[22; 32]).unwrap();
        fs::write(
            self.root.join("receipt.json"),
            receipts::assemble(self.payload.clone(), operator, review).unwrap(),
        )
        .unwrap();
    }
    pub fn expected(&self) -> Expected<'_> {
        Expected {
            app_version: &self.payload.app_version,
            target: &self.payload.target,
            bundle_manifest_sha256: self.payload.bundle_manifest_sha256.as_str(),
            production_sha256: self.payload.production_sha256.as_str(),
            tested_source_commit: &self.payload.tested_source_commit,
            review: &self.payload.review,
        }
    }
    pub fn verify(&self) -> bool {
        bundle::verify(&self.root, &self.policy, &self.expected()).is_ok()
    }
    pub fn install(&self, app: &SnapshotApplication) -> bool {
        bundle::install(app, &self.root, &self.policy, &self.expected()).is_ok()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
