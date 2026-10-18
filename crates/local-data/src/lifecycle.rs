//! Filesystem orchestration around the private SQLite lifecycle index.
use crate::{Result, checked_path, extraction::Store, hash};
use azeroth_core::{
    local_data::PerformancePreset,
    local_data_lifecycle::{Activation, JobState, JobStatus},
};
use azeroth_storage::local_data::LocalDataIndex;
use std::{fs, path::Path};

// SQLite may remove a journal/sidecar immediately after this no-follow observation.
// Do not canonicalize/re-stat the volatile leaf: validate the observed metadata.
// Root and all ancestor components are checked separately before entering here.
fn validate_index_files(
    root: &Path,
    mut metadata: impl FnMut(&Path) -> std::io::Result<fs::Metadata>,
) -> Result<()> {
    for name in [
        "lifecycle.sqlite3",
        "lifecycle.sqlite3-journal",
        "lifecycle.sqlite3-wal",
        "lifecycle.sqlite3-shm",
    ] {
        let observed = match metadata(&root.join(name)) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("Lifecycle file unavailable"),
        };
        if crate::linked(&observed) {
            return Err("Linked or reparse-point selections are unsupported");
        }
        let maximum = (if name == "lifecycle.sqlite3" { 64 } else { 65 }) * 1024 * 1024;
        if !observed.is_file() || observed.len() > maximum {
            return Err("Invalid or excessive lifecycle database file");
        }
    }
    Ok(())
}

pub fn index(root: &Path) -> Result<LocalDataIndex> {
    let root = checked_path(root)?;
    if !root.is_dir() {
        return Err("Private destination must be a directory");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&root)
            .map_err(|_| "Private destination inaccessible")?
            .permissions()
            .mode()
            & 0o077
            != 0
        {
            return Err("Private destination must permit access only to its owner");
        }
    }
    validate_index_files(&root, |path| fs::symlink_metadata(path))?;
    LocalDataIndex::open(&root.join("lifecycle.sqlite3"))
        .map_err(|_| "Private lifecycle database unavailable or unsupported")
}
pub fn selection_id(path: &Path, product: &str) -> String {
    let mut bytes = path.as_os_str().as_encoded_bytes().to_vec();
    bytes.push(0);
    bytes.extend_from_slice(product.as_bytes());
    hash(&bytes)
}
/// Recovery only when the destination writer lock proves no worker can commit.
pub fn jobs(root: &Path) -> Result<Vec<JobStatus>> {
    let mut db = index(root)?;
    let _store = match Store::open(root, PerformancePreset::Max.limits().total_storage_bytes) {
        Ok(store) => Some(store),
        Err("Another extraction owns the destination") => None,
        Err(e) => return Err(e),
    };
    if _store.is_some() {
        for job in db.jobs().map_err(|_| "Cannot read jobs")? {
            if matches!(job.state, JobState::Preparing | JobState::Running) {
                db.finish(&job.job_id, JobState::Interrupted, None)
                    .map_err(|_| "Cannot recover interrupted job")?;
            }
        }
    }
    db.jobs().map_err(|_| "Cannot read jobs")
}
pub fn status(root: &Path, id: &str) -> Result<JobStatus> {
    jobs(root)?
        .into_iter()
        .find(|j| j.job_id == id)
        .ok_or("Unknown local-data job")
}
pub fn cancel(root: &Path, id: &str) -> Result<JobStatus> {
    index(root)?
        .finish(id, JobState::Cancelled, None)
        .map_err(|_| "Cannot cancel local-data job")
}
pub fn resume_request(root: &Path, id: &str) -> Result<crate::worker::ExtractionRequest> {
    let db = index(root)?;
    let text = db.request(id).map_err(|_| "Unknown local-data job")?;
    let mut request: crate::worker::ExtractionRequest =
        serde_json::from_value(azeroth_core::strict_json::value(text.as_bytes(), 65536)?)
            .map_err(|_| "Invalid saved extraction request")?;
    if checked_path(&request.destination)? != checked_path(root)? || request.request_id != id {
        return Err("Saved extraction destination or identity changed");
    }
    request.validate()?;
    if request.protocol_version < 3 {
        request.protocol_version = 2;
    }
    request.resume = true;
    request.validate()?;
    Ok(request)
}
pub fn catalog_scope(root: &Path, digest: &str) -> Result<String> {
    let catalog = crate::verify_catalog(root, digest)?;
    Ok(hash(&crate::bounded_json(
        &(
            catalog.installation.product,
            catalog.installation.version,
            catalog.installation.build,
            catalog.installation.declared_locales,
        ),
        65536,
    )?))
}
pub fn active(root: &Path, digest: &str) -> Result<Activation> {
    let scope = catalog_scope(root, digest)?;
    index(root)?
        .active(&scope)
        .map_err(|_| "Cannot read research activation")
}
pub fn activate(root: &Path, digest: &str, expected: u64, rollback: bool) -> Result<Activation> {
    let _store = Store::open(root, PerformancePreset::Max.limits().total_storage_bytes)?;
    let scope = catalog_scope(root, digest)?;
    index(root)?
        .activate(&scope, digest, expected, rollback)
        .map_err(|_| "Activation changed, rollback target invalid or history capacity reached")
}

