//! Installed resources retain source states and fail closed on mutation.
use azeroth_app::{bundled_content as bundle, snapshots::SnapshotApplication};
use azeroth_core::types::AssetPath;
use azeroth_local_data::bundled_receipts as receipts;
use serde_json::{Value, json};
use std::fs;
#[path = "support/bundled_content_fixture.rs"]
mod support;
use support::*;
#[test]
fn exact_signed_resources_activate_before_import_and_survive_repeated_startup() {
    let fixture = Fixture::new();
    assert!(fixture.verify());
    let app = SnapshotApplication::in_memory().unwrap();
    assert!(fixture.install(&app));
    assert!(fixture.install(&app));
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    snapshot["fixture"] = json!(false);
    snapshot["provenance"]["source"] = json!("addon_snapshot");
    snapshot["producer"]["component"] = json!("addon");
    snapshot["namespace"] =
        serde_json::to_value(fixture.manifest.packs[0].key.scope.context.namespace).unwrap();
    snapshot["game"] =
        json!({"edition":"wow_forever","build":"1.60.1.70450","interface":"16001","locale":"enUS"});
    fn retarget(value: &mut Value, context: &Value) {
        match value {
            Value::Object(object) => {
                for (name, value) in object {
                    if name == "source" && value == "synthetic_fixture" {
                        *value = json!("addon_snapshot");
                    } else if name == "context" {
                        *value = context.clone();
                    } else {
                        retarget(value, context);
                    }
                }
            }
            Value::Array(array) => {
                for value in array {
                    retarget(value, context);
                }
            }
            _ => {}
        }
    }
    retarget(
        &mut snapshot,
        &serde_json::to_value(&fixture.manifest.packs[0].key.scope.context).unwrap(),
    );
    let id = app
        .import_text(&format!("ANAV1:{snapshot}"), None)
        .unwrap()
        .profile_id;
    let active = app.active_content(&id).unwrap().unwrap();
    assert_eq!(active.accepts_inferred_travel, Some(true));
    assert_eq!(active.bundle_defaults.unwrap().app_version, "1.0.0");
    fs::write(fixture.root.join("review.json"), b"tampered").unwrap();
    assert!(!fixture.install(&app));
    assert_eq!(app.active_content(&id).unwrap().unwrap().key, active.key);
}
#[test]
fn altered_missing_extra_and_linked_resources_fail_closed() {
    for change in [
        "alter",
        "missing",
        "extra",
        "directory",
        "receipt",
        "manifest",
    ] {
        let fixture = Fixture::new();
        match change {
            "alter" => fs::write(fixture.root.join("review.json"), b"different").unwrap(),
            "missing" => fs::remove_file(fixture.root.join("review.json")).unwrap(),
            "extra" => fs::write(fixture.root.join("unexpected.json"), b"extra").unwrap(),
            "directory" => {
                fs::remove_file(fixture.root.join("review.json")).unwrap();
                fs::create_dir(fixture.root.join("review.json")).unwrap();
            }
            "receipt" => fs::write(fixture.root.join("receipt.json"), b"{}").unwrap(),
            _ => fs::write(fixture.root.join("manifest.json"), b"{}").unwrap(),
        }
        assert!(!fixture.verify(), "{change}");
    }
    #[cfg(unix)]
    {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink("review.json", fixture.root.join("linked.json")).unwrap();
        assert!(!fixture.verify());
    }
}
#[test]
fn signed_but_incompatible_manifests_cannot_activate() {
    for change in [
        "format",
        "version",
        "target",
        "empty",
        "duplicate",
        "missing-pin",
        "oversize",
        "reserved",
        "scope",
        "fixture",
        "revision",
        "directory",
        "conflicts",
    ] {
        let mut f = Fixture::new();
        match change {
            "format" => f.manifest.format = "wrong".into(),
            "version" => f.manifest.schema_version = 2,
            "target" => f.manifest.target.build = "70374".into(),
            "empty" => f.manifest.packs.clear(),
            "duplicate" => f.manifest.files.push(f.manifest.files[0].clone()),
            "missing-pin" => f
                .manifest
                .files
                .retain(|r| r.path.as_str() != "review.json"),
            "oversize" => f.manifest.files[0].bytes = bundle::MAX_TOTAL_BYTES + 1,
            "reserved" => f.manifest.files[0].path = AssetPath::new("manifest.json").unwrap(),
            "scope" => {
                f.manifest.packs[0].key.scope.context.build.build =
                    azeroth_core::types::BuildId::new("1.60.1.70374").unwrap()
            }
            "fixture" => f.manifest.packs[0].key.scope.fixture = true,
            "revision" => {
                f.manifest.packs[0].key.revision =
                    azeroth_core::types::DatasetRevision::new("wrong-revision").unwrap()
            }
            "directory" => f.manifest.packs[0].directory = AssetPath::new("different").unwrap(),
            _ => {
                fs::write(f.root.join("conflicts.json"), b"[{}]").unwrap();
                let pin = reference(&f.root, "conflicts.json");
                f.manifest
                    .files
                    .iter_mut()
                    .filter(|r| r.path == pin.path)
                    .for_each(|r| *r = pin.clone());
                f.manifest.packs[0].conflicts = Some(pin);
            }
        }
        f.sign();
        assert!(
            !f.install(&SnapshotApplication::in_memory().unwrap()),
            "{change}"
        );
    }
}
#[test]
fn resource_streams_accept_large_documents_and_refuse_wrong_hash_size_type_and_links() {
    use azeroth_local_data::bundled_resources as resources;
    let fixture = Fixture::new();
    let path = fixture.root.join("stream.bin");
    let raw = vec![7; 65537];
    fs::write(&path, &raw).unwrap();
    assert_eq!(resources::read(&path, raw.len()).unwrap(), raw);
    assert!(resources::verify(&path, raw.len() as u64, hash(&raw).as_str()).is_ok());
    assert!(resources::verify(&path, raw.len() as u64 - 1, hash(&raw).as_str()).is_err());
    assert!(resources::verify(&path, raw.len() as u64 + 1, hash(&raw).as_str()).is_err());
    assert!(resources::verify(&path, raw.len() as u64, hash(b"wrong").as_str()).is_err());
    assert!(resources::verify(&path, 0, "").is_err());
    assert!(resources::verify(&path, resources::MAX_RESOURCE_BYTES + 1, "").is_err());
    assert!(resources::read(&path, 1).is_err());
    assert!(resources::read(&path, resources::MAX_DOCUMENT_BYTES + 1).is_err());
    assert!(resources::read(&fixture.root, 10).is_err());
    #[cfg(unix)]
    {
        let linked = fixture.root.join("link.bin");
        std::os::unix::fs::symlink(&path, &linked).unwrap();
        assert!(resources::read(&linked, raw.len()).is_err());
    }
}

