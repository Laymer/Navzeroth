use super::*;
use crate::release::bundle_test_fixture as shared;
#[test]
fn exact_resources_stage_atomically_and_existing_or_failed_outputs_are_preserved() {
    for change in [
        "none",
        "altered",
        "existing",
        "wrong-identity",
        "bad-parent",
        "traversal",
    ] {
        let f = shared::Fixture::new();
        let binding = release::BundledContentEvidence {
            manifest: shared::reference(&f.root, "manifest.json"),
            receipt: shared::reference(&f.root, "receipt.json"),
            review: f.manifest.review.clone(),
        };
        let mut identity = f.payload.clone();
        let mut output = f.root.join("assembled");
        match change {
            "altered" => fs::write(f.root.join("review.json"), b"altered").unwrap(),
            "existing" => {
                fs::create_dir(&output).unwrap();
                fs::write(output.join("keep.txt"), b"existing user data").unwrap();
            }
            "wrong-identity" => {
                identity.production_sha256 =
                    azeroth_core::types::Sha256Hex::new("f".repeat(64)).unwrap()
            }
            "bad-parent" => output = f.root.join("absent/output"),
            "traversal" => output = f.root.join("absent/../output"),
            _ => {}
        }
        let result = stage_verified(
            &f.root,
            &output,
            &binding,
            &identity,
            &f.manifest,
            &f.policy,
        );
        assert_eq!(result.is_ok(), change == "none", "{change}");
        if change == "none" {
            assert_eq!(
                fs::read(output.join("review.json")).unwrap(),
                fs::read(f.root.join("review.json")).unwrap()
            );
        } else if change == "existing" {
            assert_eq!(
                fs::read(output.join("keep.txt")).unwrap(),
                b"existing user data"
            );
        } else {
            assert!(!output.exists());
        }
        assert!(!fs::read_dir(&f.root).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".bundle-stage-")
        }));
    }
}
#[test]
fn bounded_stream_copy_never_overwrites_and_removes_only_its_failed_creation() {
    let f = shared::Fixture::new();
    let source = f.root.join("review.json");
    let destination = f.root.join("copied.json");
    let pin = shared::reference(&f.root, "review.json");
    assert!(resources::copy(&source, &destination, pin.bytes, pin.sha256.as_str()).is_ok());
    assert_eq!(fs::read(&source).unwrap(), fs::read(&destination).unwrap());
    assert!(resources::copy(&source, &destination, pin.bytes, pin.sha256.as_str()).is_err());
    assert!(destination.exists());
    fs::remove_file(&destination).unwrap();
    assert!(resources::copy(&source, &destination, pin.bytes, "wrong").is_err());
    assert!(!destination.exists());
    assert!(resources::copy(&source, &destination, 0, "wrong").is_err());
    assert!(
        resources::copy(
            &source,
            &f.root.join("absent/out"),
            pin.bytes,
            pin.sha256.as_str()
        )
        .is_err()
    );
}

#[test]
fn assembler_requires_exact_build_identity_and_rechecks_manifest_before_copying() {
    for change in [
        "none",
        "wrong-identity",
        "changed-manifest",
        "invalid-manifest",
    ] {
        let f = shared::Fixture::new();
        let mut binding = release::BundledContentEvidence {
            manifest: shared::reference(&f.root, "manifest.json"),
            receipt: shared::reference(&f.root, "receipt.json"),
            review: f.manifest.review.clone(),
        };
        let mut identity = f.payload.clone();
        if change == "wrong-identity" {
            identity.app_version = "1.0.1".into();
        }
        if change == "changed-manifest" || change == "invalid-manifest" {
            fs::write(f.root.join("manifest.json"), b"invalid").unwrap();
        }
        if change == "invalid-manifest" {
            binding.manifest = shared::reference(&f.root, "manifest.json");
        }
        let identity_path = f.root.with_extension("identity.json");
        fs::write(&identity_path, receipts::encode(&identity).unwrap()).unwrap();
        let expected = receipts::Expected {
            app_version: &f.payload.app_version,
            target: &f.payload.target,
            bundle_manifest_sha256: f.payload.bundle_manifest_sha256.as_str(),
            production_sha256: f.payload.production_sha256.as_str(),
            tested_source_commit: &f.payload.tested_source_commit,
            review: &f.payload.review,
        };
        let output = f.root.join("assembled");
        assert_eq!(
            assemble_verified(
                &f.root,
                &output,
                &binding,
                &identity_path,
                &expected,
                &f.policy
            )
            .is_ok(),
            change == "none",
            "{change}"
        );
        assert_eq!(output.exists(), change == "none");
        fs::remove_file(identity_path).unwrap();
    }
}