#[derive(serde::Serialize)]
pub struct SelectionView {
    pub selection_id: String,
    pub inspection: crate::Inspection,
    pub preset: PerformancePreset,
}
#[derive(serde::Serialize)]
pub struct SavedSelectionView {
    pub selection_id: String,
    pub product: String,
    pub preset: PerformancePreset,
}
pub fn select(
    root: &Path,
    installation: &Path,
    product: &str,
    preset: PerformancePreset,
) -> Result<SelectionView> {
    let path = checked_path(installation)?;
    let destination = checked_path(root)?;
    if path.starts_with(&destination) || destination.starts_with(&path) {
        return Err("Source and destination must be separate");
    }
    let inspection = crate::inspect(&path, product)?;
    let selection_id = selection_id(&path, product);
    index(root)?
        .save_selection(&azeroth_storage::local_data::SavedSelection {
            selection_id: selection_id.clone(),
            installation: path,
            product: product.into(),
            preset,
        })
        .map_err(|_| "Cannot save installation selection")?;
    Ok(SelectionView {
        selection_id,
        inspection,
        preset,
    })
}
pub fn selections(root: &Path) -> Result<Vec<SavedSelectionView>> {
    Ok(index(root)?
        .selections()
        .map_err(|_| "Cannot read saved selections")?
        .into_iter()
        .map(|s| SavedSelectionView {
            selection_id: s.selection_id,
            product: s.product,
            preset: s.preset,
        })
        .collect())
}
pub fn new_job_id() -> Result<String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Job clock unavailable")?
        .as_nanos();
    Ok(format!("job-{}-{nanos}", std::process::id()))
}

#[cfg(test)]
mod sidecar_tests {
    use super::*;
    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "nav-sidecar-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            }
            #[cfg(not(unix))]
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn disappearing_journal_is_validated_once_without_reopening_the_leaf() {
        let root = Directory::new();
        let journal = root.0.join("lifecycle.sqlite3-journal");
        fs::write(&journal, b"synthetic temporary journal").unwrap();
        let mut observations = 0;
        validate_index_files(&root.0, |path| {
            observations += 1;
            let metadata = fs::symlink_metadata(path);
            if path == journal && metadata.is_ok() {
                fs::remove_file(path).unwrap();
            }
            metadata
        })
        .unwrap();
        assert_eq!(observations, 4);
        assert!(!journal.exists());
        let index = index(&root.0).unwrap();
        assert!(index.jobs().unwrap().is_empty());
    }
    #[test]
    fn only_missing_entries_are_optional_and_access_failures_remain_explicit() {
        let root = Directory::new();
        assert!(
            validate_index_files(&root.0, |_| Err(std::io::ErrorKind::NotFound.into())).is_ok()
        );
        for kind in [
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::Other,
        ] {
            assert_eq!(
                validate_index_files(&root.0, |_| Err(kind.into())),
                Err("Lifecycle file unavailable")
            );
        }
    }
    #[test]
    fn wrong_types_and_size_ceilings_remain_refused_without_changing_files() {
        let root = Directory::new();
        let journal = root.0.join("lifecycle.sqlite3-journal");
        fs::create_dir(&journal).unwrap();
        assert_eq!(
            validate_index_files(&root.0, |p| fs::symlink_metadata(p)),
            Err("Invalid or excessive lifecycle database file")
        );
        fs::remove_dir(&journal).unwrap();
        for (name, maximum) in [
            ("lifecycle.sqlite3", 64_u64 * 1024 * 1024),
            ("lifecycle.sqlite3-journal", 65_u64 * 1024 * 1024),
        ] {
            let path = root.0.join(name);
            let file = fs::File::create(&path).unwrap();
            file.set_len(maximum).unwrap();
            assert!(validate_index_files(&root.0, |p| fs::symlink_metadata(p)).is_ok());
            file.set_len(maximum + 1).unwrap();
            drop(file);
            assert_eq!(
                validate_index_files(&root.0, |p| fs::symlink_metadata(p)),
                Err("Invalid or excessive lifecycle database file")
            );
            assert_eq!(fs::metadata(&path).unwrap().len(), maximum + 1);
            fs::remove_file(path).unwrap();
        }
    }
    #[cfg(unix)]
    #[test]
    fn sidecar_links_are_refused_without_following_or_altering_the_target() {
        let root = Directory::new();
        let target = root.0.join("target");
        fs::write(&target, b"preserve").unwrap();
        std::os::unix::fs::symlink(&target, root.0.join("lifecycle.sqlite3-journal")).unwrap();
        assert_eq!(
            validate_index_files(&root.0, |p| fs::symlink_metadata(p)),
            Err("Linked or reparse-point selections are unsupported")
        );
        assert_eq!(fs::read(target).unwrap(), b"preserve");
    }
}
