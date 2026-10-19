use super::*;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "anav-descriptor-{}-{}",
            std::process::id(),
            super::super::STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn file(&self) -> PathBuf {
        self.0.join("input.json")
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn exact_empty_and_limit_reads_preserve_bytes_and_aggregate_budget() {
    let dir = Directory::new();
    let path = dir.file();
    fs::write(&path, b"").unwrap();
    assert_eq!(read(&path, 0).unwrap(), b"");
    fs::write(&path, b"\x00\xff\r\n").unwrap();
    assert_eq!(read(&path, 4).unwrap(), b"\x00\xff\r\n");
    assert!(read(&path, 3).is_err());
    assert!(read(&dir.0, 100).is_err());
    assert!(read(&dir.0.join("absent"), 100).is_err());
    let mut budget = super::super::Budget { bytes: 0, limit: 7 };
    assert_eq!(budget.read(&path, 10).unwrap().len(), 4);
    assert!(budget.read(&path, 10).is_err());
    assert_eq!(budget.bytes, 4);
}
#[test]
fn replacement_and_changes_between_inspection_open_and_read_are_refused() {
    let dir = Directory::new();
    let path = dir.file();
    fs::write(&path, b"data").unwrap();
    assert!(
        read_with(&path, 4, |phase| if phase == 0 {
            fs::rename(&path, dir.0.join("old")).unwrap();
            fs::write(&path, b"data").unwrap();
        })
        .is_err()
    );
    assert!(
        read_with(&path, 4, |phase| if phase == 2 {
            fs::write(&path, b"grow!").unwrap();
        })
        .is_err()
    );
    fs::write(&path, b"data").unwrap();
    assert!(
        read_with(&path, 4, |phase| if phase == 1 {
            fs::write(&path, b"grows").unwrap();
        })
        .is_err()
    );
    fs::write(&path, b"data").unwrap();
    assert!(
        read_with(&path, 4, |phase| if phase == 2 {
            fs::rename(&path, dir.0.join("new")).unwrap();
        })
        .is_err()
    );
}
#[cfg(unix)]
#[test]
fn symlinks_and_fifo_swaps_are_nonblocking_and_never_consumed() {
    use std::{
        ffi::CString,
        os::unix::{ffi::OsStrExt, fs::symlink},
    };
    let dir = Directory::new();
    let path = dir.file();
    fs::write(&path, b"data").unwrap();
    let link = dir.0.join("link");
    symlink(&path, &link).unwrap();
    assert!(read(&link, 4).is_err());
    assert!(
        read_with(&path, 4, |phase| if phase == 0 {
            fs::remove_file(&path).unwrap();
            symlink(&link, &path).unwrap();
        })
        .is_err()
    );
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"data").unwrap();
    assert!(
        read_with(&path, 4, |phase| if phase == 0 {
            fs::remove_file(&path).unwrap();
            let name = CString::new(path.as_os_str().as_bytes()).unwrap();
            // SAFETY: name is a live, NUL-terminated local synthetic path.
            assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        })
        .is_err()
    );
    assert!(read(&path, 4).is_err());
}
