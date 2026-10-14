//! Local declarations/review/export. No uploads, activation or confidence promotion.
use crate::snapshots::SnapshotApplication;
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    submissions::{decode_local_observation, permits_local_export},
    types::{Sha256Hex, UtcMillis},
};
use azeroth_storage::{
    SqliteStore,
    local_submissions::{
        SubmissionAction, SubmissionGuard, SubmissionPage, SubmissionRecord, SubmissionState,
        SubmitReceipt,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_EXPORT_BYTES: usize = 32 * 1024;
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
#[derive(Clone, Debug, Serialize)]
pub struct LocalSubmissionExport {
    pub format: &'static str,
    pub schema_version: u32,
    pub text: String,
    pub byte_count: u32,
    pub sha256: Sha256Hex,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeRequest {
    expected: SubmissionGuard,
    action: SubmissionAction,
}
fn decode_request<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > 1024 {
        return Err(invalid("Local submission request exceeds bounds"));
    }
    serde_json::from_str(text).map_err(|_| invalid("Invalid local submission request"))
}
#[derive(Clone, PartialEq)]
struct Capture {
    record: SubmissionRecord,
    content_hash: Sha256Hex,
    policy_hash: Sha256Hex,
    license: String,
    attribution: String,
}
fn hash<T: Serialize>(value: &T) -> Result<Sha256Hex> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| invalid("Local export data cannot be encoded"))?;
    Sha256Hex::new(format!("{:x}", Sha256::digest(bytes)))
}
fn capture(store: &SqliteStore, expected: &SubmissionGuard) -> Result<Capture> {
    expected.validate()?;
    let record = store
        .local_submission(&expected.id)?
        .ok_or_else(|| invalid("No such local submission"))?;
    if record.guard != *expected || record.state != SubmissionState::Accepted {
        return Err(invalid(
            "Export requires the unchanged locally accepted submission",
        ));
    }
    let doc = record
        .document
        .as_ref()
        .ok_or_else(|| invalid("The local payload was erased"))?;
    // Conservative scope: only the exact currently active base may be exported.
    // Governance makes this read fail closed for ineligible dispositions.
    let content = store
        .active_content_revision(&record.base.scope)?
        .filter(|c| c.key() == record.base)
        .ok_or_else(|| invalid("The submission base is no longer active and eligible"))?;
    let source = content
        .sources
        .iter()
        .find(|s| s.id == doc.source)
        .ok_or_else(|| invalid("Local observation source is not registered"))?;
    if !permits_local_export(source, doc) {
        return Err(AppError::new(
            ErrorCode::ProviderUnavailable,
            "Recorded source policy does not permit this local export",
            false,
        ));
    }
    Ok(Capture {
        content_hash: hash(&content)?,
        policy_hash: hash(source)?,
        license: source
            .license
            .value()
            .ok_or_else(|| invalid("Recorded license is unavailable"))?
            .as_str()
            .to_owned(),
        attribution: source
            .attribution
            .value()
            .ok_or_else(|| invalid("Recorded attribution is unavailable"))?
            .as_str()
            .to_owned(),
        record,
    })
}
fn compile(captured: &Capture, now: Option<UtcMillis>) -> Result<LocalSubmissionExport> {
    #[derive(Serialize)]
    struct Document<'a> {
        format: &'static str,
        schema_version: u32,
        review_scope: &'static str,
        permission_scope: &'static str,
        guard: &'a SubmissionGuard,
        base_content_sha256: &'a Sha256Hex,
        source_policy_sha256: &'a Sha256Hex,
        license: &'a str,
        attribution: &'a str,
        exported_at_utc_ms: Option<UtcMillis>,
        observation: &'a azeroth_core::submissions::LocalObservation,
    }
    let value = Document {
        format: "azeroth-local-observation",
        schema_version: 1,
        review_scope: "local_review_only_not_verified_gameplay",
        permission_scope: "recorded_policy_not_independently_authenticated",
        guard: &captured.record.guard,
        base_content_sha256: &captured.content_hash,
        source_policy_sha256: &captured.policy_hash,
        license: &captured.license,
        attribution: &captured.attribution,
        exported_at_utc_ms: now,
        observation: captured
            .record
            .document
            .as_ref()
            .ok_or_else(|| invalid("The local payload was erased"))?,
    };
    struct Bounded {
        bytes: Vec<u8>,
    }
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_EXPORT_BYTES - self.bytes.len() {
                return Err(std::io::Error::other("local export bound"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded { bytes: Vec::new() };
    serde_json::to_writer(&mut out, &value)
        .map_err(|_| invalid("Local export exceeds its 32 KiB bound"))?;
    let sha256 = Sha256Hex::new(format!("{:x}", Sha256::digest(&out.bytes)))?;
    let byte_count = out.bytes.len() as u32;
    let text =
        String::from_utf8(out.bytes).map_err(|_| invalid("Local export cannot be encoded"))?;
    Ok(LocalSubmissionExport {
        format: "azeroth-local-observation",
        schema_version: 1,
        text,
        byte_count,
        sha256,
    })
}
impl SnapshotApplication {
    pub fn local_submission_page(
        &self,
        after: Option<&Sha256Hex>,
        limit: usize,
    ) -> Result<SubmissionPage> {
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| store.local_submission_page(after, limit))
        })
    }
    pub fn submit_local_observation(
        &self,
        text: &str,
        opt_in: bool,
        now: Option<UtcMillis>,
    ) -> Result<SubmitReceipt> {
        let doc = decode_local_observation(text)?;
        self.with_store_mut(|store| store.submit_local_observation(&doc, opt_in, now))
    }
    pub fn change_local_submission(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<SubmissionRecord> {
        let request: ChangeRequest = decode_request(text)?;
        self.with_store_mut(|store| {
            store.change_local_submission(&request.expected, request.action, now)
        })
    }
    pub fn export_local_submission(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<LocalSubmissionExport> {
        let expected: SubmissionGuard = decode_request(text)?;
        self.export_local_submission_inner(&expected, now, || {})
    }
    fn export_local_submission_inner(
        &self,
        expected: &SubmissionGuard,
        now: Option<UtcMillis>,
        before_handoff: impl FnOnce(),
    ) -> Result<LocalSubmissionExport> {
        let initial = self
            .with_store_mut(|store| store.with_immediate_read(|store| capture(store, expected)))?;
        let output = compile(&initial, now)?;
        before_handoff();
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                if capture(store, expected)? != initial {
                    return Err(invalid(
                        "Local submission export inputs changed; refresh before exporting",
                    ));
                }
                Ok(output)
            })
        })
    }
}
