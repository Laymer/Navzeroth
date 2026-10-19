//! Signed public bundle identity, separate from historical private/local receipts.
//! Both keys are operator-controlled. Review evidence is independently recorded;
//! its signature does not prove independent control or authorship of that evidence.
use crate::{
    Result,
    receipts::{decode_hex, encode_hex},
};
use azeroth_core::{
    release::{FileRef, Target},
    types::{EditionId, Sha256Hex},
};
use ed25519_dalek::{Signature as EdSignature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
pub const MAX_BYTES: usize = 64 * 1024;
pub const FORMAT: &str = "navzeroth-bundled-content-receipt";
pub const POLICY_FORMAT: &str = "navzeroth-bundled-content-signers";
const DOMAIN: &[u8] = b"NAVZEROTH\0BUNDLED-CONTENT-RECEIPT\0V1\0";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Operator,
    ReviewEvidence,
}
const ROLES: [Role; 2] = [Role::Operator, Role::ReviewEvidence];
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub schema_version: u32,
    pub app_version: String,
    pub target: Target,
    pub bundle_manifest_sha256: Sha256Hex,
    pub production_sha256: Sha256Hex,
    pub tested_source_commit: String,
    pub review: FileRef,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedSigner {
    pub role: Role,
    pub key_id: String,
    pub public_key_hex: String,
    pub identity: String,
    pub revoked: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub format: String,
    pub schema_version: u32,
    pub operator_controls_both_keys: bool,
    pub signers: [TrustedSigner; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub role: Role,
    pub key_id: String,
    pub signature_hex: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub format: String,
    pub schema_version: u32,
    pub operator_controls_both_keys: bool,
    pub payload: Payload,
    pub signatures: [Signature; 2],
}
/// Every pin comes from independently verified caller state, never the receipt.
pub struct Expected<'a> {
    pub app_version: &'a str,
    pub target: &'a Target,
    pub bundle_manifest_sha256: &'a str,
    pub production_sha256: &'a str,
    pub tested_source_commit: &'a str,
    pub review: &'a FileRef,
}
fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
}
fn digits(value: &str) -> bool {
    !value.is_empty() && value.len() <= 9 && value.bytes().all(|c| c.is_ascii_digit())
}
impl Payload {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || azeroth_core::release::is_stable(&self.app_version).is_err()
            || EditionId::new(self.target.edition.clone()).is_err()
            || !identifier(&self.target.edition)
            || self.target.version.split('.').count() != 3
            || !self.target.version.split('.').all(digits)
            || !digits(&self.target.build)
            || self.target.interface == 0
            || self.tested_source_commit.len() != 40
            || !self
                .tested_source_commit
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            || self.review.bytes == 0
            || self.review.bytes > azeroth_core::release::MAX_RELEASE_BYTES as u64
        {
            return Err("Invalid bundled-content payload");
        }
        Ok(())
    }
}
fn public_key(signer: &TrustedSigner) -> Result<VerifyingKey> {
    let key = VerifyingKey::from_bytes(&decode_hex::<32>(&signer.public_key_hex)?)
        .map_err(|_| "Invalid bundled-content public key")?;
    if key.is_weak() {
        return Err("Weak bundled-content public key");
    }
    Ok(key)
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        if self.format != POLICY_FORMAT
            || self.schema_version != 1
            || !self.operator_controls_both_keys
            || self.signers[0].key_id == self.signers[1].key_id
            || self.signers[0].public_key_hex == self.signers[1].public_key_hex
        {
            return Err("Invalid bundled-content trust policy");
        }
        for (signer, role) in self.signers.iter().zip(ROLES) {
            if signer.role != role
                || signer.revoked
                || !identifier(&signer.key_id)
                || !text(&signer.identity)
            {
                return Err("Invalid, unordered or revoked bundled-content signer");
            }
            public_key(signer)?;
        }
        Ok(())
    }
}
fn canonical<T: Serialize + DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let value: T = serde_json::from_value(azeroth_core::strict_json::value(bytes, MAX_BYTES)?)
        .map_err(|_| "Invalid bundled-content JSON schema")?;
    if encode(&value)? != bytes {
        return Err("Noncanonical bundled-content JSON");
    }
    Ok(value)
}
/// Canonical UTF-8, without whitespace or a trailing newline.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    crate::bounded_json(value, MAX_BYTES)
}
pub fn payload(bytes: &[u8]) -> Result<Payload> {
    let value: Payload = canonical(bytes)?;
    value.validate()?;
    Ok(value)
}
pub fn policy(bytes: &[u8]) -> Result<Policy> {
    let value: Policy = canonical(bytes)?;
    value.validate()?;
    Ok(value)
}
pub fn signature(bytes: &[u8]) -> Result<Signature> {
    let value: Signature = canonical(bytes)?;
    validate_signature(&value)?;
    Ok(value)
}
fn validate_signature(signature: &Signature) -> Result<()> {
    if !identifier(&signature.key_id) {
        return Err("Invalid bundled-content signature identity");
    }
    decode_hex::<64>(&signature.signature_hex)?;
    Ok(())
}
fn message(payload: &Payload) -> Result<Vec<u8>> {
    payload.validate()?;
    let bytes = encode(payload)?;
    let mut result = DOMAIN.to_vec();
    result.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    result.extend(bytes);
    Ok(result)
}
pub fn sign(
    payload: &Payload,
    policy: &Policy,
    key_id: &str,
    secret: &[u8; 32],
) -> Result<Signature> {
    policy.validate()?;
    let signer = policy
        .signers
        .iter()
        .find(|s| s.key_id == key_id)
        .ok_or("Untrusted bundled-content key identity")?;
    let key = SigningKey::from_bytes(secret);
    if key.verifying_key() != public_key(signer)? {
        return Err("Secret key differs from trusted bundled-content signer");
    }
    Ok(Signature {
        role: signer.role,
        key_id: key_id.into(),
        signature_hex: encode_hex(&key.sign(&message(payload)?).to_bytes()),
    })
}
pub fn assemble(
    payload: Payload,
    operator: Signature,
    review_evidence: Signature,
) -> Result<Vec<u8>> {
    payload.validate()?;
    if [operator.role, review_evidence.role] != ROLES || operator.key_id == review_evidence.key_id {
        return Err("Distinct ordered operator and review-evidence signatures required");
    }
    validate_signature(&operator)?;
    validate_signature(&review_evidence)?;
    encode(&Receipt {
        format: FORMAT.into(),
        schema_version: 1,
        operator_controls_both_keys: true,
        payload,
        signatures: [operator, review_evidence],
    })
}
pub fn verify(bytes: &[u8], policy: &Policy, expected: &Expected<'_>) -> Result<Receipt> {
    policy.validate()?;
    let receipt: Receipt = canonical(bytes)?;
    if receipt.format != FORMAT
        || receipt.schema_version != 1
        || !receipt.operator_controls_both_keys
    {
        return Err("Unsupported bundled-content receipt");
    }
    receipt.payload.validate()?;
    let payload = &receipt.payload;
    if payload.app_version != expected.app_version
        || payload.target != *expected.target
        || payload.bundle_manifest_sha256.as_str() != expected.bundle_manifest_sha256
        || payload.production_sha256.as_str() != expected.production_sha256
        || payload.tested_source_commit != expected.tested_source_commit
        || payload.review != *expected.review
    {
        return Err("Bundled-content receipt differs from expected release/review pins");
    }
    let message = message(payload)?;
    for ((signature, trusted), role) in receipt.signatures.iter().zip(&policy.signers).zip(ROLES) {
        validate_signature(signature)?;
        if signature.role != role || signature.key_id != trusted.key_id {
            return Err("Bundled-content signature order or identity differs from trusted policy");
        }
        public_key(trusted)?
            .verify_strict(
                &message,
                &EdSignature::from_bytes(&decode_hex::<64>(&signature.signature_hex)?),
            )
            .map_err(|_| "Bundled-content signature verification failed")?;
    }
    Ok(receipt)
}

