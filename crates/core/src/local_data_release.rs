//! Private-catalog attestation contracts. Hashes/counts are bindings, not
//! independent completeness evidence. Actual local validators and review must
//! establish their truth before signing; hosted verification proves signatures.
use crate::{
    local_data::is_sha256,
    release::{DOMAINS, LIVE_CHECKS, Method, Status, Target, WINDOWS_CHECKS},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    BundledContent,
    LocalCatalog,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SignerRole {
    Operator,
    Reviewer,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedSigner {
    pub identity: String,
    pub role: SignerRole,
    pub public_key_hex: String,
    pub revoked: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPolicy {
    pub format_version: u32,
    pub delivery_mode: DeliveryMode,
    pub target: Target,
    pub trusted_signers: BTreeMap<String, TrustedSigner>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryProof {
    pub records: u32,
    pub units: u32,
    pub private_inventory_sha256: String,
    pub independent_enumeration_sha256: String,
    pub reference_graph_validation_sha256: String,
    pub source_transform_closure_sha256: String,
    pub coverage_review_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceProof {
    pub method: Method,
    pub status: Status,
    pub evidence_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPayload {
    pub format_version: u32,
    pub candidate_source_commit: String,
    pub production_sha256: String,
    pub release_policy_sha256: String,
    pub target: Target,
    pub extractor_version: String,
    pub native_reader_revision: String,
    pub profile_sha256: String,
    pub metadata_sha256: String,
    pub private_input_closure_sha256: String,
    pub private_inventory_sha256: String,
    pub private_catalog_sha256: String,
    pub private_media_sha256: String,
    pub validation_results_sha256: String,
    pub scenario_dependencies_sha256: String,
    pub performance_workload_sha256: String,
    pub performance_results_sha256: String,
    pub domains: BTreeMap<String, InventoryProof>,
    pub acceptance: BTreeMap<String, AcceptanceProof>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSignature {
    pub key_id: String,
    pub role: SignerRole,
    pub signature_hex: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReceipt {
    pub format_version: u32,
    pub payload: ReceiptPayload,
    pub signatures: Vec<ReceiptSignature>,
}

fn identity(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 80
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn hex(text: &str, bytes: usize) -> bool {
    text.len() == bytes * 2
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn validate_policy(policy: &DeliveryPolicy) -> Result<(), &'static str> {
    if policy.format_version != 1
        || policy.target != Target::forever_70374()
        || policy.trusted_signers.len() > 64
    {
        return Err("Unsupported delivery policy");
    }
    let mut identities = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for (id, key) in &policy.trusted_signers {
        if !identity(id)
            || !identity(&key.identity)
            || !hex(&key.public_key_hex, 32)
            || !identities.insert(&key.identity)
            || !keys.insert(&key.public_key_hex)
        {
            return Err("Invalid or duplicate trusted signer");
        }
    }
    Ok(())
}
pub fn validate_payload(payload: &ReceiptPayload) -> Result<(), &'static str> {
    if payload.format_version != 1
        || payload.target != Target::forever_70374()
        || !hex(&payload.candidate_source_commit, 20)
        || !identity(&payload.extractor_version)
        || !hex(&payload.native_reader_revision, 20)
    {
        return Err("Unsupported receipt version, target or extractor identity");
    }
    for hash in [
        &payload.production_sha256,
        &payload.release_policy_sha256,
        &payload.profile_sha256,
        &payload.metadata_sha256,
        &payload.private_input_closure_sha256,
        &payload.private_inventory_sha256,
        &payload.private_catalog_sha256,
        &payload.private_media_sha256,
        &payload.validation_results_sha256,
        &payload.scenario_dependencies_sha256,
        &payload.performance_workload_sha256,
        &payload.performance_results_sha256,
    ] {
        if !is_sha256(hash) {
            return Err("Invalid receipt binding");
        }
    }
    if payload
        .domains
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != DOMAINS.into_iter().collect()
    {
        return Err("All nine private inventory domains are required");
    }
    let mut records = 0u64;
    let mut units = 0u64;
    for proof in payload.domains.values() {
        records += u64::from(proof.records);
        units += u64::from(proof.units);
        if proof.records == 0 || proof.units == 0 || proof.units > 4096 {
            return Err("Incomplete private inventory");
        }
        for hash in [
            &proof.private_inventory_sha256,
            &proof.independent_enumeration_sha256,
            &proof.reference_graph_validation_sha256,
            &proof.source_transform_closure_sha256,
            &proof.coverage_review_sha256,
        ] {
            if !is_sha256(hash) {
                return Err("Private inventory evidence binding missing");
            }
        }
    }
    if records > 250_000 || units > 4096 {
        return Err("Private gameplay inventory exceeds capacity");
    }
    if payload.acceptance.len() != LIVE_CHECKS.len() + WINDOWS_CHECKS.len() {
        return Err("Mandatory acceptance incomplete");
    }
    for (id, method) in LIVE_CHECKS.iter().map(|id| (*id, Method::LiveGame)).chain(
        WINDOWS_CHECKS
            .iter()
            .map(|id| (*id, Method::InteractiveWindows)),
    ) {
        let proof = payload
            .acceptance
            .get(id)
            .ok_or("Mandatory acceptance missing")?;
        if proof.method != method
            || proof.status != Status::Pass
            || !is_sha256(&proof.evidence_sha256)
        {
            return Err("Mandatory acceptance missing, failed or wrong method");
        }
    }
    Ok(())
}
