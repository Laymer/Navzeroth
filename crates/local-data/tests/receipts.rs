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
        local_content: None,
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
        independent_review_sha256: None,
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
    unknown.format_version = 3;
    assert!(validate_policy(&unknown).is_err());
}

/// ADR 0006 section 6: the owner holds both keys under format 2.
fn owner_fixture() -> (DeliveryPolicy, SignedReceipt) {
    let (mut policy, receipt) = fixture();
    policy.format_version = 2;
    for signer in policy.trusted_signers.values_mut() {
        signer.identity = "project-owner".into();
    }
    let mut payload = receipt.payload;
    payload.format_version = 2;
    payload.release_policy_sha256 = receipts::policy_hash(&policy).unwrap();
    payload.independent_review_sha256 = Some("e".repeat(64));
    let signatures = vec![
        receipts::sign(&payload, &policy, "operator", &[1; 32]).unwrap(),
        receipts::sign(&payload, &policy, "reviewer", &[2; 32]).unwrap(),
    ];
    (
        policy,
        SignedReceipt {
            format_version: 2,
            payload,
            signatures,
        },
    )
}
fn resign(receipt: &mut SignedReceipt, policy: &DeliveryPolicy) -> bool {
    let signed = [("operator", [1; 32]), ("reviewer", [2; 32])]
        .map(|(id, key)| receipts::sign(&receipt.payload, policy, id, &key));
    if signed.iter().any(Result::is_err) {
        return false;
    }
    receipt.signatures = signed.into_iter().map(Result::unwrap).collect();
    true
}
#[test]
fn one_owner_may_hold_both_roles_only_with_format_2_and_a_bound_review() {
    let (policy, receipt) = owner_fixture();
    assert!(verify(&receipt, &policy));

    // The same shared identity under format 1 rules is refused.
    let mut v1_policy = policy.clone();
    v1_policy.format_version = 1;
    assert!(validate_policy(&v1_policy).is_err());

    // A format 2 policy accepts format 2 receipts only.
    let mut v1 = receipt.clone();
    v1.format_version = 1;
    v1.payload.format_version = 1;
    v1.payload.independent_review_sha256 = None;
    assert!(resign(&mut v1, &policy));
    assert!(!verify(&v1, &policy));

    // Format 2 needs a review hash; format 1 must not carry one.
    let mut missing = receipt.clone();
    missing.payload.independent_review_sha256 = None;
    assert!(!resign(&mut missing, &policy));
    let mut bad = receipt.clone();
    bad.payload.independent_review_sha256 = Some("not-a-hash".into());
    assert!(!resign(&mut bad, &policy));
    let (plain_policy, mut extra) = fixture();
    extra.payload.independent_review_sha256 = Some("e".repeat(64));
    assert!(!resign(&mut extra, &plain_policy));

    // The review hash is signed: changing it breaks both signatures.
    let mut swapped = receipt.clone();
    swapped.payload.independent_review_sha256 = Some("f".repeat(64));
    assert!(!verify(&swapped, &policy));

    // Receipt and payload versions must agree.
    let mut mixed = receipt.clone();
    mixed.format_version = 1;
    assert!(!verify(&mixed, &policy));
}
#[test]
fn format_2_still_requires_distinct_keys_roles_and_unrevoked_signers() {
    let (policy, receipt) = owner_fixture();
    let mut same_key = policy.clone();
    let operator_key = same_key.trusted_signers["operator"].public_key_hex.clone();
    same_key
        .trusted_signers
        .get_mut("reviewer")
        .unwrap()
        .public_key_hex = operator_key;
    assert!(validate_policy(&same_key).is_err());

    let mut two_operators = policy.clone();
    two_operators
        .trusted_signers
        .get_mut("reviewer")
        .unwrap()
        .role = SignerRole::Operator;
    assert!(validate_policy(&two_operators).is_err());

    let mut duplicated = receipt.clone();
    duplicated.signatures[1] = duplicated.signatures[0].clone();
    assert!(!verify(&duplicated, &policy));

    let mut revoked = policy.clone();
    revoked.trusted_signers.get_mut("reviewer").unwrap().revoked = true;
    assert!(!verify(&receipt, &revoked));
}
#[test]
fn a_format_1_policy_refuses_format_2_receipts_even_with_distinct_signers() {
    let (policy, receipt) = fixture();
    let mut v2 = receipt;
    v2.format_version = 2;
    v2.payload.format_version = 2;
    v2.payload.independent_review_sha256 = Some("e".repeat(64));
    assert!(resign(&mut v2, &policy));
    assert!(!verify(&v2, &policy));
    // The same distinct signers under a format 2 policy are accepted.
    let mut v2_policy = policy;
    v2_policy.format_version = 2;
    v2.payload.release_policy_sha256 = receipts::policy_hash(&v2_policy).unwrap();
    assert!(resign(&mut v2, &v2_policy));
    assert!(verify(&v2, &v2_policy));
}
#[test]
fn format_2_refuses_untrusted_signers_and_noncanonical_review_fields() {
    let (policy, receipt) = owner_fixture();
    let mut untrusted = receipt.clone();
    untrusted.signatures[1].key_id = "untrusted".into();
    assert!(!verify(&untrusted, &policy));
    let text = serde_json::to_string(&receipt).unwrap();
    let field = format!("\"independent_review_sha256\":\"{}\",", "e".repeat(64));
    assert!(text.contains(&field));
    let expected = ExpectedBindings {
        candidate_source_commit: &receipt.payload.candidate_source_commit,
        production_sha256: &receipt.payload.production_sha256,
        release_policy_sha256: &receipt.payload.release_policy_sha256,
        native_reader_revision: &"c".repeat(40),
        profile_sha256: &"a".repeat(64),
        metadata_sha256: &"a".repeat(64),
    };
    let moved = text.replacen(&field, "", 1).replacen(
        "\"payload\":{",
        &format!("\"payload\":{{{field}"),
        1,
    );
    assert!(receipts::verify(moved.as_bytes(), &policy, &expected).is_err());
    let (plain_policy, plain) = fixture();
    let plain_text = serde_json::to_string(&plain).unwrap().replacen(
        "\"domains\":",
        "\"independent_review_sha256\":null,\"domains\":",
        1,
    );
    let plain_expected = ExpectedBindings {
        release_policy_sha256: &plain.payload.release_policy_sha256,
        ..expected
    };
    assert!(receipts::verify(plain_text.as_bytes(), &plain_policy, &plain_expected).is_err());
}
/// Hand-built messages isolate the domain tag: each format verifies only under
/// its own tag (the payload JSON also differs by format_version).
#[test]
fn signatures_verify_only_under_their_own_domain_tag() {
    use ed25519_dalek::Signer;
    const V1: &[u8] = b"NAVZEROTH\0LOCAL-CATALOG-RECEIPT\0V1\0";
    const V2: &[u8] = b"NAVZEROTH\0LOCAL-CATALOG-RECEIPT\0V2\0";
    fn signed_under(receipt: &SignedReceipt, tag: &[u8]) -> SignedReceipt {
        let bytes = serde_json::to_vec(&receipt.payload).unwrap();
        let mut message = tag.to_vec();
        message.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        message.extend(bytes);
        let mut out = receipt.clone();
        for (signature, seed) in out.signatures.iter_mut().zip([1u8, 2]) {
            signature.signature_hex = hex(&SigningKey::from_bytes(&[seed; 32])
                .sign(&message)
                .to_bytes());
        }
        out
    }
    let (policy, receipt) = owner_fixture();
    assert!(verify(&signed_under(&receipt, V2), &policy));
    assert!(!verify(&signed_under(&receipt, V1), &policy));
    let (policy, receipt) = fixture();
    assert!(verify(&signed_under(&receipt, V1), &policy));
    assert!(!verify(&signed_under(&receipt, V2), &policy));
}
#[test]
fn format_1_receipts_cannot_reuse_format_2_signatures() {
    let (policy, receipt) = owner_fixture();
    // Reusing format 2 signatures on a format 1 payload must fail, even with a
    // policy that has distinct identities.
    let (plain_policy, plain) = fixture();
    let mut forged = plain.clone();
    forged.signatures = receipt.signatures.clone();
    assert!(!verify(&forged, &plain_policy));
    assert!(verify(&receipt, &policy));
}

