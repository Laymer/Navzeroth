//! Local, bounded inventory investigation. No network access or source execution.
use azeroth_core::{
    release::{FileRef, MAX_RELEASE_BYTES},
    release_inventory::{Input, Report, compare},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

pub struct Options {
    pub root: PathBuf,
    pub input: PathBuf,
}
impl Options {
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let (mut root, mut input) = (None, None);
        for pair in args.chunks(2) {
            if pair.len() != 2 || pair[1].to_str().is_some_and(|s| s.starts_with("--")) {
                return None;
            }
            let slot = match pair[0].to_str()? {
                "--root" => &mut root,
                "--input" => &mut input,
                _ => return None,
            };
            if slot.replace(PathBuf::from(&pair[1])).is_some() {
                return None;
            }
        }
        Some(Self {
            root: root?,
            input: input?,
        })
    }
}
#[derive(Serialize)]
pub struct VerifiedReport {
    pub input_sha256: String,
    pub evidence_files_verified: usize,
    pub comparison: Report,
}
fn ordinary(path: &Path) -> Result<fs::Metadata, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "inventory path unavailable")?;
    if metadata.file_type().is_symlink() {
        return Err("inventory symlink refused");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("inventory reparse point refused");
        }
    }
    Ok(metadata)
}
fn checked_path(root: &Path, relative: &Path) -> Result<PathBuf, &'static str> {
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || relative.as_os_str().is_empty()
    {
        return Err("inventory paths must be relative without traversal");
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        ordinary(&current)?;
    }
    Ok(current)
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, &'static str> {
    let metadata = ordinary(path)?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Err("inventory file exceeds limit or is not regular");
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "inventory file unavailable")?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "inventory file read failed")?;
    if bytes.len() > limit {
        return Err("inventory file exceeds limit");
    }
    Ok(bytes)
}
pub fn report(options: &Options) -> Result<VerifiedReport, &'static str> {
    // Check root components before canonicalization so symlinks cannot disappear.
    let absolute = if options.root.is_absolute() {
        options.root.clone()
    } else {
        std::env::current_dir()
            .map_err(|_| "working directory unavailable")?
            .join(&options.root)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        if matches!(component, Component::ParentDir) {
            return Err("inventory root traversal refused");
        }
        current.push(component);
        ordinary(&current)?;
    }
    let root = fs::canonicalize(absolute).map_err(|_| "inventory root unavailable")?;
    let bytes = read(&checked_path(&root, &options.input)?, MAX_RELEASE_BYTES)?;
    let input = Input::parse(&bytes)?;
    let references = input
        .inventory
        .domains
        .iter()
        .flat_map(|d| &d.enumeration_evidence)
        .chain(input.enumeration_reviews.iter().flat_map(|r| &r.evidence))
        .chain(input.records.iter().flat_map(|r| &r.evidence))
        .chain(input.scenarios.iter().flat_map(|s| &s.evidence));
    let mut seen: BTreeMap<&str, &FileRef> = BTreeMap::new();
    let mut total = 0u64;
    for reference in references {
        if let Some(previous) = seen.insert(reference.path.as_str(), reference) {
            if previous != reference {
                return Err("conflicting evidence file pins");
            }
            continue;
        }
        total = total
            .checked_add(reference.bytes)
            .ok_or("evidence capacity overflow")?;
        if total > 1024 * 1024 * 1024 {
            return Err("evidence closure exceeds 1 GiB");
        }
        let evidence = read(
            &checked_path(&root, Path::new(reference.path.as_str()))?,
            reference.bytes as usize,
        )?;
        if evidence.len() as u64 != reference.bytes
            || format!("{:x}", Sha256::digest(&evidence)) != reference.sha256.as_str()
        {
            return Err("evidence size or hash mismatch");
        }
    }
    Ok(VerifiedReport {
        input_sha256: format!("{:x}", Sha256::digest(bytes)),
        evidence_files_verified: seen.len(),
        comparison: compare(&input)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "anav-inventory-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn options(&self) -> Options {
            Options {
                root: self.0.clone(),
                input: "input.json".into(),
            }
        }
        fn write_input(&self, with_evidence: bool) {
            let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(
                "../../../release/v1/inventory-investigation.json"
            ))
            .unwrap();
            if with_evidence {
                fs::write(
                    self.0.join("proof.txt"),
                    b"independent reviewed enumeration",
                )
                .unwrap();
                value["inventory"]["domains"][0]["enumeration_evidence"] = serde_json::json!([{"path":"proof.txt", "bytes":b"independent reviewed enumeration".len(), "sha256":format!("{:x}", Sha256::digest(b"independent reviewed enumeration"))}]);
            }
            fs::write(
                self.0.join("input.json"),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn inventory_report_verifies_hashes_and_tampering() {
        let temp = Temp::new();
        temp.write_input(true);
        let result = report(&temp.options()).unwrap();
        assert_eq!(result.evidence_files_verified, 1);
        assert_eq!(result.comparison.domains.len(), 9);
        fs::write(temp.0.join("proof.txt"), b"tampered").unwrap();
        assert!(report(&temp.options()).is_err());
    }
    #[test]
    fn repeated_evidence_pins_are_deduplicated_but_conflicts_are_refused() {
        let temp = Temp::new();
        temp.write_input(true);
        let path = temp.0.join("input.json");
        let mut input: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        input["inventory"]["domains"][1]["enumeration_evidence"] =
            input["inventory"]["domains"][0]["enumeration_evidence"].clone();
        fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
        assert_eq!(report(&temp.options()).unwrap().evidence_files_verified, 1);
        input["inventory"]["domains"][1]["enumeration_evidence"][0]["sha256"] =
            serde_json::json!("f".repeat(64));
        fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
        assert_eq!(
            report(&temp.options()).err(),
            Some("conflicting evidence file pins")
        );
    }
    #[test]
    fn inventory_report_rejects_traversal_and_oversized_input() {
        let temp = Temp::new();
        temp.write_input(false);
        let mut options = temp.options();
        options.input = "../input.json".into();
        assert!(report(&options).is_err());
        fs::write(temp.0.join("input.json"), vec![b' '; MAX_RELEASE_BYTES + 1]).unwrap();
        assert!(report(&temp.options()).is_err());
    }
    #[test]
    fn inventory_report_options_are_strict() {
        let args: Vec<_> = ["--root", ".", "--input", "release/input.json"]
            .map(OsString::from)
            .into();
        assert!(Options::parse(&args).is_some());
        let mut duplicate = args.clone();
        duplicate.extend([OsString::from("--input"), OsString::from("other")]);
        assert!(Options::parse(&duplicate).is_none());
        assert!(Options::parse(&args[..3]).is_none());
    }
    #[cfg(unix)]
    #[test]
    fn inventory_report_rejects_symlink_evidence_and_root() {
        let temp = Temp::new();
        temp.write_input(true);
        fs::rename(temp.0.join("proof.txt"), temp.0.join("real.txt")).unwrap();
        std::os::unix::fs::symlink("real.txt", temp.0.join("proof.txt")).unwrap();
        assert!(report(&temp.options()).is_err());
        std::os::unix::fs::symlink(&temp.0, temp.0.join("link")).unwrap();
        let mut options = temp.options();
        options.root = temp.0.join("link");
        assert!(report(&options).is_err());
    }
}