/// Explicit bounded CLI input selection, including externally stored secret files.
/// Refuses static links and ordinary replacement/growth; no ancestor-race sandbox claim.
pub fn read_input(path: &std::path::Path, maximum: usize) -> Result<Vec<u8>> {
    use std::{fs, io::Read};
    if maximum > MAX_BYTES {
        return Err("Bundled-content input limit exceeds bound");
    }
    let selected = crate::checked_path(path)?;
    let metadata =
        fs::symlink_metadata(&selected).map_err(|_| "Cannot inspect bundled-content input")?;
    if !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err("Bundled-content input is not a bounded regular file");
    }
    let before = crate::stamp(&metadata)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options
        .open(&selected)
        .map_err(|_| "Cannot open bundled-content input")?;
    let opened = file
        .metadata()
        .map_err(|_| "Cannot inspect bundled-content input handle")?;
    if !opened.is_file() || crate::linked(&opened) || crate::stamp(&opened)? != before {
        return Err("Bundled-content input changed before reading");
    }
    let mut bytes = Vec::new();
    (&file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read bundled-content input")?;
    let current = crate::checked_path(path)?;
    if bytes.len() as u64 != before.bytes
        || bytes.len() > maximum
        || current != selected
        || crate::stamp(
            &file
                .metadata()
                .map_err(|_| "Cannot recheck bundled-content input")?,
        )? != before
        || crate::stamp(&fs::metadata(current).map_err(|_| "Bundled-content input disappeared")?)?
            != before
    {
        return Err("Bundled-content input changed during reading");
    }
    Ok(bytes)
}
