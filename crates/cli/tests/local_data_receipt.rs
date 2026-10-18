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
                build: "70374".into(),
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
    // Written exactly as `receipt-assemble > receipt.json` would.
    assert!(!output.stdout.ends_with(b"\n"));
    fs::write(dir.join("receipt.json"), &output.stdout).unwrap();
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
    // Format 3 has distinct independent identities and future-target bindings.
    // These synthetic signatures never satisfy the pending production target.
    use azeroth_core::{
        local_data_delivery::{Delivery, DeliveryPolicyV3, ReceiptPayloadV3},
        local_data_inputs::InputTuple,
    };
    use azeroth_local_data::delivery_receipts as v3;
    use sha2::{Digest, Sha256};
    let target = Target {
        edition: "wow_forever".into(),
        version: "1.60.2".into(),
        build: "99999".into(),
        interface: 16002,
    };
    let mut signers = policy.trusted_signers.clone();
    for (id, signer) in &mut signers {
        signer.identity = format!("synthetic-{id}");
    }
    let future_policy = DeliveryPolicyV3 {
        format_version: 3,
        target: Some(target.clone()),
        trusted_signers: signers,
        development: vec![],
    };
    let hash = "a".repeat(64);
    let binding = Delivery {
        format_version: 1,
        tuple: InputTuple {
            product: "wow_classic_beta".into(),
            version: "1.60.2.99999".into(),
            build: 99999,
            locale: "enUS".into(),
        },
        catalog_sha256: hash.clone(),
        projection_sha256: hash.clone(),
        projector_profile_sha256: hash.clone(),
        media_sha256: vec![],
        guide_facts_sha256: None,
        input_sha256: vec![],
        dependency_profile_sha256: hash.clone(),
        dependencies_sha256: hash.clone(),
        fixture: false,
        local_only: true,
        verified_for_planning: false,
    };
    let delivery_hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&binding).unwrap())
    );
    let old = payload(&policy_sha256);
    let future_payload = ReceiptPayloadV3 {
        format_version: 3,
        candidate_source_commit: old.candidate_source_commit,
        production_sha256: hash.clone(),
        release_policy_sha256: v3::policy_hash(&future_policy).unwrap(),
        target,
        delivery_sha256: delivery_hash.clone(),
        binding,
        independent_review_sha256: hash.clone(),
        validation_results_sha256: hash.clone(),
        scenario_dependencies_sha256: hash.clone(),
        performance_workload_sha256: hash.clone(),
        performance_results_sha256: hash.clone(),
        domains: old.domains,
        acceptance: old.acceptance,
    };
    fs::write(
        dir.join("future-policy.json"),
        serde_json::to_vec(&future_policy).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("future-payload.json"),
        serde_json::to_vec(&future_payload).unwrap(),
    )
    .unwrap();
    for id in ["operator", "reviewer"] {
        let (ok, signature) = cli(&[
            "delivery-receipt-sign",
            "--payload",
            &path(dir, "future-payload.json"),
            "--policy",
            &path(dir, "future-policy.json"),
            "--key-id",
            id,
            "--secret-key",
            &path(dir, &format!("{id}.key")),
        ]);
        assert!(ok, "{signature}");
        fs::write(dir.join(format!("future-{id}.sig")), signature.to_string()).unwrap();
    }
    let assembled = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args([
            "local-data",
            "delivery-receipt-assemble",
            "--payload",
            &path(dir, "future-payload.json"),
            "--operator",
            &path(dir, "future-operator.sig"),
            "--reviewer",
            &path(dir, "future-reviewer.sig"),
        ])
        .output()
        .unwrap();
    assert!(assembled.status.success());
    assert!(assembled.stderr.is_empty());
    assert!(!assembled.stdout.ends_with(b"\n"));
    let canonical = assembled.stdout;
    v3::verify(
        &canonical,
        &future_policy,
        &v3::Expected {
            candidate_source_commit: &future_payload.candidate_source_commit,
            production_sha256: &hash,
            delivery_sha256: &delivery_hash,
            frozen_target: &future_payload.target,
        },
    )
    .unwrap();
    fs::write(dir.join("future-receipt.json"), canonical).unwrap();
    let (ok, refusal) = cli(&[
        "delivery-receipt-verify",
        "--receipt",
        &path(dir, "future-receipt.json"),
        "--policy",
        &path(dir, "future-policy.json"),
        "--candidate",
        &future_payload.candidate_source_commit,
        "--production",
        &hash,
        "--delivery",
        &delivery_hash,
    ]);
    assert!(!ok);
    assert_eq!(refusal["error"], "Post-launch release target is not frozen");
    let (ok, _) = cli(&[
        "delivery-receipt-assemble",
        "--payload",
        &path(dir, "future-payload.json"),
        "--operator",
        &path(dir, "future-reviewer.sig"),
        "--reviewer",
        &path(dir, "future-operator.sig"),
    ]);
    assert!(!ok);
}
