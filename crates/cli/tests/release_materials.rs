//! Synthetic temporary source contracts only. No upstream records or rights
//! claims are committed, accepted, or distributed by these transport tests.
use azeroth_cli::{
    pack,
    release::{Options, materials},
};
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
const POLICY: &str = "release/v1/accepted-origins.json";
const TARGET: &str = "release/v1/world-data-target.json";
const RAW: &[u8] =
    b"-- synthetic inert source only\nreturn os.execute('never-execute-materials')\0\xff";
struct Temp(PathBuf);
impl Temp {
    fn new(content: bool) -> Self {
        let temp = Self(std::env::temp_dir().join(format!(
            "anav-materials-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir(&temp.0).unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            "package.json",
            "apps/desktop/package.json",
            "apps/desktop/src-tauri/tauri.conf.json",
            "addons/AzerothNavigator/AzerothNavigator.toc",
            "addons/AzerothNavigator/Core/Version.lua",
            INVENTORY,
            EVIDENCE,
            POLICY,
            TARGET,
        ] {
            // Keep this synthetic preview independent of the release under test.
            // Stable-version policy is exercised by the readiness tests separately.
            let source_path = if name == INVENTORY || name == EVIDENCE {
                root.join("release/history/2026-10-19-pre-stable-v3")
                    .join(Path::new(name).file_name().unwrap())
            } else {
                root.join(name)
            };
            let source = fs::read_to_string(source_path).unwrap();
            let current = env!("CARGO_PKG_VERSION");
            let fixture = if name == "Cargo.lock" {
                source
                    .split("[[package]]")
                    .map(|part| {
                        if part.trim_start().starts_with("name = \"azeroth-") {
                            part.replace(&format!("version = \"{current}\""), "version = \"0.8.1\"")
                        } else {
                            part.to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("[[package]]")
            } else if (name == INVENTORY || name == EVIDENCE) && content {
                // Content binds only to a frozen target; while the schema 2 target is
                // pending, these material tests run on the still-supported schema 1.
                frozen_v1(&source)
            } else if name.starts_with("release/") {
                source
            } else {
                source.replace(current, "0.8.1")
            };
            temp.write(name, fixture.as_bytes());
        }
        temp.write(".gitignore", b"dist/\ntarget/\n*.log\n");
        if content {
            temp.content(&root);
        }
        temp.git(&["init", "-q"]);
        temp.git(&["config", "user.name", "Synthetic Material Test"]);
        temp.git(&["config", "user.email", "test@example.invalid"]);
        temp.commit();
        temp
    }
    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }
    fn json(&self, path: &str, value: &Value) {
        self.write(path, &serde_json::to_vec_pretty(value).unwrap());
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
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    }
    fn commit(&self) {
        self.git(&["add", "."]);
        self.git(&["commit", "-q", "-m", "synthetic transport fixture"]);
    }
    fn options(&self) -> Options {
        Options {
            root: self.0.clone(),
            version: "0.8.1".into(),
            evidence: None,
        }
    }
    fn reference(&self, path: &str) -> Value {
        let bytes = fs::read(self.0.join(path)).unwrap();
        json!({"path":path,"bytes":bytes.len(),"sha256":digest(&bytes)})
    }
    fn bind_inventory(&self) {
        let mut evidence = self.read(EVIDENCE);
        evidence["inventory_sha256"] = json!(digest(&fs::read(self.0.join(INVENTORY)).unwrap()));
        self.json(EVIDENCE, &evidence);
    }
    fn content(&self, root: &Path) {
        let input = self.0.join("pack-input");
        fs::create_dir_all(input.join("datasets")).unwrap();
        let mut dataset: Value = serde_json::from_slice(
            &fs::read(root.join("fixtures/contracts/v1/reference-dataset.json")).unwrap(),
        )
        .unwrap();
        dataset["fixture"] = json!(false);
        // Exercise the ordinary pack path in an isolated synthetic repository;
        // these test declarations are never accepted as real game evidence.
        dataset["producer"]["component"] = json!("cli");
        dataset["provenance"]["source"] = json!("manual_entry");
        dataset["namespace"] = json!("reference");
        dataset["game"]["build"] = json!("1.60.1.70374");
        dataset["game"]["interface"] = json!("16001");
        for (name, values) in dataset["payload"].as_object_mut().unwrap() {
            if name != "revision" && name != "evidence" {
                *values = json!([]);
            }
        }
        dataset["payload"]["revision"] = json!("synthetic-material-test-r1");
        let evidence = &mut dataset["payload"]["evidence"][0];
        evidence["context"] = json!({"build":{"edition":"wow_forever","build":"1.60.1.70374","interface":"16001"},"namespace":"reference"});
        evidence["license"] = json!({"status":"known","value":"Synthetic test policy only"});
        fs::write(
            input.join("datasets/one.json"),
            serde_json::to_vec_pretty(&dataset).unwrap(),
        )
        .unwrap();
        let mut sources: Value = serde_json::from_slice(
            &fs::read(root.join("fixtures/content-pack/sources.json")).unwrap(),
        )
        .unwrap();
        sources["sources"][0]["verified_revision"] =
            json!({"status":"known","value":"1".repeat(40)});
        sources["sources"][0]["offline_usable"] = json!({"status":"known","value":true});
        fs::write(
            input.join("sources.json"),
            serde_json::to_vec_pretty(&sources).unwrap(),
        )
        .unwrap();
        fs::create_dir(self.0.join("content")).unwrap();
        let output = self.0.join("content/material-test");
        let report = pack::build(&input, &output).unwrap();
        assert_eq!(report.coverage_gaps, 0);
        assert!(report.distribution_eligible);
        self.write("vendor/raw.lua", RAW);
        self.write(
            "vendor/license.txt",
            b"Synthetic original terms remain unchanged.\r\n",
        );
        self.write(
            "vendor/attribution.txt",
            b"Synthetic transport-test author.\n",
        );
        self.write(
            "docs/private-proof.txt",
            b"PRIVATE-PROOF-NOT-FOR-DISTRIBUTION\n",
        );
        let mut inventory = self.read(INVENTORY);
        inventory["sources"] = json!([{"id":"demo-source-1","revision":"1".repeat(40)}]);
        self.json(INVENTORY, &inventory);
        let mut evidence = self.read(EVIDENCE);
        evidence["sources"] = json!([{"id":"demo-source-1","revision":"1".repeat(40),"origin_edition":"synthetic-test-only","origin_build":null,
            "redistribution":"permitted","files":[self.reference("vendor/raw.lua")],
            "license_files":[self.reference("vendor/license.txt")],"attribution_files":[self.reference("vendor/attribution.txt")],
            "permission_evidence":[self.reference("docs/private-proof.txt")],"target_validation":"pass","target_evidence":[self.reference("docs/private-proof.txt")]}]);
        let proof = json!({"url":"https://example.invalid/synthetic-proof","revision":"1".repeat(40),"sha256":digest(&fs::read(self.0.join("docs/private-proof.txt")).unwrap()),"reviewed_scope":"Synthetic fixture only; no gameplay rights"});
        let source = &sources["sources"][0];
        self.json("docs/admission-registry.json", &json!({"schema_version":1,"sources":[{
            "id":"demo-source-1","revision":"1".repeat(40),"publisher":source["publisher"],
            "origin":{"edition":"synthetic-test-only","build":null},
            "rights":{"status":"admitted","license":"permissive","license_identifier":source["license"]["value"],"reviewer":"synthetic-test","attribution":source["attribution"]["value"],"obligations":"Retain synthetic notices","evidence":[proof]},
            "retrieval":{"method":"manual_local","max_total_bytes":RAW.len(),"minimum_interval_ms":1000,"max_requests":1,"access_evidence":proof},
            "files":[{"path":"vendor/raw.lua","url":"https://example.invalid/raw.lua","sha256":digest(RAW),"bytes":RAW.len(),"fields":source["allowed_fields"]["value"],"transformation":"canonical_typed_serialization"}]
        }]}));
        evidence["admission_registry"] = self.reference("docs/admission-registry.json");
        evidence["packs"] = json!([{"manifest":self.reference("content/material-test/manifest.json"),"dataset_revisions":["synthetic-material-test-r1"]}]);
        self.json(EVIDENCE, &evidence);
        self.bind_inventory();
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn frozen_v1(source: &str) -> String {
    let mut value: Value = serde_json::from_str(source).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert("schema_version".into(), json!(1));
    object.insert(
        "target".into(),
        json!({"edition":"wow_forever","version":"1.60.1","build":"70374","interface":16001}),
    );
    object.remove("slice");
    // Schema 1 has no accepted-origin binding and still lists the LIVE-* rows.
    object.remove("origin_policy");
    if let Some(checks) = object.get_mut("checks").and_then(Value::as_array_mut)
        && !checks
            .iter()
            .any(|c| c["id"].as_str().is_some_and(|id| id.starts_with("LIVE-")))
    {
        let live = azeroth_core::release::LIVE_CHECKS.map(|id| {
            json!({"id":id,"status":"unverified","method":"live_game","operator":null,
                "recorded_at":null,"environment":null,"artifacts":[]})
        });
        checks.splice(0..0, live);
    }
    if let Some(domains) = object.get_mut("domains").and_then(Value::as_array_mut) {
        for domain in domains {
            domain.as_object_mut().unwrap().remove("out_of_scope");
        }
    }
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn selected_closure_is_deterministic_inert_exact_and_private_proofs_are_excluded() {
    let temp = Temp::new(true);
    let options = temp.options();
    let one = materials::plan(&options).unwrap();
    assert_eq!(one.files.len(), 6);
    assert_eq!(one.sources.len(), 1);
    let bytes = one.bytes().unwrap();
    assert_eq!(bytes, materials::plan(&options).unwrap().bytes().unwrap());
    assert!(!String::from_utf8_lossy(&bytes).contains("private-proof"));
    assert!(!String::from_utf8_lossy(&bytes).contains("permission_evidence"));
    assert!(!String::from_utf8_lossy(&bytes).contains("admission-registry"));
    let stage = temp.0.join("dist");
    fs::create_dir(&stage).unwrap();
    let output = stage.join("materials");
    assert_eq!(
        materials::stage(&options, &output)
            .unwrap()
            .bytes()
            .unwrap(),
        bytes
    );
    assert_eq!(fs::read(output.join(materials::MANIFEST)).unwrap(), bytes);
    for file in &one.files {
        assert_eq!(
            fs::read(output.join(file.archive_path.as_str())).unwrap(),
            fs::read(temp.0.join(file.input.path.as_str())).unwrap()
        );
    }
    assert_eq!(
        fs::read(output.join("source-materials/vendor/raw.lua")).unwrap(),
        RAW
    );
    assert!(!temp.0.join("never-execute-materials").exists());
    assert!(materials::stage(&options, &output).is_err());
    assert_eq!(
        fs::read(output.join("source-materials/vendor/raw.lua")).unwrap(),
        RAW
    );
    assert!(
        !azeroth_cli::release::report(&options)
            .unwrap()
            .readiness
            .stable_ready
    );
}

#[test]
fn declared_public_notices_can_also_support_permission_without_selecting_proof_copies() {
    for notice in ["license", "attribution"] {
        for separate_copy in [false, true] {
            let temp = Temp::new(true);
            let public_path = format!("vendor/{notice}.txt");
            let proof_path = if separate_copy {
                temp.write(
                    "docs/public-notice-proof-copy.txt",
                    &fs::read(temp.0.join(&public_path)).unwrap(),
                );
                "docs/public-notice-proof-copy.txt"
            } else {
                &public_path
            };
            let mut evidence = temp.read(EVIDENCE);
            evidence["sources"][0]["permission_evidence"] = json!([temp.reference(proof_path)]);
            let mut registry = temp.read("docs/admission-registry.json");
            let hash = temp.reference(proof_path)["sha256"].clone();
            registry["sources"][0]["rights"]["evidence"][0]["sha256"] = hash.clone();
            registry["sources"][0]["retrieval"]["access_evidence"]["sha256"] = hash;
            temp.json("docs/admission-registry.json", &registry);
            evidence["admission_registry"] = temp.reference("docs/admission-registry.json");
            temp.json(EVIDENCE, &evidence);
            temp.commit();

            let manifest = materials::plan(&temp.options()).unwrap();
            assert_eq!(manifest.files.len(), 6);
            assert!(
                manifest
                    .files
                    .iter()
                    .any(|f| f.input.path.as_str() == public_path)
            );
            let bytes = manifest.bytes().unwrap();
            let public_metadata = String::from_utf8(bytes).unwrap();
            assert!(!public_metadata.contains("proof-copy"));
            assert!(!public_metadata.contains("private-proof"));
            assert!(!public_metadata.contains("permission_evidence"));
        }
    }
}

#[test]
fn empty_preview_selection_does_not_create_output() {
    let temp = Temp::new(false);
    let output = temp.0.join("output");
    let manifest = materials::stage(&temp.options(), &output).unwrap();
    assert!(manifest.files.is_empty());
    assert!(!output.exists());
}

#[test]
fn invalid_rights_source_revision_or_hash_refuses_before_mutation() {
    for kind in 0..6 {
        let temp = Temp::new(true);
        let mut evidence = temp.read(EVIDENCE);
        match kind {
            0 => evidence["sources"][0]["redistribution"] = json!("unknown"),
            1 => evidence["sources"][0]["revision"] = json!("2".repeat(40)),
            2 => evidence["sources"][0]["target_validation"] = json!("unverified"),
            3 => evidence["sources"][0]["files"][0]["sha256"] = json!("0".repeat(64)),
            4 => evidence["sources"][0]["license_files"] = json!([]),
            _ => evidence["packs"][0]["dataset_revisions"] = json!(["invented-revision"]),
        }
        temp.json(EVIDENCE, &evidence);
        let output = temp.0.join("untouched");
        assert!(
            materials::stage(&temp.options(), &output).is_err(),
            "case {kind}"
        );
        assert!(!output.exists());
        assert!(!fs::read_dir(&temp.0).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".release-materials")
        }));
    }
}

#[test]
fn proof_overlap_untracked_ignored_or_changed_material_is_refused() {
    for kind in 0..6 {
        let temp = Temp::new(true);
        let mut evidence = temp.read(EVIDENCE);
        match kind {
            0 => {
                evidence["sources"][0]["files"] = json!([temp.reference("docs/private-proof.txt")]);
                temp.json(EVIDENCE, &evidence);
            }
            1 | 2 => {
                let name = if kind == 1 {
                    "vendor/untracked.lua"
                } else {
                    "vendor/ignored.log"
                };
                temp.write(name, b"extra");
                evidence["sources"][0]["files"] = json!([temp.reference(name)]);
                temp.json(EVIDENCE, &evidence);
            }
            3 => {
                temp.write("vendor/license.txt", b"changed original terms");
                temp.commit();
            }
            4 => {
                fs::remove_file(temp.0.join("vendor/raw.lua")).unwrap();
            }
            _ => {
                temp.write(
                    "vendor/alias.lua",
                    &fs::read(temp.0.join("docs/private-proof.txt")).unwrap(),
                );
                evidence["sources"][0]["files"] = json!([temp.reference("vendor/alias.lua")]);
                temp.json(EVIDENCE, &evidence);
                temp.commit();
            }
        }
        assert!(materials::plan(&temp.options()).is_err(), "case {kind}");
    }
}

#[cfg(unix)]
#[test]
fn source_or_output_symlinks_are_never_followed() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new(true);
    let output = temp.0.join("linked");
    symlink(std::env::temp_dir(), &output).unwrap();
    assert!(materials::stage(&temp.options(), &output.join("no-output")).is_err());
    fs::remove_file(output).unwrap();
    fs::remove_file(temp.0.join("vendor/raw.lua")).unwrap();
    symlink(
        temp.0.join("vendor/license.txt"),
        temp.0.join("vendor/raw.lua"),
    )
    .unwrap();
    temp.commit();
    assert!(materials::stage(&temp.options(), &temp.0.join("out")).is_err());
}

#[test]
fn future_manifest_oversized_source_and_duplicate_pack_are_refused() {
    for kind in 0..3 {
        let temp = Temp::new(true);
        let mut evidence = temp.read(EVIDENCE);
        match kind {
            0 => evidence["schema_version"] = json!(4),
            1 => {
                evidence["sources"][0]["files"][0]["bytes"] =
                    json!(azeroth_cli::release::MAX_FILE_BYTES + 1)
            }
            _ => {
                let extra = evidence["packs"][0].clone();
                evidence["packs"].as_array_mut().unwrap().push(extra);
            }
        }
        temp.json(EVIDENCE, &evidence);
        assert!(materials::plan(&temp.options()).is_err(), "case {kind}");
    }
}

#[test]
fn material_cli_exact_options_and_no_overwrite_are_enforced() {
    let temp = Temp::new(true);
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(["release-materials", "--root"])
        .arg(&temp.0)
        .args(["--version", "0.8.1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        materials::plan(&temp.options()).unwrap().bytes().unwrap()
    );
    for args in [
        vec!["release-materials"],
        vec!["release-materials", "--root", "--version"],
        vec!["release-materials", "--output", "x", "--output", "y"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

#[test]
#[ignore = "requires NAVZEROTH_TEST_PWSH; the Windows package job runs this explicitly"]
fn powershell_packages_actual_verified_cli_selection_and_preserves_prior_output_on_refusal() {
    let shell = std::env::var_os("NAVZEROTH_TEST_PWSH").expect("Set NAVZEROTH_TEST_PWSH");
    let temp = Temp::new(true);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in [
        "Build-Windows.ps1",
        "Release-Materials.ps1",
        "Verify-ReleaseArtifacts.ps1",
    ] {
        temp.write(
            &format!("scripts/windows/{file}"),
            &fs::read(root.join("scripts/windows").join(file)).unwrap(),
        );
    }
    temp.write(
        "docs/releases/0.8.1.md",
        b"Synthetic transport test; not a published release.\n",
    );
    temp.commit();
    let bin = temp.0.join("dist/bin");
    fs::create_dir_all(&bin).unwrap();
    if cfg!(windows) {
        fs::write(bin.join("cargo.cmd"),b"@echo off\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File \"%~dp0forward.ps1\" %*\r\nexit /b %ERRORLEVEL%\r\n").unwrap();
        fs::write(
            bin.join("forward.ps1"),
            b"& $env:ANAV_REAL_CLI @($args[5..($args.Length-1)])\nexit $LASTEXITCODE\n",
        )
        .unwrap();
    } else {
        fs::write(
            bin.join("cargo"),
            b"#!/bin/sh\nshift 5\nexec \"$ANAV_REAL_CLI\" \"$@\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin.join("cargo"), fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    // Forward the normal cargo invocation to this already-built real CLI so a
    // nested Cargo build cannot wait on the parent test process's build lock.
    let mut path = vec![bin];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(path).unwrap();
    let output = temp.0.join("dist/output");
    let run = |verify: bool| {
        let mut command = Command::new(&shell);
        command
            .env("PATH", &path)
            .env("ANAV_REAL_CLI", env!("CARGO_BIN_EXE_azeroth-cli"))
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(temp.0.join(if verify {
                "scripts/windows/Verify-ReleaseArtifacts.ps1"
            } else {
                "scripts/windows/Build-Windows.ps1"
            }));
        if verify {
            command.args(["-Version", "0.8.1", "-Directory"]);
        } else {
            command.args(["-ExpectedVersion", "0.8.1", "-OutputDirectory"]);
        }
        command.arg(&output).arg("-AddonOnly").output().unwrap()
    };
    let result = run(false);
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 7);
    let before = fs::read(output.join("SHA256SUMS.txt")).unwrap();
    let result = run(true);
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    temp.write("vendor/license.txt", b"Changed terms after validation\n");
    temp.commit();
    assert!(!run(false).status.success());
    assert_eq!(fs::read(output.join("SHA256SUMS.txt")).unwrap(), before);
}

#[test]
#[ignore = "requires Git LFS; the Windows package job runs this explicitly"]
fn git_lfs_material_inputs_require_hydrated_exact_bytes() {
    let temp = Temp::new(true);
    temp.git(&["lfs", "install", "--local", "--skip-smudge"]);
    temp.write(
        ".gitattributes",
        b"vendor/lfs-source.lua filter=lfs diff=lfs merge=lfs -text\n",
    );
    temp.write("vendor/lfs-source.lua", RAW);
    let mut evidence = temp.read(EVIDENCE);
    evidence["sources"][0]["files"] = json!([temp.reference("vendor/lfs-source.lua")]);
    let mut registry = temp.read("docs/admission-registry.json");
    registry["sources"][0]["files"][0]["path"] = json!("vendor/lfs-source.lua");
    temp.json("docs/admission-registry.json", &registry);
    evidence["admission_registry"] = temp.reference("docs/admission-registry.json");
    temp.json(EVIDENCE, &evidence);
    temp.commit();
    let pointer = temp.git(&["show", "HEAD:vendor/lfs-source.lua"]);
    assert!(pointer.starts_with("version https://git-lfs.github.com/spec/v1\n"));
    assert!(pointer.contains(&format!("oid sha256:{}", digest(RAW))));
    assert!(materials::plan(&temp.options()).is_ok());

    fs::remove_file(temp.0.join("vendor/lfs-source.lua")).unwrap();
    temp.git(&["checkout", "--", "vendor/lfs-source.lua"]);
    assert_eq!(
        fs::read(temp.0.join("vendor/lfs-source.lua")).unwrap(),
        format!("{pointer}\n").as_bytes()
    );
    let status = temp.git(&["status", "--porcelain"]);
    assert!(status.is_empty(), "{status}");
    let output = temp.0.join("materials");
    assert!(materials::stage(&temp.options(), &output).is_err());
    assert!(!output.exists());

    temp.git(&["lfs", "checkout", "vendor/lfs-source.lua"]);
    let status = temp.git(&["status", "--porcelain"]);
    assert!(status.is_empty(), "{status}");
    let manifest = materials::stage(&temp.options(), &output).unwrap();
    assert_eq!(manifest.files.len(), 6);
    assert_eq!(
        fs::read(output.join("source-materials/vendor/lfs-source.lua")).unwrap(),
        RAW
    );
}

#[test]
fn v2_catalog_materials_are_verified_delivered_and_do_not_waive_missing_domains() {
    let temp = Temp::new(true);
    let old = temp.read("content/material-test/manifest.json");
    let dataset_path = old["entries"][0]["dataset"]["path"].as_str().unwrap();
    let reference = temp.read(&format!("content/material-test/{dataset_path}"));
    let content = json!({"format":"navzeroth-reference-content","schema_version":2,
        "reference":reference,"models":[],"equipment":[],"quest_facts":[],
        "acquisition_facts":[],"dungeons":[]});
    temp.json("pack-v2-input/catalog.json", &content);
    temp.write(
        "pack-v2-input/sources.json",
        &fs::read(temp.0.join("content/material-test/sources.json")).unwrap(),
    );
    let built = pack::v2::build(
        &temp.0.join("pack-v2-input"),
        &temp.0.join("content/v2-test"),
    )
    .unwrap();
    assert_eq!(built.coverage_gaps, 0);
    let mut evidence = temp.read(EVIDENCE);
    evidence["packs"][0]["manifest"] = temp.reference("content/v2-test/manifest.json");
    temp.json(EVIDENCE, &evidence);
    temp.commit();
    let selected = materials::plan(&temp.options()).unwrap();
    assert!(
        selected
            .files
            .iter()
            .any(|f| f.input.path.as_str() == "content/v2-test/catalog.json")
    );
    assert!(
        selected
            .files
            .iter()
            .any(|f| f.input.path.as_str() == "content/v2-test/sources.json")
    );
    assert!(
        !selected
            .files
            .iter()
            .any(|f| f.input.path.as_str().contains("private-proof"))
    );
    let readiness = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(!readiness.readiness.stable_ready);
    assert!(
        readiness
            .readiness
            .blockers
            .iter()
            .any(|b| b.contains("mandatory domain"))
    );
    temp.write("content/v2-test/catalog.json", b"{}");
    assert!(materials::plan(&temp.options()).is_err());
}

#[test]
fn owner_accepted_rights_select_materials_only_with_the_adr_citation() {
    for cited in [true, false] {
        let temp = Temp::new(true);
        let mut evidence = temp.read(EVIDENCE);
        let mut registry = temp.read("docs/admission-registry.json");
        let rights = &mut registry["sources"][0]["rights"];
        rights["license"] = json!("owner_accepted");
        rights["evidence"][0]["url"] = json!(
            "https://github.com/Laymer/Navzeroth/blob/main/docs/decisions/0006-v1-program.md"
        );
        rights["evidence"][0]["reviewed_scope"] = json!(if cited {
            "Owner accepts the synthetic terms; ADR 0006 §14"
        } else {
            "Owner accepts the synthetic terms"
        });
        temp.json("docs/admission-registry.json", &registry);
        evidence["admission_registry"] = temp.reference("docs/admission-registry.json");
        temp.json(EVIDENCE, &evidence);
        temp.commit();
        let plan = materials::plan(&temp.options());
        assert_eq!(plan.is_ok(), cited);
        if cited {
            assert_eq!(plan.unwrap().sources.len(), 1);
        }
    }
}

#[test]
fn offline_admission_blocks_unreviewed_noncommercial_mismatched_and_missing_claims() {
    for case in [
        "missing",
        "hash",
        "noncommercial",
        "unknown_license",
        "unreviewed",
        "source",
        "revision",
        "unselected_fields",
        "transformation",
        "grant",
        "origin",
        "publisher",
        "license",
        "attribution",
        "fields",
        "file_path",
        "file_hash",
    ] {
        let temp = Temp::new(true);
        let mut evidence = temp.read(EVIDENCE);
        let mut registry = temp.read("docs/admission-registry.json");
        let source = &mut registry["sources"][0];
        match case {
            "missing" => {
                evidence
                    .as_object_mut()
                    .unwrap()
                    .remove("admission_registry");
            }
            "hash" => evidence["admission_registry"]["sha256"] = json!("0".repeat(64)),
            "noncommercial" => source["rights"]["license"] = json!("noncommercial"),
            "unknown_license" => source["rights"]["license"] = json!("unknown"),
            "unreviewed" => source["rights"]["status"] = json!("unreviewed"),
            "source" => source["id"] = json!("unrelated"),
            "revision" => source["revision"] = json!("2".repeat(40)),
            "unselected_fields" => {
                let mut other = source["files"][0].clone();
                other["path"] = json!("vendor/not-selected.lua");
                source["files"][0]["fields"] = json!(["unrelated"]);
                source["files"].as_array_mut().unwrap().push(other);
                source["retrieval"]["max_requests"] = json!(2);
                source["retrieval"]["max_total_bytes"] = json!(RAW.len() * 2);
            }
            "grant" => source["rights"]["evidence"][0]["sha256"] = json!("0".repeat(64)),
            "origin" => source["origin"]["edition"] = json!("unrelated"),
            "publisher" => source["publisher"] = json!("unrelated"),
            "license" => source["rights"]["license_identifier"] = json!("unrelated"),
            "attribution" => source["rights"]["attribution"] = json!("unrelated"),
            "fields" => source["files"][0]["fields"] = json!(["unrelated"]),
            "transformation" => source["files"][0]["transformation"] = json!("unrelated"),
            "file_path" => source["files"][0]["path"] = json!("vendor/unrelated.lua"),
            "file_hash" => source["files"][0]["sha256"] = json!("0".repeat(64)),
            _ => unreachable!(),
        }
        if !matches!(case, "missing" | "hash") {
            temp.json("docs/admission-registry.json", &registry);
            evidence["admission_registry"] = temp.reference("docs/admission-registry.json");
        }
        temp.json(EVIDENCE, &evidence);
        temp.commit();
        assert!(materials::plan(&temp.options()).is_err(), "{case}");
        let report = azeroth_cli::release::report(&temp.options()).unwrap();
        assert!(!report.readiness.candidate_ready, "{case}");
        assert!(!report.readiness.stable_ready, "{case}");
        assert!(
            report
                .readiness
                .blockers
                .iter()
                .any(|b| b.to_ascii_lowercase().contains("admission")
                    || b.contains("admitted")
                    || b.contains("transformations")
                    || b.contains("original")
                    || b.contains("Original")
                    || b.contains("bytes changed")
                    || b.contains("hash")),
            "{case}: {:?}",
            report.readiness.blockers
        );
    }
}

#[test]
fn admission_registry_remains_private_even_if_relabelled_as_a_public_notice() {
    let temp = Temp::new(true);
    temp.write(
        "vendor/registry-copy.json",
        &fs::read(temp.0.join("docs/admission-registry.json")).unwrap(),
    );
    let mut evidence = temp.read(EVIDENCE);
    evidence["sources"][0]["license_files"] = json!([temp.reference("vendor/registry-copy.json")]);
    temp.json(EVIDENCE, &evidence);
    temp.commit();
    assert!(
        materials::plan(&temp.options())
            .unwrap_err()
            .contains("Private admission registry")
    );
}

/// Turns a content-bearing schema 1 fixture into schema 2 with a pending target.
fn pending_v2(temp: &Temp) {
    let mut inventory = temp.read(INVENTORY);
    inventory["schema_version"] = json!(2);
    inventory["target"] = Value::Null;
    inventory["slice"] = json!({"id":"synthetic-slice","races":["human"],"zones":["elwynn_forest"],
        "dungeons":[],"level_min":1,"level_max":20,"classes":null,"party_sizes":[1,2]});
    for domain in inventory["domains"].as_array_mut().unwrap() {
        let name = domain["domain"].as_str().unwrap().to_owned();
        domain["out_of_scope"] = json!([{"id":format!("{name}-outside"),
            "scope":"Synthetic outside","reason":"Synthetic boundary"}]);
    }
    temp.json(INVENTORY, &inventory);
    let mut evidence = temp.read(EVIDENCE);
    evidence["schema_version"] = json!(2);
    evidence["target"] = Value::Null;
    temp.json(EVIDENCE, &evidence);
    temp.bind_inventory();
    temp.commit();
}

#[test]
fn content_cannot_be_selected_or_bound_while_the_target_is_pending() {
    let temp = Temp::new(true);
    pending_v2(&temp);
    let error = materials::plan(&temp.options()).unwrap_err();
    assert!(error.contains("target is pending"), "{error}");
    let output = temp.0.join("untouched");
    assert!(materials::stage(&temp.options(), &output).is_err());
    assert!(!output.exists());
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(
        report
            .readiness
            .blockers
            .iter()
            .any(|b| b.contains("Release target is pending; content cannot be bound")),
        "{:?}",
        report.readiness.blockers
    );
    assert_eq!(report.verified_content_packs, 0);
}

#[test]
fn a_self_chosen_schema_2_target_cannot_bind_content() {
    let temp = Temp::new(true);
    pending_v2(&temp);
    let frozen =
        json!({"edition":"wow_forever","version":"1.60.1","build":"70374","interface":16001});
    let mut inventory = temp.read(INVENTORY);
    inventory["target"] = frozen.clone();
    temp.json(INVENTORY, &inventory);
    let mut evidence = temp.read(EVIDENCE);
    evidence["target"] = frozen;
    temp.json(EVIDENCE, &evidence);
    temp.bind_inventory();
    temp.commit();
    assert!(materials::plan(&temp.options()).is_err());
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    let blockers = &report.readiness.blockers;
    assert!(
        blockers
            .iter()
            .any(|b| b.contains("frozen target does not match"))
    );
    assert!(
        blockers
            .iter()
            .any(|b| b.contains("differs from the frozen target"))
    );
    assert_eq!(report.verified_content_packs, 0);
}

#[test]
fn mixed_inventory_and_evidence_schema_versions_are_refused() {
    let temp = Temp::new(true);
    let mut evidence = temp.read(EVIDENCE);
    evidence["schema_version"] = json!(2);
    temp.json(EVIDENCE, &evidence);
    temp.commit();
    assert!(materials::plan(&temp.options()).is_err());
}

/// Schema 3 (ADR 0007): a v2 pack whose only source is of 1.12 origin. The pack
/// source lists its origin and the target edition; release evidence records the
/// true origin, and target validation is left to the accepted-origin policy.
fn world_v3(temp: &Temp, origin: &str, evidence_origin: &str, policy: &Value) {
    let context = json!({"build":{"edition":"wow_forever","build":"1.60.1.70374","interface":"16001"},"namespace":"reference"});
    let old = temp.read("content/material-test/manifest.json");
    let dataset_path = old["entries"][0]["dataset"]["path"].as_str().unwrap();
    let mut reference = temp.read(&format!("content/material-test/{dataset_path}"));
    reference["payload"]["revision"] = json!("synthetic-world-r1");
    reference["payload"]["evidence"][0]["confidence"] = json!("cross_edition_unverified");
    let fixture: Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/contracts/v1/reference-dataset.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut rule = fixture["payload"]["rules"][0].clone();
    rule["context"] = context.clone();
    let mut map = fixture["payload"]["map_spaces"][0].clone();
    map["key"]["context"] = context;
    map["evidence"] = json!(["demo-evidence-1"]);
    reference["payload"]["rules"] = json!([rule]);
    reference["payload"]["map_spaces"] = json!([map]);
    let content = json!({"format":"navzeroth-reference-content","schema_version":2,
        "reference":reference,"models":[],"equipment":[],"quest_facts":[],
        "acquisition_facts":[],"dungeons":[]});
    temp.json("world-input/catalog.json", &content);
    let mut sources = temp.read("content/material-test/sources.json");
    sources["sources"][0]["editions"] = json!({"status":"known","value":[origin,"wow_forever"]});
    sources["sources"][0]["access"] = json!({"status":"known","value":"file_download"});
    temp.json("world-input/sources.json", &sources);
    let built = pack::v2::build(
        &temp.0.join("world-input"),
        &temp.0.join("content/world-test"),
    )
    .unwrap();
    // The pack verifier keeps the strict, policy-free count.
    assert!(built.coverage_gaps > 0);
    assert!(built.distribution_eligible);
    temp.json(POLICY, policy);
    temp.write(
        TARGET,
        &serde_json::to_vec(&json!({"format":"navzeroth-world-data-target","schema_version":1,
            "decision":"ADR 0007 §6","edition":"wow_forever","build":"1.60.1.70374","interface":"16001",
            "locale":"enUS","detected_from":"synthetic test"}))
        .unwrap(),
    );
    let mut registry = temp.read("docs/admission-registry.json");
    registry["sources"][0]["origin"] = json!({"edition":evidence_origin,"build":null});
    temp.json("docs/admission-registry.json", &registry);
    let target =
        json!({"edition":"wow_forever","version":"1.60.1","build":"70374","interface":16001});
    let revision = "synthetic-world-r1";
    let rule_id = json!([revision, "rules", "demo-rule-1"]).to_string();
    let map_id = json!([revision, "map_spaces", "demo-map-1", "0", null]).to_string();
    let mut inventory = temp.read(INVENTORY);
    inventory["schema_version"] = json!(3);
    inventory["target"] = target.clone();
    inventory["slice"] = json!({"id":"synthetic-slice","races":["human"],"zones":["elwynn_forest"],
        "dungeons":[],"level_min":1,"level_max":20,"classes":["warrior"],"party_sizes":[1]});
    for domain in inventory["domains"].as_array_mut().unwrap() {
        let name = domain["domain"].as_str().unwrap().to_owned();
        domain["out_of_scope"] = json!([{"id":format!("{name}-outside"),
            "scope":"Synthetic outside","reason":"Synthetic boundary"}]);
        let ids = match name.as_str() {
            "reminders" => Some(&rule_id),
            "geography" => Some(&map_id),
            _ => None,
        };
        if let Some(id) = ids {
            domain["units"][0]["required_record_ids"] = json!([id]);
        }
    }
    temp.json(INVENTORY, &inventory);
    let mut evidence = temp.read(EVIDENCE);
    evidence["schema_version"] = json!(3);
    evidence["target"] = target;
    evidence["checks"] = Value::Array(
        evidence["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["id"].as_str().unwrap().starts_with("WIN-"))
            .cloned()
            .collect(),
    );
    evidence["origin_policy"] = temp.reference(POLICY);
    evidence["admission_registry"] = temp.reference("docs/admission-registry.json");
    let source = &mut evidence["sources"][0];
    source["origin_edition"] = json!(evidence_origin);
    source["target_validation"] = json!("unverified");
    source["target_evidence"] = json!([]);
    let units: Vec<_> = inventory["domains"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| ["reminders", "geography"].contains(&d["domain"].as_str().unwrap()))
        .map(|d| {
            (
                d["units"][0]["id"].clone(),
                d["units"][0]["required_record_ids"].clone(),
            )
        })
        .collect();
    evidence["coverage"] = Value::Array(
        units
            .into_iter()
            .map(|(unit, ids)| {
                json!({"unit_id":unit,"status":"pass","included_record_ids":ids,
                    "source_ids":["demo-source-1"],"unresolved_conflicts":0,"unsupported_corrections":0,
                    "validation_evidence":[temp.reference("docs/private-proof.txt")]})
            })
            .collect(),
    );
    evidence["packs"] = json!([{"manifest":temp.reference("content/world-test/manifest.json"),
        "dataset_revisions":[revision]}]);
    temp.json(EVIDENCE, &evidence);
    temp.bind_inventory();
    temp.commit();
}
fn repository_policy() -> Value {
    serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(POLICY),
        )
        .unwrap(),
    )
    .unwrap()
}
fn has(report: &azeroth_cli::release::Report, text: &str) -> bool {
    report.readiness.blockers.iter().any(|b| b.contains(text))
}

#[test]
fn accepted_origins_and_unknowns_bind_a_world_pack_under_schema_3() {
    let temp = Temp::new(true);
    world_v3(&temp, "vanilla_1_12", "vanilla_1_12", &repository_policy());
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert_eq!(
        report.verified_content_packs, 1,
        "{:?}",
        report.readiness.blockers
    );
    for absent in [
        "coverage gaps",
        "target validation is incomplete",
        "accepted-origin policy",
        "Mandatory acceptance rows",
        "Release target",
        "frozen target",
    ] {
        assert!(
            !has(&report, absent),
            "{absent}: {:?}",
            report.readiness.blockers
        );
    }
    let world = report.world_data.as_ref().unwrap();
    assert_eq!(world.origin_accepted_sources, ["demo-source-1"]);
    assert!(world.accepted_origins >= 2, "{world:?}");
    assert!(world.accepted_unknowns >= 1, "{world:?}");
    assert_eq!(world.remaining_gaps, 0);
    assert_eq!(world.accept_unknowns, Some(true));
    assert_eq!(
        world.origin_policy_sha256.as_deref(),
        temp.reference(POLICY)["sha256"].as_str()
    );
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["report_version"], 3);
    assert!(json["world_data"]["packs"][0]["listed_accepted_unknowns"].is_array());
    // Materials accept the origin-validated source without target evidence.
    let plan = materials::plan(&temp.options()).unwrap();
    assert_eq!(plan.sources.len(), 1);
    assert_eq!(plan.sources[0].origin_edition, "vanilla_1_12");
}

#[test]
fn unknowns_block_when_the_policy_does_not_accept_them() {
    let temp = Temp::new(true);
    let mut policy = repository_policy();
    policy["accept_unknowns"] = json!(false);
    world_v3(&temp, "vanilla_1_12", "vanilla_1_12", &policy);
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(has(
        &report,
        "coverage gaps the accepted-origin policy does not accept"
    ));
    let world = report.world_data.unwrap();
    assert!(world.remaining_gaps > 0);
    assert_eq!(world.accepted_unknowns, 0);
}

#[test]
fn unaccepted_or_mismatched_origins_and_unbound_policies_block() {
    let temp = Temp::new(true);
    world_v3(
        &temp,
        "wow_classic_tbc",
        "wow_classic_tbc",
        &repository_policy(),
    );
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(has(&report, "target validation is incomplete"));
    assert!(has(&report, "coverage gaps"));
    assert!(materials::plan(&temp.options()).is_err());

    let temp = Temp::new(true);
    world_v3(
        &temp,
        "wow_classic_era",
        "vanilla_1_12",
        &repository_policy(),
    );
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(has(
        &report,
        "V2 source editions differ from the release source origin"
    ));

    let temp = Temp::new(true);
    world_v3(&temp, "vanilla_1_12", "vanilla_1_12", &repository_policy());
    let mut policy = repository_policy();
    policy["precedence"] = json!(["authored"]);
    temp.json(POLICY, &policy);
    temp.commit();
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(
        has(&report, "Evidence file hash or size does not match") || has(&report, "does not bind")
    );
    temp.write(POLICY, b"{\"format\":\"other\"}");
    temp.commit();
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(has(&report, "Accepted-origin policy is missing or invalid"));
    fs::remove_file(temp.0.join(TARGET)).unwrap();
    temp.commit();
    let report = azeroth_cli::release::report(&temp.options()).unwrap();
    assert!(has(&report, "World-data target file is missing"));
    assert!(materials::plan(&temp.options()).is_err());
}

fn corresponding_source_fixture() -> Temp {
    let temp = Temp::new(true);
    world_v3(&temp, "vanilla_1_12", "vanilla_1_12", &repository_policy());
    let mut inventory = temp.read(INVENTORY);
    inventory["schema_version"] = json!(4);
    temp.json(INVENTORY, &inventory);
    temp.write(
        "vendor/mixed-generator-source.sql",
        b"-- synthetic permitted mixed source\nSELECT 1;\n",
    );
    let mut evidence = temp.read(EVIDENCE);
    evidence["schema_version"] = json!(4);
    evidence["corresponding_source_files"] =
        json!([temp.reference("vendor/mixed-generator-source.sql")]);
    temp.json(EVIDENCE, &evidence);
    temp.bind_inventory();
    temp.commit();
    temp
}

#[test]
fn schema4_corresponding_sources_export_exact_bytes_as_their_real_category() {
    let temp = corresponding_source_fixture();
    let plan = materials::plan(&temp.options()).unwrap();
    assert_eq!(plan.schema_version, 2);
    assert_eq!(plan.corresponding_source_files.len(), 1);
    assert_eq!(plan.bundled_packs.len(), 1);
    assert!(!plan.bundled_packs[0].key.scope.fixture);
    assert_eq!(plan.bundled_packs[0].manifest, plan.packs[0].manifest);
    assert_eq!(
        plan.bundled_packs[0].key.revision.as_str(),
        plan.packs[0].dataset_revisions[0]
    );
    assert_eq!(plan.catalog_files.len(), 1);
    assert!(
        plan.catalog_files[0]
            .path
            .as_str()
            .ends_with("catalog.json")
    );
    let file = plan
        .files
        .iter()
        .find(|f| f.kind == materials::Kind::CorrespondingSource)
        .unwrap();
    assert_eq!(
        file.input.path.as_str(),
        "vendor/mixed-generator-source.sql"
    );
    assert!(
        plan.sources
            .iter()
            .all(|s| !s.license_files.contains(&file.input)
                && !s.attribution_files.contains(&file.input))
    );
    let staged = temp.0.join("dist/corresponding");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    materials::stage(&temp.options(), &staged).unwrap();
    assert_eq!(
        fs::read(staged.join(file.archive_path.as_str())).unwrap(),
        fs::read(temp.0.join(file.input.path.as_str())).unwrap()
    );
}

#[test]
fn corresponding_sources_deduplicate_archive_bytes_while_retaining_both_roles() {
    let temp = corresponding_source_fixture();
    let mut evidence = temp.read(EVIDENCE);
    let notice = evidence["sources"][0]["license_files"][0].clone();
    evidence["corresponding_source_files"] = json!([notice.clone()]);
    temp.json(EVIDENCE, &evidence);
    let plan = materials::plan(&temp.options()).unwrap();
    let files: Vec<_> = plan
        .files
        .iter()
        .filter(|f| f.input.path.as_str() == notice["path"].as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].kind, materials::Kind::SourceMaterial);
    assert_eq!(
        files[0].additional_kinds,
        [materials::Kind::CorrespondingSource]
    );
}

#[test]
fn corresponding_sources_refuse_bad_hash_missing_duplicate_and_unlisted_resources() {
    for mutation in ["hash", "missing", "duplicate", "unlisted", "empty", "zero"] {
        let temp = corresponding_source_fixture();
        let mut evidence = temp.read(EVIDENCE);
        match mutation {
            "hash" => evidence["corresponding_source_files"][0]["sha256"] = json!("0".repeat(64)),
            "missing" => {
                evidence["corresponding_source_files"][0]["path"] = json!("vendor/absent.sql")
            }
            "duplicate" => {
                let f = evidence["corresponding_source_files"][0].clone();
                evidence["corresponding_source_files"]
                    .as_array_mut()
                    .unwrap()
                    .push(f);
            }
            "empty" => evidence["corresponding_source_files"] = json!([]),
            "zero" => evidence["corresponding_source_files"][0]["bytes"] = json!(0),
            "unlisted" => {
                temp.write("release/unlisted.sql", b"unlisted");
                evidence["corresponding_source_files"] =
                    json!([temp.reference("release/unlisted.sql")]);
            }
            _ => unreachable!(),
        }
        temp.json(EVIDENCE, &evidence);
        assert!(materials::plan(&temp.options()).is_err(), "{mutation}");
    }
}

#[test]
fn historical_evidence_refuses_corresponding_source_field_even_null_or_empty() {
    let temp = corresponding_source_fixture();
    for version in [1, 2, 3] {
        for field in [
            json!(null),
            json!([]),
            temp.read(EVIDENCE)["corresponding_source_files"].clone(),
        ] {
            let mut evidence = temp.read(EVIDENCE);
            evidence["schema_version"] = json!(version);
            evidence["corresponding_source_files"] = field;
            assert!(
                azeroth_core::release::decode_evidence(&serde_json::to_vec(&evidence).unwrap())
                    .is_err()
            );
        }
    }
}

#[test]
fn raw_normalization_conflicts_remain_versioned_policy_bound_materials() {
    for mutation in [
        "none",
        "format",
        "revision",
        "precedence",
        "tamper",
        "historical",
    ] {
        let temp = corresponding_source_fixture();
        let mut evidence = temp.read(EVIDENCE);
        let revision = evidence["packs"][0]["dataset_revisions"][0].clone();
        let mut report = json!({"format":"navzeroth-world-content-conflicts","schema_version":1,"revision":revision,"precedence":temp.read(POLICY)["precedence"],"conflicts":[{"kind":"synthetic normalization only","fact":"fixture","not_a_SourceConflict":true}]});
        if mutation == "format" {
            report["format"] = json!("typed-source-conflicts");
        }
        if mutation == "revision" {
            report["revision"] = json!("wrong");
        }
        if mutation == "precedence" {
            report["precedence"] = json!([]);
        }
        temp.json("content/raw-normalization.json", &report);
        evidence["normalization_conflict_files"] =
            json!([temp.reference("content/raw-normalization.json")]);
        if mutation == "tamper" {
            evidence["normalization_conflict_files"][0]["sha256"] = json!("0".repeat(64));
        }
        if mutation == "historical" {
            evidence["schema_version"] = json!(3);
            evidence
                .as_object_mut()
                .unwrap()
                .remove("corresponding_source_files");
        }
        temp.json(EVIDENCE, &evidence);
        temp.commit();
        let result = materials::plan(&temp.options());
        if mutation == "none" {
            let plan = result.unwrap();
            assert_eq!(plan.normalization_conflict_files.len(), 1);
            assert_eq!(
                plan.bundled_packs[0].normalization_conflicts.as_ref(),
                Some(&plan.normalization_conflict_files[0])
            );
            assert!(
                plan.files
                    .iter()
                    .any(|f| f.kind == materials::Kind::NormalizationConflicts)
            );
        } else {
            assert!(result.is_err(), "{mutation}");
        }
    }
}
