#[path = "../build_support.rs"]
mod build_support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "nav-input-profile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let t = Self(root);
        for directory in [
            "crates/local-data/src",
            "crates/core/src/contract",
            "crates/ingest/src",
            "crates/storage/src",
        ] {
            fs::create_dir_all(t.0.join(directory)).unwrap();
        }
        for file in [
            "crates/local-data/build.rs",
            "crates/local-data/build_support.rs",
            "crates/local-data/Cargo.toml",
            "Cargo.lock",
            "Cargo.toml",
            "crates/core/Cargo.toml",
            "crates/ingest/Cargo.toml",
            "crates/storage/Cargo.toml",
            "crates/core/src/contract/snapshot.rs",
        ] {
            fs::write(t.0.join(file), "original\n").unwrap();
        }
        t
    }
    fn profile(&self) -> String {
        build_support::input_profile(&self.0.join("crates/local-data"))
            .unwrap()
            .0
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn transitive_validation_changes_and_new_modules_change_the_analyzer_identity() {
    let t = Tree::new();
    let before = t.profile();
    fs::write(
        t.0.join("crates/core/src/contract/snapshot.rs"),
        "changed validation\n",
    )
    .unwrap();
    let changed = t.profile();
    assert_ne!(before, changed);
    fs::write(
        t.0.join("crates/ingest/src/new_validation.rs"),
        "new validator\n",
    )
    .unwrap();
    assert_ne!(changed, t.profile());
}
#[test]
fn lf_and_crlf_have_the_same_profile() {
    let t = Tree::new();
    let before = t.profile();
    fs::write(
        t.0.join("crates/core/src/contract/snapshot.rs"),
        "original\r\n",
    )
    .unwrap();
    assert_eq!(before, t.profile());
}

#[test]
fn workspace_and_dependency_manifests_are_part_of_the_identity() {
    let t = Tree::new();
    for file in ["Cargo.toml", "crates/ingest/Cargo.toml"] {
        let before = t.profile();
        fs::write(t.0.join(file), "changed features\n").unwrap();
        assert_ne!(before, t.profile());
    }
    let before = t.profile();
    fs::write(t.0.join("crates/core/build.rs"), "fn main() {}\n").unwrap();
    assert_ne!(before, t.profile());
}
