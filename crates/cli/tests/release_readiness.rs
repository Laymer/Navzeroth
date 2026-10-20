use azeroth_cli::release::{Options, report};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const INVENTORY: &str = "release/v1/coverage-inventory.json";
const EVIDENCE: &str = "release/v1/acceptance-evidence.json";
struct Temp(PathBuf);
impl Temp {
    fn new(version: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "anav-release-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let temp = Self(path);
        temp.write(
            "Cargo.toml",
            &format!("[workspace.package]\nversion = \"{version}\"\n"),
        );
        for name in [
            "package.json",
            "apps/desktop/package.json",
            "apps/desktop/src-tauri/tauri.conf.json",
        ] {
            temp.write(name, &json!({"version":version}).to_string());
        }
        temp.write(
            "addons/AzerothNavigator/AzerothNavigator.toc",
            &format!("## Version: {version}\n"),
        );
        temp.write(
            "addons/AzerothNavigator/Core/Version.lua",
            &format!("addon = \"{version}\",\n"),
        );
        let lock: String = [
            "azeroth-app",
            "azeroth-cli",
            "azeroth-core",
            "azeroth-desktop",
            "azeroth-ingest",
            "azeroth-lua-harness",
            "azeroth-local-data",
            "azeroth-storage",
        ]
        .map(|name| format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"))
        .concat();
        temp.write("Cargo.lock", &lock);
        temp.write("product.txt", "unchanged product\n");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for file in [
            INVENTORY,
            EVIDENCE,
            "release/v1/accepted-origins.json",
            "release/v1/world-data-target.json",
        ] {
            let source_path = if file == INVENTORY || file == EVIDENCE {
                source
                    .join("release/history/2026-10-19-pre-stable-v3")
                    .join(Path::new(file).file_name().unwrap())
            } else {
                source.join(file)
            };
            temp.write(file, &fs::read_to_string(source_path).unwrap());
        }
        temp.git(&["init", "-q"]);
        temp.git(&["config", "user.name", "Synthetic Release Test"]);
        temp.git(&["config", "user.email", "test@example.invalid"]);
        temp.commit();
        temp
    }
    fn write(&self, name: &str, text: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn json(&self, name: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(name)).unwrap()).unwrap()
    }
    fn json_write(&self, name: &str, value: &Value) {
        self.write(name, &serde_json::to_string_pretty(value).unwrap());
    }
    fn git(&self, args: &[&str]) -> String {
        let result = Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    }
    fn commit(&self) {
        self.git(&["add", "."]);
        self.git(&["commit", "-q", "-m", "synthetic test"]);
    }
    fn options(&self, version: &str) -> Options {
        Options {
            root: self.0.clone(),
            version: version.into(),
            evidence: None,
        }
    }
    fn bind(&self, version: &str) {
        let current = report(&self.options(version)).unwrap();
        let mut evidence = self.json(EVIDENCE);
        evidence["tested_source_commit"] = json!(current.candidate_source_commit);
        evidence["production_sha256"] = json!(current.production_sha256);
        evidence["inventory_sha256"] = json!(digest(&fs::read(self.0.join(INVENTORY)).unwrap()));
        self.json_write(EVIDENCE, &evidence);
        self.commit();
    }
    fn cli(&self, version: &str) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(["release-readiness", "--root"])
            .arg(&self.0)
            .args(["--version", version])
            .output()
            .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn blocked(report: &azeroth_cli::release::Report, text: &str) -> bool {
    report.readiness.blockers.iter().any(|b| b.contains(text))
}

#[test]
fn stable_missing_acceptance_fails_and_prereleases_keep_honest_blockers() {
    for (version, allowed) in [("1.0.0", false), ("1.0.0-rc.1", true), ("0.8.1", true)] {
        let temp = Temp::new(version);
        let output = temp.cli(version);
        assert_eq!(output.status.success(), allowed);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["publication_allowed"], allowed);
        assert_eq!(value["stable_ready"], false);
        assert!(value["blockers"].as_array().unwrap().len() > 20);
        assert!(output.stderr.is_empty());
        fs::remove_file(temp.0.join(EVIDENCE)).unwrap();
        assert!(blocked(
            &report(&temp.options(version)).unwrap(),
            "Acceptance evidence is missing"
        ));
    }
}
#[test]
fn exact_tested_ancestor_survives_doc_only_commit_but_not_product_changes() {
    let temp = Temp::new("1.0.0");
    temp.bind("1.0.0");
    let initial = report(&temp.options("1.0.0")).unwrap();
    assert!(!blocked(&initial, "Tested source"));
    assert!(!blocked(&initial, "Production working"));
    temp.write("docs/report.md", "evidence-only note\n");
    temp.commit();
    let docs = report(&temp.options("1.0.0")).unwrap();
    assert_eq!(initial.production_sha256, docs.production_sha256);
    assert!(!blocked(&docs, "Tested source"));
    temp.write("product.txt", "changed\n");
    assert!(blocked(
        &report(&temp.options("1.0.0")).unwrap(),
        "Production working"
    ));
    temp.commit();
    let changed = report(&temp.options("1.0.0")).unwrap();
    assert!(blocked(&changed, "Tested source"));
    assert_ne!(changed.production_sha256, initial.production_sha256);
}