fn local_content(catalogs: Vec<ExpectedCatalog>) -> LocalContent {
    LocalContent {
        extractor_version: "0.9.0".into(),
        native_reader_revision: "c".repeat(40),
        profile_sha256: "a".repeat(64),
        metadata_sha256: "a".repeat(64),
        expected_catalogs: catalogs,
    }
}
fn catalog(build: &str, locale: &str, digest: &str) -> ExpectedCatalog {
    ExpectedCatalog {
        build: build.into(),
        locale: locale.into(),
        catalog_sha256: digest.into(),
    }
}
/// A hybrid policy whose tested-content digest declares the fixture catalog.
fn hybrid_fixture(catalogs: Vec<ExpectedCatalog>) -> (DeliveryPolicy, SignedReceipt) {
    let (mut policy, mut receipt) = owner_fixture();
    policy.delivery_mode = DeliveryMode::Hybrid;
    policy.local_content = Some(local_content(catalogs));
    receipt.payload.release_policy_sha256 = receipts::policy_hash(&policy).unwrap();
    assert!(resign(&mut receipt, &policy));
    (policy, receipt)
}
fn verify_hybrid(receipt: &SignedReceipt, policy: &DeliveryPolicy) -> Result<(), &'static str> {
    receipts::verify_hybrid(
        &serde_json::to_vec(receipt).unwrap(),
        policy,
        &"b".repeat(40),
        &"a".repeat(64),
    )
    .map(|_| ())
}
#[test]
fn hybrid_receipts_bind_the_declared_tested_content_digest() {
    let (policy, receipt) = hybrid_fixture(vec![catalog("70374", "enUS", &"a".repeat(64))]);
    assert_eq!(verify_hybrid(&receipt, &policy), Ok(()));

    let (policy, receipt) = hybrid_fixture(vec![catalog("70374", "enUS", &"d".repeat(64))]);
    assert_eq!(
        verify_hybrid(&receipt, &policy),
        Err("Receipt catalog is not the declared enUS tested-content digest")
    );
    let (policy, receipt) = hybrid_fixture(vec![]);
    assert_eq!(
        verify_hybrid(&receipt, &policy),
        Err("Hybrid delivery has no declared expected local catalog digests")
    );

    // The extractor identity comes from the policy, not from the receipt.
    let (mut policy, receipt) = hybrid_fixture(vec![catalog("70374", "enUS", &"a".repeat(64))]);
    policy.local_content.as_mut().unwrap().extractor_version = "1.0.0".into();
    let mut receipt = receipt;
    receipt.payload.release_policy_sha256 = receipts::policy_hash(&policy).unwrap();
    assert!(resign(&mut receipt, &policy));
    assert_eq!(
        verify_hybrid(&receipt, &policy),
        Err("Receipt extractor differs from the tested extractor")
    );
    let (mut policy, mut receipt) = hybrid_fixture(vec![catalog("70374", "enUS", &"a".repeat(64))]);
    policy.local_content.as_mut().unwrap().profile_sha256 = "d".repeat(64);
    receipt.payload.release_policy_sha256 = receipts::policy_hash(&policy).unwrap();
    assert!(resign(&mut receipt, &policy));
    assert!(verify_hybrid(&receipt, &policy).is_err());
}
#[test]
fn only_the_enus_digest_clears_a_hybrid_receipt() {
    // The fixture catalog ("a" digest) is declared, but only for deDE.
    let (policy, receipt) = hybrid_fixture(vec![
        catalog("70374", "enUS", &"d".repeat(64)),
        catalog("70374", "deDE", &"a".repeat(64)),
    ]);
    assert_eq!(
        verify_hybrid(&receipt, &policy),
        Err("Receipt catalog is not the declared enUS tested-content digest")
    );
    // The plain verifier refuses hybrid policies outright.
    let bytes = serde_json::to_vec(&receipt).unwrap();
    let expected = ExpectedBindings {
        candidate_source_commit: &"b".repeat(40),
        production_sha256: &"a".repeat(64),
        release_policy_sha256: &receipt.payload.release_policy_sha256,
        native_reader_revision: &"c".repeat(40),
        profile_sha256: &"a".repeat(64),
        metadata_sha256: &"a".repeat(64),
    };
    assert_eq!(
        receipts::verify(&bytes, &policy, &expected).map(|_| ()),
        Err("Hybrid receipts require verify_hybrid")
    );
}
#[test]
fn hybrid_receipts_are_bound_to_the_tested_candidate_and_hybrid_mode() {
    let (policy, receipt) = hybrid_fixture(vec![catalog("70374", "enUS", &"a".repeat(64))]);
    let bytes = serde_json::to_vec(&receipt).unwrap();
    assert!(receipts::verify_hybrid(&bytes, &policy, &"d".repeat(40), &"a".repeat(64)).is_err());
    assert!(receipts::verify_hybrid(&bytes, &policy, &"b".repeat(40), &"d".repeat(64)).is_err());
    let mut tampered = receipt.clone();
    tampered.payload.private_catalog_sha256 = "d".repeat(64);
    assert!(verify_hybrid(&tampered, &policy).is_err());
    let (local_policy, local_receipt) = owner_fixture();
    assert_eq!(
        verify_hybrid(&local_receipt, &local_policy),
        Err("Receipt requires a hybrid delivery policy")
    );
}
#[test]
fn hybrid_policies_are_strict() {
    let (policy, _) = hybrid_fixture(vec![catalog("70374", "enUS", &"a".repeat(64))]);
    assert!(validate_policy(&policy).is_ok());
    let mut v1 = policy.clone();
    v1.format_version = 1;
    assert!(validate_policy(&v1).is_err());
    let mut missing = policy.clone();
    missing.local_content = None;
    assert!(validate_policy(&missing).is_err());
    let mut bundled = policy.clone();
    bundled.delivery_mode = DeliveryMode::BundledContent;
    assert!(validate_policy(&bundled).is_err());
    for bad in [
        vec![catalog("70374", "en-US", &"a".repeat(64))],
        vec![catalog("build", "enUS", &"a".repeat(64))],
        vec![catalog("70374", "enUS", "short")],
        vec![
            catalog("70374", "enUS", &"a".repeat(64)),
            catalog("70374", "enUS", &"b".repeat(64)),
        ],
    ] {
        let (policy, _) = hybrid_fixture(vec![]);
        let mut policy = policy;
        policy.local_content.as_mut().unwrap().expected_catalogs = bad;
        assert!(validate_policy(&policy).is_err());
    }
    // Every catalog must be for the target build, and enUS is required.
    for bad in [
        vec![catalog("70425", "enUS", &"a".repeat(64))],
        vec![catalog("70374", "deDE", &"a".repeat(64))],
    ] {
        let (policy, _) = hybrid_fixture(vec![]);
        let mut policy = policy;
        policy.local_content.as_mut().unwrap().expected_catalogs = bad;
        assert!(validate_policy(&policy).is_err());
    }
    // Two locales of one build are distinct tested-content digests.
    let mut locales = policy;
    locales.local_content.as_mut().unwrap().expected_catalogs = vec![
        catalog("70374", "enUS", &"a".repeat(64)),
        catalog("70374", "deDE", &"b".repeat(64)),
    ];
    assert!(validate_policy(&locales).is_ok());
}

