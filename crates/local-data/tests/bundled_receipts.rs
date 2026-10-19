//! Synthetic public bundle receipts. Constants are test seeds, never production keys.
use azeroth_core::{
    release::{FileRef, Target},
    types::{AssetPath, Sha256Hex},
};
use azeroth_local_data::{bundled_receipts::*, receipts::public_key_hex};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
fn hash(c: char) -> Sha256Hex {
    Sha256Hex::new(c.to_string().repeat(64)).unwrap()
}
fn payload_value() -> Payload {
    Payload {
        schema_version: 1,
        app_version: "1.0.0".into(),
        target: Target::forever_70374(),
        bundle_manifest_sha256: hash('a'),
        production_sha256: hash('b'),
        tested_source_commit: "c".repeat(40),
        review: FileRef {
            path: AssetPath::new("docs/reviews/synthetic-review.json").unwrap(),
            bytes: 10,
            sha256: hash('d'),
        },
    }
}
fn policy_value() -> Policy {
    Policy {
        format: POLICY_FORMAT.into(),
        schema_version: 1,
        operator_controls_both_keys: true,
        signers: [
            (Role::Operator, "operator", [11; 32]),
            (Role::ReviewEvidence, "review-evidence", [22; 32]),
        ]
        .map(|(role, id, secret)| TrustedSigner {
            role,
            key_id: id.into(),
            public_key_hex: public_key_hex(&secret),
            identity: "Same synthetic operator; not independent control".into(),
            revoked: false,
        }),
    }
}
fn expected(p: &Payload) -> Expected<'_> {
    Expected {
        app_version: &p.app_version,
        target: &p.target,
        bundle_manifest_sha256: p.bundle_manifest_sha256.as_str(),
        production_sha256: p.production_sha256.as_str(),
        tested_source_commit: &p.tested_source_commit,
        review: &p.review,
    }
}
fn signed(p: &Payload, policy: &Policy) -> Vec<u8> {
    assemble(
        p.clone(),
        sign(p, policy, "operator", &[11; 32]).unwrap(),
        sign(p, policy, "review-evidence", &[22; 32]).unwrap(),
    )
    .unwrap()
}
fn receipt() -> Receipt {
    serde_json::from_slice(&signed(&payload_value(), &policy_value())).unwrap()
}

#[test]
fn canonical_two_role_receipt_binds_every_caller_pin_without_claiming_independent_keys() {
    let p = payload_value();
    let policy = policy_value();
    assert_eq!(payload(&encode(&p).unwrap()).unwrap(), p);
    let trusted = azeroth_local_data::bundled_receipts::policy(&encode(&policy).unwrap()).unwrap();
    let bytes = signed(&p, &trusted);
    let verified = verify(&bytes, &trusted, &expected(&p)).unwrap();
    assert!(verified.operator_controls_both_keys);
    assert_eq!(verified.payload, p);
    for sig in &verified.signatures {
        assert_eq!(signature(&encode(sig).unwrap()).unwrap().key_id, sig.key_id);
    }
    let mut variants = Vec::new();
    let mut changed = p.clone();
    changed.app_version = "1.0.1".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.target.build = "70450".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.bundle_manifest_sha256 = hash('e');
    variants.push(changed);
    let mut changed = p.clone();
    changed.production_sha256 = hash('e');
    variants.push(changed);
    let mut changed = p.clone();
    changed.tested_source_commit = "e".repeat(40);
    variants.push(changed);
    let mut changed = p.clone();
    changed.review.sha256 = hash('e');
    variants.push(changed);
    let mut changed = p.clone();
    changed.review.bytes += 1;
    variants.push(changed);
    let mut changed = p.clone();
    changed.review.path = AssetPath::new("docs/reviews/different.json").unwrap();
    variants.push(changed);
    for changed in variants {
        assert!(verify(&bytes, &trusted, &expected(&changed)).is_err());
    }
}

