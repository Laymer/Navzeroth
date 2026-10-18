//! Source-closed identity for the input analyzer and all first-party validators.
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
fn sources(root: &Path, relative: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if kind.is_dir() {
            sources(root, &path, files)?;
        } else if kind.is_file() && path.extension().is_some_and(|v| v == "rs") {
            files.push(path);
        }
        if files.len() > 4096 {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    Ok(())
}
pub fn input_profile(root: &Path) -> io::Result<(String, Vec<PathBuf>)> {
    let mut files = vec![
        PathBuf::from("build.rs"),
        PathBuf::from("build_support.rs"),
        PathBuf::from("Cargo.toml"),
        PathBuf::from("../../Cargo.lock"),
        PathBuf::from("../../Cargo.toml"),
        PathBuf::from("../core/Cargo.toml"),
        PathBuf::from("../ingest/Cargo.toml"),
        PathBuf::from("../storage/Cargo.toml"),
    ];
    for directory in ["src", "../core/src", "../ingest/src", "../storage/src"] {
        sources(root, Path::new(directory), &mut files)?;
    }
    for directory in [".", "../core", "../ingest", "../storage"] {
        for entry in fs::read_dir(root.join(directory))? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(io::ErrorKind::InvalidData.into());
            }
            if kind.is_file() && entry.path().extension().is_some_and(|v| v == "rs") {
                files.push(Path::new(directory).join(entry.file_name()));
            }
        }
    }
    files.sort();
    files.dedup();
    let mut profile = Sha256::new();
    for relative in &files {
        let path = root.join(relative);
        if fs::symlink_metadata(&path)?.file_type().is_symlink()
            || fs::metadata(&path)?.len() > 4 * 1024 * 1024
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let name = relative.to_string_lossy().replace('\\', "/");
        let text = fs::read_to_string(path)?.replace("\r\n", "\n");
        profile.update((name.len() as u64).to_le_bytes());
        profile.update(name.as_bytes());
        profile.update((text.len() as u64).to_le_bytes());
        profile.update(text.as_bytes());
    }
    Ok((format!("{:x}", profile.finalize()), files))
}