#[test]
fn native_startup_requires_build_pins_and_publishes_only_after_verified_activation() {
    let f = Fixture::new();
    let database = f.root.with_extension("sqlite3");
    let identity = receipts::encode(&f.payload).unwrap();
    let policy = receipts::encode(&f.policy).unwrap();
    let host = bundle::start(&database, &f.root, "1.0.0", Some(&identity), &policy).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let status = host.status().unwrap();
        if status.state != "checking" {
            assert_eq!(status.state, "ready", "{:?}", status.error);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        host.with_application(|app| app.stage_content_set(&[f.manifest.packs[0].key.clone()]))
            .is_ok()
    );
    assert!(bundle::start(&database, &f.root, "1.0.0", None, &policy).is_err());
    assert!(bundle::start(&database, &f.root, "1.0.1", None, &policy).is_err());
    assert!(bundle::start(&database, &f.root, "invalid", None, &policy).is_err());
    assert!(bundle::start(&database, &f.root, "1.0.0", Some(b"{}"), &policy).is_err());
    assert!(bundle::start(&database, &f.root, "1.0.1", Some(&identity), &policy).is_err());
    assert!(bundle::start(&database, &f.root, "1.0.0", Some(&identity), b"{}").is_err());
    let compiled =
        receipts::policy(include_bytes!("../../../content/bundled/signers-v1.json")).unwrap();
    assert!(compiled.operator_controls_both_keys);
    drop(host);
    let developer = bundle::start(&database, &f.root, "1.0.0-rc.1", None, &policy).unwrap();
    drop(developer);
    let _ = fs::remove_file(database);
}

#[test]
fn normalization_conflicts_remain_distinct_and_bind_the_catalog_revision_and_precedence() {
    let mut f = Fixture::new();
    let origins: Value =
        serde_json::from_slice(&fs::read(f.root.join("accepted-origins.json")).unwrap()).unwrap();
    let mut conflicts = json!({"format":"navzeroth-world-content-conflicts","schema_version":1,
        "revision":f.manifest.packs[0].key.revision,"precedence":origins["precedence"],
        "conflicts":[{"kind":"value_disagreement","fact":"synthetic:source-choice","chosen":{"origin":"authored","value":"Synthetic"},"rejected":[],"rule":"Synthetic example, not an admitted game fact"}]});
    for valid in [true, false] {
        if !valid {
            conflicts["revision"] = json!("wrong-revision");
        }
        fs::write(
            f.root.join("normalization-conflicts.json"),
            serde_json::to_vec(&conflicts).unwrap(),
        )
        .unwrap();
        let pin = reference(&f.root, "normalization-conflicts.json");
        f.manifest.files.retain(|r| r.path != pin.path);
        f.manifest.files.push(pin.clone());
        f.manifest.packs[0].normalization_conflicts = Some(pin);
        f.sign();
        assert_eq!(f.install(&SnapshotApplication::in_memory().unwrap()), valid);
    }
    assert!(
        fs::read_to_string(f.root.join("normalization-conflicts.json"))
            .unwrap()
            .contains("synthetic:source-choice")
    );
}

#[test]
fn signed_content_with_remaining_policy_gaps_and_excessive_tree_depth_is_refused() {
    let mut f = Fixture::new();
    let mut origins: Value =
        serde_json::from_slice(&fs::read(f.root.join("accepted-origins.json")).unwrap()).unwrap();
    origins["accept_unknowns"] = json!(false);
    fs::write(
        f.root.join("accepted-origins.json"),
        serde_json::to_vec(&origins).unwrap(),
    )
    .unwrap();
    let pin = reference(&f.root, "accepted-origins.json");
    f.manifest
        .files
        .iter_mut()
        .filter(|r| r.path == pin.path)
        .for_each(|r| *r = pin.clone());
    f.manifest.accepted_origins = pin;
    f.sign();
    assert!(f.verify());
    assert!(!f.install(&SnapshotApplication::in_memory().unwrap()));
    let mut directory = f.root.clone();
    for _ in 0..18 {
        directory.push("deep");
        fs::create_dir(&directory).unwrap();
    }
    assert!(!f.verify());
}