#[test]
fn explicit_delivery_policy_is_fingerprinted_and_unknown_modes_never_bypass_gates() {
    let temp = Temp::new("0.8.1");
    let before = report(&temp.options("0.8.1")).unwrap();
    let mut policy = json!({"format_version":1,"delivery_mode":"bundled_content","target":{"edition":"wow_forever","version":"1.60.1","build":"70374","interface":16001},"trusted_signers":{}});
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    let bundled = report(&temp.options("0.8.1")).unwrap();
    assert_ne!(before.production_sha256, bundled.production_sha256);
    policy["delivery_mode"] = json!("local_catalog");
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    let local = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(&local, "Local-catalog delivery requires"));
    assert!(!local.readiness.stable_ready);
    policy["delivery_mode"] = json!("unknown");
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    assert!(report(&temp.options("0.8.1")).is_err());
}
#[test]
fn hybrid_delivery_blocks_until_digests_and_a_valid_receipt_exist() {
    let temp = Temp::new("0.8.1");
    let target =
        json!({"edition":"wow_forever","version":"1.60.1","build":"70374","interface":16001});
    // Format 2 bundled delivery: signers stay irrelevant, nothing new blocks.
    let mut policy = json!({"format_version":2,"delivery_mode":"bundled_content","target":target,"trusted_signers":{}});
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    let bundled = report(&temp.options("0.8.1")).unwrap();
    assert!(!blocked(&bundled, "Hybrid"));
    // Hybrid without a local content binding is an invalid policy.
    policy["delivery_mode"] = json!("hybrid");
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    assert!(report(&temp.options("0.8.1")).is_err());
    policy["local_content"] = json!({"extractor_version":"0.9.2","native_reader_revision":"c".repeat(40),
        "profile_sha256":"a".repeat(64),"metadata_sha256":"a".repeat(64),"expected_catalogs":[]});
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    let undeclared = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(
        &undeclared,
        "no declared expected local catalog digests"
    ));
    assert!(!undeclared.readiness.stable_ready);
    policy["local_content"]["expected_catalogs"] =
        json!([{"build":"70374","locale":"enUS","catalog_sha256":"a".repeat(64)}]);
    temp.json_write("config/release-delivery-v1.json", &policy);
    temp.commit();
    assert!(blocked(
        &report(&temp.options("0.8.1")).unwrap(),
        "requires a signed local-catalog receipt"
    ));
    temp.bind("0.8.1");
    temp.write(
        "release/v1/local-catalog-receipt.json",
        "{\"format_version\":2}",
    );
    let invalid = report(&temp.options("0.8.1")).unwrap();
    assert!(!blocked(
        &invalid,
        "requires a signed local-catalog receipt"
    ));
    assert!(
        invalid
            .readiness
            .blockers
            .iter()
            .any(|b| b.contains("receipt") || b.contains("Receipt")),
        "{:?}",
        invalid.readiness.blockers
    );
    assert!(!invalid.readiness.stable_ready);
}
#[test]
fn untracked_product_and_staged_product_are_blockers() {
    let temp = Temp::new("0.8.1");
    temp.bind("0.8.1");
    temp.write("untracked.txt", "not in tested source");
    assert!(blocked(
        &report(&temp.options("0.8.1")).unwrap(),
        "Production working"
    ));
    temp.git(&["add", "untracked.txt"]);
    assert!(blocked(
        &report(&temp.options("0.8.1")).unwrap(),
        "Production working"
    ));
}
#[test]
fn ignored_addon_inputs_are_blocked_even_with_a_clean_git_status() {
    let temp = Temp::new("1.0.0");
    temp.write(".gitignore", "*.log\ntarget/\n");
    temp.commit();
    temp.bind("1.0.0");
    assert!(!blocked(
        &report(&temp.options("1.0.0")).unwrap(),
        "Addon packaging inputs"
    ));
    for name in [
        "addons/AzerothNavigator/private.log",
        "addons/AzerothNavigator/target/extra.lua",
    ] {
        temp.write(name, "must not enter a source-bound package");
        assert!(temp.git(&["status", "--porcelain"]).is_empty());
        let result = report(&temp.options("1.0.0")).unwrap();
        assert!(
            blocked(
                &result,
                "Addon packaging inputs contain an untracked or ignored file"
            ),
            "{}: {:?}",
            name,
            result.readiness.blockers
        );
        fs::remove_file(temp.0.join(name)).unwrap();
    }
}
#[test]
fn missing_or_nonancestor_tested_commit_is_not_accepted() {
    let temp = Temp::new("1.0.0");
    temp.bind("1.0.0");
    let mut evidence = temp.json(EVIDENCE);
    evidence["tested_source_commit"] = json!("f".repeat(40));
    temp.json_write(EVIDENCE, &evidence);
    assert!(blocked(
        &report(&temp.options("1.0.0")).unwrap(),
        "missing, unrelated"
    ));
    let tree = temp.git(&["rev-parse", "HEAD^{tree}"]);
    let other = temp.git(&["commit-tree", &tree, "-m", "unrelated synthetic root"]);
    evidence["tested_source_commit"] = json!(other);
    temp.json_write(EVIDENCE, &evidence);
    assert!(blocked(
        &report(&temp.options("1.0.0")).unwrap(),
        "missing, unrelated"
    ));
}
#[test]
fn wrong_target_inventory_hash_and_source_version_are_not_promoted() {
    let temp = Temp::new("0.8.1");
    temp.bind("0.8.1");
    let mut evidence = temp.json(EVIDENCE);
    evidence["target"] =
        json!({"edition":"wow_forever","version":"1.60.1","build":"70338","interface":16001});
    evidence["inventory_sha256"] = json!("f".repeat(64));
    temp.json_write(EVIDENCE, &evidence);
    let result = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(&result, "target differs"));
    assert!(blocked(&result, "inventory revision"));
    temp.write("package.json", "{\"version\":\"0.8.0\"}");
    assert!(
        report(&temp.options("0.8.1"))
            .unwrap_err()
            .contains("version differs")
    );
    assert!(!temp.cli("0.8.1").status.success());
}
#[test]
fn future_unknown_duplicate_and_oversized_manifests_fail_even_for_preview() {
    let temp = Temp::new("0.8.1");
    let original = fs::read(temp.0.join(EVIDENCE)).unwrap();
    let mut value = temp.json(EVIDENCE);
    value["schema_version"] = json!(azeroth_core::release::SCHEMA_STABLE + 1);
    temp.json_write(EVIDENCE, &value);
    assert!(report(&temp.options("0.8.1")).is_err());
    value["schema_version"] = json!(3);
    value["unexpected"] = json!(true);
    temp.json_write(EVIDENCE, &value);
    assert!(report(&temp.options("0.8.1")).is_err());
    let duplicate = String::from_utf8(original).unwrap().replacen(
        "\"schema_version\": 3",
        "\"schema_version\": 3, \"schema_version\": 3",
        1,
    );
    temp.write(EVIDENCE, &duplicate);
    assert!(report(&temp.options("0.8.1")).is_err());
    let file = fs::File::create(temp.0.join(EVIDENCE)).unwrap();
    file.set_len(azeroth_core::release::MAX_RELEASE_BYTES as u64 + 1)
        .unwrap();
    assert!(report(&temp.options("0.8.1")).is_err());
}
#[test]
fn evidence_hashes_are_checked_without_running_or_echoing_contents() {
    let temp = Temp::new("0.8.1");
    temp.write("docs/proof.txt", "return os.execute('private-sentinel')");
    let contents = fs::read(temp.0.join("docs/proof.txt")).unwrap();
    let mut value = temp.json(EVIDENCE);
    value["checks"][0]["artifacts"] =
        json!([{"path":"docs/proof.txt","bytes":contents.len(),"sha256":digest(&contents)}]);
    temp.json_write(EVIDENCE, &value);
    let good = report(&temp.options("0.8.1")).unwrap();
    // The proof and the bound accepted-origin policy (ADR 0007).
    assert_eq!(good.verified_evidence_files, 2);
    temp.write(
        "docs/proof.txt",
        "different content with same file identity",
    );
    let bad = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(&bad, "Evidence file"));
    let output = temp.cli("0.8.1");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-sentinel"));
}
#[test]
fn oversized_file_refs_refuse_before_allocating_or_opening() {
    let temp = Temp::new("0.8.1");
    let mut value = temp.json(EVIDENCE);
    value["checks"][0]["artifacts"] = json!([{"path":"docs/absent.bin","bytes":azeroth_cli::release::MAX_FILE_BYTES+1,"sha256":"f".repeat(64)}]);
    temp.json_write(EVIDENCE, &value);
    assert!(blocked(
        &report(&temp.options("0.8.1")).unwrap(),
        "size limit"
    ));
}
#[test]
fn selected_evidence_cannot_escape_root_and_malformed_options_are_errors() {
    let temp = Temp::new("0.8.1");
    let mut options = temp.options("0.8.1");
    options.evidence = Some(PathBuf::from("../private-sentinel.json"));
    assert!(report(&options).is_err());
    for args in [
        vec!["release-readiness"],
        vec!["release-readiness", "--root", "--version"],
        vec![
            "release-readiness",
            "--root",
            ".",
            "--root",
            ".",
            "--version",
            "0.8.1",
        ],
        vec!["release-readiness", "--private-sentinel", "x"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-sentinel"));
    }
}
#[cfg(unix)]
#[test]
fn symlinked_evidence_file_parent_and_selected_manifest_are_refused() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new("0.8.1");
    temp.write("docs/proof.txt", "proof");
    symlink(temp.0.join("docs"), temp.0.join("shortcut")).unwrap();
    let mut value = temp.json(EVIDENCE);
    value["checks"][0]["artifacts"] =
        json!([{"path":"shortcut/proof.txt","bytes":5,"sha256":digest(b"proof")}]);
    temp.json_write(EVIDENCE, &value);
    assert!(blocked(&report(&temp.options("0.8.1")).unwrap(), "Symlink"));
    symlink(temp.0.join(EVIDENCE), temp.0.join("linked.json")).unwrap();
    let mut options = temp.options("0.8.1");
    options.evidence = Some(PathBuf::from("linked.json"));
    assert!(report(&options).is_err());
}

