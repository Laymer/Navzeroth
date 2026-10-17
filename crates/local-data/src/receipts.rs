//! Canonical domain-separated Ed25519 receipt verification. Trust always comes
//! from caller-owned production policy, never from the receipt. No key generation,
//! filesystem key lookup, automatic signing or release-readiness bypass exists.
use crate::{Result, hash};
use azeroth_core::local_data_release::{
    self as contract, DeliveryMode, DeliveryPolicy, ReceiptPayload, ReceiptSignature,
    SignedReceipt, SignerRole,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use std::collections::BTreeSet;
const MAX_RECEIPT: usize = 128 * 1024;
const DOMAIN: &[u8] = b"NAVZEROTH\0LOCAL-CATALOG-RECEIPT\0V1\0";
const DOMAIN_V2: &[u8] = b"NAVZEROTH\0LOCAL-CATALOG-RECEIPT\0V2\0";
pub struct ExpectedBindings<'a> {
    pub candidate_source_commit: &'a str,
    pub production_sha256: &'a str,
    pub release_policy_sha256: &'a str,
    pub native_reader_revision: &'a str,
    pub profile_sha256: &'a str,
    pub metadata_sha256: &'a str,
}

fn canonical<T: serde::Serialize + serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let value: T = serde_json::from_value(azeroth_core::strict_json::value(bytes, MAX_RECEIPT)?)
        .map_err(|_| "Invalid receipt or policy schema")?;
    if serde_json::to_vec(&value).map_err(|_| "Canonical encoding failed")? != bytes {
        return Err("Noncanonical receipt or policy encoding");
    }
    Ok(value)
}
fn payload_bytes(payload: &ReceiptPayload) -> Result<Vec<u8>> {
    contract::validate_payload(payload)?;
    let bytes = crate::bounded_json(payload, MAX_RECEIPT)?;
    let mut message = if payload.format_version == 2 {
        DOMAIN_V2
    } else {
        DOMAIN
    }
    .to_vec();
    message.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    message.extend(bytes);
    Ok(message)
}
fn decode_hex<const N: usize>(text: &str) -> Result<[u8; N]> {
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid signature or public key encoding");
    }
    let mut bytes = [0; N];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
            .map_err(|_| "Invalid hexadecimal encoding")?;
    }
    Ok(bytes)
}
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
/// Delivery modes whose local content is attested by signed receipts.
fn signed_mode(policy: &DeliveryPolicy) -> bool {
    matches!(
        policy.delivery_mode,
        DeliveryMode::LocalCatalog | DeliveryMode::Hybrid
    )
}
pub fn policy(bytes: &[u8]) -> Result<DeliveryPolicy> {
    let policy: DeliveryPolicy =
        serde_json::from_value(azeroth_core::strict_json::value(bytes, MAX_RECEIPT)?)
            .map_err(|_| "Invalid delivery policy schema")?;
    contract::validate_policy(&policy)?;
    Ok(policy)
}
pub fn policy_hash(policy: &DeliveryPolicy) -> Result<String> {
    contract::validate_policy(policy)?;
    Ok(hash(&crate::bounded_json(policy, MAX_RECEIPT)?))
}

/// This primitive signs an explicitly supplied, validated payload. The caller is
/// responsible for actual private validation/operator review before invoking it.
/// It neither prepares a receipt from counts nor establishes completeness.
pub fn sign(
    payload: &ReceiptPayload,
    policy: &DeliveryPolicy,
    key_id: &str,
    secret: &[u8; 32],
) -> Result<ReceiptSignature> {
    contract::validate_policy(policy)?;
    if !signed_mode(policy) || payload.release_policy_sha256 != policy_hash(policy)? {
        return Err("Receipt does not match local-catalog policy");
    }
    let trusted = policy
        .trusted_signers
        .get(key_id)
        .filter(|k| !k.revoked)
        .ok_or("Signing identity untrusted or revoked")?;
    let signing = SigningKey::from_bytes(secret);
    if signing.verifying_key().to_bytes() != decode_hex::<32>(&trusted.public_key_hex)? {
        return Err("Private key does not match trusted identity");
    }
    let signature = signing.sign(&payload_bytes(payload)?);
    Ok(ReceiptSignature {
        key_id: key_id.into(),
        role: trusted.role,
        signature_hex: encode_hex(&signature.to_bytes()),
    })
}

