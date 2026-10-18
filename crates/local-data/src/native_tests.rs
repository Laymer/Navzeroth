//! Synthetic ABI responses only. These tests do not decode a real CASC archive.
use super::*;
use std::{cell::RefCell, ffi::CStr};
#[derive(Clone)]
struct Response {
    decrypt: u32,
    open: u32,
    null_storage: bool,
    stats: u32,
    by_id: u32,
    by_name: u32,
    null_file: bool,
    size: u64,
    read_error: u32,
    read_done: u32,
    data: Vec<u8>,
    count_error: u32,
    count: Option<u32>,
    frames: Vec<FrameLayout>,
    frame_error: u32,
    plain_error: u32,
    plain_done: u32,
    opens: usize,
    closes: usize,
    file_opens: usize,
    file_closes: usize,
    names: Vec<String>,
    reads: Vec<(u64, u32)>,
    bad_handle: bool,
}
impl Default for Response {
    fn default() -> Self {
        Self {
            decrypt: 0,
            open: 0,
            null_storage: false,
            stats: 0,
            by_id: 0,
            by_name: 0,
            null_file: false,
            size: 8,
            read_error: 0,
            read_done: 8,
            data: vec![1, 2, 3, 4, 99, 99, 99, 99],
            count_error: 0,
            count: None,
            frames: vec![
                FrameLayout {
                    start: 0,
                    length: 4,
                    encrypted: false,
                },
                FrameLayout {
                    start: 4,
                    length: 4,
                    encrypted: true,
                },
            ],
            frame_error: 0,
            plain_error: 0,
            plain_done: 4,
            opens: 0,
            closes: 0,
            file_opens: 0,
            file_closes: 0,
            names: vec![],
            reads: vec![],
            bad_handle: false,
        }
    }
}
thread_local! {static RESPONSE: RefCell<Response> = RefCell::new(Response::default());}
fn storage() -> *mut c_void {
    std::ptr::without_provenance_mut(1)
}
fn file() -> *mut c_void {
    std::ptr::without_provenance_mut(2)
}
unsafe extern "C" fn open(
    _: *const NativeChar,
    _: *const NativeChar,
    _: u32,
    _: u32,
    out: *mut *mut c_void,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.opens += 1;
        // SAFETY: the Rust adapter passes a live initialized output pointer.
        unsafe {
            *out = if s.open != 0 || s.null_storage {
                std::ptr::null_mut()
            } else {
                storage()
            };
        }
        s.open
    })
}
unsafe extern "C" fn close(handle: *mut c_void) {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != storage();
        s.closes += 1;
    });
}
unsafe extern "C" fn decrypt() -> u32 {
    RESPONSE.with(|s| s.borrow().decrypt)
}
unsafe extern "C" fn stats(
    handle: *mut c_void,
    features: *mut u32,
    local: *mut u32,
    total: *mut u32,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != storage();
        // SAFETY: three live aligned scalar outputs supplied by the adapter.
        unsafe {
            *features = 0;
            *local = 0;
            *total = 0;
        }
        s.stats
    })
}
unsafe extern "C" fn file_open(
    handle: *mut c_void,
    _: u32,
    out: *mut *mut c_void,
    size: *mut u64,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != storage();
        // SAFETY: live output pointers; tokens are opaque and never dereferenced.
        unsafe {
            *size = s.size;
            *out = if s.by_id != 0 || s.null_file {
                std::ptr::null_mut()
            } else {
                s.file_opens += 1;
                file()
            };
        }
        s.by_id
    })
}
unsafe extern "C" fn name_open(
    handle: *mut c_void,
    name: *const std::ffi::c_char,
    out: *mut *mut c_void,
    size: *mut u64,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != storage();
        // SAFETY: adapter supplies a live NUL-terminated CString and scalar outputs.
        unsafe {
            s.names
                .push(CStr::from_ptr(name).to_string_lossy().into_owned());
            *size = s.size;
            *out = if s.by_name != 0 || s.null_file {
                std::ptr::null_mut()
            } else {
                s.file_opens += 1;
                file()
            };
        }
        s.by_name
    })
}
unsafe extern "C" fn file_close(handle: *mut c_void) {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != file();
        s.file_closes += 1;
    });
}
unsafe extern "C" fn read(
    handle: *mut c_void,
    buffer: *mut c_void,
    length: u32,
    done: *mut u32,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != file();
        s.reads.push((0, length));
        let count = if s.read_error == 0 {
            length
        } else {
            s.read_done.min(length)
        };
        let Some(bytes) = s.data.get(..count as usize) else {
            return 4;
        };
        // SAFETY: adapter allocates length bytes; count is bounded by that length.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast(), count as usize);
            *done = count;
        }
        s.read_error
    })
}
unsafe extern "C" fn frame_count(handle: *mut c_void, count: *mut u32) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != file();
        // SAFETY: live scalar output supplied by adapter.
        unsafe {
            *count = s.count.unwrap_or(s.frames.len() as u32);
        }
        s.count_error
    })
}
unsafe extern "C" fn frame(
    handle: *mut c_void,
    index: u32,
    start: *mut u64,
    length: *mut u32,
    encrypted: *mut u32,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != file();
        if s.frame_error != 0 {
            return s.frame_error;
        }
        let Some(row) = s.frames.get(index as usize) else {
            return 4;
        };
        // SAFETY: live aligned scalar outputs supplied by the adapter.
        unsafe {
            *start = row.start;
            *length = row.length;
            *encrypted = u32::from(row.encrypted);
        }
        0
    })
}
unsafe extern "C" fn read_at(
    handle: *mut c_void,
    offset: u64,
    buffer: *mut c_void,
    length: u32,
    done: *mut u32,
) -> u32 {
    RESPONSE.with(|s| {
        let mut s = s.borrow_mut();
        s.bad_handle |= handle != file();
        s.reads.push((offset, length));
        let count = if s.plain_error == 0 {
            length
        } else {
            s.plain_done.min(length)
        };
        let Some(end) = offset.checked_add(u64::from(count)) else {
            return 4;
        };
        let Some(bytes) = s.data.get(offset as usize..end as usize) else {
            return 4;
        };
        // SAFETY: count is no larger than the adapter's length-byte allocation.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast(), count as usize);
            *done = count;
        }
        s.plain_error
    })
}
static SYNTHETIC: Calls = Calls {
    open,
    close,
    decryption_disabled: decrypt,
    stats,
    file_open,
    read,
    name_open,
    file_close,
    frame_count,
    frame,
    read_at,
};

