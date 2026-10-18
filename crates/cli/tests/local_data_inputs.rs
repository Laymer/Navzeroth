//! Synthetic selected inputs only. No installed cache or personal export is read.
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InstallationMetadata},
    local_data_catalog::{DistributionClass, ResearchCatalog},
};
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
            "nav-input-cli-{}-{}",
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
fn call(args: &[&str]) -> (i32, serde_json::Value) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(args)
        .output()
        .unwrap();
    assert!(output.stderr.is_empty());
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
#[test]
fn selected_cache_stage_verify_and_pages_share_the_bounded_research_contract() {
    let s = Store::new();
    let c = s.catalog(70450, "unknown");
    let source = s.0.with_extension("source");
    fs::create_dir_all(source.join("enUS")).unwrap();
    let file = source.join("enUS/questcache.wdb");
    fs::write(&file, b"WQSTsynthetic").unwrap();
    let dest = s.0.to_str().unwrap();
    let file = file.to_str().unwrap();
    let (code, value) = call(&[
        "local-data",
        "cache-stage-fixture",
        "--destination",
        dest,
        "--catalog",
        &c,
        "--input",
        file,
        "--kind",
        "quest",
        "--locale",
        "enUS",
        "--preset",
        "small",
    ]);
    assert_eq!(code, 0);
    assert_eq!(value["manifest"]["fixture"], true);
    assert!(value["manifest"]["record_count"].is_null());
    assert_eq!(value["manifest"]["unsupported"], "unsupported_cache_format");
    assert!(!value.to_string().contains(file));
    let digest = value["input_sha256"].as_str().unwrap();
    let (code, verified) = call(&[
        "local-data",
        "input-verify",
        "--destination",
        dest,
        "--input",
        digest,
    ]);
    assert_eq!(code, 0);
    assert_eq!(verified, value["manifest"]);
    let (code, _) = call(&[
        "local-data",
        "input-page",
        "--destination",
        dest,
        "--input",
        digest,
        "--offset",
        "0",
        "--limit",
        "201",
    ]);
    assert_eq!(code, 1);
    for kind in ["chat", "unknown"] {
        let (code, value) = call(&[
            "local-data",
            "cache-stage-fixture",
            "--destination",
            dest,
            "--catalog",
            &c,
            "--input",
            file,
            "--kind",
            kind,
            "--locale",
            "enUS",
        ]);
        assert_eq!(code, 1);
        assert!(!value.to_string().contains(file));
    }
    let (code, _) = call(&[
        "local-data",
        "input-verify",
        "--destination",
        dest,
        "--input",
        digest,
        "--input",
        digest,
    ]);
    assert_eq!(code, 2);
    fs::remove_dir_all(source).unwrap();
}

#[test]
fn research_cli_round_trip_preserves_explicit_activation_and_unknown_completeness() {
    let s = Store::new();
    let catalog = s.catalog(70450, "synthetic_empty_catalog");
    let dest = s.0.to_str().unwrap();
    let run = |command: &str, options: &[&str], expected: i32| {
        let mut args = vec!["local-data", command, "--destination", dest];
        args.extend_from_slice(options);
        let (code, value) = call(&args);
        assert_eq!(code, expected, "{command}: {value}");
        value
    };
    let coverage = run("coverage", &["--catalog", &catalog], 0);
    assert!(coverage["complete_target_inventory"].is_null());
    assert_eq!(coverage["gameplay_projection_available"], false);
    for command in ["verify", "status"] {
        run(command, &["--catalog", &catalog], 0);
    }
    let projection = run("project", &["--catalog", &catalog, "--locale", "enUS"], 3);
    let projection = projection["projection_sha256"].as_str().unwrap();
    let verified = run("projection-verify", &["--projection", projection], 0);
    assert_eq!(verified["verified"], true);
    let facts = run("guide-facts-build", &["--projection", projection], 0);
    let facts = facts["guide_facts_sha256"].as_str().unwrap();
    let facts_report = run("guide-facts-verify", &["--guide-facts", facts], 0);
    assert_eq!(facts_report["verified_for_planning"], false);
    let discovery = run(
        "media-discover",
        &["--catalog", &catalog, "--locale", "enUS"],
        0,
    );
    assert!(discovery["candidates"].as_array().unwrap().is_empty());
    let tooltip = run("tooltip", &["--projection", projection, "--spell", "1"], 0);
    assert_eq!(tooltip["verified_for_gameplay"], false);
    let delivery = run(
        "delivery-build",
        &["--projection", projection, "--guide-facts", facts],
        0,
    );
    let delivery = delivery["delivery_sha256"].as_str().unwrap();
    assert_eq!(
        run("delivery-verify", &["--delivery", delivery], 0)["fixture"],
        true
    );
    assert_eq!(
        run("delivery-active", &["--delivery", delivery], 0)["generation"],
        0
    );
    let page = run(
        "delivery-page",
        &[
            "--delivery",
            delivery,
            "--domain",
            "equipment",
            "--offset",
            "0",
            "--limit",
            "200",
        ],
        0,
    );
    assert!(page["rows"].as_array().unwrap().is_empty());
    assert!(page["total"].is_null());
    assert_eq!(
        run(
            "delivery-activate",
            &["--delivery", delivery, "--expected", "0"],
            0
        )["generation"],
        1
    );
    assert_eq!(
        run(
            "delivery-rollback",
            &["--delivery", delivery, "--expected", "1"],
            0
        )["generation"],
        2
    );
    assert!(
        run(
            "delivery-diff",
            &["--before", delivery, "--after", delivery],
            0
        )["changed"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let inventory = run("inventory-build", &["--delivery", delivery], 0);
    let inventory = inventory["inventory_sha256"].as_str().unwrap();
    let summary = run("inventory-verify", &["--inventory", inventory], 0);
    assert_eq!(summary["acceptance_ready"], false);
    for domain in summary["domains"].as_array().unwrap() {
        assert_eq!(domain["independently_complete"], false);
        let page = run(
            "inventory-page",
            &[
                "--inventory",
                inventory,
                "--domain",
                domain["domain"].as_str().unwrap(),
                "--offset",
                "0",
                "--limit",
                "200",
            ],
            0,
        );
        assert_eq!(page["total"], domain["imported_count"]);
        assert_eq!(page["independently_complete"], false);
    }
    let designs = run("route-designs", &["--inventory", inventory], 0);
    assert!(!serde_json::to_string(&designs).unwrap().contains(dest));
    assert_eq!(
        run("delivery-active", &["--delivery", delivery], 0)["generation"],
        2
    );
}
