//! Synthetic selected inputs only. No installed cache or personal export is read.
use azeroth_core::local_data_inputs::InputKind;
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InstallationMetadata, PerformancePreset},
    local_data_catalog::{DistributionClass, ResearchCatalog},
};
use azeroth_local_data::inputs;
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
            "nav-input-boundary-{}-{}",
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
fn source(store: &Store, name: &str, bytes: &[u8]) -> PathBuf {
    let directory = store.0.with_extension("source").join("enUS");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(name);
    fs::write(&path, bytes).unwrap();
    path
}
fn hotfix() -> Vec<u8> {
    let mut bytes = b"XFTH".to_vec();
    bytes.extend(9u32.to_le_bytes());
    bytes.extend(70450u32.to_le_bytes());
    bytes.extend([0; 32]);
    bytes.extend(b"XFTH");
    for v in [1u32, 5, 7, 123, 9, 3] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend([1, 0, 0, 0]);
    bytes.extend(b"abc");
    bytes
}
fn cleanup_source(store: &Store) {
    let _ = fs::remove_dir_all(store.0.with_extension("source"));
}
#[test]
fn fixture_cache_is_private_deterministic_and_bound_to_source_catalog_and_profile() {
    let s = Store::new();
    let catalog = s.catalog(70450, "unknown");
    let p = source(&s, "DBCache.bin", &hotfix());
    let a = inputs::stage_cache_fixture(
        &s.0,
        &catalog,
        &p,
        InputKind::HotfixCache,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    assert!(a.manifest.fixture);
    assert_eq!(a.manifest.record_count, Some(1));
    assert_eq!(a.manifest.conflict_count, Some(0));
    assert!(a.manifest.unsupported.is_none());
    let b = inputs::stage_cache_fixture(
        &s.0,
        &catalog,
        &p,
        InputKind::HotfixCache,
        "enUS",
        PerformancePreset::Balanced,
    )
    .unwrap();
    assert_eq!(a.input_sha256, b.input_sha256);
    assert_eq!(inputs::verify(&s.0, &a.input_sha256).unwrap(), a.manifest);
    let manifest = fs::read(s.0.join("inputs").join(&a.input_sha256)).unwrap();
    assert!(
        !String::from_utf8(manifest)
            .unwrap()
            .contains(p.to_str().unwrap())
    );
    fs::write(
        s.0.join("objects").join(&a.manifest.source_sha256),
        b"altered",
    )
    .unwrap();
    assert!(inputs::verify(&s.0, &a.input_sha256).is_err());
    cleanup_source(&s);
}
#[test]
fn unsupported_cache_is_explicit_and_does_not_claim_zero_records() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let p = source(&s, "questcache.wdb", b"WQSTarbitrary");
    let staged = inputs::stage_cache_fixture(
        &s.0,
        &c,
        &p,
        InputKind::QuestCache,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    assert_eq!(
        staged.manifest.unsupported.as_deref(),
        Some("unsupported_cache_format")
    );
    assert_eq!(staged.manifest.record_count, None);
    assert!(inputs::verify(&s.0, &staged.input_sha256).is_ok());
    cleanup_source(&s);
}
#[test]
fn mixed_build_locale_unallowlisted_name_and_nested_source_fail_before_sealing() {
    let s = Store::new();
    let c = s.catalog(70425, "unknown");
    let p = source(&s, "DBCache.bin", &hotfix());
    assert!(
        inputs::stage_cache_fixture(
            &s.0,
            &c,
            &p,
            InputKind::HotfixCache,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    let c = s.catalog(70450, "unknown");
    assert!(
        inputs::stage_cache_fixture(
            &s.0,
            &c,
            &p,
            InputKind::HotfixCache,
            "enGB",
            PerformancePreset::Small
        )
        .is_err()
    );
    let bad = source(&s, "chatcache.wdb", b"private");
    assert!(
        inputs::stage_cache_fixture(
            &s.0,
            &c,
            &bad,
            InputKind::ItemCache,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    let nested = s.0.join("enUS");
    fs::create_dir(&nested).unwrap();
    let path = nested.join("DBCache.bin");
    fs::write(&path, hotfix()).unwrap();
    assert!(
        inputs::stage_cache_fixture(
            &s.0,
            &c,
            &path,
            InputKind::HotfixCache,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    cleanup_source(&s);
}
#[cfg(unix)]
#[test]
fn selected_symlink_and_mutable_cache_without_native_closed_check_are_refused() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let p = source(&s, "original", &hotfix());
    let link = p.with_file_name("DBCache.bin");
    std::os::unix::fs::symlink(&p, &link).unwrap();
    assert!(
        inputs::stage_cache_fixture(
            &s.0,
            &c,
            &link,
            InputKind::HotfixCache,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    assert!(
        inputs::stage_cache(
            &s.0,
            &c,
            &p,
            InputKind::HotfixCache,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    cleanup_source(&s);
}
#[test]
fn observation_preserves_states_and_fixture_lineage_without_claiming_eligibility() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let json = include_str!("../../../fixtures/contracts/v1/character-snapshot.json")
        .replace("UNVERIFIED_DEMO", "70450")
        .replace("enGB", "enUS");
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    let p = source(&s, "snapshot.txt", format!("ANAV1:{json}").as_bytes());
    let staged = inputs::stage_observation(
        &s.0,
        &c,
        &p,
        InputKind::CharacterSnapshot,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    assert!(staged.manifest.fixture);
    assert!(
        staged
            .manifest
            .gaps
            .iter()
            .any(|s| s == "universal_eligibility_unknown")
    );
    assert_eq!(staged.manifest.locale_evidence, "recorded_addon_context");
    assert!(inputs::verify(&s.0, &staged.input_sha256).is_ok());
    let original = azeroth_core::contract::decode_any(&json.to_string()).unwrap();
    let retained: serde_json::Value = serde_json::from_slice(
        &fs::read(s.0.join("objects").join(&staged.manifest.analysis_sha256)).unwrap(),
    )
    .unwrap();
    assert_eq!(retained, serde_json::to_value(original).unwrap());
    let p = source(
        &s,
        "snapshot.txt",
        format!("ANAV1:{}", json.to_string().replace("enUS", "enGB")).as_bytes(),
    );
    assert!(
        inputs::stage_observation(
            &s.0,
            &c,
            &p,
            InputKind::CharacterSnapshot,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    cleanup_source(&s);
}
#[test]
fn historical_session_locale_stays_unknown_and_turn_in_is_only_contextual() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let value = serde_json::json!({"format":"anav-session-events","version":1,"fixture":true,"batch":{"character":"c1","context":{"build":{"edition":"wow_forever","build":"70450","interface":"16001"},"namespace":"demo"},"events":[{"session":"s1","sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}},{"session":"s1","sequence":2,"elapsed_ms":15,"kind":{"type":"quest_turn_in","quest":"q1"}}]}});
    let p = source(
        &s,
        "session.txt",
        format!("ANAVSESSION1:{value}").as_bytes(),
    );
    let staged = inputs::stage_observation(
        &s.0,
        &c,
        &p,
        InputKind::SessionEvents,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    assert_eq!(staged.manifest.locale_evidence, "unrecorded_addon_locale");
    assert!(
        staged
            .manifest
            .gaps
            .iter()
            .any(|s| s == "observation_locale_unknown")
    );
    assert!(inputs::verify(&s.0, &staged.input_sha256).is_ok());
    cleanup_source(&s);
}
#[test]
fn pages_are_revision_bound_and_unsupported_inputs_have_no_fake_rows() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let p = source(&s, "DBCache.bin", &hotfix());
    let a = inputs::stage_cache_fixture(
        &s.0,
        &c,
        &p,
        InputKind::HotfixCache,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    let page = inputs::cache_page(&s.0, &a.input_sha256, 0, 200).unwrap();
    assert_eq!(page.revision, a.input_sha256);
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.next_offset, None);
    assert!(inputs::cache_page(&s.0, &a.input_sha256, 0, 201).is_err());
    assert!(inputs::cache_page(&s.0, &a.input_sha256, 2, 1).is_err());
    assert!(inputs::cache_page(&s.0, &a.input_sha256, 0, 0).is_err());
    assert!(
        inputs::cache_page(&s.0, &a.input_sha256, 1, 1)
            .unwrap()
            .rows
            .is_empty()
    );
    let p = source(&s, "questcache.wdb", b"WQSTunknown");
    let a = inputs::stage_cache_fixture(
        &s.0,
        &c,
        &p,
        InputKind::QuestCache,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    assert!(inputs::cache_page(&s.0, &a.input_sha256, 0, 1).is_err());
    cleanup_source(&s);
}
#[test]
fn input_readers_reject_unknown_versions_analysis_substitution_and_changed_tuples() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let p = source(&s, "DBCache.bin", &hotfix());
    let a = inputs::stage_cache_fixture(
        &s.0,
        &c,
        &p,
        InputKind::HotfixCache,
        "enUS",
        PerformancePreset::Small,
    )
    .unwrap();
    for field in [
        "format_version",
        "analyzer_profile_sha256",
        "record_count",
        "locale_evidence",
    ] {
        let mut value = serde_json::to_value(&a.manifest).unwrap();
        value[field] = match field {
            "format_version" => serde_json::json!(2),
            "record_count" => serde_json::json!(99),
            "locale_evidence" => serde_json::json!("recorded_addon_context"),
            _ => serde_json::json!("0".repeat(64)),
        };
        let bytes = serde_json::to_vec(&value).unwrap();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        fs::write(s.0.join("inputs").join(&digest), bytes).unwrap();
        assert!(inputs::verify(&s.0, &digest).is_err(), "field {field}");
    }
    fs::write(s.0.join("objects").join(&a.manifest.analysis_sha256), b"{}").unwrap();
    assert!(inputs::verify(&s.0, &a.input_sha256).is_err());
    cleanup_source(&s);
}
