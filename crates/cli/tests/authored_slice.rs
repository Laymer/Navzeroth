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
    assert_eq!(matrix.schema_version, 2);
    assert_eq!(matrix.target, None);
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
