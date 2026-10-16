use azeroth_core::{
    local_data_release::*,
    release::{DOMAINS, LIVE_CHECKS, Method, Status, Target, WINDOWS_CHECKS},
};
use azeroth_local_data::receipts::{self, ExpectedBindings};
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn fixture() -> (DeliveryPolicy, SignedReceipt) {
    let mut policy = DeliveryPolicy {
        format_version: 1,
        delivery_mode: DeliveryMode::LocalCatalog,
        target: Target::forever_70374(),
        trusted_signers: BTreeMap::new(),
    };
    for (id, seed, role) in [
        ("operator", 1, SignerRole::Operator),
        ("reviewer", 2, SignerRole::Reviewer),
    ] {
        policy.trusted_signers.insert(
            id.into(),
            TrustedSigner {
                identity: format!("synthetic-{id}"),
                role,
                public_key_hex: hex(&SigningKey::from_bytes(&[seed; 32])
                    .verifying_key()
                    .to_bytes()),
                revoked: false,
            },
        );
    }
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
    let payload = ReceiptPayload {
        format_version: 1,
        candidate_source_commit: "b".repeat(40),
        production_sha256: digest.clone(),
        release_policy_sha256: receipts::policy_hash(&policy).unwrap(),
        target: Target::forever_70374(),
        extractor_version: "0.9.0".into(),
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
        domains: DOMAINS
            .into_iter()
            .map(|d| (d.into(), proof.clone()))
            .collect(),
        acceptance: LIVE_CHECKS
            .into_iter()
            .map(|c| {
                (
                    c.into(),
                    AcceptanceProof {
                        method: Method::LiveGame,
                        status: Status::Pass,
                        evidence_sha256: digest.clone(),
                    },
                )
            })
            .chain(WINDOWS_CHECKS.into_iter().map(|c| {
                (
                    c.into(),
                    AcceptanceProof {
                        method: Method::InteractiveWindows,
                        status: Status::Pass,
                        evidence_sha256: digest.clone(),
                    },
                )
            }))
            .collect(),
    };
    let signatures = vec![
        receipts::sign(&payload, &policy, "operator", &[1; 32]).unwrap(),
        receipts::sign(&payload, &policy, "reviewer", &[2; 32]).unwrap(),
    ];
    (
        policy,
        SignedReceipt {
            format_version: 1,
            payload,
            signatures,
        },
    )
}
fn verify(receipt: &SignedReceipt, policy: &DeliveryPolicy) -> bool {
    receipts::verify(
        &serde_json::to_vec(receipt).unwrap(),
        policy,
        &ExpectedBindings {
            candidate_source_commit: &"b".repeat(40),
            production_sha256: &"a".repeat(64),
            release_policy_sha256: &receipts::policy_hash(policy).unwrap(),
            native_reader_revision: &"c".repeat(40),
            profile_sha256: &"a".repeat(64),
            metadata_sha256: &"a".repeat(64),
        },
    )
    .is_ok()
}
#[test]
fn canonical_receipt_binds_candidate_policy_and_two_distinct_trusted_reviewers() {
    let (policy, receipt) = fixture();
    assert!(verify(&receipt, &policy));
    let bytes = serde_json::to_vec_pretty(&receipt).unwrap();
    assert!(
        receipts::verify(
            &bytes,
            &policy,
            &ExpectedBindings {
                candidate_source_commit: &receipt.payload.candidate_source_commit,
                production_sha256: &receipt.payload.production_sha256,
                release_policy_sha256: &receipt.payload.release_policy_sha256,
                native_reader_revision: &"c".repeat(40),
                profile_sha256: &"a".repeat(64),
                metadata_sha256: &"a".repeat(64),
            }
        )
        .is_err()
    );
    assert!(receipts::sign(&receipt.payload, &policy, "operator", &[9; 32]).is_err());
    let mut stale = receipt.clone();
    stale.payload.profile_sha256 = "d".repeat(64);
    stale.signatures = vec![
        receipts::sign(&stale.payload, &policy, "operator", &[1; 32]).unwrap(),
        receipts::sign(&stale.payload, &policy, "reviewer", &[2; 32]).unwrap(),
    ];
    assert!(!verify(&stale, &policy));
}
#[test]
fn tampering_stale_bindings_unknown_keys_revocation_and_missing_review_fail() {
    let (policy, receipt) = fixture();
    let mutations: &[fn(&mut SignedReceipt)] = &[
        |r| r.payload.private_catalog_sha256 = "d".repeat(64),
        |r| r.payload.private_inventory_sha256 = "d".repeat(64),
        |r| r.payload.candidate_source_commit = "d".repeat(40),
        |r| r.payload.production_sha256 = "d".repeat(64),
        |r| r.payload.metadata_sha256 = "d".repeat(64),
        |r| r.payload.target.build = "70425".into(),
        |r| {
            r.payload.acceptance.get_mut("LIVE-BUILD").unwrap().status = Status::Fail;
        },
        |r| {
            r.payload.acceptance.remove("WIN-PERFORMANCE");
        },
        |r| {
            r.payload.domains.remove("quests");
        },
        |r| r.payload.domains.get_mut("quests").unwrap().records = 0,
        |r| {
            r.signatures.pop();
        },
        |r| r.signatures[1] = r.signatures[0].clone(),
        |r| r.signatures[1].key_id = "untrusted".into(),
        |r| r.signatures[1].signature_hex = "0".repeat(128),
        |r| r.format_version = 2,
    ];
    for mutate in mutations {
        let mut changed = receipt.clone();
        mutate(&mut changed);
        assert!(!verify(&changed, &policy));
    }
    let mut changed = policy.clone();
    changed.trusted_signers.get_mut("reviewer").unwrap().revoked = true;
    assert!(!verify(&receipt, &changed));
    changed = policy.clone();
    changed.delivery_mode = DeliveryMode::BundledContent;
    assert!(!verify(&receipt, &changed));
    changed = policy.clone();
    changed
        .trusted_signers
        .get_mut("reviewer")
        .unwrap()
        .identity = "synthetic-operator".into();
    assert!(validate_policy(&changed).is_err());
}
#[test]
fn unknown_modes_versions_self_supplied_keys_and_duplicate_members_fail_closed() {
    let (policy, receipt) = fixture();
    let text = serde_json::to_string(&receipt).unwrap();
    let expected = ExpectedBindings {
        candidate_source_commit: &receipt.payload.candidate_source_commit,
        production_sha256: &receipt.payload.production_sha256,
        release_policy_sha256: &receipt.payload.release_policy_sha256,
        native_reader_revision: &"c".repeat(40),
        profile_sha256: &"a".repeat(64),
        metadata_sha256: &"a".repeat(64),
    };
    for text in [
        text.replacen('{', "{\"format_version\":1,", 1),
        text.replacen('{', "{\"trusted_key\":\"fake\",", 1),
    ] {
        assert!(receipts::verify(text.as_bytes(), &policy, &expected).is_err());
    }
    let text = serde_json::to_string(&policy).unwrap();
    assert!(receipts::policy(text.replace("local_catalog", "unknown").as_bytes()).is_err());
    let mut unknown = policy;
    unknown.format_version = 2;
    assert!(validate_policy(&unknown).is_err());
}
