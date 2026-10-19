//! `verify-world-data` (task V2.7e). Synthetic shards in temporary folders only.
use azeroth_cli::world_data::{MAX_REPORTED_ERRORS, Options, Report, verify};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const BUILD: &str = "1.12.1.5875";
const FOLDER: &str = "professions/notes/vanilla_1_12/1.12.1.5875";

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "anav-world-data-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, rel: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
    fn shard(&self, rel: &str, value: &Value) -> Value {
        let bytes = serde_json::to_vec(value).unwrap();
        self.write(rel, &bytes);
        json!({"path":rel,"sha256":format!("{:x}", Sha256::digest(&bytes)),"bytes":bytes.len(),
            "records":value["records"].as_array().unwrap().len()})
    }
    fn options(&self, extra: &[&str]) -> Options {
        let mut args: Vec<OsString> = vec!["--input".into(), self.0.clone().into()];
        args.extend(extra.iter().map(OsString::from));
        Options::parse(&args).unwrap()
    }
    fn verify(&self) -> Report {
        verify(&self.options(&[])).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn note(text: &str) -> Value {
    json!({"kind":"profession_note","profession":"Mining","text":text})
}
fn shard(dataset: &str, build: Option<&str>, index: u32, count: u32, records: usize) -> Value {
    json!({
        "format":"navzeroth-world-data","schema_version":1,"dataset":dataset,
        "record_kind":"profession_note",
        "origin":{"edition":"vanilla_1_12","build":build.map_or(json!({"status":"unknown"}), |b| json!({"status":"known","value":b})),"locale":"enUS"},
        "target":{"edition":"wow_forever","build":{"status":"unknown"}},
        "confidence":"cross_edition_unverified",
        "sources":[{"id":"cmangos-classic-db","url":"https://github.com/cmangos/classic-db"}],
        "generator":{"name":"gen_test","version":"1","credit":"Data generators based on work by NotInTheBand (TALOD), used with permission."},
        "shard":{"index":index,"count":count},
        "records":(0..records).map(|i| note(&format!("note {i}"))).collect::<Vec<_>>()
    })
}
fn manifest(files: Vec<Value>) -> Value {
    json!({"format":"navzeroth-world-data-run","schema_version":1,
        "generators":[{"name":"gen_test","status":"ok","files":files},{"name":"gen_other","status":"skipped","files":[]}],
        "sources":[{"url":"https://github.com/cmangos/classic-db","status":"ok","sha256":"a".repeat(64),"bytes":3},
            {"url":"https://wago.tools/db2/AreaTable/csv","status":200}]})
}
/// A passing tree: one two-part set, one unknown-build set, ignored extras.
fn tree(t: &Temp) -> Vec<Value> {
    let files = vec![
        t.shard(
            &format!("{FOLDER}/part-0000.json"),
            &shard("professions.notes", Some(BUILD), 0, 2, 2),
        ),
        t.shard(
            &format!("{FOLDER}/part-0001.json"),
            &shard("professions.notes", Some(BUILD), 1, 2, 1),
        ),
        t.shard(
            "world/notes/vanilla_1_12/unknown-build/part-0000.json",
            &shard("world.notes", None, 0, 1, 4),
        ),
    ];
    t.write("facts/world/part-0000.json", b"not a shard");
    t.write(".cache/x.json", b"not a shard");
    t.write(&format!("{FOLDER}/README.txt"), b"ignored");
    files
}
fn errors(report: &Report) -> String {
    report.errors.join("\n")
}
fn has(report: &Report, needle: &str) {
    assert!(
        report.errors.iter().any(|e| e.contains(needle)),
        "missing {needle:?} in {}",
        errors(report)
    );
    assert!(!report.passes);
}
fn cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("verify-world-data")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn a_consistent_tree_with_its_manifest_passes_with_a_deterministic_summary() {
    let t = Temp::new();
    let files = tree(&t);
    t.write("run-manifest.json", manifest(files).to_string());
    let report = t.verify();
    assert!(report.passes, "{}", errors(&report));
    assert!(report.manifest_checked);
    assert_eq!(
        (report.files, report.records, report.error_count),
        (3, 7, 0)
    );
    assert_eq!(report.datasets["professions.notes"], 3);
    assert_eq!(report.datasets["world.notes"], 4);
    assert_eq!(report.folders[0].path, FOLDER);
    assert_eq!(report.folders[0].origin_build.as_deref(), Some(BUILD));
    assert_eq!(report.folders[1].origin_build, None);

    let one = cli(&["--input".as_ref(), t.0.as_os_str()]);
    let two = cli(&["--input".as_ref(), t.0.as_os_str()]);
    assert!(one.status.success());
    assert_eq!(one.stdout, two.stdout);
    let value: Value = serde_json::from_slice(&one.stdout).unwrap();
    assert_eq!(value["format"], "navzeroth-world-data-verification");
    assert_eq!(value["folders"][0]["record_kind"], "profession_note");
    assert!(!String::from_utf8_lossy(&one.stdout).contains(t.0.to_str().unwrap()));
}

#[test]
fn the_manifest_is_optional_and_can_live_elsewhere() {
    let t = Temp::new();
    let files = tree(&t);
    let report = t.verify();
    assert!(report.passes && !report.manifest_checked);
    let elsewhere = Temp::new();
    let path = elsewhere.write("run.json", manifest(files).to_string());
    let report = verify(&t.options(&["--manifest", path.to_str().unwrap()])).unwrap();
    assert!(
        report.passes && report.manifest_checked,
        "{}",
        errors(&report)
    );
    let missing = elsewhere.0.join("absent.json");
    has(
        &verify(&t.options(&["--manifest", missing.to_str().unwrap()])).unwrap(),
        "must be an existing regular file",
    );
    has(
        &verify(&t.options(&["--manifest", elsewhere.0.to_str().unwrap()])).unwrap(),
        "must be an existing regular file",
    );
}

#[test]
fn invalid_and_oversized_shards_are_reported_without_reading_them_fully() {
    let t = Temp::new();
    tree(&t);
    t.write("world/bad/vanilla_1_12/unknown-build/part-0000.json", b"{");
    let mut wrong = shard("world.bad2", None, 0, 1, 1);
    wrong["records"][0]["kind"] = json!("npc");
    t.write(
        "world/bad2/vanilla_1_12/unknown-build/part-0000.json",
        wrong.to_string(),
    );
    let big = t.write("world/big/vanilla_1_12/unknown-build/part-0000.json", b"");
    fs::OpenOptions::new()
        .write(true)
        .open(&big)
        .unwrap()
        .set_len(azeroth_core::world_data::MAX_SHARD_BYTES as u64 + 1)
        .unwrap();
    let report = t.verify();
    has(
        &report,
        "world/bad/vanilla_1_12/unknown-build/part-0000.json: World-data shard is not valid JSON",
    );
    has(
        &report,
        "world/bad2/vanilla_1_12/unknown-build/part-0000.json:",
    );
    has(
        &report,
        "world/big/vanilla_1_12/unknown-build/part-0000.json: larger than 16 MiB",
    );
    assert_eq!(report.files, 5);
}

#[test]
fn every_shard_set_inconsistency_is_named() {
    let t = Temp::new();
    // Envelope fields differ between parts of one set.
    let base = shard("set.a", Some(BUILD), 0, 2, 1);
    t.write(
        "set/a/vanilla_1_12/1.12.1.5875/part-0000.json",
        base.to_string(),
    );
    let mut other = shard("set.a", Some(BUILD), 1, 2, 1);
    other["confidence"] = json!("disputed_or_stale");
    other["generator"]["version"] = json!("2");
    t.write(
        "set/a/vanilla_1_12/1.12.1.5875/part-0001.json",
        other.to_string(),
    );
    // Counts disagree.
    t.write(
        "set/b/vanilla_1_12/unknown-build/part-0000.json",
        shard("set.b", None, 0, 2, 1).to_string(),
    );
    t.write(
        "set/b/vanilla_1_12/unknown-build/part-0001.json",
        shard("set.b", None, 1, 3, 1).to_string(),
    );
    // Duplicate index; a missing index; a misnamed file.
    t.write(
        "set/c/vanilla_1_12/unknown-build/part-0000.json",
        shard("set.c", None, 0, 3, 1).to_string(),
    );
    t.write(
        "set/c/vanilla_1_12/unknown-build/copy.json",
        shard("set.c", None, 0, 3, 1).to_string(),
    );
    // Folder differs from dataset/edition/build.
    t.write(
        "set/d/wow_forever/unknown-build/part-0000.json",
        shard("set.d", None, 0, 1, 1).to_string(),
    );
    t.write("loose.json", shard("set.e", None, 0, 1, 1).to_string());
    // An invalid part leaves its set incomplete.
    t.write(
        "set/f/vanilla_1_12/unknown-build/part-0000.json",
        shard("set.f", None, 0, 2, 1).to_string(),
    );
    t.write("set/f/vanilla_1_12/unknown-build/part-0001.json", b"[]");
    let report = t.verify();
    has(
        &report,
        "set/a/vanilla_1_12/1.12.1.5875/part-0001.json: confidence, generator differ from set/a/vanilla_1_12/1.12.1.5875/part-0000.json",
    );
    has(
        &report,
        "set/b/vanilla_1_12/unknown-build: shards disagree on the shard count",
    );
    has(
        &report,
        "set/c/vanilla_1_12/unknown-build/part-0000.json: duplicate shard index 0",
    );
    has(
        &report,
        "set/c/vanilla_1_12/unknown-build: shard indexes are not exactly 0..2",
    );
    has(
        &report,
        "set/c/vanilla_1_12/unknown-build/copy.json: file name must be part-0000.json",
    );
    has(
        &report,
        "set/d/wow_forever/unknown-build: folder must be set/d/vanilla_1_12/unknown-build/",
    );
    has(
        &report,
        ".: folder must be set/e/vanilla_1_12/unknown-build/",
    );
    has(
        &report,
        "set/f/vanilla_1_12/unknown-build: shard indexes are not exactly 0..1",
    );
}

#[test]
fn manifest_files_must_match_disk_exactly() {
    let t = Temp::new();
    let mut files = tree(&t);
    let first = format!("{FOLDER}/part-0000.json");
    files[0]["sha256"] = json!("0".repeat(64));
    files[1]["bytes"] = json!(1);
    files[2]["records"] = json!(99);
    let unlisted = files.pop().unwrap();
    files.push(
        json!({"path":"absent/part-0000.json","sha256":"0".repeat(64),"bytes":1,"records":1}),
    );
    files.push(files[0].clone());
    for path in [
        "/abs/part-0000.json",
        "a/../b.json",
        "a\\b.json",
        "C:/x.json",
        "a//b.json",
        "./a.json",
        "",
    ] {
        files.push(json!({"path":path,"sha256":"0".repeat(64),"bytes":1,"records":1}));
    }
    let mut m = manifest(files);
    m["generators"][1]["status"] = json!("failed");
    m["generators"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"gen_test","status":"ok","files":[]}));
    m["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"url":"http://insecure.invalid","status":"ok"}));
    m["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"url":"https://ok.invalid","status":"ok","sha256":"ABC"}));
    m["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"url":"https://ok.invalid","status":42}));
    t.write("run-manifest.json", m.to_string());
    let report = t.verify();
    has(
        &report,
        &format!("{first}: SHA-256 differs from the run manifest"),
    );
    has(
        &report,
        "part-0001.json: byte count differs from the run manifest",
    );
    has(
        &report,
        "run manifest: absent/part-0000.json is not a world-data shard on disk",
    );
    has(&report, &format!("run manifest: {first} is listed twice"));
    has(
        &report,
        &format!(
            "{}: not listed in the run manifest",
            unlisted["path"].as_str().unwrap()
        ),
    );
    has(&report, "run manifest: generator gen_other failed");
    has(&report, "generator names must be unique");
    assert_eq!(
        report
            .errors
            .iter()
            .filter(|e| e.contains("not relative with / separators"))
            .count(),
        7
    );
    assert_eq!(
        report
            .errors
            .iter()
            .filter(|e| e.contains("a source needs an https URL"))
            .count(),
        3
    );

    // Record count mismatch on a listed shard.
    let t = Temp::new();
    let mut files = tree(&t);
    files[2]["records"] = json!(99);
    t.write("run-manifest.json", manifest(files).to_string());
    has(
        &t.verify(),
        "unknown-build/part-0000.json: record count differs",
    );
}

#[test]
fn malformed_manifests_are_refused() {
    for (body, needle) in [
        ("{".to_owned(), "not a valid navzeroth-world-data-run document"),
        (
            r#"{"format":"navzeroth-world-data-run","schema_version":1,"generators":[],"sources":[],"extra":1}"#.to_owned(),
            "not a valid",
        ),
        (
            r#"{"format":"navzeroth-world-data-run","format":"x","schema_version":1,"generators":[],"sources":[]}"#.to_owned(),
            "not a valid",
        ),
        (
            r#"{"format":"other","schema_version":1,"generators":[],"sources":[]}"#.to_owned(),
            "format must be navzeroth-world-data-run v1",
        ),
        (
            r#"{"format":"navzeroth-world-data-run","schema_version":2,"generators":[],"sources":[]}"#.to_owned(),
            "format must be",
        ),
        (
            r#"{"format":"navzeroth-world-data-run","schema_version":1,"generators":[{"name":"g","status":"maybe","files":[]}],"sources":[]}"#.to_owned(),
            "not a valid",
        ),
    ] {
        let t = Temp::new();
        t.write("run-manifest.json", body);
        has(&t.verify(), needle);
    }
    // An empty tree with an empty manifest passes.
    let t = Temp::new();
    t.write(
        "run-manifest.json",
        r#"{"format":"navzeroth-world-data-run","schema_version":1,"generators":[],"sources":[]}"#,
    );
    let report = t.verify();
    assert!(report.passes && report.manifest_checked && report.files == 0);
}

#[test]
fn reported_errors_are_capped_but_counted() {
    let t = Temp::new();
    for i in 0..MAX_REPORTED_ERRORS + 5 {
        t.write(&format!("bad/part-{i:04}.json"), b"{");
    }
    let report = t.verify();
    assert_eq!(report.error_count, MAX_REPORTED_ERRORS + 5);
    assert_eq!(report.errors.len(), MAX_REPORTED_ERRORS);
}

#[test]
fn deep_nesting_and_non_directories_are_refused() {
    let t = Temp::new();
    t.write(&format!("{}x.json", "d/".repeat(40)), b"{}");
    assert!(verify(&t.options(&[])).is_err());
    let file = t.write("file.json", b"{}");
    let args: Vec<OsString> = vec!["--input".into(), file.into()];
    assert!(verify(&Options::parse(&args).unwrap()).is_err());
    let out = cli(&["--input".as_ref(), t.0.join("absent").as_os_str()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("verify-world-data: Input must be"));
}

#[cfg(unix)]
#[test]
fn links_and_non_utf8_names_are_reported() {
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let t = Temp::new();
    tree(&t);
    let target = t.0.join(FOLDER).join("part-0000.json");
    symlink(&target, t.0.join("linked.json")).unwrap();
    symlink(t.0.join(FOLDER), t.0.join("linked-dir")).unwrap();
    fs::write(
        t.0.join(std::ffi::OsStr::from_bytes(b"bad-\xff.json")),
        b"{}",
    )
    .unwrap();
    let report = t.verify();
    has(&report, "linked.json: symbolic links are refused");
    has(&report, "linked-dir: symbolic links are refused");
    has(&report, ".: a file name is not UTF-8");
    let link = Temp::new();
    let path = link.0.join("input");
    symlink(&t.0, &path).unwrap();
    let args: Vec<OsString> = vec!["--input".into(), path.into()];
    assert!(verify(&Options::parse(&args).unwrap()).is_err());
    let manifest_link = t.0.join("run-manifest.json");
    symlink(&target, &manifest_link).unwrap();
    has(&t.verify(), "must be an existing regular file");
}

#[test]
fn failing_trees_exit_one_and_bad_arguments_exit_two() {
    let t = Temp::new();
    t.write("bad.json", b"{");
    let out = cli(&["--input".as_ref(), t.0.as_os_str()]);
    assert_eq!(out.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["passes"], false);
    let dir = t.0.as_os_str();
    for args in [
        &[][..],
        &["--input".as_ref()],
        &["--input".as_ref(), dir, "--input".as_ref(), dir],
        &["--input".as_ref(), "--manifest".as_ref()],
        &["--manifest".as_ref(), dir],
        &["--other".as_ref(), dir],
        &["--input".as_ref(), "".as_ref()],
        &[
            "--input".as_ref(),
            dir,
            "--manifest".as_ref(),
            dir,
            "--x".as_ref(),
            dir,
        ],
    ] {
        let out = cli(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("verify-world-data --input"));
    }
}

#[test]
fn shards_written_by_the_python_envelope_library_verify() {
    let python = ["python3", "python"].into_iter().find(|p| {
        Command::new(p)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    });
    let Some(python) = python else {
        eprintln!("python unavailable; envelope round trip skipped");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data-generators");
    let t = Temp::new();
    let script = r#"
import sys
sys.path.insert(0, sys.argv[1])
from navzeroth_datagen import envelope as e
o = e.origin(e.VANILLA, "1.12.1.5875", product="cmangos")
src = [e.source("cmangos-classic-db", "https://github.com/cmangos/classic-db")]
recs = [{"profession": "Mining", "text": "note %d" % i} for i in range(50)]
e.write("professions.notes", "profession_note", recs, origin=o, sources=src,
        generator=e.generator("gen_test"), out=sys.argv[2], max_bytes=1500)
e.write("world.notes", "profession_note", recs[:3], origin=e.origin(e.FOREVER), sources=src,
        generator=e.generator("gen_test"), out=sys.argv[2])
"#;
    let out = Command::new(python)
        .arg("-I")
        .arg("-c")
        .arg(script)
        .arg(&root)
        .arg(&t.0)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = t.verify();
    assert!(report.passes, "{}", errors(&report));
    assert!(report.folders[0].shards > 1);
    assert_eq!(report.datasets["professions.notes"], 50);
    assert_eq!(
        report.folders[1].path,
        "world/notes/wow_forever/unknown-build"
    );
}
