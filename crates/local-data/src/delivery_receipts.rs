//! Future frozen-target format 3. Historical receipt readers/signatures are unchanged.
use crate::{
    Result, hash,
    receipts::{canonical, decode_hex, encode_hex},
};
use azeroth_core::{
    local_data_delivery::{DeliveryPolicyV3, ReceiptPayloadV3, SIGNATURE_ROLES, SignedReceiptV3},
    local_data_release::ReceiptSignature,
    release::Target,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
const MAX: usize = 256 * 1024;
const DOMAIN: &[u8] = b"NAVZEROTH\0BOUND-DELIVERY-RECEIPT\0V3\0";
pub fn policy(bytes: &[u8]) -> Result<DeliveryPolicyV3> {
    let p: DeliveryPolicyV3 = canonical(bytes)?;
    p.validate()?;
    Ok(p)
}
pub fn policy_hash(policy: &DeliveryPolicyV3) -> Result<String> {
    policy.validate()?;
    Ok(hash(&crate::bounded_json(policy, MAX)?))
}
fn message(payload: &ReceiptPayloadV3) -> Result<Vec<u8>> {
    payload.validate()?;
    let bytes = crate::bounded_json(payload, MAX)?;
    let mut result = DOMAIN.to_vec();
    result.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    result.extend(bytes);
    Ok(result)
}
fn bindings(payload: &ReceiptPayloadV3, policy: &DeliveryPolicyV3) -> Result<()> {
    policy.validate()?;
    payload.validate()?;
    if policy.target.as_ref() != Some(&payload.target)
        || payload.release_policy_sha256 != policy_hash(policy)?
        || payload.delivery_sha256 != hash(&crate::bounded_json(&payload.binding, MAX)?)
    {
        return Err("Receipt target/policy/delivery binding mismatch");
    }
    Ok(())
}
pub fn sign(
    payload: &ReceiptPayloadV3,
    policy: &DeliveryPolicyV3,
    key_id: &str,
    secret: &[u8; 32],
) -> Result<ReceiptSignature> {
    bindings(payload, policy)?;
    let trusted = policy
        .trusted_signers
        .get(key_id)
        .filter(|k| !k.revoked)
        .ok_or("Receipt signer untrusted or revoked")?;
    let key = SigningKey::from_bytes(secret);
    if encode_hex(key.verifying_key().as_bytes()) != trusted.public_key_hex {
        return Err("Secret key does not match trusted signer");
    }
    Ok(ReceiptSignature {
        key_id: key_id.into(),
        role: trusted.role,
        signature_hex: encode_hex(&key.sign(&message(payload)?).to_bytes()),
    })
}
pub struct Expected<'a> {
    pub candidate_source_commit: &'a str,
    pub production_sha256: &'a str,
    pub delivery_sha256: &'a str,
    pub frozen_target: &'a Target,
}
/// Caller supplies the independently frozen production target, never a receipt-chosen target.
pub fn verify(
    bytes: &[u8],
    policy: &DeliveryPolicyV3,
    expected: &Expected<'_>,
) -> Result<SignedReceiptV3> {
    let receipt: SignedReceiptV3 = canonical(bytes)?;
    if receipt.format_version != 3 || receipt.signatures.len() != 2 {
        return Err("Unsupported bound-delivery receipt");
    }
    bindings(&receipt.payload, policy)?;
    if receipt.payload.target != *expected.frozen_target
        || receipt.payload.candidate_source_commit != expected.candidate_source_commit
        || receipt.payload.production_sha256 != expected.production_sha256
        || receipt.payload.delivery_sha256 != expected.delivery_sha256
    {
        return Err("Stale production target or delivery binding");
    }
    let message = message(&receipt.payload)?;
    for (signature, role) in receipt.signatures.iter().zip(SIGNATURE_ROLES) {
        if signature.role != role {
            return Err("Noncanonical receipt signature roles");
        }
        let trusted = policy
            .trusted_signers
            .get(&signature.key_id)
            .filter(|k| !k.revoked && k.role == role)
            .ok_or("Receipt signer untrusted or revoked")?;
        let key = VerifyingKey::from_bytes(&decode_hex::<32>(&trusted.public_key_hex)?)
            .map_err(|_| "Invalid trusted key")?;
        key.verify_strict(
            &message,
            &Signature::from_bytes(&decode_hex::<64>(&signature.signature_hex)?),
        )
        .map_err(|_| "Bound-delivery signature invalid")?;
    }
    Ok(receipt)
}
/// Production entry point stays blocked while ADR 0006's launch target is pending.
pub fn verify_current_release(
    bytes: &[u8],
    policy: &DeliveryPolicyV3,
    candidate: &str,
    production: &str,
    delivery: &str,
) -> Result<SignedReceiptV3> {
    let frozen = azeroth_core::release::frozen_release_target()
        .ok_or("Post-launch release target is not frozen")?;
    verify(
        bytes,
        policy,
        &Expected {
            candidate_source_commit: candidate,
            production_sha256: production,
            delivery_sha256: delivery,
            frozen_target: &frozen,
        },
    )
}
pub fn read_payload(bytes: &[u8]) -> Result<ReceiptPayloadV3> {
    let payload: ReceiptPayloadV3 =
        serde_json::from_value(azeroth_core::strict_json::value(bytes, MAX)?)
            .map_err(|_| "Invalid format 3 receipt payload")?;
    payload.validate()?;
    Ok(payload)
}
pub fn assemble(
    payload: ReceiptPayloadV3,
    operator: ReceiptSignature,
    reviewer: ReceiptSignature,
) -> Result<Vec<u8>> {
    payload.validate()?;
    if [operator.role, reviewer.role] != SIGNATURE_ROLES || operator.key_id == reviewer.key_id {
        return Err("Independent ordered receipt signatures required");
    }
    decode_hex::<64>(&operator.signature_hex)?;
    decode_hex::<64>(&reviewer.signature_hex)?;
    crate::bounded_json(
        &SignedReceiptV3 {
            format_version: 3,
            payload,
            signatures: vec![operator, reviewer],
        },
        MAX,
    )
}
