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
    /// ADR 0006 section 5: bundled packs plus content each user builds from
    /// their own client, bound by `local_content` and a signed receipt.
    Hybrid,
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
    /// Required for (and only for) hybrid delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_content: Option<LocalContent>,
}
/// The tested-content digest of hybrid delivery: the pinned extractor identity
/// and the semantic catalog digests a user's local build must reproduce.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalContent {
    pub extractor_version: String,
    pub native_reader_revision: String,
    pub profile_sha256: String,
    pub metadata_sha256: String,
    /// Empty means not yet declared (they are frozen after launch); the
    /// release gate blocks until it is filled.
    pub expected_catalogs: Vec<ExpectedCatalog>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCatalog {
    pub build: String,
    pub locale: String,
    pub catalog_sha256: String,
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
    /// Format 2 (ADR 0006 section 6): SHA-256 of the independent review record
    /// the reviewer signature attests to. Required in format 2, absent in 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independent_review_sha256: Option<String>,
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
/// Format 1 requires distinct operator and reviewer identities. Format 2
/// (ADR 0006 section 6) lets one owner identity hold one operator key and one
/// reviewer key; keys are always distinct, and receipts must then bind an
/// independent review record.
pub fn validate_policy(policy: &DeliveryPolicy) -> Result<(), &'static str> {
    if !matches!(policy.format_version, 1 | 2)
        || policy.target != Target::forever_70374()
        || policy.trusted_signers.len() > 64
    {
        return Err("Unsupported delivery policy");
    }
    let mut roles_by_identity = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for (id, key) in &policy.trusted_signers {
        let identity_ok = if policy.format_version == 1 {
            identities.insert(&key.identity)
        } else {
            roles_by_identity.insert((&key.identity, key.role))
        };
        if !identity(id)
            || !identity(&key.identity)
            || !hex(&key.public_key_hex, 32)
            || !identity_ok
            || !keys.insert(&key.public_key_hex)
        {
            return Err("Invalid or duplicate trusted signer");
        }
    }
    match (policy.delivery_mode, &policy.local_content) {
        (DeliveryMode::Hybrid, Some(local)) if policy.format_version == 2 => {
            validate_local_content(local, &policy.target)
        }
        (DeliveryMode::Hybrid, _) => Err("Hybrid delivery requires format 2 and local content"),
        (_, Some(_)) => Err("Local content binding requires hybrid delivery"),
        (_, None) => Ok(()),
    }
}
fn locale(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 4
        && b[..2].iter().all(u8::is_ascii_lowercase)
        && b[2..].iter().all(u8::is_ascii_uppercase)
}
fn build(text: &str) -> bool {
    !text.is_empty() && text.len() <= 9 && text.bytes().all(|b| b.is_ascii_digit())
}
/// The locale every hybrid release is tested with (ADR 0006 section 7).
pub const REQUIRED_LOCALE: &str = "enUS";
fn validate_local_content(local: &LocalContent, target: &Target) -> Result<(), &'static str> {
    if !identity(&local.extractor_version)
        || !hex(&local.native_reader_revision, 20)
        || !is_sha256(&local.profile_sha256)
        || !is_sha256(&local.metadata_sha256)
        || local.expected_catalogs.len() > 64
    {
        return Err("Invalid local content binding");
    }
    let mut keys = BTreeSet::new();
    for catalog in &local.expected_catalogs {
        if !build(&catalog.build)
            || catalog.build != target.build
            || !locale(&catalog.locale)
            || !is_sha256(&catalog.catalog_sha256)
            || !keys.insert((&catalog.build, &catalog.locale))
        {
            return Err("Invalid or duplicate expected local catalog");
        }
    }
    // Declared digests must include the tested enUS catalog; empty means
    // undeclared and blocks at the release gate.
    if !local.expected_catalogs.is_empty()
        && !local
            .expected_catalogs
            .iter()
            .any(|c| c.locale == REQUIRED_LOCALE)
    {
        return Err("Expected local catalogs must include the enUS catalog");
    }
    Ok(())
}
pub fn validate_payload(payload: &ReceiptPayload) -> Result<(), &'static str> {
    let review_ok = match (payload.format_version, &payload.independent_review_sha256) {
        (1, None) => true,
        (2, Some(hash)) => is_sha256(hash),
        _ => false,
    };
    if !review_ok
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
    validate_inventory_acceptance(&payload.domains, &payload.acceptance)
}
/// Shared evidence invariants; target/version/signature rules remain format-specific.
pub(crate) fn validate_inventory_acceptance(
    domains: &BTreeMap<String, InventoryProof>,
    acceptance: &BTreeMap<String, AcceptanceProof>,
) -> Result<(), &'static str> {
    if domains.keys().map(String::as_str).collect::<BTreeSet<_>>() != DOMAINS.into_iter().collect()
    {
        return Err("All nine private inventory domains are required");
    }
    let mut records = 0u64;
    let mut units = 0u64;
    for proof in domains.values() {
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
    if acceptance.len() != LIVE_CHECKS.len() + WINDOWS_CHECKS.len() {
        return Err("Mandatory acceptance incomplete");
    }
    for (id, method) in LIVE_CHECKS.iter().map(|id| (*id, Method::LiveGame)).chain(
        WINDOWS_CHECKS
            .iter()
            .map(|id| (*id, Method::InteractiveWindows)),
    ) {
        let proof = acceptance.get(id).ok_or("Mandatory acceptance missing")?;
        if proof.method != method
            || proof.status != Status::Pass
            || !is_sha256(&proof.evidence_sha256)
        {
            return Err("Mandatory acceptance missing, failed or wrong method");
        }
    }
    Ok(())
}
