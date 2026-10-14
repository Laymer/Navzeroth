use azeroth_cli::pack::{MAX_ASSET_BYTES, MAX_MANIFEST_BYTES, MAX_SOURCES_BYTES, build, verify};
use azeroth_core::contract::MAX_DATASET_BYTES;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "anav-pack-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn input(&self) -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = self.0.join("input");
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
        input
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
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn modify(path: &Path, change: impl FnOnce(&mut Value)) {
    let mut value = read(path);
    change(&mut value);
    write(path, &value);
}
fn files(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, result: &mut Vec<(PathBuf, Vec<u8>)>) {
        for child in fs::read_dir(dir).unwrap() {
            let p = child.unwrap().path();
            if p.is_dir() {
                walk(root, &p, result)
            } else {
                result.push((
                    p.strip_prefix(root).unwrap().to_owned(),
                    fs::read(p).unwrap(),
                ));
            }
        }
    }
    let mut result = Vec::new();
    walk(path, path, &mut result);
    result.sort();
    result
}
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn builds_deterministically_verifies_exact_bytes_and_preserves_source_facts() {
    let temp = Temp::new();
    let input = temp.input();
    let one = temp.0.join("one");
    let two = temp.0.join("two");
    let report = build(&input, &one).unwrap();
    assert_eq!(report.datasets, 1);
    assert_eq!(report.fixtures, 1);
    assert_eq!(report.files, 6);
    assert!(report.coverage_gaps > 0);
    assert!(report.distribution_eligible);
    build(&input, &two).unwrap();
    assert_eq!(files(&one), files(&two));
    assert_eq!(verify(&one).unwrap().total_bytes, report.total_bytes);
    let manifest = read(&one.join("manifest.json"));
    assert_eq!(manifest["purpose"], "local_reference");
    assert_eq!(
        manifest["entries"][0]["context"]["build"]["build"],
        "UNVERIFIED_DEMO"
    );
    let dataset = read(&one.join("entries/0000/dataset.json"));
    assert_eq!(
        dataset["provenance"]["created_at_utc_ms"],
        1791792000000_i64
    );
    assert_eq!(
        dataset["payload"]["evidence"][0]["license"]["status"],
        "unknown"
    );
    let sources = read(&one.join("sources.json"));
    assert_eq!(sources["sources"][0]["offline_usable"]["value"], false);
    assert_eq!(sources["sources"][0]["last_retrieval_utc_ms"]["value"], 0);
    let output = cli()
        .args(["verify-pack", "--input"])
        .arg(&one)
        .output()
        .unwrap();
    assert!(output.status.success());
    let r: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(r["datasets"], 1);
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 3);
}
#[test]
fn exact_contexts_remain_separate_and_duplicate_identities_fail() {
    let temp = Temp::new();
    let input = temp.input();
    let a = input.join("datasets/synthetic.json");
    let mut second = read(&a);
    second["payload"]["revision"] = json!("second-revision");
    fn alternate_context(value: &mut Value) {
        match value {
            Value::String(s) if s == "UNVERIFIED_DEMO" => *s = "UNVERIFIED_DEMO_TWO".to_owned(),
            Value::Object(map) => map.values_mut().for_each(alternate_context),
            Value::Array(values) => values.iter_mut().for_each(alternate_context),
            _ => {}
        }
    }
    alternate_context(&mut second);
    write(&input.join("datasets/second.json"), &second);
    let out = temp.0.join("pack");
    assert_eq!(build(&input, &out).unwrap().datasets, 2);
    assert!(out.join("entries/0001/dataset.json").is_file());
    fs::copy(&a, input.join("datasets/second.json")).unwrap();
    assert!(build(&input, &temp.0.join("duplicate")).is_err());
    modify(&a, |v| v["game"]["build"] = json!("foreign-build"));
    fs::remove_file(input.join("datasets/second.json")).unwrap();
    assert!(build(&input, &temp.0.join("mixed-context")).is_err());
}
#[test]
fn unknown_conditional_negative_or_incomplete_policy_is_never_permission() {
    for (field, value) in [
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
        ("allowed_fields", json!({"status":"unknown"})),
        ("transformations", json!({"status":"unknown"})),
        (
            "editions",
            json!({"status":"known","value":["wow_classic"]}),
        ),
    ] {
        let temp = Temp::new();
        let input = temp.input();
        modify(&input.join("sources.json"), |v| {
            v["sources"][0][field] = value
        });
        let out = temp.0.join("pack");
        assert!(!build(&input, &out).unwrap().distribution_eligible);
        assert!(!verify(&out).unwrap().distribution_eligible);
    }
}
#[test]
fn unregistered_evidence_and_image_sources_and_duplicate_registry_fail() {
    for image_source in [false, true] {
        let temp = Temp::new();
        let input = temp.input();
        modify(&input.join("datasets/synthetic.json"), |v| {
            if image_source {
                v["payload"]["map_images"][0]["source"] = json!("missing-source");
            } else {
                v["payload"]["evidence"][0]["source"] = json!("missing-source");
            }
        });
        assert!(build(&input, &temp.0.join("pack")).is_err());
    }
    let temp = Temp::new();
    let input = temp.input();
    modify(&input.join("sources.json"), |v| {
        let duplicate = v["sources"][0].clone();
        v["sources"].as_array_mut().unwrap().push(duplicate);
    });
    assert!(build(&input, &temp.0.join("pack")).is_err());
}
#[test]
fn manifest_tamper_future_headers_context_coverage_and_policy_fail() {
    let changes: [fn(&mut Value); 11] = [
        |v| v["schema_version"] = json!(2),
        |v| v["purpose"] = json!("redistributable"),
        |v| v["entries"][0]["context"]["build"]["edition"] = json!("wow_classic"),
        |v| v["entries"][0]["fixture"] = json!(false),
        |v| v["entries"][0]["revision"] = json!("forged"),
        |v| v["entries"][0]["coverage"]["gaps"] = json!(0),
        |v| v["distribution_eligible"] = json!(false),
        |v| v["sources"]["bytes"] = json!(0),
        |v| v["sources"]["path"] = json!("../outside"),
        |v| v["unknown_private_member"] = json!("private-sentinel"),
        |v| {
            let duplicate = v["entries"][0].clone();
            v["entries"].as_array_mut().unwrap().push(duplicate);
        },
    ];
    for change in changes {
        let temp = Temp::new();
        let input = temp.input();
        let out = temp.0.join("pack");
        build(&input, &out).unwrap();
        modify(&out.join("manifest.json"), change);
        assert!(verify(&out).is_err());
    }
}
#[test]
fn hashes_and_exact_file_sets_refuse_tampering_missing_extra_or_empty_directories() {
    for case in 0..4 {
        let temp = Temp::new();
        let input = temp.input();
        let out = temp.0.join("pack");
        build(&input, &out).unwrap();
        match case {
            0 => fs::write(out.join("entries/0000/dataset.json"), b"private-tamper").unwrap(),
            1 => fs::remove_file(out.join("sources.json")).unwrap(),
            2 => fs::write(out.join("unexpected.lua"), b"never execute").unwrap(),
            _ => fs::create_dir(out.join("extra-empty")).unwrap(),
        };
        assert!(verify(&out).is_err());
    }
}
#[test]
fn source_future_unknown_dataset_extensions_and_nested_unknowns_fail_safely() {
    for case in 0..5 {
        let temp = Temp::new();
        let input = temp.input();
        match case {
            0 => modify(&input.join("sources.json"), |v| {
                v["schema_version"] = json!(2)
            }),
            1 => modify(&input.join("datasets/synthetic.json"), |v| {
                v["schema_version"] = json!(2)
            }),
            2 => modify(&input.join("datasets/synthetic.json"), |v| {
                v["private_extension"] = json!("private-sentinel")
            }),
            3 => modify(&input.join("datasets/synthetic.json"), |v| {
                v["payload"]["evidence"][0]["private_member"] = json!("private-sentinel")
            }),
            _ => modify(&input.join("sources.json"), |v| {
                v["sources"][0]["private_member"] = json!("private-sentinel")
            }),
        }
        let output = cli()
            .args(["build-content", "--input"])
            .arg(&input)
            .arg("--output")
            .arg(temp.0.join("pack"))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            !String::from_utf8(output.stderr)
                .unwrap()
                .contains("sentinel")
        );
        assert!(!temp.0.join("pack").exists());
    }
}
#[test]
fn bounded_files_and_decoded_png_limits_refuse_oversized_inputs() {
    for (relative, limit) in [
        ("datasets/synthetic.json", MAX_DATASET_BYTES),
        ("sources.json", MAX_SOURCES_BYTES),
        (
            "atlas/synthetic-continent/layers/roads.png",
            MAX_ASSET_BYTES,
        ),
    ] {
        let temp = Temp::new();
        let input = temp.input();
        fs::OpenOptions::new()
            .write(true)
            .open(input.join(relative))
            .unwrap()
            .set_len(limit as u64 + 1)
            .unwrap();
        assert!(build(&input, &temp.0.join("pack")).is_err());
    }
    let temp = Temp::new();
    let input = temp.input();
    let out = temp.0.join("pack");
    build(&input, &out).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(out.join("manifest.json"))
        .unwrap()
        .set_len(MAX_MANIFEST_BYTES as u64 + 1)
        .unwrap();
    assert!(verify(&out).is_err());
    for oversized in [false, true] {
        let temp = Temp::new();
        let input = temp.input();
        let path = input.join("atlas/synthetic-continent/layers/roads.png");
        let image = image::RgbImage::new(if oversized { 8193 } else { 1 }, 1);
        image.save(&path).unwrap();
        let sha = digest(&fs::read(path).unwrap());
        modify(&input.join("datasets/synthetic.json"), |v| {
            v["payload"]["map_images"][0]["layers"][0]["sha256"] = json!(sha)
        });
        assert!(build(&input, &temp.0.join("pack")).is_err());
    }
}
#[test]
fn existing_output_survives_successful_input_and_malformed_input_failures() {
    let temp = Temp::new();
    let input = temp.input();
    let out = temp.0.join("pack");
    build(&input, &out).unwrap();
    let old = files(&out);
    assert!(build(&input, &out).is_err());
    assert_eq!(files(&out), old);
    fs::write(
        input.join("datasets/synthetic.json"),
        b"private-invalid-sentinel",
    )
    .unwrap();
    assert!(build(&input, &out).is_err());
    assert_eq!(files(&out), old);
    assert!(verify(&out).is_ok());
    let empty = temp.0.join("already-empty");
    fs::create_dir(&empty).unwrap();
    assert!(build(&input, &empty).is_err());
    assert!(empty.is_dir());
    assert_eq!(fs::read_dir(empty).unwrap().count(), 0);
}
#[test]
fn duplicate_json_members_and_malformed_cli_never_echo_arbitrary_input() {
    let temp = Temp::new();
    let input = temp.input();
    fs::write(
        input.join("sources.json"),
        b"{\"schema_version\":1,\"schema_version\":1,\"sources\":[]}",
    )
    .unwrap();
    assert!(build(&input, &temp.0.join("pack")).is_err());
    for args in [
        &["build-content", "--private-sentinel"][..],
        &["verify-pack", "--input", "private-sentinel", "--json"],
        &["build-content", "--input", "private-sentinel"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            !String::from_utf8(output.stderr)
                .unwrap()
                .contains("sentinel")
        );
    }
}
#[cfg(unix)]
#[test]
fn symlink_components_and_pack_symlinks_are_refused() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new();
    let input = temp.input();
    let link = temp.0.join("linked-input");
    symlink(&input, &link).unwrap();
    assert!(build(&link, &temp.0.join("pack")).is_err());
    let parent = temp.0.join("linked-parent");
    symlink(&temp.0, &parent).unwrap();
    assert!(build(&input, &parent.join("pack")).is_err());
    let asset = input.join("atlas/synthetic-continent/layers/roads.png");
    let real = input.join("real.png");
    fs::rename(&asset, &real).unwrap();
    symlink(&real, &asset).unwrap();
    assert!(build(&input, &temp.0.join("pack")).is_err());
    fs::remove_file(&asset).unwrap();
    fs::rename(&real, &asset).unwrap();
    let out = temp.0.join("pack");
    build(&input, &out).unwrap();
    let sources = out.join("sources.json");
    let real = temp.0.join("real-sources.json");
    fs::rename(&sources, &real).unwrap();
    symlink(&real, &sources).unwrap();
    assert!(verify(&out).is_err());
    assert!(build(&input, &temp.0.join("foo/../traversal")).is_err());
}

