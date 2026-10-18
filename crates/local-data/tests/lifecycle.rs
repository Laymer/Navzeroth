//! Synthetic catalogs and private run records only; no game data required.
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InstallationMetadata, PerformancePreset},
    local_data_catalog::{DistributionClass, ResearchCatalog},
    local_data_lifecycle::{Checkpoint, CheckpointBinding, JobState},
};
use azeroth_local_data::{lifecycle, worker::ExtractionRequest};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Store(PathBuf);
impl Store {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "nav-lifecycle-boundary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        for dir in ["catalogs", "objects"] {
            fs::create_dir(root.join(dir)).unwrap();
        }
        Self(root)
    }
    fn catalog(&self, build: u32, marker: &str) -> String {
        let c = ResearchCatalog {
            format_version: 1,
            classification: DistributionClass::LocalOnly,
            installation: InstallationMetadata {
                product: "wow_classic_beta".into(),
                version: format!("1.60.1.{build}"),
                build,
                build_key: "a".repeat(32),
                cdn_key: "b".repeat(32),
                declared_locales: vec!["enUS".into()],
            },
            build_info_sha256: "a".repeat(64),
            extractor_version: "synthetic".into(),
            native_revision: CASCLIB_REVISION.into(),
            metadata_sha256: azeroth_local_data::metadata::METADATA_SHA256.into(),
            parser_profile: "synthetic".into(),
            extractor_profile_sha256: None,
            archive_statistics: None,
            input_hashes: BTreeMap::new(),
            tables: vec![],
            gaps: vec![marker.into()],
            gameplay_projection_sha256: None,
        };
        let bytes = serde_json::to_vec(&c).unwrap();
        let hash = format!("{:x}", Sha256::digest(&bytes));
        fs::write(self.0.join("catalogs").join(&hash), bytes).unwrap();
        hash
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn cp() -> Checkpoint {
    Checkpoint {
        format_version: 1,
        binding: CheckpointBinding {
            format_version: 1,
            source_sha256: "a".repeat(64),
            extractor_sha256: "b".repeat(64),
            selection_sha256: "c".repeat(64),
            preset: PerformancePreset::Small,
            total: 1,
        },
        tables: vec![],
        gaps: vec![],
    }
}
#[test]
fn corrupt_missing_stale_and_cross_build_rollback_preserve_previous_activation() {
    let store = Store::new();
    let first = store.catalog(70450, "first");
    let second = store.catalog(70450, "second");
    let other = store.catalog(70425, "other-build");
    assert_eq!(
        lifecycle::activate(&store.0, &first, 0, false)
            .unwrap()
            .generation,
        1
    );
    assert!(lifecycle::activate(&store.0, &second, 0, false).is_err());
    assert!(lifecycle::activate(&store.0, &other, 0, true).is_err());
    fs::write(store.0.join("catalogs").join(&second), b"altered").unwrap();
    assert!(lifecycle::activate(&store.0, &second, 1, false).is_err());
    assert_eq!(
        lifecycle::active(&store.0, &first).unwrap().catalog_sha256,
        Some(first.clone())
    );
    assert!(lifecycle::activate(&store.0, &"0".repeat(64), 1, true).is_err());
    let second = store.catalog(70450, "second");
    lifecycle::activate(&store.0, &second, 1, false).unwrap();
    let restored = lifecycle::activate(&store.0, &first, 2, true).unwrap();
    assert_eq!(restored.catalog_sha256, Some(first));
    assert!(!restored.verified_for_planning);
}
#[test]
fn recovery_requires_writer_lock_and_cancelled_jobs_remain_resumable() {
    let store = Store::new();
    let source = store.0.parent().unwrap().join("synthetic-selected-client");
    let request = ExtractionRequest {
        protocol_version: 1,
        resume: false,
        request_id: "recover".into(),
        installation: source,
        product: "wow_classic_beta".into(),
        destination: store.0.clone(),
        preset: PerformancePreset::Small,
        tables: vec![],
    };
    let mut db = lifecycle::index(&store.0).unwrap();
    db.begin(
        "recover",
        &serde_json::to_string(&request).unwrap(),
        &cp(),
        false,
    )
    .unwrap();
    let lock = fs::File::create(store.0.join("writer.lock")).unwrap();
    lock.lock().unwrap();
    assert_eq!(
        lifecycle::status(&store.0, "recover").unwrap().state,
        JobState::Running
    );
    drop(lock);
    assert_eq!(
        lifecycle::status(&store.0, "recover").unwrap().state,
        JobState::Interrupted
    );
    db.begin("recover", "ignored", &cp(), true).unwrap();
    assert_eq!(
        lifecycle::cancel(&store.0, "recover").unwrap().state,
        JobState::Cancelled
    );
    let resumed = lifecycle::resume_request(&store.0, "recover").unwrap();
    assert!(resumed.resume);
    assert_eq!(resumed.protocol_version, 2);
    assert_eq!(resumed.preset, PerformancePreset::Small);
    assert_eq!(resumed.request_id, "recover");
    let moved = Store::new();
    let mut other = lifecycle::index(&moved.0).unwrap();
    other
        .begin(
            "recover",
            &serde_json::to_string(&request).unwrap(),
            &cp(),
            false,
        )
        .unwrap();
    assert!(lifecycle::resume_request(&moved.0, "recover").is_err());
}
#[cfg(unix)]
#[test]
fn lifecycle_database_and_catalog_paths_refuse_symlinks() {
    use std::os::unix::fs::symlink;
    let store = Store::new();
    let outside = Store::new();
    fs::write(outside.0.join("private"), b"do-not-modify").unwrap();
    symlink(outside.0.join("private"), store.0.join("lifecycle.sqlite3")).unwrap();
    assert!(lifecycle::index(&store.0).is_err());
    assert_eq!(
        fs::read(outside.0.join("private")).unwrap(),
        b"do-not-modify"
    );
}

#[test]
fn saved_unknown_worker_protocol_is_rejected_before_resume_conversion() {
    let store = Store::new();
    let request = ExtractionRequest {
        protocol_version: 99,
        resume: false,
        request_id: "unknown".into(),
        installation: store.0.clone(),
        product: "wow_classic_beta".into(),
        destination: store.0.clone(),
        preset: PerformancePreset::Small,
        tables: vec![],
    };
    lifecycle::index(&store.0)
        .unwrap()
        .begin(
            "unknown",
            &serde_json::to_string(&request).unwrap(),
            &cp(),
            false,
        )
        .unwrap();
    assert!(lifecycle::resume_request(&store.0, "unknown").is_err());
}
#[cfg(unix)]
#[test]
fn dangling_writer_lock_is_refused_without_creating_its_target() {
    use std::os::unix::fs::symlink;
    let store = Store::new();
    let outside = Store::new();
    let target = outside.0.join("absent");
    symlink(&target, store.0.join("writer.lock")).unwrap();
    assert!(lifecycle::jobs(&store.0).is_err());
    assert!(!target.exists());
}
