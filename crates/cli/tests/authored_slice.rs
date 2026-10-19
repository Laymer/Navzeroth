//! V2.4: the committed authored slice input builds into a v2 pack that verifies,
//! with a schema 2 party matrix for solo and duo bound to the authored records.
//! Building a pack never activates it for planning.
use azeroth_cli::pack::{v2::build, verify};
use azeroth_core::release_party::{SupportMatrix, workflows_for};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn input() -> PathBuf {
    // The pack builder refuses parent traversal, so resolve the repository path.
    // On Windows this yields a verbatim `\\?\C:\...` path, which the builder must
    // accept: a user can select one too.
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/authored/slice-v1"))
        .unwrap()
}
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn temp(name: &str) -> Temp {
    let path = std::env::temp_dir().join(format!("anav-authored-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();
    Temp(path)
}
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn authored_slice_builds_and_verifies_with_solo_and_duo_support() {
    let t = temp("build");
    let output = t.0.join("pack");
    let report = build(&input(), &output).unwrap();
    assert_eq!(report.fixtures, 0);
    assert_eq!(report.datasets, 1);
    // Unknown game facts stay counted gaps; authoring never fills them.
    assert!(report.coverage_gaps > 0);
    let verified = verify(&output).unwrap();
    assert_eq!(verified.coverage_gaps, report.coverage_gaps);
    assert_eq!(verified.total_bytes, report.total_bytes);

    // The committed inputs are already canonical, so the pack holds the same bytes.
    for file in ["catalog.json", "sources.json", "party-support.json"] {
        assert_eq!(
            fs::read(output.join(file)).unwrap(),
            fs::read(input().join(file)).unwrap(),
            "{file} is not canonical; rebuild and copy the pack's file back"
        );
    }
    let catalog = fs::read(input().join("catalog.json")).unwrap();
    let matrix =
        SupportMatrix::decode(&fs::read(input().join("party-support.json")).unwrap()).unwrap();
    assert_eq!(
        matrix.catalog_sha256.as_str(),
        format!("{:x}", Sha256::digest(&catalog))
    );
    assert_eq!(matrix.schema_version, 3);
    assert_eq!(
        matrix.target,
        Some(
            azeroth_core::release::decode_world_data_target(
                &fs::read(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../release/v1/world-data-target.json")
                )
                .unwrap()
            )
            .unwrap()
        )
    );
    assert_eq!(matrix.supported_sizes, [1, 2]);
    let mut expected: Vec<_> = [1u8, 2]
        .into_iter()
        .flat_map(|s| workflows_for(s).map(move |w| (s, w)))
        .collect();
    let mut actual: Vec<_> = matrix.rows.iter().map(|r| (r.size, r.workflow)).collect();
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
    assert_eq!(matrix.rows.len(), 12);
}

#[test]
fn edited_procedures_or_catalog_refuse_to_build() {
    let t = temp("tamper");
    let changed = t.0.join("procedure");
    copy(&input(), &changed);
    let path = changed.join("procedures/duo-agreement.md");
    let mut text = fs::read(&path).unwrap();
    text.extend_from_slice(b"\nUnreviewed addition.\n");
    fs::write(&path, text).unwrap();
    assert!(build(&changed, &t.0.join("out-1")).is_err());

    // A catalog change without a new party matrix no longer matches its hash.
    let revised = t.0.join("catalog");
    copy(&input(), &revised);
    let path = revised.join("catalog.json");
    let text = String::from_utf8(fs::read(&path).unwrap()).unwrap();
    fs::write(
        &path,
        text.replace("\"title\": \"The Deadmines\"", "\"title\": \"Deadmines\""),
    )
    .unwrap();
    assert!(build(&revised, &t.0.join("out-2")).is_err());
}

/// The same catalog with an invented game fact: a known (empty) reward list.
fn invented(catalog: &[u8]) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(catalog).unwrap();
    value["dungeons"][0]["rewards"] = serde_json::json!({"status": "known", "value": []});
    // Canonical pack bytes, as the builder writes them; the v2 graph still decodes.
    let content = azeroth_core::content_v2::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    let mut bytes = serde_json::to_vec_pretty(&serde_json::to_value(&content).unwrap()).unwrap();
    bytes.push(b'\n');
    bytes
}

#[test]
fn authored_rules_refuse_invented_game_facts_in_build_and_verify() {
    const REFUSAL: &str = "Authored content cannot declare a game fact";
    let t = temp("invented");
    let changed = t.0.join("input");
    copy(&input(), &changed);
    // Without the party matrix, so its catalog hash cannot be what refuses.
    fs::remove_file(changed.join("party-support.json")).unwrap();
    let clean = t.0.join("clean");
    build(&changed, &clean).unwrap();
    let catalog = changed.join("catalog.json");
    fs::write(&catalog, invented(&fs::read(&catalog).unwrap())).unwrap();
    assert_eq!(build(&changed, &t.0.join("refused")).err(), Some(REFUSAL));
    assert!(!t.0.join("refused").exists());

    // A pack whose manifest records the tampered catalog's true size and digest
    // (and the one fewer coverage gap) is refused by verify-pack too.
    let bytes = invented(&fs::read(clean.join("catalog.json")).unwrap());
    let manifest_path = clean.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["catalog"]["bytes"] = bytes.len().into();
    manifest["catalog"]["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    let gaps = manifest["coverage_gaps"].as_u64().unwrap();
    manifest["coverage_gaps"] = (gaps - 1).into();
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    manifest_bytes.push(b'\n');
    fs::write(clean.join("catalog.json"), &bytes).unwrap();
    fs::write(&manifest_path, manifest_bytes).unwrap();
    assert_eq!(verify(&clean).err(), Some(REFUSAL));

    // The CLI reports the same refusal for both commands.
    let cli = |args: &[&std::ffi::OsStr]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(args)
            .output()
            .unwrap()
    };
    let cli_out = t.0.join("cli-out");
    for output in [
        cli(&[
            "build-content-v2".as_ref(),
            "--input".as_ref(),
            changed.as_os_str(),
            "--output".as_ref(),
            cli_out.as_os_str(),
        ]),
        cli(&[
            "verify-pack".as_ref(),
            "--input".as_ref(),
            clean.as_os_str(),
        ]),
    ] {
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(REFUSAL));
    }
    assert!(!cli_out.exists());

    // A second, non-authored source cannot switch the authored rules off.
    let mixed = t.0.join("mixed");
    copy(&changed, &mixed);
    let sources_path = mixed.join("sources.json");
    let mut sources: serde_json::Value =
        serde_json::from_slice(&fs::read(&sources_path).unwrap()).unwrap();
    let mut other = sources["sources"][0].clone();
    other["id"] = "another-source".into();
    other["access"]["value"] = "manual_entry".into();
    sources["sources"].as_array_mut().unwrap().push(other);
    fs::write(&sources_path, serde_json::to_vec(&sources).unwrap()).unwrap();
    assert_eq!(
        build(&mixed, &t.0.join("mixed-out")).err(),
        Some("Authored pack must declare exactly one source")
    );
}