#[test]
fn altered_payload_fails_even_when_expected_pins_are_changed_to_match_it() {
    let policy = policy_value();
    let mut changed = receipt();
    changed.payload.production_sha256 = hash('e');
    assert!(
        verify(
            &encode(&changed).unwrap(),
            &policy,
            &expected(&changed.payload)
        )
        .is_err()
    );
    let p = payload_value();
    let mut changed = receipt();
    let encoded = encode(&p).unwrap();
    let mut historical_domain = b"NAVZEROTH\0LOCAL-CATALOG-RECEIPT\0V1\0".to_vec();
    historical_domain.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
    historical_domain.extend(encoded);
    let wrong = SigningKey::from_bytes(&[11; 32])
        .sign(&historical_domain)
        .to_bytes();
    changed.signatures[0].signature_hex = wrong.iter().map(|b| format!("{b:02x}")).collect();
    assert!(
        verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err(),
        "domain/length prefix must be signed"
    );
}

#[test]
fn missing_duplicate_unknown_or_unordered_signatures_cannot_verify_or_assemble() {
    let p = payload_value();
    let policy = policy_value();
    let good = receipt();
    let mut changed = good.clone();
    changed.signatures.swap(0, 1);
    assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    let mut changed = good.clone();
    changed.signatures[1] = changed.signatures[0].clone();
    assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    let mut changed = good.clone();
    changed.signatures[0].key_id = "untrusted".into();
    assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    let mut changed = good.clone();
    changed.signatures[0].signature_hex = "ff".repeat(64);
    assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    let mut changed = good.clone();
    changed.signatures[0].signature_hex = "FF".repeat(64);
    assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    let mut changed = good.clone();
    changed.signatures[0].key_id = "bad key".into();
    assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    assert!(
        assemble(
            p.clone(),
            good.signatures[1].clone(),
            good.signatures[0].clone()
        )
        .is_err()
    );
    let mut duplicate = good.signatures[1].clone();
    duplicate.key_id = good.signatures[0].key_id.clone();
    assert!(assemble(p.clone(), good.signatures[0].clone(), duplicate).is_err());
    let mut malformed = good.signatures[0].clone();
    malformed.signature_hex = "short".into();
    assert!(signature(&encode(&malformed).unwrap()).is_err());
    assert!(assemble(p.clone(), malformed, good.signatures[1].clone()).is_err());
    for signatures in [
        json!([]),
        json!([good.signatures[0]]),
        json!([good.signatures[0], good.signatures[1], good.signatures[1]]),
    ] {
        let mut raw = serde_json::to_value(&good).unwrap();
        raw["signatures"] = signatures;
        assert!(verify(&serde_json::to_vec(&raw).unwrap(), &policy, &expected(&p)).is_err());
    }
    let raw = encode(&good).unwrap();
    let unknown = String::from_utf8(raw)
        .unwrap()
        .replace("review_evidence", "unknown_role");
    assert!(verify(unknown.as_bytes(), &policy, &expected(&p)).is_err());
}

#[test]
fn caller_policy_rejects_revoked_duplicate_weak_keys_and_false_disclosures() {
    let p = payload_value();
    let good = policy_value();
    let bytes = signed(&p, &good);
    assert!(sign(&p, &good, "missing", &[11; 32]).is_err());
    assert!(sign(&p, &good, "operator", &[22; 32]).is_err());
    let mut variants = Vec::new();
    let mut changed = good.clone();
    changed.signers[1].revoked = true;
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers.swap(0, 1);
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers[1].key_id = changed.signers[0].key_id.clone();
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers[1].public_key_hex = changed.signers[0].public_key_hex.clone();
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers[0].public_key_hex = "00".repeat(32);
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers[0].public_key_hex = "broken".into();
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers[0].identity = " ".into();
    variants.push(changed);
    let mut changed = good.clone();
    changed.signers[0].key_id = "bad key".into();
    variants.push(changed);
    let mut changed = good.clone();
    changed.operator_controls_both_keys = false;
    variants.push(changed);
    let mut changed = good.clone();
    changed.schema_version = 2;
    variants.push(changed);
    let mut changed = good.clone();
    changed.format = "local-receipt".into();
    variants.push(changed);
    for changed in variants {
        assert!(changed.validate().is_err());
        assert!(verify(&bytes, &changed, &expected(&p)).is_err());
        assert!(azeroth_local_data::bundled_receipts::policy(&encode(&changed).unwrap()).is_err());
    }
    // An equally shaped attacker policy is not selected from the receipt.
    let mut other = good.clone();
    other.signers[0].public_key_hex = public_key_hex(&[33; 32]);
    assert!(verify(&bytes, &other, &expected(&p)).is_err());
}

