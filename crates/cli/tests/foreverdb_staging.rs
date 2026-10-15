use azeroth_cli::foreverdb::{Options, report};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Case {
    root: PathBuf,
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "navzeroth-foreverdb-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join("source/ForeverDB/Data")).unwrap();
        let c = Self { root };
        let meta=br#"local _, ns = ... ns.meta={version="synthetic-r1",built=0,build="1.60.1.70338",cap=30,site="https://foreverdb.net",addon="0.34.0"}"#.to_vec();
        let make = |names: &[&str]| {
            let mut s = "local _, ns = ...\n".to_owned();
            for name in names {
                s.push_str(&format!("ns.{name}={{}}\n"));
            }
            s.into_bytes()
        };
        let f = make(&[
            "qfQuests",
            "qfPoi",
            "qfSeen",
            "qfNpcNames",
            "qfNpcSpots",
            "qfItems",
            "qfArea",
            "qfByMap",
        ]);
        let classic = make(&[
            "qcQuests",
            "qcNpc",
            "qcObj",
            "qcItem",
            "qcByMap",
            "qcExcl",
            "dgClassic",
        ]);
        let mut manifest = vec![];
        for (name, bytes) in [
            ("Meta.lua", meta),
            ("QuestForever.lua", f),
            ("QuestClassic.lua", classic),
        ] {
            let path = format!("ForeverDB/Data/{name}");
            fs::write(c.root.join("source").join(&path), &bytes).unwrap();
            manifest.push(serde_json::json!({"path":path,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes))}));
        }
        fs::write(
            c.root.join("members.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        c
    }
    fn args(&self) -> Vec<OsString> {
        vec![
            "--input".into(),
            self.root.join("source").into_os_string(),
            "--manifest".into(),
            self.root.join("members.json").into_os_string(),
        ]
    }
    fn run(&self) -> Result<serde_json::Value, &'static str> {
        report(&Options::parse(&self.args()).unwrap()).map(|r| serde_json::to_value(r).unwrap())
    }
}
#[test]
fn selected_manifest_pins_files_but_does_not_admit_or_authenticate_archive() {
    let c = Case::new();
    let r = c.run().unwrap();
    assert_eq!(
        r["staging"]["metadata"]["bundle_origin_build"],
        "1.60.1.70338"
    );
    assert_eq!(
        r["archive_identity"],
        "not established from extracted files"
    );
    assert_eq!(r["staging"]["publication_allowed"], false);
    assert!(r["staging"]["independently_required_quest_ids"].is_null());
    let cmd = std::process::Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("stage-foreverdb")
        .args(c.args())
        .output()
        .unwrap();
    assert!(cmd.status.success());
    assert!(cmd.stderr.is_empty());
    let out: serde_json::Value = serde_json::from_slice(&cmd.stdout).unwrap();
    assert_eq!(r, out);
    fs::write(c.root.join("source/ForeverDB/Data/Meta.lua"), "changed").unwrap();
    assert!(c.run().is_err());
}
#[test]
fn malformed_manifest_declarations_are_rejected() {
    let c = Case::new();
    let path = c.root.join("members.json");
    let original = fs::read(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&original).unwrap();
    for (key, value) in [
        ("path", serde_json::json!("../Meta.lua")),
        ("bytes", serde_json::json!(9_000_000)),
        ("sha256", serde_json::json!("not-a-hash")),
        ("unknown", serde_json::json!(true)),
    ] {
        let mut v = parsed.clone();
        v[0][key] = value;
        fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(c.run().is_err());
    }
    let mut duplicate = parsed.clone();
    duplicate.as_array_mut().unwrap().push(parsed[0].clone());
    fs::write(&path, serde_json::to_vec(&duplicate).unwrap()).unwrap();
    assert!(c.run().is_err());
    fs::write(&path, r#"[{"path":"a","path":"b","bytes":0,"sha256":"a"}]"#).unwrap();
    assert!(c.run().is_err());
    fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(c.run().is_err());
}
#[test]
fn bounded_exact_options_and_safe_paths() {
    let c = Case::new();
    let mut a = c.args();
    a.push("--activate".into());
    assert!(Options::parse(&a).is_none());
    a = c.args();
    a[0] = "--manifest".into();
    assert!(Options::parse(&a).is_none());
    a = c.args();
    a[1] = "x".repeat(4097).into();
    assert!(Options::parse(&a).is_none());
    let mut a = c.args();
    a[1] = c.root.join("source/../source").into_os_string();
    assert!(report(&Options::parse(&a).unwrap()).is_err());
}
#[cfg(unix)]
#[test]
fn refuses_symlinked_source_files_and_manifest_paths() {
    use std::os::unix::fs::symlink;
    let c = Case::new();
    let path = c.root.join("source/ForeverDB/Data/Meta.lua");
    let actual = c.root.join("actual.lua");
    fs::rename(&path, &actual).unwrap();
    symlink(&actual, &path).unwrap();
    assert!(c.run().is_err());
    let alias = c.root.join("alias");
    symlink(c.root.join("source"), &alias).unwrap();
    let mut a = c.args();
    a[1] = alias.into_os_string();
    assert!(report(&Options::parse(&a).unwrap()).is_err());
    let m = Case::new();
    let selected = m.root.join("members.json");
    let actual = m.root.join("actual-members.json");
    fs::rename(&selected, &actual).unwrap();
    symlink(&actual, &selected).unwrap();
    assert!(m.run().is_err());
}
