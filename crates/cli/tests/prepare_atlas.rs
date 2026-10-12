use azeroth_cli::atlas::{self, Offset, Paths};
use image::RgbImage;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
}
fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("azeroth-atlas-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
/// A content root holding the synthetic poster and a manifest describing it.
fn content_root(name: &str, poster: &[u8], mutate: impl FnOnce(&mut Value)) -> (PathBuf, PathBuf) {
    let root = temp(name);
    let source_dir = root.join("atlas/synthetic-continent/source");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("synthetic-poster.png"), poster).unwrap();
    let mut manifest = json!({
        "atlas_manifest_version": 1,
        "image_id": "demo-image-1",
        "source": {"file": "atlas/synthetic-continent/source/synthetic-poster.png", "sha256": hex(poster),
            "size_bytes": poster.len(), "width_px": 160, "height_px": 120, "registry_source_id": "demo-source-1"},
        "frame": {"width_px": 64, "height_px": 96},
        "layers": [
            {"id": "roads", "kind": "roads_traced", "crop": {"x": 8, "y": 12, "width": 64, "height": 96}, "output": "atlas/synthetic-continent/layers/roads.png"},
            {"id": "flight", "kind": "flight_reference", "crop": {"x": 88, "y": 12, "width": 64, "height": 96}, "output": "atlas/synthetic-continent/layers/flight.png"}],
        "overview": {"id": "overview", "from_layer": "roads", "divisor": 2, "output": "atlas/synthetic-continent/layers/overview.png"},
        "alignment": {"reference_layer": "roads", "max_residual": 8},
        "limits": {"max_source_bytes": 1048576, "max_dimension_px": 512}
    });
    mutate(&mut manifest);
    let manifest_path = root.join("atlas/synthetic-continent/atlas.manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    (root, manifest_path)
}
fn run(root: &Path, manifest: &Path, extra: &[&str]) -> std::process::Output {
    cli()
        .arg("prepare-atlas")
        .arg("--manifest")
        .arg(manifest)
        .arg("--content-root")
        .arg(root)
        .args(extra)
        .output()
        .unwrap()
}
fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}
fn committed_poster() -> PathBuf {
    repo().join("content/demo/atlas/synthetic-continent/source/synthetic-poster.png")
}

#[test]
fn committed_synthetic_poster_matches_the_generator() {
    let committed = fs::read(committed_poster()).expect(
        "content/demo synthetic poster is committed; regenerate with: cargo test -p azeroth-cli write_synthetic_poster -- --ignored",
    );
    assert_eq!(
        committed,
        atlas::synthetic_poster_png(),
        "synthetic poster drifted from its generator"
    );
}
#[test]
#[ignore = "writes the committed synthetic poster"]
fn write_synthetic_poster() {
    fs::create_dir_all(committed_poster().parent().unwrap()).unwrap();
    fs::write(committed_poster(), atlas::synthetic_poster_png()).unwrap();
}

