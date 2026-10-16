use azeroth_core::local_data::{
    FORMAT_VERSION, MetadataBundle, MetadataObject, MetadataRole, PerformancePreset,
    TableDefinition, WOWDBDEFS_REVISION,
};
use azeroth_local_data::{MetadataPolicy, inspect, plan, verify_metadata};
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
        let dir = std::env::temp_dir().join(format!(
            "navzeroth-local-data-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn directory(&self, name: &str) -> PathBuf {
        let dir = self.0.join(name);
        fs::create_dir(&dir).unwrap();
        dir
    }
    fn installation(&self) -> PathBuf {
        let dir = self.directory("Client ü 测试");
        fs::write(dir.join(".build.info"), build_info()).unwrap();
        dir
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn build_info() -> String {
    format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70425|{}|{}|enUS text\n",
        "a".repeat(32),
        "b".repeat(32)
    )
}

#[test]
fn unicode_inspection_is_repeatable_read_only_and_sanitized() {
    let temp = Temp::new();
    let client = temp.installation();
    // Excluded files are never needed for metadata inspection.
    fs::create_dir(client.join("WTF")).unwrap();
    fs::write(
        client.join("WTF/private.lua"),
        b"personal export must stay untouched",
    )
    .unwrap();
    let first = inspect(&client, "wow_classic_beta").unwrap();
    let second = inspect(&client, "wow_classic_beta").unwrap();
    let first = serde_json::to_string(&first).unwrap();
    assert_eq!(first, serde_json::to_string(&second).unwrap());
    assert!(!first.contains(&temp.0.to_string_lossy().to_string()));
    assert!(!first.contains("private.lua"));
    assert_eq!(
        fs::read_to_string(client.join(".build.info")).unwrap(),
        build_info()
    );
    assert_eq!(
        fs::read(client.join("WTF/private.lua")).unwrap(),
        b"personal export must stay untouched"
    );
    assert_eq!(fs::read_dir(&client).unwrap().count(), 2);
}

#[test]
fn preliminary_plan_refuses_related_destinations_and_preserves_unknown_estimates() {
    let temp = Temp::new();
    let client = temp.installation();
    let destination = temp.directory("Private catalog");
    let result = plan(
        &client,
        "wow_classic_beta",
        &destination,
        PerformancePreset::Small,
    )
    .unwrap();
    assert!(!result.extraction_ready);
    assert!(!result.activation_automatic);
    assert_eq!(result.actual_workers, 0);
    assert_eq!(result.required_storage_bytes, None);
    assert_eq!(result.available_storage_bytes, None);
    assert_eq!(result.limits.workers, 1);
    assert!(!result.blockers.is_empty());
    assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
    for bad in [&temp.0, &client, &client.join("missing")] {
        assert!(plan(&client, "wow_classic_beta", bad, PerformancePreset::Max).is_err());
    }
    assert!(inspect(&client, "wow").is_err());
    assert!(inspect(&client.join(".build.info"), "wow_classic_beta").is_err());
}

fn metadata_fixture(temp: &Temp) -> (PathBuf, PathBuf, String) {
    let objects = temp.directory("objects");
    let definition = b"synthetic definition, not client data";
    let license = b"synthetic test license, not upstream admission";
    let definition_hash = digest(definition);
    let license_hash = digest(license);
    fs::write(objects.join(&definition_hash), definition).unwrap();
    fs::write(objects.join(&license_hash), license).unwrap();
    let bundle = MetadataBundle {
        format_version: FORMAT_VERSION,
        wowdbdefs_revision: WOWDBDEFS_REVISION.into(),
        objects: vec![
            MetadataObject {
                sha256: definition_hash.clone(),
                bytes: definition.len() as u64,
                role: MetadataRole::Definition,
            },
            MetadataObject {
                sha256: license_hash.clone(),
                bytes: license.len() as u64,
                role: MetadataRole::License,
            },
        ],
        tables: vec![TableDefinition {
            table: "Synthetic".into(),
            file_data_id: 123,
            product: "wow_classic_beta".into(),
            build: 70425,
            layout_hash: "0123abcd".into(),
            definition_sha256: definition_hash,
            license_sha256: license_hash,
            transform: "test_only_v1".into(),
        }],
    };
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let hash = digest(&bytes);
    let file = temp.0.join("bundle.json");
    fs::write(&file, bytes).unwrap();
    (file, objects, hash)
}

#[test]
fn metadata_verification_checks_reviewed_policy_every_object_and_compiled_transforms() {
    let temp = Temp::new();
    let (bundle, objects, hash) = metadata_fixture(&temp);
    let policy = MetadataPolicy {
        reviewed_bundle_sha256: &hash,
        implemented_transforms: &["test_only_v1"],
    };
    let verified = verify_metadata(&bundle, &objects, &policy).unwrap();
    assert_eq!(verified.tables[0].build, 70425);
    let no_transforms = MetadataPolicy {
        reviewed_bundle_sha256: &hash,
        implemented_transforms: &[],
    };
    assert!(verify_metadata(&bundle, &objects, &no_transforms).is_err());
    let untrusted = MetadataPolicy {
        reviewed_bundle_sha256: &"0".repeat(64),
        implemented_transforms: &["test_only_v1"],
    };
    assert!(verify_metadata(&bundle, &objects, &untrusted).is_err());
    let object = objects.join(&verified.objects[0].sha256);
    let original = fs::read(&object).unwrap();
    let mut changed = original.clone();
    changed[0] ^= 1;
    fs::write(&object, changed).unwrap();
    assert!(verify_metadata(&bundle, &objects, &policy).is_err());
    fs::write(&object, original).unwrap();
    fs::remove_file(&object).unwrap();
    assert!(verify_metadata(&bundle, &objects, &policy).is_err());
    fs::write(object, b"too short").unwrap();
    assert!(verify_metadata(&bundle, &objects, &policy).is_err());
}

#[cfg(unix)]
#[test]
fn symlink_installation_ancestors_and_metadata_objects_are_rejected() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new();
    let client = temp.installation();
    let link = temp.0.join("alias");
    symlink(&client, &link).unwrap();
    assert!(inspect(&link, "wow_classic_beta").is_err());
    assert!(inspect(&temp.0.join("alias/.."), "wow_classic_beta").is_err());
    let real = temp.0.join("real-build-info");
    fs::rename(client.join(".build.info"), &real).unwrap();
    symlink(real, client.join(".build.info")).unwrap();
    assert!(inspect(&client, "wow_classic_beta").is_err());
    let (bundle, objects, hash) = metadata_fixture(&temp);
    let policy = MetadataPolicy {
        reviewed_bundle_sha256: &hash,
        implemented_transforms: &["test_only_v1"],
    };
    let object = fs::read_dir(&objects)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let outside = temp.0.join("outside-object");
    fs::rename(&object, &outside).unwrap();
    symlink(outside, object).unwrap();
    assert!(verify_metadata(&bundle, &objects, &policy).is_err());
}

#[test]
fn oversized_nonregular_and_missing_metadata_fail_with_safe_errors() {
    let temp = Temp::new();
    let client = temp.installation();
    let file = client.join(".build.info");
    let handle = fs::OpenOptions::new().write(true).open(&file).unwrap();
    handle
        .set_len(azeroth_core::local_data::MAX_BUILD_INFO_BYTES as u64 + 1)
        .unwrap();
    let error = inspect(&client, "wow_classic_beta").unwrap_err();
    assert!(!error.contains(&temp.0.to_string_lossy().to_string()));
    drop(handle);
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    assert!(inspect(&client, "wow_classic_beta").is_err());
    assert!(inspect(Path::new(""), "wow_classic_beta").is_err());
}
