//! Task claim: independent v2 pack/release integration regressions.
//! Owner / sole writer: Codex `/root/source_admission`.
//! Baseline: 0ff8a29; branch test/content-pack-v2; 2026-10-15.
//! Synthetic data verifies engineering behavior, never actual source rights or game facts.
use azeroth_cli::pack::{v2::build, verify};
use azeroth_core::{content_v2, release::Target, release_party::WORKFLOWS};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "anav-v2-pack-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn input(&self) -> PathBuf {
        let input = self.0.join("input");
        fs::create_dir(&input).unwrap();
        fs::write(
            input.join("catalog.json"),
            include_bytes!("../../../fixtures/contracts/v2/reference-content.json"),
        )
        .unwrap();
        fs::write(
            input.join("sources.json"),
            include_bytes!("../../../fixtures/content-pack/sources.json"),
        )
        .unwrap();
        input
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn canonical(v: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(v).unwrap();
    bytes.push(b'\n');
    bytes
}
fn write(path: &Path, v: &Value) {
    fs::write(path, canonical(v)).unwrap();
}
fn change(path: &Path, f: impl FnOnce(&mut Value)) {
    let mut v = read(path);
    f(&mut v);
    write(path, &v);
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, dir: &Path, result: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                visit(root, &p, result)
            } else {
                result.push((p.strip_prefix(root).unwrap().into(), fs::read(p).unwrap()));
            }
        }
    }
    let mut result = vec![];
    visit(root, root, &mut result);
    result.sort();
    result
}
fn target_context(value: &mut Value) {
    match value {
        Value::Object(o) => {
            if o.get("build").is_some_and(Value::is_string) {
                o.insert("build".into(), json!("1.60.1.70374"));
            }
            if o.contains_key("interface") {
                o.insert("interface".into(), json!("16001"));
            }
            for v in o.values_mut() {
                target_context(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                target_context(v)
            }
        }
        _ => {}
    }
}
fn add_party(input: &Path, retarget: bool) {
    let path = input.join("catalog.json");
    if retarget {
        change(&path, target_context);
    }
    let content = content_v2::decode(&fs::read(&path).unwrap()).unwrap();
    // Hash the exact builder's canonical typed catalog, not authored whitespace.
    let bytes = canonical(&serde_json::to_value(content).unwrap());
    let catalog_hash = digest(&bytes);
    fs::create_dir(input.join("evidence")).unwrap();
    let procedure = b"Synthetic procedure, not observed gameplay";
    let proof = b"Synthetic applicability evidence, not source permission";
    fs::write(input.join("evidence/procedure.txt"), procedure).unwrap();
    fs::write(input.join("evidence/proof.txt"), proof).unwrap();
    let rows:Vec<_>=WORKFLOWS.into_iter().enumerate().map(|(n,workflow)|json!({"id":format!("synthetic-workflow-{n}"),"size":2,"workflow":workflow,
        "dependencies":[{"domain":"quests","record_id":"[\"demo-r1\",\"quests\",\"demo-quest-1\"]"}],
        "procedure":{"path":"evidence/procedure.txt","bytes":procedure.len(),"sha256":digest(procedure)},
        "supporting_evidence":[{"path":"evidence/proof.txt","bytes":proof.len(),"sha256":digest(proof)}]})).collect();
    write(
        &input.join("party-support.json"),
        &json!({"schema_version":1,"target":Target::forever_70374(),"catalog_sha256":catalog_hash,"supported_sizes":[2],"rows":rows}),
    );
}
#[test]
fn v2_roundtrip_is_deterministic_and_preserves_authored_states() {
    let t = Temp::new();
    let input = t.input();
    let one = t.output();
    let two = t.0.join("two");
    let report = build(&input, &one).unwrap();
    build(&input, &two).unwrap();
    assert_eq!(files(&one), files(&two));
    assert_eq!(report.report_version, 2);
    assert_eq!(report.fixtures, 1);
    assert!(report.coverage_gaps > 0);
    assert_eq!(verify(&one).unwrap().total_bytes, report.total_bytes);
    let c = read(&one.join("catalog.json"));
    assert_eq!(c["quest_facts"][0]["shareable"]["value"], json!(false));
    assert_eq!(
        c["acquisition_facts"][0]["rewards"]["value"][0]["quantity"],
        json!(0)
    );
    assert_eq!(
        c["acquisition_facts"][0]["entitlement"]["status"],
        json!("restricted")
    );
    assert_eq!(c["models"][0]["proficiency"]["status"], json!("unknown"));
    let source = read(&one.join("sources.json"));
    assert_eq!(
        source["sources"][0]["offline_usable"]["value"],
        json!(false)
    );
}
#[test]
fn v2_verify_rejects_noncanonical_tampered_extra_unknown_and_future_files() {
    let t = Temp::new();
    let input = t.input();
    let out = t.output();
    build(&input, &out).unwrap();
    let path = out.join("manifest.json");
    let manifest = fs::read(&path).unwrap();
    fs::write(&path, serde_json::to_vec(&read(&path)).unwrap()).unwrap();
    assert!(verify(&out).is_err());
    fs::write(&path, &manifest).unwrap();
    change(&path, |v| v["unknown"] = json!(true));
    assert!(verify(&out).is_err());
    fs::write(&path, &manifest).unwrap();
    change(&path, |v| v["schema_version"] = json!(3));
    assert!(verify(&out).is_err());
    fs::write(&path, &manifest).unwrap();
    fs::write(out.join("extra.txt"), b"extra").unwrap();
    assert!(verify(&out).is_err());
    fs::remove_file(out.join("extra.txt")).unwrap();
    fs::write(out.join("catalog.json"), b"{}").unwrap();
    assert!(verify(&out).is_err());
}
#[test]
fn v2_declared_coverage_and_rights_cannot_be_overridden() {
    let t = Temp::new();
    let input = t.input();
    let out = t.output();
    build(&input, &out).unwrap();
    let path = out.join("manifest.json");
    let original = fs::read(&path).unwrap();
    change(&path, |v| v["coverage_gaps"] = json!(0));
    assert!(verify(&out).is_err());
    fs::write(&path, original).unwrap();
    change(&path, |v| v["distribution_eligible"] = json!(false));
    assert!(verify(&out).is_err());
    change(&input.join("sources.json"), |v| {
        v["sources"][0]["redistribution"] = json!({"status":"unknown"})
    });
    let report = build(&input, &t.0.join("unknown-rights")).unwrap();
    assert!(!report.distribution_eligible);
}
#[test]
fn v2_party_material_closure_is_exact_and_target_bound() {
    let t = Temp::new();
    let input = t.input();
    add_party(&input, true);
    let out = t.output();
    let report = build(&input, &out).unwrap();
    assert_eq!(report.fixtures, 1);
    assert_eq!(report.files, 6);
    let manifest = read(&out.join("manifest.json"));
    assert_eq!(manifest["supporting_files"].as_array().unwrap().len(), 2);
    assert!(verify(&out).is_ok());
    fs::write(out.join("evidence/proof.txt"), b"tampered").unwrap();
    assert!(verify(&out).is_err());
    let mixed = Temp::new();
    let input = mixed.input();
    add_party(&input, false);
    assert!(build(&input, &mixed.output()).is_err());
}
#[test]
fn v2_party_requires_concrete_dependencies_matching_hash_and_complete_evidence() {
    for mutation in 0..3 {
        let t = Temp::new();
        let input = t.input();
        add_party(&input, true);
        match mutation {
            0 => change(&input.join("party-support.json"), |v| {
                v["rows"][0]["dependencies"][0]["record_id"] = json!("local-profile-or-scenario")
            }),
            1 => change(&input.join("party-support.json"), |v| {
                v["catalog_sha256"] = json!("0".repeat(64))
            }),
            _ => {
                fs::remove_file(input.join("evidence/proof.txt")).unwrap();
            }
        }
        assert!(build(&input, &t.output()).is_err());
        assert!(!t.output().exists());
    }
}
#[test]
fn v2_authoring_rejects_nested_unknown_fields_and_duplicate_members() {
    let t = Temp::new();
    let input = t.input();
    change(&input.join("catalog.json"), |v| {
        v["models"][0]["profile"]["untyped"] = json!(true)
    });
    assert!(build(&input, &t.output()).is_err());
    fs::write(
        input.join("catalog.json"),
        include_bytes!("../../../fixtures/contracts/v2/reference-content.json"),
    )
    .unwrap();
    let bytes = fs::read_to_string(input.join("catalog.json"))
        .unwrap()
        .replacen("\"quantity\": 0", "\"quantity\": 0, \"quantity\": 0", 1);
    fs::write(input.join("catalog.json"), bytes).unwrap();
    assert!(build(&input, &t.output()).is_err());
}

#[test]
fn v2_cli_dispatch_and_failed_build_preserve_existing_output() {
    let t = Temp::new();
    let input = t.input();
    let out = t.output();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(["build-content-v2", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["report_version"], json!(2));
    let before = files(&out);
    change(&input.join("catalog.json"), |v| {
        v["models"][0]["evidence"] = json!([])
    });
    assert!(build(&input, &out).is_err());
    assert_eq!(files(&out), before);
    let verify = std::process::Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(["verify-pack", "--input"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(verify.status.success());
}
#[cfg(unix)]
#[test]
fn v2_refuses_symlink_catalog_and_supporting_material() {
    let t = Temp::new();
    let input = t.input();
    fs::rename(input.join("catalog.json"), input.join("real.json")).unwrap();
    std::os::unix::fs::symlink("real.json", input.join("catalog.json")).unwrap();
    assert!(build(&input, &t.output()).is_err());
    let t = Temp::new();
    let input = t.input();
    add_party(&input, true);
    fs::rename(
        input.join("evidence/proof.txt"),
        input.join("evidence/real.txt"),
    )
    .unwrap();
    std::os::unix::fs::symlink("real.txt", input.join("evidence/proof.txt")).unwrap();
    assert!(build(&input, &t.output()).is_err());
}