#[test]
fn strict_schema_canonical_encoding_bounds_and_payload_identity_are_enforced() {
    let p = payload_value();
    let policy = policy_value();
    let good = receipt();
    let bytes = encode(&good).unwrap();
    let mut variants = vec![
        bytes.iter().copied().chain(*b"\n").collect::<Vec<_>>(),
        vec![b' '; MAX_BYTES + 1],
        b"{}".to_vec(),
    ];
    variants.push(
        String::from_utf8(bytes.clone())
            .unwrap()
            .replacen(
                "\"schema_version\":1",
                "\"schema_version\":1,\"schema_version\":1",
                1,
            )
            .into_bytes(),
    );
    variants.push(
        String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("{", "{\"unknown\":1,", 1)
            .into_bytes(),
    );
    for raw in variants {
        assert!(verify(&raw, &policy, &expected(&p)).is_err());
    }
    for field in ["schema_version", "format", "disclosure"] {
        let mut changed = good.clone();
        match field {
            "schema_version" => changed.schema_version = 2,
            "format" => changed.format = "old-local-receipt".into(),
            _ => changed.operator_controls_both_keys = false,
        };
        assert!(verify(&encode(&changed).unwrap(), &policy, &expected(&p)).is_err());
    }
    let mut variants = Vec::new();
    let mut changed = p.clone();
    changed.schema_version = 2;
    variants.push(changed);
    let mut changed = p.clone();
    changed.app_version = "invalid".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.target.edition = "bad edition".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.target.version = "1.2".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.target.version = "1.x.3".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.target.build = "".into();
    variants.push(changed);
    let mut changed = p.clone();
    changed.target.interface = 0;
    variants.push(changed);
    let mut changed = p.clone();
    changed.tested_source_commit = "0".repeat(39);
    variants.push(changed);
    let mut changed = p.clone();
    changed.tested_source_commit = "z".repeat(40);
    variants.push(changed);
    let mut changed = p.clone();
    changed.review.bytes = 0;
    variants.push(changed);
    let mut changed = p.clone();
    changed.review.bytes = 17 * 1024 * 1024;
    variants.push(changed);
    for changed in variants {
        assert!(payload(&encode(&changed).unwrap()).is_err());
    }
    assert!(encode(&vec![0u8; MAX_BYTES]).is_err());
    assert!(
        serde_json::from_slice::<azeroth_core::local_data_release::SignedReceipt>(&bytes).is_err()
    );
    assert!(
        serde_json::from_slice::<azeroth_core::local_data_delivery::SignedReceiptV3>(&bytes)
            .is_err()
    );
}

#[test]
fn selected_input_reads_refuse_oversized_nonregular_and_linked_files() {
    let root = std::env::temp_dir().join(format!("anav-bundle-input-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("selected.json");
    std::fs::write(&path, b"selected").unwrap();
    assert_eq!(read_input(&path, MAX_BYTES).unwrap(), b"selected");
    assert!(read_input(&path, MAX_BYTES + 1).is_err());
    assert!(read_input(&path, 2).is_err());
    assert!(read_input(&root, MAX_BYTES).is_err());
    assert!(read_input(&root.join("missing"), MAX_BYTES).is_err());
    #[cfg(unix)]
    {
        let link = root.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(read_input(&link, MAX_BYTES).is_err());
        let fifo = root.join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(read_input(&fifo, MAX_BYTES).is_err());
    }
    std::fs::remove_dir_all(root).unwrap();
}
