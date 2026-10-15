use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Case {
    root: PathBuf,
    registry: PathBuf,
    plan: PathBuf,
    cache: PathBuf,
    blob: String,
}
fn hash(bytes: &[u8]) -> String {
    azeroth_cli::source_pipeline::digest(bytes)
}
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "navzeroth-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        let facts=serde_json::to_vec(&json!({"schema_version":1,"facts":[{"domain":"quests","record":"quest/1","field":"shareable","datum":{"status":"known","value":false},"confidence":"publisher_documented","observed_at_utc_ms":0}]})).unwrap();
        let blob = hash(&facts);
        fs::write(cache.join(&blob), &facts).unwrap();
        let license = b"Synthetic licence evidence. Test fixture only; no real grant.";
        let proofhash = hash(license);
        fs::write(cache.join(&proofhash), license).unwrap();
        let proof = json!({"url":format!("https://raw.githubusercontent.com/synthetic/fixture/{}/LICENSE","a".repeat(40)),"revision":"a".repeat(40),"sha256":proofhash,"reviewed_scope":"synthetic fixture"});
        let registry = root.join("registry.json");
        let data = json!({"schema_version":1,"sources":[{"id":"synthetic","revision":"a".repeat(40),"publisher":"synthetic/fixture","origin":{"edition":"wow_forever","build":"1.60.1.70374"},"rights":{"status":"admitted","license":"permissive","license_identifier":"synthetic-test-only","reviewer":"fixture","attribution":"fixture","obligations":"fixture","evidence":[proof.clone()]},"retrieval":{"method":"https_pinned","max_total_bytes":facts.len()+license.len(),"minimum_interval_ms":1000,"max_requests":2,"access_evidence":proof},"files":[{"path":"facts.json","url":format!("https://raw.githubusercontent.com/synthetic/fixture/{}/facts.json","a".repeat(40)),"sha256":blob,"bytes":facts.len(),"fields":["quests.shareable"],"transformation":"knowledge_v1"}]}]});
        fs::write(&registry, serde_json::to_vec(&data).unwrap()).unwrap();
        let plan = root.join("plan.json");
        fs::write(&plan,serde_json::to_vec(&json!({"schema_version":1,"target":{"edition":"wow_forever","build":"1.60.1.70374"},"as_of_utc_ms":1,"inputs":[{"source":"synthetic","file":"facts.json","adapter":"knowledge_v1"}]})).unwrap()).unwrap();
        Self {
            root,
            registry,
            plan,
            cache,
            blob,
        }
    }
    fn sync(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(["source-sync", "--registry"])
            .arg(&self.registry)
            .arg("--cache")
            .arg(&self.cache)
            .args(extra)
            .env("PATH", "")
            .output()
            .unwrap()
    }
    fn build(&self, output: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(["source-build", "--registry"])
            .arg(&self.registry)
            .arg("--plan")
            .arg(&self.plan)
            .arg("--cache")
            .arg(&self.cache)
            .arg("--output")
            .arg(output)
            .env("PATH", "")
            .output()
            .unwrap()
    }
    fn change_registry(&self, change: impl FnOnce(&mut Value)) {
        let mut v: Value = serde_json::from_slice(&fs::read(&self.registry).unwrap()).unwrap();
        change(&mut v);
        fs::write(&self.registry, serde_json::to_vec(&v).unwrap()).unwrap();
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn offline_build_is_deterministic_including_actual_unchanged_cache_and_false_zero() {
    let c = Case::new();
    assert!(c.sync(&[]).status.success());
    let a = c.root.join("a");
    let b = c.root.join("b");
    let first = c.build(&a);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(c.build(&b).status.success());
    for name in [
        "catalog.json",
        "closure.json",
        "build.json",
        "plan.json",
        "registry.json",
        "NOTICES.txt",
    ] {
        assert_eq!(
            fs::read(a.join(name)).unwrap(),
            fs::read(b.join(name)).unwrap()
        );
    }
    // A built material closure is sufficient for another network-free build.
    let rebuilt = c.root.join("rebuilt");
    let result = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("source-build")
        .arg("--registry")
        .arg(a.join("registry.json"))
        .arg("--plan")
        .arg(a.join("plan.json"))
        .arg("--cache")
        .arg(a.join("materials/objects"))
        .arg("--output")
        .arg(&rebuilt)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(a.join("catalog.json")).unwrap(),
        fs::read(rebuilt.join("catalog.json")).unwrap()
    );
    let catalog: Value =
        serde_json::from_slice(&fs::read(a.join("catalog.json")).unwrap()).unwrap();
    assert_eq!(
        catalog["entries"][0]["candidates"][0]["fact"]["datum"]["value"],
        false
    );
    assert_eq!(
        catalog["entries"][0]["candidates"][0]["fact"]["observed_at_utc_ms"],
        0
    );
    assert_eq!(catalog["publication_authorized"], false);
    assert_eq!(catalog["domains"].as_array().unwrap().len(), 9);
    assert!(!c.build(&a).status.success());
    assert!(!fs::read(c.cache.join(&c.blob)).unwrap().is_empty());
}
#[test]
fn missing_tampered_and_unlicensed_inputs_never_publish_output_or_fetch_implicitly() {
    let c = Case::new();
    fs::write(c.cache.join(&c.blob), b"tampered").unwrap();
    assert!(!c.sync(&[]).status.success());
    assert!(!c.build(&c.root.join("out")).status.success());
    assert!(!c.root.join("out").exists());
    assert_eq!(fs::read(c.cache.join(&c.blob)).unwrap(), b"tampered");
    fs::remove_file(c.cache.join(&c.blob)).unwrap();
    let missing = c.build(&c.root.join("missing"));
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("Offline source closure is incomplete")
    );
    assert!(!c.root.join("missing").exists());
    c.change_registry(|v| v["sources"][0]["rights"]["status"] = json!("unreviewed"));
    assert!(c.sync(&[]).status.success());
    assert!(!c.build(&c.root.join("unreviewed")).status.success());
}
#[test]
fn transformation_and_field_allowlists_and_licence_material_are_mandatory() {
    let c = Case::new();
    c.change_registry(|v| v["sources"][0]["files"][0]["fields"] = json!(["quests.other"]));
    assert!(!c.build(&c.root.join("fields")).status.success());
    c.change_registry(|v| v["sources"][0]["files"][0]["transformation"] = json!("something_else"));
    assert!(!c.build(&c.root.join("transform")).status.success());
    let v: Value = serde_json::from_slice(&fs::read(&c.registry).unwrap()).unwrap();
    fs::remove_file(
        c.cache.join(
            v["sources"][0]["rights"]["evidence"][0]["sha256"]
                .as_str()
                .unwrap(),
        ),
    )
    .unwrap();
    assert!(!c.sync(&[]).status.success());
}
#[test]
fn strict_options_schema_paths_and_output_preservation() {
    let c = Case::new();
    assert!(!c.sync(&["--check-updates"]).status.success());
    assert!(!c.sync(&["--fetch", "--fetch"]).status.success());
    let old = c.root.join("old");
    fs::create_dir(&old).unwrap();
    fs::write(old.join("sentinel"), b"keep").unwrap();
    assert!(!c.build(&old).status.success());
    assert_eq!(fs::read(old.join("sentinel")).unwrap(), b"keep");
    fs::write(&c.plan, b"{\"schema_version\":1,\"schema_version\":1}").unwrap();
    assert!(!c.build(&c.root.join("bad")).status.success());
    assert!(!c.root.join("bad").exists());
}
#[cfg(unix)]
#[test]
fn source_and_cache_links_are_refused() {
    use std::os::unix::fs::symlink;
    let c = Case::new();
    let saved = c.root.join("saved");
    fs::rename(c.cache.join(&c.blob), &saved).unwrap();
    symlink(&saved, c.cache.join(&c.blob)).unwrap();
    assert!(!c.sync(&[]).status.success());
    fs::remove_file(c.cache.join(&c.blob)).unwrap();
    fs::rename(saved, c.cache.join(&c.blob)).unwrap();
    let copy = c.root.join("cache-copy");
    fs::rename(&c.cache, &copy).unwrap();
    symlink(&copy, &c.cache).unwrap();
    assert!(!c.sync(&[]).status.success());
}
#[test]
fn committed_licensed_materials_build_without_network_on_every_platform() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let c = Case::new();
    let out = c.root.join("real");
    let result = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("source-build")
        .arg("--registry")
        .arg(root.join("docs/sources/source-admission-v1.json"))
        .arg("--plan")
        .arg(root.join("content/source-build-v1.json"))
        .arg("--cache")
        .arg(root.join("content/source-materials/objects"))
        .arg("--output")
        .arg(&out)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let catalog: Value =
        serde_json::from_slice(&fs::read(out.join("catalog.json")).unwrap()).unwrap();
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 1681);
    assert!(
        catalog["domains"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["required_inventory"].is_null() && d["target_facts"] == 0)
    );
}