#[test]
fn prepares_layers_deterministically_with_hashes_and_alignment() {
    let poster = atlas::synthetic_poster_png();
    let (root, manifest) = content_root("ok", &poster, |_| {});
    let out_a = temp("out-a");
    let out_b = temp("out-b");
    for out in [&out_a, &out_b] {
        let output = run(&root, &manifest, &["--output", out.to_str().unwrap()]);
        assert!(output.status.success(), "{}", stderr(&output));
    }
    let generated: Value = serde_json::from_str(
        &fs::read_to_string(
            out_a
                .join("atlas/synthetic-continent")
                .join(atlas::GENERATED_FILE),
        )
        .unwrap(),
    )
    .unwrap();
    let layers = generated["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 3);
    assert_eq!(
        (
            layers[0]["width_px"].as_u64(),
            layers[0]["height_px"].as_u64()
        ),
        (Some(64), Some(96))
    );
    assert_eq!(
        (
            layers[2]["width_px"].as_u64(),
            layers[2]["frame_divisor"].as_u64()
        ),
        (Some(32), Some(2))
    );
    assert!(generated.get("generated_at").is_none(), "no timestamps");
    for layer in layers {
        let path = out_a.join(layer["file"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap();
        assert_eq!(hex(&bytes), layer["sha256"], "{}", path.display());
        assert_eq!(bytes.len() as u64, layer["size_bytes"].as_u64().unwrap());
        assert_eq!(
            bytes,
            fs::read(out_b.join(layer["file"].as_str().unwrap())).unwrap(),
            "deterministic"
        );
    }
    let alignment = generated["alignment"].as_array().unwrap();
    assert_eq!(alignment[1]["offset_px"], json!({"x": 0, "y": 0}));
    assert!(alignment[1]["residual"].as_f64().unwrap() < 8.);
    let record_a = fs::read(
        out_a
            .join("atlas/synthetic-continent")
            .join(atlas::GENERATED_FILE),
    )
    .unwrap();
    let record_b = fs::read(
        out_b
            .join("atlas/synthetic-continent")
            .join(atlas::GENERATED_FILE),
    )
    .unwrap();
    assert_eq!(record_a, record_b);
    // Without --output the outputs land under the content root itself.
    let output = run(&root, &manifest, &[]);
    assert!(output.status.success());
    assert!(
        root.join("atlas/synthetic-continent/layers/overview.png")
            .exists()
    );
    assert_eq!(
        fs::read(
            root.join("atlas/synthetic-continent")
                .join(atlas::GENERATED_FILE)
        )
        .unwrap(),
        record_a
    );
}

#[test]
fn odd_frames_round_the_overview_up() {
    let poster = atlas::synthetic_poster_png();
    let (root, manifest) = content_root("odd", &poster, |m| {
        m["frame"] = json!({"width_px": 63, "height_px": 95});
        for layer in m["layers"].as_array_mut().unwrap() {
            layer["crop"]["width"] = json!(63);
            layer["crop"]["height"] = json!(95);
        }
    });
    let output = run(&root, &manifest, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let generated: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("atlas/synthetic-continent")
                .join(atlas::GENERATED_FILE),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        (
            generated["layers"][2]["width_px"].as_u64(),
            generated["layers"][2]["height_px"].as_u64()
        ),
        (Some(32), Some(48))
    );
}

#[test]
fn rejects_mismatched_sources_and_crops() {
    let poster = atlas::synthetic_poster_png();
    type Case = (&'static str, Box<dyn FnOnce(&mut Value)>, &'static str);
    let cases: Vec<Case> = vec![
        (
            "hash",
            Box::new(|m| m["source"]["sha256"] = json!("0".repeat(64))),
            "digest",
        ),
        (
            "size",
            Box::new(|m| m["source"]["size_bytes"] = json!(1)),
            "size",
        ),
        (
            "dims",
            Box::new(|m| m["source"]["width_px"] = json!(161)),
            "dimensions differ",
        ),
        (
            "outside",
            Box::new(|m| m["layers"][1]["crop"]["x"] = json!(100)),
            "outside the source",
        ),
        (
            "unequal",
            Box::new(|m| m["layers"][1]["crop"]["width"] = json!(60)),
            "differs from the frame",
        ),
        (
            "caps",
            Box::new(|m| m["limits"]["max_dimension_px"] = json!(9000)),
            "hard caps",
        ),
        (
            "unknown",
            Box::new(|m| m["tiles"] = json!({})),
            "not a valid atlas manifest",
        ),
        (
            "shifted",
            Box::new(|m| m["layers"][1]["crop"]["x"] = json!(90)),
            "offset by (-2, 0)",
        ),
    ];
    for (name, mutate, expected) in cases {
        let (root, manifest) = content_root(name, &poster, mutate);
        let output = run(&root, &manifest, &[]);
        assert_eq!(output.status.code(), Some(1), "{name}");
        let message = stderr(&output);
        assert!(message.contains(expected), "{name}: {message}");
    }
    // Not a PNG at all.
    let (root, manifest) = content_root("notpng", b"not a png at all", |_| {});
    let output = run(&root, &manifest, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("not a decodable PNG"));
}

#[test]
fn lfs_pointers_are_refused_in_prepare_and_required_in_check_mode() {
    let poster = atlas::synthetic_poster_png();
    let pointer = |sha: &str, size: usize| {
        format!("version https://git-lfs.github.com/spec/v1\noid sha256:{sha}\nsize {size}\n")
    };
    let (root, manifest) = content_root("pointer", &poster, |_| {});
    let source = root.join("atlas/synthetic-continent/source/synthetic-poster.png");
    // Materialized source: prepare works, check mode refuses the raw blob.
    let output = run(&root, &manifest, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let output = run(&root, &manifest, &["--check-pointer"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("not a Git LFS pointer"));
    // Pointer source: prepare refuses with the materialization hint.
    fs::write(&source, pointer(&hex(&poster), poster.len())).unwrap();
    let output = run(&root, &manifest, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("not materialized"));
    // Pointers everywhere with matching oids: check mode passes.
    let generated: Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("atlas/synthetic-continent")
                .join(atlas::GENERATED_FILE),
        )
        .unwrap(),
    )
    .unwrap();
    for layer in generated["layers"].as_array().unwrap() {
        let path = root.join(layer["file"].as_str().unwrap());
        fs::write(
            &path,
            pointer(
                layer["sha256"].as_str().unwrap(),
                layer["size_bytes"].as_u64().unwrap() as usize,
            ),
        )
        .unwrap();
    }
    let output = run(&root, &manifest, &["--check-pointer"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Checked 4")
    );
    // A pointer with another oid fails.
    fs::write(&source, pointer(&"1".repeat(64), poster.len())).unwrap();
    let output = run(&root, &manifest, &["--check-pointer"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("does not match"));
}

#[test]
fn alignment_measurement_finds_integer_shifts() {
    let poster = atlas::synthetic_poster();
    let reference = image::imageops::crop_imm(&poster, 8, 12, 64, 96).to_image();
    let shifted = image::imageops::crop_imm(&poster, 91, 10, 64, 96).to_image();
    let (offset, residual) = atlas::measure_alignment(&reference, &shifted);
    assert_eq!(offset, Offset { x: -3, y: 2 });
    assert!(residual < 8., "{residual}");
    let unrelated = RgbImage::from_fn(64, 96, |x, y| {
        image::Rgb([(x * 7 % 251) as u8, (y * 5 % 251) as u8, 0])
    });
    let (_, residual) = atlas::measure_alignment(&reference, &unrelated);
    assert!(residual > 8., "{residual}");
}

#[test]
fn symlinked_assets_are_refused() {
    #[cfg(unix)]
    {
        let poster = atlas::synthetic_poster_png();
        let (root, manifest) = content_root("symlink", &poster, |_| {});
        let real = root.join("atlas/synthetic-continent/source/synthetic-poster.png");
        let elsewhere = temp("elsewhere").join("poster.png");
        fs::rename(&real, &elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &real).unwrap();
        let output = run(&root, &manifest, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("symbolic link"));
        let _ = Paths {
            manifest: &manifest,
            content_root: &root,
            output: None,
        };
    }
}