#[test]
fn input_filename_and_registry_order_do_not_change_pack_bytes() {
    let temp = Temp::new();
    let input = temp.input();
    let a = input.join("datasets/synthetic.json");
    let mut second = read(&a);
    second["payload"]["revision"] = json!("second-revision");
    write(&input.join("datasets/second.json"), &second);
    modify(&input.join("sources.json"), |v| {
        let mut extra = v["sources"][0].clone();
        extra["id"] = json!("additional-unused-synthetic-source");
        v["sources"].as_array_mut().unwrap().push(extra);
    });
    let one = temp.0.join("one");
    build(&input, &one).unwrap();
    fs::rename(&a, input.join("datasets/z-original.json")).unwrap();
    modify(&input.join("sources.json"), |v| {
        v["sources"].as_array_mut().unwrap().reverse()
    });
    let two = temp.0.join("two");
    build(&input, &two).unwrap();
    assert_eq!(files(&one), files(&two));
}
#[test]
fn updating_integrity_metadata_cannot_bypass_future_dataset_validation() {
    let temp = Temp::new();
    let input = temp.input();
    let out = temp.0.join("pack");
    build(&input, &out).unwrap();
    let dataset = out.join("entries/0000/dataset.json");
    modify(&dataset, |v| v["schema_version"] = json!(2));
    let bytes = fs::read(&dataset).unwrap();
    modify(&out.join("manifest.json"), |v| {
        v["entries"][0]["dataset"]["bytes"] = json!(bytes.len());
        v["entries"][0]["dataset"]["sha256"] = json!(digest(&bytes));
    });
    assert!(verify(&out).is_err());
}

#[test]
fn successful_cli_build_reports_only_allowlisted_counts_and_verifies() {
    let temp = Temp::new();
    let input = temp.input();
    let out = temp.0.join("private-path-sentinel");
    let output = cli()
        .args(["build-content", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("sentinel"));
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["datasets"], 1);
    assert_eq!(report.as_object().unwrap().len(), 7);
    assert!(verify(&out).is_ok());
}
#[test]
fn dataset_collection_bound_is_enforced_before_publishing() {
    let temp = Temp::new();
    let input = temp.input();
    let mut data = read(&input.join("datasets/synthetic.json"));
    fs::remove_file(input.join("datasets/synthetic.json")).unwrap();
    for n in 0..=azeroth_cli::pack::MAX_DATASETS {
        data["payload"]["revision"] = json!(format!("synthetic-{n:03}"));
        write(&input.join(format!("datasets/{n:03}.json")), &data);
    }
    assert!(build(&input, &temp.0.join("pack")).is_err());
    assert!(!temp.0.join("pack").exists());
}