#[test]
fn metadata_permissions_are_explicit_bound_and_checked_before_network() {
    let c = Case::new();
    let policy = c.root.join("updates.json");
    let valid = json!({
        "schema_version": 1,
        "registry_sha256": hash(&fs::read(&c.registry).unwrap()),
        "method": "github_commit_metadata",
        "reviewer": "fixture", "reviewed_on": "2026-10-15", "scope": "fixture metadata only",
        "access_evidence_url": "https://docs.github.com/en/rest/commits/commits#list-commits",
        "max_requests": 1, "max_response_bytes": 1024, "max_total_bytes": 1024,
        "minimum_interval_ms": 1000,
        "repositories": [{"source_id": "synthetic", "repository": "synthetic/fixture"}]
    });
    let run = |v: &Value| {
        fs::write(&policy, serde_json::to_vec(v).unwrap()).unwrap();
        c.sync(&["--fetch", "--check-updates", policy.to_str().unwrap()])
    };
    // PATH is empty, so a permitted metadata request yields unknown freshness.
    // The retained source build still succeeds without curl or a network.
    let accepted = run(&valid);
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    let report: Value = serde_json::from_slice(&accepted.stdout).unwrap();
    assert_eq!(
        report["upstream_updates"][0]["status"],
        "upstream_freshness_unknown"
    );
    assert_eq!(
        report["updates_policy_sha256"],
        hash(&fs::read(&policy).unwrap())
    );
    for (field, value) in [
        ("schema_version", json!(2)),
        ("registry_sha256", json!("0".repeat(64))),
        ("method", json!("scrape")),
        ("access_evidence_url", json!("https://example.invalid")),
        ("max_requests", json!(0)),
        ("max_response_bytes", json!(1_048_577)),
        ("max_total_bytes", json!(1023)),
        ("minimum_interval_ms", json!(999)),
        (
            "repositories",
            json!([{"source_id":"synthetic","repository":"other/project"}]),
        ),
        (
            "repositories",
            json!([{"source_id":"missing","repository":"synthetic/fixture"}]),
        ),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        let refused = run(&invalid);
        assert!(!refused.status.success(), "accepted invalid {field}");
        assert!(String::from_utf8_lossy(&refused.stderr).contains("Metadata"));
    }
    c.change_registry(|v| v["sources"][0]["rights"]["status"] = json!("unreviewed"));
    let mut unadmitted = valid;
    unadmitted["registry_sha256"] = json!(hash(&fs::read(&c.registry).unwrap()));
    assert!(!run(&unadmitted).status.success());
}

#[test]
fn direct_library_callers_cannot_enable_network_for_an_offline_build() {
    use azeroth_cli::source_pipeline::{self, Options};
    let c = Case::new();
    let options = Options {
        registry: c.registry.clone(),
        cache: c.cache.clone(),
        plan: Some(c.plan.clone()),
        output: Some(c.root.join("out")),
        fetch: false,
        updates_policy: Some(c.root.join("must-not-read-or-fetch.json")),
    };
    assert_eq!(
        source_pipeline::build(&options).err().unwrap(),
        "Source builds never permit network acquisition"
    );
    assert_eq!(
        source_pipeline::sync(&options).err().unwrap(),
        "Metadata requests require explicit network acquisition"
    );
    assert!(!c.root.join("out").exists());
}
