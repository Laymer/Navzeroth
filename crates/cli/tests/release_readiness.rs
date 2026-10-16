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
        for file in [INVENTORY, EVIDENCE] {
            temp.write(file, &fs::read_to_string(source.join(file)).unwrap());
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
    evidence["target"]["build"] = json!("70338");
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
    value["schema_version"] = json!(2);
    temp.json_write(EVIDENCE, &value);
    assert!(report(&temp.options("0.8.1")).is_err());
    value["schema_version"] = json!(1);
    value["unexpected"] = json!(true);
    temp.json_write(EVIDENCE, &value);
    assert!(report(&temp.options("0.8.1")).is_err());
    let duplicate = String::from_utf8(original).unwrap().replacen(
        "\"schema_version\": 1",
        "\"schema_version\": 1, \"schema_version\": 1",
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
    assert_eq!(good.verified_evidence_files, 1);
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
    assert_eq!(value["pending"].as_array().unwrap().len(), 26);
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
