//! External downloaded-draft evidence. No network, signatures or human claims.
//! API snapshots are independently reviewed recorded evidence, not API authentication.
use super::{Budget, Evidence, Result, hash};
use azeroth_core::{
    release::{self, FileRef},
    types::AssetPath,
};
use serde::{Deserialize, de::DeserializeOwned};
use std::path::Path;

const POLICY: &str = "config/release-draft-smoke-v1.json";
const MAX: usize = 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u32,
    repository: String,
    workflow_path: String,
    job_name: String,
    runner_environment: String,
    os: String,
    runner_label: String,
    runner_group_name: String,
    native_test_step_name: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub source_commit: String,
    pub production_sha256: String,
    pub installer_sha256: String,
    pub release_tag: String,
    pub release_id: u64,
    pub asset_id: u64,
    pub asset_name: String,
    pub asset_bytes: u64,
    pub run_id: u64,
    pub run_attempt: u64,
    pub job_id: u64,
    pub workflow_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResult {
    schema_version: u32,
    identity: Identity,
    phase: String,
    human_performed: bool,
    downloaded: bool,
    unsigned_verified: bool,
    installation_verified: bool,
    first_start_verified: bool,
    bundled_resources_verified: bool,
    planning_verified: bool,
    restart_persistence_verified: bool,
    ui_interaction_verified: bool,
    download_receipt: FileRef,
    native_result: FileRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    schema_version: u32,
    identity: Identity,
    repository: String,
    workflow_path: String,
    job_name: String,
    runner_environment: String,
    os: String,
    phase: String,
    run_api: FileRef,
    job_api: FileRef,
    release_api: FileRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    schema_version: u32,
    identity: Identity,
    raw_result: FileRef,
    provenance: FileRef,
    status: String,
    reviewer: String,
    recorded_at: String,
    human_performed: bool,
}
// API/verification snapshots retain their original additional provider/detail fields.
// Typed required fields reject duplicate authority-bearing keys.
#[derive(Deserialize)]
struct NativeResult {
    schema_version: u32,
    source_commit: String,
    installer_sha256: String,
    completed: bool,
    passed: bool,
    human_performed: bool,
    ui_interaction_verified: bool,
    phase: String,
    checks: NativeChecks,
    detail_files: Vec<FileRef>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeChecks {
    first_start: bool,
    bundled_activation: bool,
    source_backed_planning: bool,
    restart_persistence: bool,
}
#[derive(Deserialize)]
struct DownloadReceipt {
    workflow_run_id: String,
    workflow_run_attempt: String,
    schema_version: u32,
    phase: String,
    version: String,
    source_commit: String,
    installer_signature: String,
    application_signature: String,
    dependency_closure_verified: bool,
    addon_zip_matches_source: bool,
    assets: Vec<DownloadedAsset>,
}
#[derive(Deserialize)]
struct DownloadedAsset {
    name: String,
    bytes: u64,
    sha256: String,
}
#[derive(Deserialize)]
struct RunApi {
    id: u64,
    run_attempt: u64,
    head_sha: String,
    repository: Repository,
    path: String,
    event: String,
    status: String,
    conclusion: String,
}
#[derive(Deserialize)]
struct Repository {
    full_name: String,
}
#[derive(Deserialize)]
struct JobStep {
    name: String,
    status: String,
    conclusion: String,
}
#[derive(Deserialize)]
struct JobApi {
    steps: Vec<JobStep>,
    labels: Vec<String>,
    runner_group_name: String,
    id: u64,
    run_id: u64,
    run_attempt: u64,
    head_sha: String,
    name: String,
    status: String,
    conclusion: String,
}
#[derive(Deserialize)]
struct ReleaseApi {
    id: u64,
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<ReleasedAsset>,
}
#[derive(Deserialize)]
struct ReleasedAsset {
    id: u64,
    name: String,
    size: u64,
}
fn same(a: &Identity, b: &Identity) -> bool {
    a.source_commit == b.source_commit
        && a.production_sha256 == b.production_sha256
        && a.installer_sha256 == b.installer_sha256
        && a.release_tag == b.release_tag
        && a.release_id == b.release_id
        && a.asset_id == b.asset_id
        && a.asset_name == b.asset_name
        && a.asset_bytes == b.asset_bytes
        && a.run_id == b.run_id
        && a.run_attempt == b.run_attempt
        && a.job_id == b.job_id
        && a.workflow_sha256 == b.workflow_sha256
}
fn same_ref(a: &FileRef, b: &FileRef) -> bool {
    a.path == b.path && a.sha256 == b.sha256 && a.bytes == b.bytes
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|_| "Invalid downloaded-draft JSON evidence")
}
fn read<T: DeserializeOwned>(root: &Path, budget: &mut Budget, file: &FileRef) -> Result<T> {
    if file.bytes == 0 || file.bytes > MAX as u64 {
        return Err("Downloaded-draft evidence exceeds role bound");
    }
    budget.verify(root, file)?;
    let bytes = budget.read(&root.join(file.path.as_str()), MAX)?;
    if bytes.len() as u64 != file.bytes || hash(&bytes) != file.sha256.as_str() {
        return Err("Downloaded-draft evidence changed after verification");
    }
    json(&bytes)
}
pub(super) fn verify(
    root: &Path,
    evidence: &Evidence,
    source: &str,
    production: &str,
    version: &str,
    budget: &mut Budget,
) -> Result<()> {
    let Some(refs) = &evidence.draft_installer_smoke else {
        return Ok(());
    };
    let policy: Policy = json(&budget.read(&root.join(POLICY), 64 * 1024)?)?;
    let raw: RawResult = read(root, budget, &refs.raw_result)?;
    let p: Provenance = read(root, budget, &refs.provenance)?;
    let review: Review = read(root, budget, &refs.independent_review)?;
    let i = &raw.identity;
    let suffix = policy
        .workflow_path
        .strip_prefix(".github/workflows/")
        .filter(|p| !p.contains('/'))
        .ok_or("Invalid downloaded-draft workflow policy")?;
    AssetPath::new(suffix).map_err(|_| "Invalid downloaded-draft workflow policy")?;
    let workflow = budget.read(&root.join(&policy.workflow_path), MAX)?;
    if evidence.schema_version != release::SCHEMA_AUTOMATED_DRAFT
        || policy.schema_version != 1
        || raw.schema_version != 1
        || p.schema_version != 1
        || review.schema_version != 1
        || !same(i, &p.identity)
        || !same(i, &review.identity)
        || i.source_commit != source
        || i.production_sha256 != production
        || evidence
            .candidate_installer_sha256
            .as_ref()
            .map(|s| s.as_str())
            != Some(i.installer_sha256.as_str())
        || i.release_tag != format!("v{version}")
        || i.workflow_sha256 != hash(&workflow)
        || [
            i.release_id,
            i.asset_id,
            i.asset_bytes,
            i.run_id,
            i.run_attempt,
            i.job_id,
        ]
        .contains(&0)
        || !text(&i.asset_name)
        || !i.asset_name.ends_with(".exe")
        || p.repository != policy.repository
        || p.workflow_path != policy.workflow_path
        || p.job_name != policy.job_name
        || p.runner_environment != policy.runner_environment
        || p.os != policy.os
        || policy.runner_environment != "github-hosted"
        || policy.os != "Windows"
        || raw.phase != "draft"
        || p.phase != "draft"
        || raw.human_performed
        || review.human_performed
        || !raw.downloaded
        || !raw.unsigned_verified
        || !raw.installation_verified
        || !raw.first_start_verified
        || !raw.bundled_resources_verified
        || !raw.planning_verified
        || !raw.restart_persistence_verified
        || !raw.ui_interaction_verified
        || review.status != "pass"
        || !text(&review.reviewer)
        || !text(&review.recorded_at)
        || !same_ref(&review.raw_result, &refs.raw_result)
        || !same_ref(&review.provenance, &refs.provenance)
    {
        return Err("Downloaded-draft identity, outcome or independent review differs");
    }
    let download: DownloadReceipt = read(root, budget, &raw.download_receipt)?;
    let native: NativeResult = read(root, budget, &raw.native_result)?;
    let downloaded_asset = download.assets.iter().find(|a| a.name == i.asset_name);
    if download
        .assets
        .iter()
        .filter(|a| a.name == i.asset_name)
        .count()
        != 1
        || download.schema_version != 3
        || download.phase != "draft"
        || download.version != version
        || download.source_commit != source
        || download.workflow_run_id != i.run_id.to_string()
        || download.workflow_run_attempt != i.run_attempt.to_string()
        || download.installer_signature != "NotSigned"
        || download.application_signature != "NotSigned"
        || !download.dependency_closure_verified
        || !download.addon_zip_matches_source
        || downloaded_asset
            .is_none_or(|a| a.bytes != i.asset_bytes || a.sha256 != i.installer_sha256)
        || native.schema_version != 1
        || native.source_commit != source
        || native.installer_sha256 != i.installer_sha256
        || !native.completed
        || !native.passed
        || native.human_performed
        || !native.ui_interaction_verified
        || native.phase != "draft"
        || !native.checks.first_start
        || !native.checks.bundled_activation
        || !native.checks.source_backed_planning
        || !native.checks.restart_persistence
        || native.detail_files.is_empty()
        || native.detail_files.len() > 16
    {
        return Err("Downloaded-draft native result or download receipt differs");
    }
    let mut paths = std::collections::BTreeSet::new();
    for detail in &native.detail_files {
        if !paths.insert(detail.path.as_str()) {
            return Err("Repeated downloaded-draft native detail");
        }
        budget.verify_limit(root, detail, 8 * 1024 * 1024)?;
    }
    let run: RunApi = read(root, budget, &p.run_api)?;
    let job: JobApi = read(root, budget, &p.job_api)?;
    let release: ReleaseApi = read(root, budget, &p.release_api)?;
    let asset = release.assets.iter().find(|a| a.id == i.asset_id);
    let steps: Vec<_> = job
        .steps
        .iter()
        .filter(|s| s.name == policy.native_test_step_name)
        .collect();
    if steps.len() != 1
        || steps[0].status != "completed"
        || steps[0].conclusion != "success"
        || release.assets.iter().filter(|a| a.id == i.asset_id).count() != 1
        || release
            .assets
            .iter()
            .filter(|a| a.name == i.asset_name)
            .count()
            != 1
        || run.id != i.run_id
        || run.run_attempt != i.run_attempt
        || run.head_sha != source
        || run.repository.full_name != policy.repository
        || run.path != policy.workflow_path
        || run.event != "workflow_dispatch"
        || run.status != "completed"
        || run.conclusion != "success"
        || job.id != i.job_id
        || job.run_id != i.run_id
        || job.run_attempt != i.run_attempt
        || job.head_sha != source
        || job.name != policy.job_name
        || !job.labels.contains(&policy.runner_label)
        || job.runner_group_name != policy.runner_group_name
        || job.status != "completed"
        || job.conclusion != "success"
        || release.id != i.release_id
        || release.tag_name != i.release_tag
        || !release.draft
        || release.prerelease
        || asset.is_none_or(|a| a.name != i.asset_name || a.size != i.asset_bytes)
    {
        return Err("Downloaded-draft GitHub API provenance differs");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    const SOURCE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const PROD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    struct Fixture {
        root: PathBuf,
        raw: Value,
        provenance: Value,
        review: Value,
        run: Value,
        job: Value,
        release: Value,
        download: Value,
        native: Value,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn pin(root: &Path, path: &str, value: &Value) -> Value {
        let bytes = serde_json::to_vec(value).unwrap();
        let file = root.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, &bytes).unwrap();
        json!({"path":path,"bytes":bytes.len(),"sha256":hash(&bytes)})
    }
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "navzeroth-draft-smoke-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir_all(root.join("config")).unwrap();
            fs::write(
                root.join(POLICY),
                include_bytes!("../../../../config/release-draft-smoke-v1.json"),
            )
            .unwrap();
            let workflow = b"reviewed workflow bytes";
            fs::create_dir_all(root.join(".github/workflows")).unwrap();
            fs::write(
                root.join(".github/workflows/verify-release-downloads.yml"),
                workflow,
            )
            .unwrap();
            let identity = json!({"source_commit":SOURCE,"production_sha256":PROD,"installer_sha256":PROD,"release_tag":"v1.0.0","release_id":11,"asset_id":12,"asset_name":"setup.exe","asset_bytes":50,"run_id":13,"run_attempt":2,"job_id":14,"workflow_sha256":hash(workflow)});
            let raw = json!({"schema_version":1,"identity":identity,"phase":"draft","human_performed":false,"downloaded":true,"unsigned_verified":true,"installation_verified":true,"first_start_verified":true,"bundled_resources_verified":true,"planning_verified":true,"restart_persistence_verified":true,"ui_interaction_verified":true});
            let provenance = json!({"schema_version":1,"identity":identity,"repository":"Laymer/Navzeroth","workflow_path":".github/workflows/verify-release-downloads.yml","job_name":"Verify downloaded unsigned release","runner_environment":"github-hosted","os":"Windows","phase":"draft"});
            let review = json!({"schema_version":1,"identity":identity,"status":"pass","reviewer":"independent synthetic engineering reviewer","recorded_at":"2026-10-23T12:00:00Z","human_performed":false});
            let run = json!({"id":13,"run_attempt":2,"head_sha":SOURCE,"repository":{"full_name":"Laymer/Navzeroth"},"path":".github/workflows/verify-release-downloads.yml","event":"workflow_dispatch","status":"completed","conclusion":"success"});
            let job = json!({"id":14,"run_id":13,"run_attempt":2,"head_sha":SOURCE,"name":"Verify downloaded unsigned release","status":"completed","conclusion":"success","labels":["windows-2025"],"runner_group_name":"GitHub Actions","steps":[{"name":"Download and verify exact release","status":"completed","conclusion":"success"}]});
            let release = json!({"id":11,"tag_name":"v1.0.0","draft":true,"prerelease":false,"assets":[{"id":12,"name":"setup.exe","size":50}]});
            let download = json!({"schema_version":3,"phase":"draft","version":"1.0.0","source_commit":SOURCE,"installer_signature":"NotSigned","application_signature":"NotSigned","dependency_closure_verified":true,"addon_zip_matches_source":true,"workflow_run_id":"13","workflow_run_attempt":"2","assets":[{"name":"setup.exe","bytes":50,"sha256":PROD}]});
            let native = json!({"schema_version":1,"source_commit":SOURCE,"installer_sha256":PROD,"completed":true,"passed":true,"human_performed":false,"phase":"draft","ui_interaction_verified":true,"checks":{"first_start":true,"bundled_activation":true,"source_backed_planning":true,"restart_persistence":true},"stages":["synthetic only"]});
            Self {
                root,
                raw,
                provenance,
                review,
                run,
                job,
                release,
                download,
                native,
            }
        }
        fn evidence(&self) -> Evidence {
            let mut p = self.provenance.clone();
            for (key, value) in [
                ("run_api", &self.run),
                ("job_api", &self.job),
                ("release_api", &self.release),
            ] {
                p[key] = pin(&self.root, &format!("external/{key}.json"), value);
            }
            let provenance = pin(&self.root, "external/provenance.json", &p);
            let mut raw = self.raw.clone();
            raw["download_receipt"] = pin(&self.root, "external/download.json", &self.download);
            let mut native = self.native.clone();
            if native.get("detail_files").is_none() {
                native["detail_files"] = json!([pin(
                    &self.root,
                    "external/native-detail.json",
                    &json!({"engineering":"synthetic"})
                )]);
            }
            raw["native_result"] = pin(&self.root, "external/native.json", &native);
            let raw = pin(&self.root, "external/raw.json", &raw);
            let mut review = self.review.clone();
            review["raw_result"] = raw.clone();
            review["provenance"] = provenance.clone();
            let review = pin(&self.root, "external/review.json", &review);
            serde_json::from_value(json!({"schema_version":5,"target":null,"inventory_sha256":PROD,"tested_source_commit":SOURCE,"production_sha256":PROD,"checks":[],"coverage":[],"sources":[],"packs":[],"optional_deferrals":[],"candidate_installer_sha256":PROD,"draft_installer_smoke":{"raw_result":raw,"provenance":provenance,"independent_review":review}})).unwrap()
        }
        fn check(&self) -> Result<()> {
            verify(
                &self.root,
                &self.evidence(),
                SOURCE,
                PROD,
                "1.0.0",
                &mut Budget::default(),
            )
        }
    }
    #[test]
    fn exact_recorded_draft_and_api_binding_passes_without_human_claim() {
        let f = Fixture::new();
        assert!(f.check().is_ok());
        let mut e = f.evidence();
        e.draft_installer_smoke = None;
        assert!(verify(&f.root, &e, SOURCE, PROD, "1.0.0", &mut Budget::default()).is_ok());
    }
    #[test]
    fn outcome_and_identity_mutations_fail_even_with_rehashed_evidence() {
        for key in [
            "downloaded",
            "unsigned_verified",
            "installation_verified",
            "first_start_verified",
            "bundled_resources_verified",
            "planning_verified",
            "restart_persistence_verified",
            "ui_interaction_verified",
        ] {
            let mut f = Fixture::new();
            f.raw[key] = json!(false);
            assert!(f.check().is_err(), "{key}");
        }
        for key in [
            "source_commit",
            "production_sha256",
            "installer_sha256",
            "release_tag",
            "asset_name",
            "workflow_sha256",
        ] {
            let mut f = Fixture::new();
            f.raw["identity"][key] = json!("different");
            assert!(f.check().is_err(), "{key}");
        }
        for key in [
            "release_id",
            "asset_id",
            "asset_bytes",
            "run_id",
            "run_attempt",
            "job_id",
        ] {
            let mut f = Fixture::new();
            f.raw["identity"][key] = json!(0);
            assert!(f.check().is_err(), "{key}");
        }
        for key in [
            "repository",
            "workflow_path",
            "job_name",
            "runner_environment",
            "os",
            "phase",
        ] {
            let mut f = Fixture::new();
            f.provenance[key] = json!("wrong");
            assert!(f.check().is_err(), "{key}");
        }
        for key in ["raw", "provenance", "review"] {
            let mut f = Fixture::new();
            match key {
                "raw" => f.raw["schema_version"] = json!(2),
                "provenance" => f.provenance["schema_version"] = json!(2),
                _ => f.review["schema_version"] = json!(2),
            };
            assert!(f.check().is_err());
        }
        for (key, val) in [
            ("status", json!("fail")),
            ("reviewer", json!("")),
            ("recorded_at", json!("bad\ntext")),
            ("human_performed", json!(true)),
        ] {
            let mut f = Fixture::new();
            f.review[key] = val;
            assert!(f.check().is_err());
        }
        let mut f = Fixture::new();
        f.raw["human_performed"] = json!(true);
        assert!(f.check().is_err());
        f.raw["human_performed"] = json!(false);
        f.raw["phase"] = json!("public");
        assert!(f.check().is_err());
    }
    #[test]
    fn recorded_github_and_download_failures_cannot_be_promoted() {
        for key in [
            "id",
            "run_attempt",
            "head_sha",
            "path",
            "event",
            "status",
            "conclusion",
        ] {
            let mut f = Fixture::new();
            f.run[key] = json!("wrong");
            assert!(f.check().is_err(), "run {key}");
        }
        for key in [
            "id",
            "run_id",
            "run_attempt",
            "head_sha",
            "name",
            "status",
            "conclusion",
        ] {
            let mut f = Fixture::new();
            f.job[key] = json!("wrong");
            assert!(f.check().is_err(), "job {key}");
        }
        let mut f = Fixture::new();
        f.run["repository"]["full_name"] = json!("foreign/repository");
        assert!(f.check().is_err());
        for (key, val) in [
            ("id", json!(0)),
            ("tag_name", json!("v0.9.2")),
            ("draft", json!(false)),
            ("prerelease", json!(true)),
            ("assets", json!([])),
        ] {
            let mut f = Fixture::new();
            f.release[key] = val;
            assert!(f.check().is_err());
        }
        for key in ["name", "size"] {
            let mut f = Fixture::new();
            f.release["assets"][0][key] = json!("wrong");
            assert!(f.check().is_err());
        }
        for key in [
            "schema_version",
            "phase",
            "version",
            "source_commit",
            "installer_signature",
            "application_signature",
            "dependency_closure_verified",
            "addon_zip_matches_source",
            "workflow_run_id",
            "workflow_run_attempt",
        ] {
            let mut f = Fixture::new();
            f.download[key] = json!("wrong");
            assert!(f.check().is_err(), "download {key}");
        }
        for key in ["bytes", "sha256"] {
            let mut f = Fixture::new();
            f.download["assets"][0][key] = json!("wrong");
            assert!(f.check().is_err());
        }
        for key in [
            "schema_version",
            "source_commit",
            "installer_sha256",
            "completed",
            "passed",
            "human_performed",
        ] {
            let mut f = Fixture::new();
            f.native[key] = json!("wrong");
            assert!(f.check().is_err(), "native {key}");
        }
    }
    #[test]
    fn malformed_missing_oversized_or_changed_resources_fail_closed() {
        let f = Fixture::new();
        let e = f.evidence();
        let path = f.root.join("external/raw.json");
        fs::write(&path, b"{}").unwrap();
        assert!(verify(&f.root, &e, SOURCE, PROD, "1.0.0", &mut Budget::default()).is_err());
        let mut e = f.evidence();
        e.schema_version = 4;
        assert!(verify(&f.root, &e, SOURCE, PROD, "1.0.0", &mut Budget::default()).is_err());
        let mut e = f.evidence();
        e.draft_installer_smoke.as_mut().unwrap().raw_result.bytes = MAX as u64 + 1;
        assert!(verify(&f.root, &e, SOURCE, PROD, "1.0.0", &mut Budget::default()).is_err());
        let e = f.evidence();
        fs::remove_file(&path).unwrap();
        assert!(verify(&f.root, &e, SOURCE, PROD, "1.0.0", &mut Budget::default()).is_err());
        assert!(json::<RawResult>(b"{}").is_err());
    }
    #[test]
    fn native_ui_detail_and_exact_checks_are_mandatory() {
        for key in [
            "first_start",
            "bundled_activation",
            "source_backed_planning",
            "restart_persistence",
        ] {
            let mut f = Fixture::new();
            f.native["checks"][key] = json!(false);
            assert!(f.check().is_err());
        }
        for (key, value) in [
            ("detail_files", json!([])),
            (
                "detail_files",
                json!([{"path":"external/missing","bytes":1,"sha256":PROD}]),
            ),
            ("phase", json!("public")),
            ("ui_interaction_verified", json!(false)),
        ] {
            let mut f = Fixture::new();
            f.native[key] = value;
            assert!(f.check().is_err());
        }
        let mut f = Fixture::new();
        let detail = pin(
            &f.root,
            "external/detail.json",
            &json!({"sample":"engineering only"}),
        );
        f.native["detail_files"] = json!([detail, detail]);
        assert!(f.check().is_err());
        f.native["detail_files"] = json!(vec![detail; 17]);
        assert!(f.check().is_err());
        let mut f = Fixture::new();
        f.native["checks"]["unexpected"] = json!(true);
        assert!(f.check().is_err());
    }
    #[test]
    fn malformed_caller_policy_cannot_read_outside_source_tree() {
        for path in ["../outside.yml", "/tmp/outside.yml", "C:\\outside.yml"] {
            let f = Fixture::new();
            let mut policy: Value = serde_json::from_slice(include_bytes!(
                "../../../../config/release-draft-smoke-v1.json"
            ))
            .unwrap();
            policy["workflow_path"] = json!(path);
            fs::write(f.root.join(POLICY), serde_json::to_vec(&policy).unwrap()).unwrap();
            assert_eq!(f.check(), Err("Invalid downloaded-draft workflow policy"));
        }
    }
    #[test]
    fn ambiguous_assets_and_unexecuted_native_step_cannot_pass() {
        let mut f = Fixture::new();
        let asset = f.download["assets"][0].clone();
        f.download["assets"].as_array_mut().unwrap().push(asset);
        assert!(f.check().is_err());
        for mutation in ["same", "id", "name"] {
            let mut f = Fixture::new();
            let mut asset = f.release["assets"][0].clone();
            if mutation == "id" {
                asset["name"] = json!("other.exe");
            }
            if mutation == "name" {
                asset["id"] = json!(99);
            }
            f.release["assets"].as_array_mut().unwrap().push(asset);
            assert!(f.check().is_err());
        }
        for mutation in ["missing", "duplicate", "skipped", "failure", "in_progress"] {
            let mut f = Fixture::new();
            match mutation {
                "missing" => f.job["steps"] = json!([]),
                "duplicate" => {
                    let step = f.job["steps"][0].clone();
                    f.job["steps"].as_array_mut().unwrap().push(step);
                }
                "in_progress" => f.job["steps"][0]["status"] = json!("in_progress"),
                other => f.job["steps"][0]["conclusion"] = json!(other),
            }
            assert!(f.check().is_err(), "{mutation}");
        }
    }
    #[test]
    fn download_receipt_run_identity_is_exact_not_coerced() {
        for key in ["workflow_run_id", "workflow_run_attempt"] {
            for value in [json!(13), json!("013"), json!("13 "), Value::Null] {
                let mut f = Fixture::new();
                f.download[key] = value;
                assert!(f.check().is_err());
            }
        }
    }
}