fn fixture_pack(temp: &Temp, destination: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = temp.0.join("synthetic-input");
    fs::create_dir_all(input.join("datasets")).unwrap();
    fs::copy(
        root.join("fixtures/contracts/v1/reference-dataset-imagery.json"),
        input.join("datasets/synthetic.json"),
    )
    .unwrap();
    fs::copy(
        root.join("fixtures/content-pack/sources.json"),
        input.join("sources.json"),
    )
    .unwrap();
    for name in ["roads", "flight", "overview"] {
        let relative = format!("atlas/synthetic-continent/layers/{name}.png");
        let target = input.join(&relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(root.join("content/demo").join(relative), target).unwrap();
    }
    let output = temp.0.join(destination);
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    azeroth_cli::pack::build(&input, &output).unwrap();
    let manifest = fs::read(output.join("manifest.json")).unwrap();
    let parsed: Value = serde_json::from_slice(&manifest).unwrap();
    let mut evidence = temp.json(EVIDENCE);
    evidence["packs"] = json!([{"manifest":{"path":format!("{destination}/manifest.json"),"bytes":manifest.len(),"sha256":digest(&manifest)},"dataset_revisions":[parsed["entries"][0]["revision"]]}]);
    temp.json_write(EVIDENCE, &evidence);
    temp.commit();
}
#[test]
fn verified_fixture_pack_still_blocks_stable_content_acceptance() {
    let temp = Temp::new("1.0.0");
    fixture_pack(&temp, "content/offline-synthetic");
    temp.bind("1.0.0");
    let result = report(&temp.options("1.0.0")).unwrap();
    assert!(blocked(&result, "fixture content, coverage gaps"));
    assert!(!result.readiness.publication_allowed);
    let path = temp
        .0
        .join("content/offline-synthetic/entries/0000/dataset.json");
    fs::write(path, b"{}").unwrap();
    let result = report(&temp.options("1.0.0")).unwrap();
    assert!(blocked(&result, "Evidence file size does not match"));
}
#[test]
fn packs_in_evidence_only_paths_cannot_evade_production_fingerprint() {
    let temp = Temp::new("1.0.0");
    fixture_pack(&temp, "release/v1/pack");
    let result = report(&temp.options("1.0.0")).unwrap();
    assert!(blocked(
        &result,
        "Pack manifest must be tracked production data"
    ));
}

#[test]
fn candidate_command_defers_initial_binding_without_waiving_content_or_integrity() {
    let temp = Temp::new("0.8.1");
    let mut e = temp.json(EVIDENCE);
    e["inventory_sha256"] = json!(digest(&fs::read(temp.0.join(INVENTORY)).unwrap()));
    temp.json_write(EVIDENCE, &e);
    temp.commit();
    let result = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(["candidate-readiness", "--root"])
        .arg(&temp.0)
        .args(["--version", "0.8.1"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["publication_allowed"], false);
    assert_eq!(value["candidate_ready"], false); // no complete content
    // The 10 WIN-* observations and the initial binding (ADR 0007: no LIVE-* rows).
    assert_eq!(value["pending"].as_array().unwrap().len(), 11);
    assert!(!value["blockers"].as_array().unwrap().iter().any(|v| {
        v.as_str()
            .unwrap()
            .contains("Tested source commit is missing or invalid")
    }));
    e["tested_source_commit"] = json!("f".repeat(40));
    temp.json_write(EVIDENCE, &e);
    let report = report(&temp.options("0.8.1")).unwrap();
    assert!(!report.readiness.candidate_ready);
    assert!(
        report
            .readiness
            .blockers
            .iter()
            .any(|v| v.contains("Tested source"))
    );
}

/// A receipt payload bound to `candidate` and `production`, for `catalog`.
fn hybrid_payload(
    policy_sha256: &str,
    candidate: &str,
    production: &str,
    catalog: &str,
) -> azeroth_core::local_data_release::ReceiptPayload {
    use azeroth_core::{local_data_release::*, release::*};
    let digest = "a".repeat(64);
    let proof = InventoryProof {
        records: 1,
        units: 1,
        private_inventory_sha256: digest.clone(),
        independent_enumeration_sha256: digest.clone(),
        reference_graph_validation_sha256: digest.clone(),
        source_transform_closure_sha256: digest.clone(),
        coverage_review_sha256: digest.clone(),
    };
    let pass = |method| AcceptanceProof {
        method,
        status: Status::Pass,
        evidence_sha256: digest.clone(),
    };
    ReceiptPayload {
        format_version: 2,
        candidate_source_commit: candidate.into(),
        production_sha256: production.into(),
        release_policy_sha256: policy_sha256.into(),
        target: Target::forever_70374(),
        extractor_version: "0.9.2".into(),
        native_reader_revision: "c".repeat(40),
        profile_sha256: digest.clone(),
        metadata_sha256: digest.clone(),
        private_input_closure_sha256: digest.clone(),
        private_inventory_sha256: digest.clone(),
        private_catalog_sha256: catalog.into(),
        private_media_sha256: digest.clone(),
        validation_results_sha256: digest.clone(),
        scenario_dependencies_sha256: digest.clone(),
        performance_workload_sha256: digest.clone(),
        performance_results_sha256: digest.clone(),
        independent_review_sha256: Some("e".repeat(64)),
        domains: DOMAINS.map(|d| (d.into(), proof.clone())).into(),
        acceptance: LIVE_CHECKS
            .map(|c| (c.to_owned(), pass(Method::LiveGame)))
            .into_iter()
            .chain(WINDOWS_CHECKS.map(|c| (c.to_owned(), pass(Method::InteractiveWindows))))
            .collect(),
    }
}
fn signed_receipt(
    policy: &azeroth_core::local_data_release::DeliveryPolicy,
    payload: azeroth_core::local_data_release::ReceiptPayload,
) -> Vec<u8> {
    use azeroth_local_data::receipts;
    let operator = receipts::sign(&payload, policy, "operator", &[1; 32]).unwrap();
    let reviewer = receipts::sign(&payload, policy, "reviewer", &[2; 32]).unwrap();
    receipts::assemble(payload, operator, reviewer).unwrap()
}
#[test]
fn a_valid_receipt_for_the_tested_commit_clears_the_hybrid_blockers() {
    use azeroth_local_data::receipts;
    let temp = Temp::new("0.8.1");
    let signers: serde_json::Map<String, Value> = [
        ("operator", "operator", [1u8; 32]),
        ("reviewer", "reviewer", [2u8; 32]),
    ]
    .into_iter()
    .map(|(id, role, secret)| {
        (
            id.to_owned(),
            json!({"identity":"project-owner","role":role,
                "public_key_hex":receipts::public_key_hex(&secret),"revoked":false}),
        )
    })
    .collect();
    let policy_value = json!({"format_version":2,"delivery_mode":"hybrid",
        "target":{"edition":"wow_forever","version":"1.60.1","build":"70374","interface":16001},
        "trusted_signers":signers,
        "local_content":{"extractor_version":"0.9.2","native_reader_revision":"c".repeat(40),
            "profile_sha256":"a".repeat(64),"metadata_sha256":"a".repeat(64),
            "expected_catalogs":[{"build":"70374","locale":"enUS","catalog_sha256":"a".repeat(64)},
                {"build":"70374","locale":"deDE","catalog_sha256":"d".repeat(64)}]}});
    temp.json_write("config/release-delivery-v1.json", &policy_value);
    temp.commit();
    let policy =
        receipts::policy(&fs::read(temp.0.join("config/release-delivery-v1.json")).unwrap())
            .unwrap();
    let policy_sha256 = receipts::policy_hash(&policy).unwrap();
    let before = report(&temp.options("0.8.1")).unwrap();
    let tested = before.candidate_source_commit.clone();
    temp.bind("0.8.1");
    let head = temp.git(&["rev-parse", "HEAD"]);
    assert_ne!(
        tested, head,
        "evidence commit moves HEAD past the tested commit"
    );
    let hybrid = |text: &str| {
        text.contains("receipt") || text.contains("Receipt") || text.contains("Hybrid")
    };
    let receipt = |candidate: &str, catalog: &str| {
        signed_receipt(
            &policy,
            hybrid_payload(
                &policy_sha256,
                candidate,
                &before.production_sha256,
                catalog,
            ),
        )
    };
    let write = |bytes: &[u8]| {
        fs::write(temp.0.join("release/v1/local-catalog-receipt.json"), bytes).unwrap()
    };

    write(&receipt(&tested, &"a".repeat(64)));
    let valid = report(&temp.options("0.8.1")).unwrap();
    assert!(
        !valid.readiness.blockers.iter().any(|b| hybrid(b)),
        "{:?}",
        valid.readiness.blockers
    );

    write(&receipt(&head, &"a".repeat(64)));
    let stale = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(&stale, "Stale candidate or policy binding"));

    write(&receipt(&tested, &"d".repeat(64)));
    let wrong_locale = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(
        &wrong_locale,
        "not the declared enUS tested-content digest"
    ));

    write(&receipt(&tested, &"f".repeat(64)));
    let undeclared = report(&temp.options("0.8.1")).unwrap();
    assert!(blocked(
        &undeclared,
        "not the declared enUS tested-content digest"
    ));
}
