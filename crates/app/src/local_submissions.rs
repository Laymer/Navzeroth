//! Local declarations/review/export. No uploads, activation or confidence promotion.
use crate::snapshots::SnapshotApplication;
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    submissions::{decode_local_observation, permits_local_export},
    types::{Sha256Hex, UtcMillis},
};
pub use azeroth_storage::local_submissions::{
    SubmissionAction, SubmissionGuard, SubmissionPage, SubmissionRecord, SubmissionState,
    SubmitReceipt,
};
use azeroth_storage::{SqliteStore, content_dispositions::DispositionGuard};
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
    disposition_guard: DispositionGuard,
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
    let (content, disposition) = store
        .usable_content_revision(&content.key())?
        .ok_or_else(|| invalid("The submission base is unavailable"))?;
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
        content_hash: disposition.content_sha256.clone(),
        disposition_guard: disposition.guard(),
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
        base_disposition_revision: u64,
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
        base_disposition_revision: captured.disposition_guard.state_revision,
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

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_storage::content::ContentRevision;
    use serde_json::{Value, json};
    fn permitted() -> Value {
        let mut v: Value = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        let source = &mut v["sources"][0];
        for (key, value) in [
            ("redistribution", json!("permitted")),
            ("license", json!("Synthetic test-only license")),
            ("attribution", json!("Synthetic author")),
            ("editions", json!(["wow_forever"])),
            ("verified_revision", json!("demo-r1")),
            ("allowed_fields", json!(["local_observation.price.v1"])),
            ("transformations", json!(["local_observation_export.v1"])),
        ] {
            source[key] = json!({"status":"known","value":value});
        }
        v
    }
    fn setup(value: Value) -> SnapshotApplication {
        let app = SnapshotApplication::in_memory().unwrap();
        let content: ContentRevision = serde_json::from_value(value).unwrap();
        app.with_store_mut(|store| {
            store.stage_content_revision(&content)?;
            store.activate_content_revision(&content.key())?;
            Ok(())
        })
        .unwrap();
        app
    }
    fn submit(app: &SnapshotApplication) -> SubmissionRecord {
        app.submit_local_observation(
            include_str!("../../../fixtures/desktop/submissions/synthetic-price.json"),
            true,
            None,
        )
        .unwrap()
        .record
    }
    fn accept(app: &SnapshotApplication) -> SubmissionRecord {
        let p = submit(app);
        app.change_local_submission(
            &json!({"expected":p.guard,"action":"accept"}).to_string(),
            None,
        )
        .unwrap()
    }
    fn export(
        app: &SnapshotApplication,
        record: &SubmissionRecord,
    ) -> Result<LocalSubmissionExport> {
        app.export_local_submission(&serde_json::to_string(&record.guard).unwrap(), None)
    }
    #[test]
    fn export_requires_accepted_exact_guard_and_preserves_zero_without_transport_or_confidence() {
        let app = setup(permitted());
        let pending = submit(&app);
        assert!(export(&app, &pending).is_err());
        let accepted = accept(&app);
        assert!(export(&app, &pending).is_err());
        let result = export(&app, &accepted).unwrap();
        assert!(result.byte_count <= 32768);
        assert_eq!(result.byte_count as usize, result.text.len());
        assert_eq!(
            result.sha256,
            Sha256Hex::new(format!("{:x}", Sha256::digest(result.text.as_bytes()))).unwrap()
        );
        let v: Value = serde_json::from_str(&result.text).unwrap();
        assert_eq!(v["format"], "azeroth-local-observation");
        assert_eq!(v["observation"]["payload"]["copper"]["value"], 0);
        assert!(v.get("confidence").is_none());
        assert!(v.get("character").is_none());
        assert!(!result.text.starts_with("ANAV1:"));
        assert_eq!(export(&app, &accepted).unwrap().text, result.text);
    }
    #[test]
    fn source_policy_unknown_conditional_negative_or_missing_machine_constraints_block_export() {
        for (field, replacement) in [
            ("redistribution", json!({"status":"unknown"})),
            (
                "redistribution",
                json!({"status":"known","value":"conditional"}),
            ),
            (
                "redistribution",
                json!({"status":"known","value":"not_permitted"}),
            ),
            ("license", json!({"status":"unknown"})),
            ("attribution", json!({"status":"unknown"})),
            ("editions", json!({"status":"unknown"})),
            (
                "verified_revision",
                json!({"status":"known","value":"other-revision"}),
            ),
            (
                "allowed_fields",
                json!({"status":"known","value":["unrelated_field"]}),
            ),
            (
                "transformations",
                json!({"status":"known","value":["unrelated_transform"]}),
            ),
        ] {
            let mut v = permitted();
            v["sources"][0][field] = replacement;
            let app = setup(v);
            let accepted = accept(&app);
            assert!(export(&app, &accepted).is_err(), "{field}");
        }
    }
    #[test]
    fn revoked_or_withdrawn_payloads_never_export() {
        for action in ["withdraw", "revoke"] {
            let app = setup(permitted());
            let old = accept(&app);
            let new = app
                .change_local_submission(
                    &json!({"expected":old.guard,"action":action}).to_string(),
                    Some(UtcMillis::new(0).unwrap()),
                )
                .unwrap();
            assert!(export(&app, &old).is_err());
            assert!(export(&app, &new).is_err());
        }
    }
    #[test]
    fn mutations_during_compilation_fail_final_cas_or_base_capture() {
        let app = setup(permitted());
        let old = accept(&app);
        assert!(
            app.export_local_submission_inner(&old.guard, None, || {
                app.change_local_submission(
                    &json!({"expected":old.guard,"action":"withdraw"}).to_string(),
                    None,
                )
                .unwrap();
            })
            .is_err()
        );
        let app = setup(permitted());
        let old = accept(&app);
        assert!(
            app.export_local_submission_inner(&old.guard, None, || {
                app.with_store_mut(|store| {
                    let mut v = permitted();
                    v["envelope"]["payload"]["revision"] = json!("other-r2");
                    let c: ContentRevision = serde_json::from_value(v).unwrap();
                    store.stage_content_revision(&c)?;
                    store.activate_content_revision(&c.key())?;
                    Ok(())
                })
                .unwrap();
            })
            .is_err()
        );
    }
    #[test]
    fn restricting_then_releasing_and_reactivating_the_identical_base_refuses_export_aba() {
        use azeroth_storage::content_dispositions::{
            DispositionMutation, DispositionReason, DispositionState,
        };
        let app = setup(permitted());
        let old = accept(&app);
        let outcome = app.export_local_submission_inner(&old.guard, None, || {
            app.with_store_mut(|store| {
                let (_, before) = store.usable_content_revision(&old.base)?.unwrap();
                let restricted = store.change_content_disposition(
                    &DispositionMutation {
                        guard: before.guard(),
                        state: DispositionState::Quarantined,
                        reason_code: DispositionReason::OperatorReview,
                    },
                    None,
                )?;
                store.change_content_disposition(
                    &DispositionMutation {
                        guard: restricted.disposition.guard(),
                        state: DispositionState::Eligible,
                        reason_code: DispositionReason::OperatorReview,
                    },
                    None,
                )?;
                store.activate_content_revision(&old.base)?;
                Ok(())
            })
            .unwrap();
        });
        assert!(outcome.is_err());
        assert!(export(&app, &old).is_ok());
    }
    #[test]
    fn declared_and_export_timestamps_preserve_unknown_zero_and_signed_safe_values() {
        let app = setup(permitted());
        let row = accept(&app);
        for now in [
            None,
            Some(UtcMillis::new(0).unwrap()),
            Some(UtcMillis::new(-1).unwrap()),
        ] {
            let output = app
                .export_local_submission(&serde_json::to_string(&row.guard).unwrap(), now)
                .unwrap();
            let value: Value = serde_json::from_str(&output.text).unwrap();
            assert_eq!(
                value["exported_at_utc_ms"],
                serde_json::to_value(now).unwrap()
            );
            assert!(value["observation"]["observed_at_utc_ms"].is_null());
            assert!(value.get("clock_trust").is_none());
        }
    }
    #[test]
    fn malformed_future_or_personal_input_is_refused_without_echo() {
        let app = setup(permitted());
        for text in ["private secret", &"p".repeat(16385), "{}"] {
            let e = app.submit_local_observation(text, true, None).unwrap_err();
            assert!(!e.to_string().contains("private secret"));
        }
        assert!(
            app.export_local_submission(&"x".repeat(1025), None)
                .is_err()
        );
        let row = submit(&app);
        let mut v = serde_json::to_value(&row.guard).unwrap();
        v["private_name"] = json!("secret");
        assert!(app.export_local_submission(&v.to_string(), None).is_err());
    }
    #[test]
    fn bounded_export_writer_refuses_expansion_before_handoff() {
        let app = setup(permitted());
        let row = accept(&app);
        let mut c = app
            .with_store_mut(|s| s.with_immediate_read(|s| capture(s, &row.guard)))
            .unwrap();
        c.attribution = "é".repeat(16384);
        assert!(compile(&c, None).is_err());
    }
}