/// Test-only ABI provider for the real staging/checkpoint orchestration. No
/// production caller can select it, and its bytes never establish availability.
pub(crate) fn stage_with_bytes<T>(bytes: Vec<u8>, action: impl FnOnce() -> T) -> T {
    RESPONSE.with(|s| {
        *s.borrow_mut() = Response {
            size: bytes.len() as u64,
            data: bytes,
            ..Response::default()
        }
    });
    let result = action();
    RESPONSE.with(|s| {
        let state = s.borrow();
        assert!(!state.bad_handle);
        assert_eq!(state.opens, state.closes);
        assert_eq!(state.file_opens, state.file_closes);
    });
    result
}

pub(crate) fn open_staging(
    root: &Path,
    product: &str,
    locale: u32,
    build: u32,
) -> crate::Result<Archive> {
    Archive::open_using(root, product, locale, build, &SYNTHETIC)
}
fn run<T>(response: Response, operation: impl FnOnce(&Archive) -> T) -> (T, Response) {
    RESPONSE.with(|s| *s.borrow_mut() = response);
    let archive = Archive::open_using(
        Path::new("synthetic-unused"),
        "synthetic",
        0,
        70450,
        &SYNTHETIC,
    )
    .unwrap();
    let result = operation(&archive);
    drop(archive);
    let state = RESPONSE.with(|s| s.borrow().clone());
    assert!(!state.bad_handle);
    assert_eq!(state.closes, 1);
    assert_eq!(state.file_opens, state.file_closes);
    (result, state)
}
fn partial_response() -> Response {
    Response {
        read_error: 2,
        read_done: 4,
        ..Response::default()
    }
}
#[test]
fn whole_empty_and_named_fallback_reads_preserve_bytes_and_release_every_handle() {
    let (read, state) = run(Response::default(), |archive| archive.read_file_id(1, 8));
    assert!(matches!(read,Ok(Logical::Whole(bytes)) if bytes==[1,2,3,4,99,99,99,99]));
    assert_eq!(state.reads, [(0, 8)]);
    let (read, state) = run(
        Response {
            by_id: 1,
            ..Response::default()
        },
        |archive| archive.read(Some(1), "Synthetic", "db2", 8),
    );
    assert!(matches!(read, Ok(Logical::Whole(_))));
    assert_eq!(state.names, ["DBFilesClient\\Synthetic.db2"]);
    let (read, _) = run(
        Response {
            size: 0,
            data: vec![],
            ..Response::default()
        },
        |archive| archive.read(None, "Empty", "db2", 0),
    );
    assert!(matches!(read,Ok(Logical::Whole(bytes)) if bytes.is_empty()));
    let (stats, _) = run(Response::default(), |archive| archive.statistics());
    assert_eq!(stats, Ok((0, 0, 0)));
    let (stats, _) = run(
        Response {
            stats: 6,
            ..Response::default()
        },
        |archive| archive.statistics(),
    );
    assert_eq!(stats, Err("Archive statistics unavailable"));
}
#[test]
fn invalid_ids_names_null_file_and_quotas_refuse_without_retaining_bytes() {
    for (id, name, maximum, response, expected) in [
        (Some(0), None, 8, Response::default(), ReadFailure::Invalid),
        (None, None, 8, Response::default(), ReadFailure::Missing),
        (
            None,
            Some("NUL\0name".into()),
            8,
            Response::default(),
            ReadFailure::Invalid,
        ),
        (
            Some(1),
            None,
            8,
            Response {
                null_file: true,
                ..Response::default()
            },
            ReadFailure::Invalid,
        ),
        (Some(1), None, 7, Response::default(), ReadFailure::Capacity),
        (
            Some(1),
            None,
            usize::MAX,
            Response {
                size: u64::from(u32::MAX) + 1,
                ..Response::default()
            },
            ReadFailure::Capacity,
        ),
    ] {
        let (actual, _) = run(response, |archive| archive.read_selected(id, name, maximum));
        assert!(matches!(actual,Err(value) if value==expected));
    }
    for error in [1, 2, 3, 4, 5, 6, 7, 8, 99] {
        let (actual, _) = run(
            Response {
                by_id: error,
                ..Response::default()
            },
            |archive| archive.read_file_id(1, 8),
        );
        assert!(matches!(actual,Err(value) if value==classify(error)));
    }
}
#[test]
fn encrypted_prefix_is_reread_only_as_verified_plain_spans() {
    let (actual, state) = run(partial_response(), |archive| archive.read_file_id(1, 8));
    let Ok(Logical::Partial(partial)) = actual else {
        panic!("Expected explicit partial logical file");
    };
    assert_eq!(partial.len(), 8);
    assert_eq!(partial.plain_bytes(), 4);
    assert_eq!(partial.unavailable().len(), 1);
    assert_eq!(
        (
            partial.unavailable()[0].offset,
            partial.unavailable()[0].length
        ),
        (4, 4)
    );
    assert_eq!(state.reads, [(0, 8), (0, 4)]); // Encrypted span is never passed to read_at.
    assert!(partial.input().available(0, 4));
    assert!(!partial.input().available(4, 8));
    assert!(!partial.input().available(3, 5));
    assert!(partial.retained_bytes().ends_with(&[1, 2, 3, 4]));
    assert!(
        !partial
            .retained_bytes()
            .windows(4)
            .any(|bytes| bytes == [99, 99, 99, 99])
    );
    let (actual, state) = run(
        Response {
            read_error: 2,
            read_done: 0,
            ..Response::default()
        },
        |archive| archive.read_file_id(1, 8),
    );
    assert!(matches!(actual, Err(ReadFailure::Encrypted)));
    assert_eq!(state.reads, [(0, 8)]);
    for error in [3, 4, 6, 7, 8] {
        let (actual, state) = run(
            Response {
                read_error: error,
                ..Response::default()
            },
            |archive| archive.read_file_id(1, 8),
        );
        assert!(matches!(actual,Err(value) if value==classify(error)));
        assert_eq!(state.reads, [(0, 8)]);
    }
}
#[test]
fn malformed_frame_counts_layouts_and_plain_run_failures_never_become_partial_success() {
    for count in [0, 9, MAX_FRAMES as u32 + 1] {
        let (actual, _) = run(
            Response {
                count: Some(count),
                ..partial_response()
            },
            |archive| archive.read_file_id(1, 8),
        );
        assert!(matches!(actual, Err(ReadFailure::Invalid)));
    }
    for (response, expected) in [
        (
            Response {
                count_error: 6,
                ..partial_response()
            },
            ReadFailure::Unavailable,
        ),
        (
            Response {
                frame_error: 4,
                ..partial_response()
            },
            ReadFailure::Invalid,
        ),
        (
            Response {
                frames: vec![
                    FrameLayout {
                        start: 1,
                        length: 4,
                        encrypted: false,
                    },
                    FrameLayout {
                        start: 4,
                        length: 4,
                        encrypted: true,
                    },
                ],
                ..partial_response()
            },
            ReadFailure::Invalid,
        ),
    ] {
        let (actual, _) = run(response, |archive| archive.read_file_id(1, 8));
        assert!(matches!(actual,Err(value) if value==expected));
    }
    for error in [2, 4, 6, 7, 8] {
        let (actual, _) = run(
            Response {
                plain_error: error,
                ..partial_response()
            },
            |archive| archive.read_file_id(1, 8),
        );
        let expected = if error == 2 {
            ReadFailure::ShortRead
        } else {
            classify(error)
        };
        assert!(matches!(actual,Err(value) if value==expected));
    }
}
#[test]
fn opening_refuses_decryption_errors_nul_and_missing_handles_before_storage_use() {
    for (response, product, expected) in [
        (
            Response {
                decrypt: 3,
                ..Response::default()
            },
            "synthetic",
            "Native reader can decrypt; refusing to open storage",
        ),
        (
            Response {
                open: 6,
                ..Response::default()
            },
            "synthetic",
            "Local archive storage unavailable; no download attempted",
        ),
        (
            Response {
                null_storage: true,
                ..Response::default()
            },
            "synthetic",
            "Native storage handle missing",
        ),
        (
            Response::default(),
            "NUL\0product",
            "Native selection contains NUL",
        ),
    ] {
        RESPONSE.with(|s| *s.borrow_mut() = response);
        let error =
            match Archive::open_using(Path::new("synthetic-unused"), product, 0, 70450, &SYNTHETIC)
            {
                Ok(archive) => {
                    drop(archive);
                    None
                }
                Err(error) => Some(error),
            };
        assert_eq!(error, Some(expected));
        assert_eq!(RESPONSE.with(|s| s.borrow().closes), 0);
    }
    assert!(native_text(std::ffi::OsStr::new("NUL\0path")).is_err());
    let text = native_text(std::ffi::OsStr::new("synthetic")).unwrap();
    assert_eq!(text.last(), Some(&0));
    assert_eq!(text.iter().filter(|&&c| c == 0).count(), 1);
}

#[test]
fn linked_offline_shim_refuses_empty_storage_without_creating_source_files() {
    let root = std::env::temp_dir().join(format!(
        "nav-native-empty-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let outcome = match Archive::open(&root, "wow_classic_beta", 0, 70450) {
        Ok(archive) => {
            drop(archive);
            None
        }
        Err(error) => Some(error),
    };
    let entries = std::fs::read_dir(&root).unwrap().count();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        outcome,
        Some("Local archive storage unavailable; no download attempted")
    );
    assert_eq!(entries, 0);
    assert!(decryption_disabled());
}