fn future_fixture() -> (
    azeroth_core::local_data_delivery::DeliveryPolicyV3,
    azeroth_core::local_data_delivery::SignedReceiptV3,
) {
    use azeroth_core::{local_data_delivery::*, local_data_inputs::InputTuple};
    use azeroth_local_data::delivery_receipts as v3;
    use sha2::{Digest, Sha256};
    let (old, receipt) = fixture();
    let target = Target {
        edition: "wow_forever".into(),
        version: "1.60.2".into(),
        build: "99999".into(),
        interface: 16002,
    };
    let policy = DeliveryPolicyV3 {
        format_version: 3,
        target: Some(target.clone()),
        trusted_signers: old.trusted_signers,
        development: vec![],
    };
    let digest = "a".repeat(64);
    let binding = Delivery {
        format_version: 1,
        tuple: InputTuple {
            product: "wow_classic_beta".into(),
            version: "1.60.2.99999".into(),
            build: 99999,
            locale: "enUS".into(),
        },
        catalog_sha256: digest.clone(),
        projection_sha256: digest.clone(),
        projector_profile_sha256: digest.clone(),
        media_sha256: vec![digest.clone()],
        guide_facts_sha256: None,
        input_sha256: vec![],
        dependency_profile_sha256: digest.clone(),
        dependencies_sha256: digest.clone(),
        fixture: false,
        local_only: true,
        verified_for_planning: false,
    };
    let delivery_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&binding).unwrap())
    );
    let payload = ReceiptPayloadV3 {
        format_version: 3,
        candidate_source_commit: receipt.payload.candidate_source_commit,
        production_sha256: digest.clone(),
        release_policy_sha256: v3::policy_hash(&policy).unwrap(),
        target,
        delivery_sha256,
        binding,
        independent_review_sha256: digest.clone(),
        validation_results_sha256: digest.clone(),
        scenario_dependencies_sha256: digest.clone(),
        performance_workload_sha256: digest.clone(),
        performance_results_sha256: digest,
        domains: receipt.payload.domains,
        acceptance: receipt.payload.acceptance,
    };
    let signatures = vec![
        v3::sign(&payload, &policy, "operator", &[1; 32]).unwrap(),
        v3::sign(&payload, &policy, "reviewer", &[2; 32]).unwrap(),
    ];
    (
        policy,
        SignedReceiptV3 {
            format_version: 3,
            payload,
            signatures,
        },
    )
}
#[test]
fn future_receipts_bind_all_components_but_current_pending_release_still_refuses() {
    use azeroth_local_data::delivery_receipts as v3;
    let (policy, receipt) = future_fixture();
    let p = &receipt.payload;
    let expected = v3::Expected {
        candidate_source_commit: &p.candidate_source_commit,
        production_sha256: &p.production_sha256,
        delivery_sha256: &p.delivery_sha256,
        frozen_target: &p.target,
    };
    let bytes = serde_json::to_vec(&receipt).unwrap();
    assert!(v3::verify(&bytes, &policy, &expected).is_ok());
    assert!(
        v3::verify_current_release(
            &bytes,
            &policy,
            &p.candidate_source_commit,
            &p.production_sha256,
            &p.delivery_sha256
        )
        .is_err()
    );
    let mut changed = receipt.clone();
    changed.payload.binding.media_sha256 = vec!["f".repeat(64)];
    assert!(v3::verify(&serde_json::to_vec(&changed).unwrap(), &policy, &expected).is_err());
    changed = receipt.clone();
    changed.payload.binding.projector_profile_sha256 = "e".repeat(64);
    assert!(v3::verify(&serde_json::to_vec(&changed).unwrap(), &policy, &expected).is_err());
    changed = receipt.clone();
    changed.payload.domains.remove("quests");
    assert!(v3::verify(&serde_json::to_vec(&changed).unwrap(), &policy, &expected).is_err());
    changed = receipt.clone();
    changed.signatures.swap(0, 1);
    assert!(v3::verify(&serde_json::to_vec(&changed).unwrap(), &policy, &expected).is_err());
    let mut revoked = policy.clone();
    revoked.trusted_signers.get_mut("reviewer").unwrap().revoked = true;
    assert!(v3::verify(&bytes, &revoked, &expected).is_err());
}
#[test]
fn pending_targets_duplicate_signers_fixtures_and_unknown_v3_versions_refuse() {
    use azeroth_local_data::delivery_receipts as v3;
    let (mut policy, mut receipt) = future_fixture();
    policy.target = None;
    assert!(policy.validate().is_ok());
    assert!(v3::sign(&receipt.payload, &policy, "operator", &[1; 32]).is_err());
    policy.target = Some(receipt.payload.target.clone());
    let identity = policy.trusted_signers["operator"].identity.clone();
    policy.trusted_signers.get_mut("reviewer").unwrap().identity = identity;
    assert!(policy.validate().is_err());
    let (policy, _) = future_fixture();
    receipt.payload.binding.fixture = true;
    assert!(v3::sign(&receipt.payload, &policy, "operator", &[1; 32]).is_err());
    let mut policy = policy;
    policy.format_version = 4;
    assert!(policy.validate().is_err());
}
