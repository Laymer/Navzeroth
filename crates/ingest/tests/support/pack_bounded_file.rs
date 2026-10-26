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

#[test]
fn identical_bytes_and_modified_time_replacement_is_not_the_same_file() {
    for checkpoint in [0, 1, 2] {
        let dir = Directory::new();
        let path = dir.file();
        fs::write(&path, b"data").unwrap();
        let original = fs::metadata(&path).unwrap();
        let original_stamp = descriptor_stamp(&fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(read(&path, 4).unwrap(), b"data");
        assert!(
            read_with(&path, 4, |phase| if phase == checkpoint {
                fs::rename(&path, dir.0.join("retained-original")).unwrap();
                fs::write(&path, b"data").unwrap();
                let replacement = fs::OpenOptions::new().write(true).open(&path).unwrap();
                replacement
                    .set_times(fs::FileTimes::new().set_modified(original.modified().unwrap()))
                    .unwrap();
                #[cfg(windows)]
                {
                    use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
                    use windows_sys::Win32::{
                        Foundation::FILETIME, Storage::FileSystem::SetFileTime,
                    };
                    let creation = original.creation_time();
                    let creation = FILETIME {
                        dwLowDateTime: creation as u32,
                        dwHighDateTime: (creation >> 32) as u32,
                    };
                    // SAFETY: live writable handle and valid creation-time input;
                    // null access/write pointers retain the existing values.
                    assert_ne!(
                        unsafe {
                            SetFileTime(
                                replacement.as_raw_handle(),
                                &creation,
                                std::ptr::null(),
                                std::ptr::null(),
                            )
                        },
                        0
                    );
                }
                drop(replacement);
                let changed = fs::metadata(&path).unwrap();
                assert_eq!(changed.len(), original.len());
                assert_eq!(changed.modified().unwrap(), original.modified().unwrap());
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    assert_eq!(changed.creation_time(), original.creation_time());
                    assert_eq!(changed.file_attributes(), original.file_attributes());
                }
                assert_eq!(fs::read(&path).unwrap(), b"data");
                assert_ne!(
                    descriptor_stamp(&fs::File::open(&path).unwrap()).unwrap(),
                    original_stamp
                );
            })
            .is_err()
        );
        // The new, now-stable selected file is readable on its own.
        assert_eq!(read(&path, 4).unwrap(), b"data");
    }
}

#[test]
fn component_directory_replacement_refuses_but_unrelated_children_do_not() {
    let dir = Directory::new();
    let parent = dir.0.join("selected");
    fs::create_dir(&parent).unwrap();
    let path = parent.join("input.json");
    fs::write(&path, b"data").unwrap();
    assert_eq!(
        read_with(&path, 4, |phase| if phase == 1 {
            fs::write(parent.join("unrelated"), b"new child").unwrap();
        })
        .unwrap(),
        b"data"
    );
    #[cfg(windows)]
    let selected = descriptor_stamp(&fs::File::open(&path).unwrap()).unwrap();
    assert!(
        read_with(&path, 4, |phase| if phase == 0 {
            let retained = dir.0.join("retained-directory");
            fs::rename(&parent, &retained).unwrap();
            fs::create_dir(&parent).unwrap();
            fs::rename(retained.join("input.json"), &path).unwrap();
            #[cfg(windows)]
            assert_eq!(
                descriptor_stamp(&fs::File::open(&path).unwrap()).unwrap(),
                selected
            );
        })
        .is_err()
    );
    assert_eq!(read(&path, 4).unwrap(), b"data");
}
