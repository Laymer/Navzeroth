//! Explicit receipt CLI (task V1.2) with synthetic keys and payloads only.
use azeroth_core::{
    local_data_release::*,
    release::{DOMAINS, LIVE_CHECKS, Method, Status, Target, WINDOWS_CHECKS},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn cli(args: &[&str]) -> (bool, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("local-data")
        .args(args)
        .output()
        .unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    (
        output.status.success(),
        serde_json::from_str(text.trim()).unwrap_or(Value::Null),
    )
}
fn path(dir: &Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_owned()
}
fn payload(policy_sha256: &str) -> ReceiptPayload {
    let digest = "a".repeat(64);
    let proof = InventoryProof {
        records: 1,
        units: 1,
        private_inventory_sha256: digest.clone(),
        independent_enumeration_sha256: digest.clone(),
        reference_graph_validation_sha256: digest.clone(),
        source_transform_closure_sha256: digest.clone(),
        coverage_review_sha256: digest.clone(),
    };
    let proof_for = |method| AcceptanceProof {
        method,
        status: Status::Pass,
        evidence_sha256: digest.clone(),
    };
    ReceiptPayload {
        format_version: 2,
        candidate_source_commit: "b".repeat(40),
        production_sha256: digest.clone(),
        release_policy_sha256: policy_sha256.into(),
        target: Target::forever_70374(),
        extractor_version: "0.9.2".into(),
        native_reader_revision: "c".repeat(40),
        profile_sha256: digest.clone(),
        metadata_sha256: digest.clone(),
        private_input_closure_sha256: digest.clone(),
        private_inventory_sha256: digest.clone(),
        private_catalog_sha256: digest.clone(),
        private_media_sha256: digest.clone(),
        validation_results_sha256: digest.clone(),
        scenario_dependencies_sha256: digest.clone(),
        performance_workload_sha256: digest.clone(),
        performance_results_sha256: digest.clone(),
        independent_review_sha256: Some("e".repeat(64)),
        domains: DOMAINS.map(|d| (d.into(), proof.clone())).into(),
        acceptance: LIVE_CHECKS
            .map(|c| (c.to_owned(), proof_for(Method::LiveGame)))
            .into_iter()
            .chain(WINDOWS_CHECKS.map(|c| (c.to_owned(), proof_for(Method::InteractiveWindows))))
            .collect(),
    }
}

#[test]
fn owner_signs_assembles_and_verifies_a_hybrid_receipt_explicitly() {
    let temp = Temp(std::env::temp_dir().join(format!("anav-receipt-{}", std::process::id())));
    fs::create_dir_all(&temp.0).unwrap();
    let dir = temp.0.as_path();
    fs::write(dir.join("operator.key"), format!("{}\n", "01".repeat(32))).unwrap();
    fs::write(dir.join("reviewer.key"), "02".repeat(32)).unwrap();
    let mut signers = BTreeMap::new();
    for (id, role) in [
        ("operator", SignerRole::Operator),
        ("reviewer", SignerRole::Reviewer),
    ] {
        let (ok, value) = cli(&[
            "receipt-public-key",
            "--secret-key",
            &path(dir, &format!("{id}.key")),
        ]);
        assert!(ok, "{value}");
        let key = value["public_key_hex"].as_str().unwrap().to_owned();
        assert!(!key.contains(&"01".repeat(32)) && key.len() == 64);
        signers.insert(
            id.to_owned(),
            TrustedSigner {
                identity: "project-owner".into(),
                role,
                public_key_hex: key,
                revoked: false,
            },
        );
    }
    let policy = DeliveryPolicy {
        format_version: 2,
        delivery_mode: DeliveryMode::Hybrid,
        target: Target::forever_70374(),
        trusted_signers: signers,
        local_content: Some(LocalContent {
            extractor_version: "0.9.2".into(),
            native_reader_revision: "c".repeat(40),
            profile_sha256: "a".repeat(64),
            metadata_sha256: "a".repeat(64),
            expected_catalogs: vec![ExpectedCatalog {
                build: "70425".into(),
                locale: "enUS".into(),
                catalog_sha256: "a".repeat(64),
            }],
        }),
    };
    fs::write(
        dir.join("policy.json"),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let policy_sha256 = azeroth_local_data::receipts::policy_hash(&policy).unwrap();
    fs::write(
        dir.join("payload.json"),
        serde_json::to_vec_pretty(&payload(&policy_sha256)).unwrap(),
    )
    .unwrap();
    for id in ["operator", "reviewer"] {
        let (ok, value) = cli(&[
            "receipt-sign",
            "--payload",
            &path(dir, "payload.json"),
            "--policy",
            &path(dir, "policy.json"),
            "--key-id",
            id,
            "--secret-key",
            &path(dir, &format!("{id}.key")),
        ]);
        assert!(ok, "{value}");
        fs::write(dir.join(format!("{id}.sig")), value.to_string()).unwrap();
    }
    // Swapped signature files are refused.
    let (ok, _) = cli(&[
        "receipt-assemble",
        "--payload",
        &path(dir, "payload.json"),
        "--operator",
        &path(dir, "reviewer.sig"),
        "--reviewer",
        &path(dir, "operator.sig"),
    ]);
    assert!(!ok);
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args([
            "local-data",
            "receipt-assemble",
            "--payload",
            &path(dir, "payload.json"),
            "--operator",
            &path(dir, "operator.sig"),
            "--reviewer",
            &path(dir, "reviewer.sig"),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let receipt = String::from_utf8(output.stdout).unwrap();
    fs::write(dir.join("receipt.json"), receipt.trim_end()).unwrap();
    let verify = |candidate: &str| {
        cli(&[
            "receipt-verify",
            "--receipt",
            &path(dir, "receipt.json"),
            "--policy",
            &path(dir, "policy.json"),
            "--candidate",
            candidate,
            "--production",
            &"a".repeat(64),
        ])
    };
    let (ok, value) = verify(&"b".repeat(40));
    assert!(ok, "{value}");
    assert_eq!(value["verified"], true);
    let (ok, _) = verify(&"d".repeat(40));
    assert!(!ok, "a different candidate is refused");
    // An unknown key id and a missing option are refused.
    let (ok, _) = cli(&[
        "receipt-sign",
        "--payload",
        &path(dir, "payload.json"),
        "--policy",
        &path(dir, "policy.json"),
        "--key-id",
        "stranger",
        "--secret-key",
        &path(dir, "operator.key"),
    ]);
    assert!(!ok);
    let (ok, _) = cli(&["receipt-verify", "--receipt", &path(dir, "receipt.json")]);
    assert!(!ok);
    // A malformed key file is refused without echoing it.
    fs::write(dir.join("bad.key"), "not-a-key").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args([
            "local-data",
            "receipt-public-key",
            "--secret-key",
            &path(dir, "bad.key"),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("not-a-key"));
}