/// Local-catalog receipts. Hybrid receipts must go through `verify_hybrid`,
/// which adds the policy's extractor and declared-digest checks.
pub fn verify(
    bytes: &[u8],
    policy: &DeliveryPolicy,
    expected: &ExpectedBindings<'_>,
) -> Result<SignedReceipt> {
    if policy.delivery_mode == DeliveryMode::Hybrid {
        return Err("Hybrid receipts require verify_hybrid");
    }
    verify_signed(bytes, policy, expected)
}
fn verify_signed(
    bytes: &[u8],
    policy: &DeliveryPolicy,
    expected: &ExpectedBindings<'_>,
) -> Result<SignedReceipt> {
    contract::validate_policy(policy)?;
    let receipt: SignedReceipt = canonical(bytes)?;
    // A policy accepts receipts of its own format only, so a format 1 policy
    // refuses everything it refused before.
    if !matches!(receipt.format_version, 1 | 2)
        || receipt.format_version != policy.format_version
        || receipt.payload.format_version != receipt.format_version
        || receipt.signatures.len() != 2
        || !signed_mode(policy)
    {
        return Err("Receipt requires local-catalog policy and two signatures");
    }
    if receipt.signatures[0].role != SignerRole::Operator
        || receipt.signatures[1].role != SignerRole::Reviewer
    {
        return Err("Noncanonical signature order");
    }
    let payload = &receipt.payload;
    if payload.candidate_source_commit != expected.candidate_source_commit
        || payload.production_sha256 != expected.production_sha256
        || payload.native_reader_revision != expected.native_reader_revision
        || payload.profile_sha256 != expected.profile_sha256
        || payload.metadata_sha256 != expected.metadata_sha256
        || payload.release_policy_sha256 != expected.release_policy_sha256
        || payload.release_policy_sha256 != policy_hash(policy)?
    {
        return Err("Stale candidate or policy binding");
    }
    let message = payload_bytes(payload)?;
    // One owner may hold both roles only under format 2, whose payload binds
    // the independent review record.
    let shared_identity_allowed = receipt.format_version == 2;
    let mut roles = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for signature in &receipt.signatures {
        let trusted = policy
            .trusted_signers
            .get(&signature.key_id)
            .filter(|k| !k.revoked)
            .ok_or("Receipt signer untrusted or revoked")?;
        if trusted.role != signature.role
            || !roles.insert(signature.role)
            || !(identities.insert(&trusted.identity) || shared_identity_allowed)
            || !keys.insert(&trusted.public_key_hex)
        {
            return Err("Distinct trusted operator and reviewer required");
        }
        let key = VerifyingKey::from_bytes(&decode_hex::<32>(&trusted.public_key_hex)?)
            .map_err(|_| "Invalid trusted public key")?;
        key.verify_strict(
            &message,
            &Signature::from_bytes(&decode_hex::<64>(&signature.signature_hex)?),
        )
        .map_err(|_| "Receipt signature verification failed")?;
    }
    if !roles.contains(&SignerRole::Operator) || !roles.contains(&SignerRole::Reviewer) {
        return Err("Independent reviewer signature missing");
    }
    Ok(receipt)
}

/// Hybrid delivery (ADR 0006 section 5): the extractor identity comes from the
/// production policy, never from the receipt, and the receipt's private catalog
/// must be one of the declared tested-content digests.
pub fn verify_hybrid(
    bytes: &[u8],
    policy: &DeliveryPolicy,
    candidate_source_commit: &str,
    production_sha256: &str,
) -> Result<SignedReceipt> {
    let local = match (policy.delivery_mode, &policy.local_content) {
        (DeliveryMode::Hybrid, Some(local)) => local,
        _ => return Err("Receipt requires a hybrid delivery policy"),
    };
    if local.expected_catalogs.is_empty() {
        return Err("Hybrid delivery has no declared expected local catalog digests");
    }
    let receipt = verify_signed(
        bytes,
        policy,
        &ExpectedBindings {
            candidate_source_commit,
            production_sha256,
            release_policy_sha256: &policy_hash(policy)?,
            native_reader_revision: &local.native_reader_revision,
            profile_sha256: &local.profile_sha256,
            metadata_sha256: &local.metadata_sha256,
        },
    )?;
    if receipt.payload.extractor_version != local.extractor_version {
        return Err("Receipt extractor differs from the tested extractor");
    }
    // The release is tested with the enUS catalog; other declared locales are
    // digests users compare locally.
    if !local.expected_catalogs.iter().any(|c| {
        c.locale == contract::REQUIRED_LOCALE
            && c.catalog_sha256 == receipt.payload.private_catalog_sha256
    }) {
        return Err("Receipt catalog is not the declared enUS tested-content digest");
    }
    Ok(receipt)
}

/// Strict, validated payload for the explicit receipt CLI. Duplicate keys,
/// unknown fields and invalid bindings refuse.
pub fn read_payload(bytes: &[u8]) -> Result<ReceiptPayload> {
    let payload: ReceiptPayload =
        serde_json::from_value(azeroth_core::strict_json::value(bytes, MAX_RECEIPT)?)
            .map_err(|_| "Invalid receipt payload schema")?;
    contract::validate_payload(&payload)?;
    Ok(payload)
}
pub fn read_signature(bytes: &[u8]) -> Result<ReceiptSignature> {
    serde_json::from_value(azeroth_core::strict_json::value(bytes, 4096)?)
        .map_err(|_| "Invalid receipt signature schema")
}
/// A key file holds exactly 64 lowercase hexadecimal characters, optionally
/// followed by one newline. Keys are never generated, searched for or printed.
pub fn read_secret_key(bytes: &[u8]) -> Result<[u8; 32]> {
    let text = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let text = std::str::from_utf8(text).map_err(|_| "Invalid secret key file")?;
    decode_hex::<32>(text).map_err(|_| "Invalid secret key file")
}
/// Canonical receipt bytes (operator signature first), as `verify` requires.
pub fn assemble(
    payload: ReceiptPayload,
    operator: ReceiptSignature,
    reviewer: ReceiptSignature,
) -> Result<Vec<u8>> {
    contract::validate_payload(&payload)?;
    if operator.role != SignerRole::Operator || reviewer.role != SignerRole::Reviewer {
        return Err("Operator and reviewer signatures are required in that order");
    }
    let receipt = SignedReceipt {
        format_version: payload.format_version,
        payload,
        signatures: vec![operator, reviewer],
    };
    serde_json::to_vec(&receipt).map_err(|_| "Canonical encoding failed")
}
/// Public key (hex) for a caller-supplied secret key, so the owner can add it
/// to the production policy's trusted signers. The secret is never printed.
pub fn public_key_hex(secret: &[u8; 32]) -> String {
    encode_hex(&SigningKey::from_bytes(secret).verifying_key().to_bytes())
}
