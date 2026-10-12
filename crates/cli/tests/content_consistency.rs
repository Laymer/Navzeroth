//! Committed content stays consistent with its generated atlas records and the
//! files on disk, without needing Git LFS objects: pointers are checked by oid.
use azeroth_core::{contract::*, types::AssetPath};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn resolve(root: &Path, asset: &AssetPath) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in asset.segments() {
        path.push(segment);
        let meta =
            fs::symlink_metadata(&path).unwrap_or_else(|_| panic!("{} exists", path.display()));
        assert!(
            !meta.file_type().is_symlink(),
            "{} is a symlink",
            path.display()
        );
    }
    path
}
/// Digest of a materialized file, or the oid of a Git LFS pointer.
fn digest_or_oid(path: &Path) -> (String, u64) {
    let bytes = fs::read(path).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap_or("");
    if text.starts_with("version https://git-lfs.github.com/spec/v1") {
        let oid = text
            .lines()
            .find_map(|l| l.strip_prefix("oid sha256:"))
            .unwrap()
            .to_owned();
        let size = text
            .lines()
            .find_map(|l| l.strip_prefix("size "))
            .unwrap()
            .parse()
            .unwrap();
        (oid, size)
    } else {
        (
            Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            bytes.len() as u64,
        )
    }
}
fn datasets() -> Vec<(PathBuf, PathBuf)> {
    let mut list = vec![(
        repo().join("fixtures/contracts/v1/reference-dataset-imagery.json"),
        repo().join("content/demo"),
    )];
    let forever = repo().join("content/forever/datasets");
    let mut files: Vec<_> = fs::read_dir(&forever)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "the forever atlas dataset is committed");
    list.extend(
        files
            .into_iter()
            .map(|f| (f, repo().join("content/forever"))),
    );
    list
}

#[test]
fn datasets_match_generated_records_and_files() {
    for (dataset_path, root) in datasets() {
        let text = fs::read_to_string(&dataset_path).unwrap();
        let envelope = decode_envelope::<ReferenceDataset>(&text)
            .unwrap_or_else(|e| panic!("{}: {e:?}", dataset_path.display()));
        let build = envelope.data_context().build;
        assert!(
            !envelope.payload.map_images.is_empty(),
            "{}: map images decoded",
            dataset_path.display()
        );
        for image in &envelope.payload.map_images {
            assert!(image.applies_to_build(&build));
            let first = resolve(&root, &image.layers[0].file);
            let generated_path = first
                .ancestors()
                .map(|a| a.join(azeroth_cli::atlas::GENERATED_FILE))
                .find(|p| p.exists())
                .unwrap_or_else(|| panic!("generated record near {}", first.display()));
            let generated: azeroth_cli::atlas::Generated =
                serde_json::from_str(&fs::read_to_string(&generated_path).unwrap()).unwrap();
            assert_eq!(generated.image_id, image.id);
            assert_eq!(generated.frame.width_px, image.width_px);
            assert_eq!(generated.frame.height_px, image.height_px);
            assert_eq!(
                generated.layers.len(),
                image.layers.len(),
                "{}",
                image.id.as_str()
            );
            for layer in &image.layers {
                let record = generated
                    .layers
                    .iter()
                    .find(|g| g.file == layer.file)
                    .unwrap_or_else(|| {
                        panic!("{} listed in the generated record", layer.file.as_str())
                    });
                assert_eq!(
                    (
                        record.sha256.as_str(),
                        record.width_px,
                        record.height_px,
                        record.frame_divisor
                    ),
                    (
                        layer.sha256.as_str(),
                        layer.width_px.get(),
                        layer.height_px.get(),
                        layer.frame_divisor.get()
                    ),
                    "{}",
                    layer.file.as_str()
                );
                let (digest, size) = digest_or_oid(&resolve(&root, &layer.file));
                assert_eq!(
                    digest,
                    layer.sha256.as_str(),
                    "{} digest",
                    layer.file.as_str()
                );
                assert_eq!(size, record.size_bytes, "{} size", layer.file.as_str());
            }
            // The source itself must be present as bytes or as a pointer with the declared oid.
            let (digest, size) = digest_or_oid(&resolve(&root, &generated.source.file));
            assert_eq!(
                (digest, size),
                (
                    generated.source.sha256.as_str().to_owned(),
                    generated.source.size_bytes
                )
            );
        }
    }
}
