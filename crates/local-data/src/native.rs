//! The only archive FFI boundary. Handles never leave this module or move between
//! threads. The worker is the sole production caller. All native access is serialized.
use crate::partial::{AssembleError, FrameLayout, MAX_FRAMES, PartialFile};
use std::{
    ffi::{CString, c_void},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
    sync::{Mutex, MutexGuard},
};

static NATIVE: Mutex<()> = Mutex::new(());
#[cfg(windows)]
type NativeChar = u16;
#[cfg(not(windows))]
type NativeChar = std::ffi::c_char;

unsafe extern "C" {
    fn nav_casc_open(
        path: *const NativeChar,
        product: *const NativeChar,
        locale: u32,
        expected_build: u32,
        storage: *mut *mut c_void,
    ) -> u32;
    fn nav_casc_close(storage: *mut c_void);
    fn nav_casc_decryption_disabled() -> u32;
    fn nav_casc_stats(
        storage: *mut c_void,
        features: *mut u32,
        local: *mut u32,
        total: *mut u32,
    ) -> u32;
    fn nav_casc_file_open(
        storage: *mut c_void,
        id: u32,
        file: *mut *mut c_void,
        size: *mut u64,
    ) -> u32;
    fn nav_casc_read(file: *mut c_void, buffer: *mut c_void, length: u32, done: *mut u32) -> u32;
    fn nav_casc_name_open(
        storage: *mut c_void,
        name: *const std::ffi::c_char,
        file: *mut *mut c_void,
        size: *mut u64,
    ) -> u32;
    fn nav_casc_file_close(file: *mut c_void);
    fn nav_casc_frame_count(file: *mut c_void, count: *mut u32) -> u32;
    fn nav_casc_frame(
        file: *mut c_void,
        index: u32,
        start: *mut u64,
        length: *mut u32,
        encrypted: *mut u32,
    ) -> u32;
    fn nav_casc_read_at(
        file: *mut c_void,
        offset: u64,
        buffer: *mut c_void,
        length: u32,
        done: *mut u32,
    ) -> u32;
}

// Private native ABI contract. Production construction always selects LINKED;
// tests supply deterministic responses without a runtime backend selector.
// A file carries the same calls as its owning storage.
struct Calls {
    open: unsafe extern "C" fn(
        path: *const NativeChar,
        product: *const NativeChar,
        locale: u32,
        expected_build: u32,
        storage: *mut *mut c_void,
    ) -> u32,
    close: unsafe extern "C" fn(storage: *mut c_void),
    decryption_disabled: unsafe extern "C" fn() -> u32,
    stats: unsafe extern "C" fn(
        storage: *mut c_void,
        features: *mut u32,
        local: *mut u32,
        total: *mut u32,
    ) -> u32,
    file_open: unsafe extern "C" fn(
        storage: *mut c_void,
        id: u32,
        file: *mut *mut c_void,
        size: *mut u64,
    ) -> u32,
    read: unsafe extern "C" fn(
        file: *mut c_void,
        buffer: *mut c_void,
        length: u32,
        done: *mut u32,
    ) -> u32,
    name_open: unsafe extern "C" fn(
        storage: *mut c_void,
        name: *const std::ffi::c_char,
        file: *mut *mut c_void,
        size: *mut u64,
    ) -> u32,
    file_close: unsafe extern "C" fn(file: *mut c_void),
    frame_count: unsafe extern "C" fn(file: *mut c_void, count: *mut u32) -> u32,
    frame: unsafe extern "C" fn(
        file: *mut c_void,
        index: u32,
        start: *mut u64,
        length: *mut u32,
        encrypted: *mut u32,
    ) -> u32,
    read_at: unsafe extern "C" fn(
        file: *mut c_void,
        offset: u64,
        buffer: *mut c_void,
        length: u32,
        done: *mut u32,
    ) -> u32,
}
static LINKED: Calls = Calls {
    open: nav_casc_open,
    close: nav_casc_close,
    decryption_disabled: nav_casc_decryption_disabled,
    stats: nav_casc_stats,
    file_open: nav_casc_file_open,
    read: nav_casc_read,
    name_open: nav_casc_name_open,
    file_close: nav_casc_file_close,
    frame_count: nav_casc_frame_count,
    frame: nav_casc_frame,
    read_at: nav_casc_read_at,
};

fn decryption_disabled_locked(calls: &Calls) -> bool {
    // SAFETY: no arguments; the shim allocates and releases its own scratch
    // storage object and touches no shared state.
    unsafe { (calls.decryption_disabled)() == 0 }
}

/// Whether the linked CascLib build never decrypts: built-in keys are not
/// loaded and an encrypted frame naming a key from CascLib's static table is
/// still reported as encrypted, not decrypted. Serialized with other native calls.
pub(crate) fn decryption_disabled() -> bool {
    match NATIVE.lock() {
        Ok(_serial) => decryption_disabled_locked(&LINKED),
        Err(_) => false,
    }
}

/// A strict logical read: the whole verified file, or only its verified plain
/// frames when later frames are encrypted with keys that are not available.
#[derive(Debug)]
pub(crate) enum Logical {
    Whole(Vec<u8>),
    Partial(PartialFile),
}

/// Why a strict logical read did not produce the whole file. Each variant is a
/// distinct, reported cause; none implies another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ReadFailure {
    Missing,
    /// The first BLTE frame needing decryption uses a key that is not available
    /// and no earlier frame was produced.
    Encrypted,
    /// Earlier frames were produced and verified, then a frame encrypted with an
    /// unavailable key stopped the read. The logical file stays unavailable; the
    /// verified prefix is neither retained nor zero-filled.
    EncryptedFrames,
    Unsupported,
    Unavailable,
    /// CascLib reported `ERROR_FILE_CORRUPT`: a frame checksum mismatch, a
    /// malformed BLTE/encryption header or failed decompression.
    Invalid,
    /// End-of-data or a missing file part reported while opening the file
    /// (reading BLTE headers). Open stage only: local data streams zero-fill a
    /// truncated span at read time, which then fails the strict frame MD5 and is
    /// reported as `Invalid`. On Windows code 1006 is also the OS
    /// `ERROR_FILE_INVALID`, so that OS error would land here too.
    Truncated,
    /// The read returned fewer bytes than declared with no recorded cause. Most
    /// likely a raw data read failure after verified frames (for example an I/O or
    /// sharing error while the game runs): CascLib's frame reader overwrites such
    /// errors with success. Not encryption; mapped to the unavailable/encrypted
    /// disposition only because there is no unknown disposition.
    ShortRead,
    Capacity,
}

pub(crate) struct Archive {
    handle: NonNull<c_void>,
    calls: &'static Calls,
    _serial: MutexGuard<'static, ()>,
    _thread: PhantomData<Rc<()>>,
}

#[cfg(windows)]
fn native_text(value: &std::ffi::OsStr) -> Result<Vec<NativeChar>, &'static str> {
    use std::os::windows::ffi::OsStrExt;
    let mut value: Vec<_> = value.encode_wide().collect();
    if value.contains(&0) {
        return Err("Native selection contains NUL");
    }
    value.push(0);
    Ok(value)
}
#[cfg(not(windows))]
fn native_text(value: &std::ffi::OsStr) -> Result<Vec<NativeChar>, &'static str> {
    use std::os::unix::ffi::OsStrExt;
    let value = CString::new(value.as_bytes()).map_err(|_| "Native selection contains NUL")?;
    Ok(value
        .as_bytes_with_nul()
        .iter()
        .map(|b| *b as NativeChar)
        .collect())
}

impl Archive {
    pub(crate) fn statistics(&self) -> Result<(u32, u32, u32), &'static str> {
        let (mut features, mut local, mut total) = (0, 0, 0);
        // SAFETY: live storage, properly aligned initialized scalar outputs.
        if unsafe {
            (self.calls.stats)(self.handle.as_ptr(), &mut features, &mut local, &mut total)
        } != 0
        {
            return Err("Archive statistics unavailable");
        }
        Ok((features, local, total))
    }
    pub(crate) fn open(
        path: &Path,
        product: &str,
        locale: u32,
        expected_build: u32,
    ) -> Result<Self, &'static str> {
        Self::open_using(path, product, locale, expected_build, &LINKED)
    }
    fn open_using(
        path: &Path,
        product: &str,
        locale: u32,
        expected_build: u32,
        calls: &'static Calls,
    ) -> Result<Self, &'static str> {
        let serial = NATIVE.lock().map_err(|_| "Native reader unavailable")?;
        if !decryption_disabled_locked(calls) {
            return Err("Native reader can decrypt; refusing to open storage");
        }
        let path = native_text(path.as_os_str())?;
        let product = native_text(std::ffi::OsStr::new(product))?;
        let mut handle = std::ptr::null_mut();
        // SAFETY: both text buffers are NUL-terminated, live throughout the call;
        // shim initializes the output handle and copies selected strings internally.
        let error = unsafe {
            (calls.open)(
                path.as_ptr(),
                product.as_ptr(),
                locale,
                expected_build,
                &mut handle,
            )
        };
        if error != 0 {
            return Err("Local archive storage unavailable; no download attempted");
        }
        Ok(Self {
            handle: NonNull::new(handle).ok_or("Native storage handle missing")?,
            calls,
            _serial: serial,
            _thread: PhantomData,
        })
    }
    pub(crate) fn read(
        &self,
        id: Option<u32>,
        name: &str,
        extension: &str,
        maximum: usize,
    ) -> Result<Logical, ReadFailure> {
        self.read_selected(
            id,
            Some(format!("DBFilesClient\\{name}.{extension}")),
            maximum,
        )
    }
    pub(crate) fn read_file_id(&self, id: u32, maximum: usize) -> Result<Logical, ReadFailure> {
        self.read_selected(Some(id), None, maximum)
    }
    fn read_selected(
        &self,
        id: Option<u32>,
        name: Option<String>,
        maximum: usize,
    ) -> Result<Logical, ReadFailure> {
        if id == Some(0) {
            return Err(ReadFailure::Invalid);
        }
        let (mut file, mut size) = (std::ptr::null_mut(), 0);
        // SAFETY: owned live storage; valid output pointers; shim closes on failure.
        let mut error = if let Some(id) = id {
            unsafe { (self.calls.file_open)(self.handle.as_ptr(), id, &mut file, &mut size) }
        } else {
            1
        };
        if error == 1
            && let Some(name) = name
        {
            let name = CString::new(name).map_err(|_| ReadFailure::Invalid)?;
            // SAFETY: static virtual prefix and reviewed table identifier, NUL-terminated.
            error = unsafe {
                (self.calls.name_open)(self.handle.as_ptr(), name.as_ptr(), &mut file, &mut size)
            };
        }
        if error != 0 {
            return Err(classify(error));
        }
        let file = NativeFile {
            handle: NonNull::new(file).ok_or(ReadFailure::Invalid)?,
            calls: self.calls,
        };
        let size = usize::try_from(size).map_err(|_| ReadFailure::Capacity)?;
        if size > maximum || size > u32::MAX as usize {
            return Err(ReadFailure::Capacity);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| ReadFailure::Capacity)?;
        bytes.resize(size, 0);
        // SAFETY: owned file and initialized allocation of exactly the requested
        // length. Native reads cannot exceed length; no encrypted zero-fill flag.
        let mut done = 0;
        let error = unsafe {
            (self.calls.read)(
                file.handle.as_ptr(),
                bytes.as_mut_ptr().cast(),
                size as u32,
                &mut done,
            )
        };
        if error != 0 {
            // The short buffer is dropped here; it is never retained or filled.
            drop(bytes);
            let failure = classify_read(error, done);
            if failure == ReadFailure::EncryptedFrames {
                return file.partial(size).map(Logical::Partial);
            }
            return Err(failure);
        }
        Ok(Logical::Whole(bytes))
    }
}
impl NativeFile {
    /// Frame-aware strict read after an encrypted frame stopped the whole read.
    /// Every frame's encoded bytes are MD5-checked; frames in BLTE mode `E` are
    /// gaps and never decoded; plain runs are strictly re-read and verified.
    fn partial(&self, size: usize) -> Result<PartialFile, ReadFailure> {
        let mut count = 0;
        // SAFETY: owned live file handle and an initialized scalar output.
        let error = unsafe { (self.calls.frame_count)(self.handle.as_ptr(), &mut count) };
        if error != 0 {
            return Err(classify(error));
        }
        let count = count as usize;
        if count == 0 || count > MAX_FRAMES || count > size {
            return Err(ReadFailure::Invalid);
        }
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(count)
            .map_err(|_| ReadFailure::Capacity)?;
        for index in 0..count as u32 {
            let (mut start, mut length, mut encrypted) = (0, 0, 0);
            // SAFETY: owned live file handle; initialized scalar outputs. The shim
            // bounds the index and frees the encoded frame copy before returning.
            let error = unsafe {
                (self.calls.frame)(
                    self.handle.as_ptr(),
                    index,
                    &mut start,
                    &mut length,
                    &mut encrypted,
                )
            };
            if error != 0 {
                return Err(classify(error));
            }
            frames.push(FrameLayout {
                start,
                length,
                encrypted: encrypted != 0,
            });
        }
        PartialFile::assemble(size, &frames, |offset, length| {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(|_| ReadFailure::Capacity)?;
            bytes.resize(length, 0);
            let mut done = 0;
            // SAFETY: owned file and an initialized allocation of exactly `length`
            // bytes (assemble bounds it by the logical size, below u32::MAX).
            let error = unsafe {
                (self.calls.read_at)(
                    self.handle.as_ptr(),
                    offset as u64,
                    bytes.as_mut_ptr().cast(),
                    length as u32,
                    &mut done,
                )
            };
            if error != 0 {
                // A plain run cannot stop on encryption; any short read is its cause.
                return Err(match classify_read(error, done) {
                    ReadFailure::EncryptedFrames | ReadFailure::Encrypted => ReadFailure::ShortRead,
                    failure => failure,
                });
            }
            Ok(bytes)
        })
        .map_err(|error| match error {
            AssembleError::Read(failure) => failure,
            // An inconsistent frame layout is not a verified partial file.
            AssembleError::Layout(_) => ReadFailure::Invalid,
        })
    }
}
fn classify(error: u32) -> ReadFailure {
    match error {
        1 => ReadFailure::Missing,
        2 => ReadFailure::Encrypted,
        3 => ReadFailure::Unsupported,
        4 => ReadFailure::Invalid,
        5 => ReadFailure::Capacity,
        7 => ReadFailure::Truncated,
        8 => ReadFailure::ShortRead,
        _ => ReadFailure::Unavailable,
    }
}
/// Read-stage status: an encrypted frame reached after produced frames is a
/// partially encrypted file, distinct from one encrypted from its first frame.
fn classify_read(error: u32, done: u32) -> ReadFailure {
    match classify(error) {
        ReadFailure::Encrypted if done > 0 => ReadFailure::EncryptedFrames,
        failure => failure,
    }
}
struct NativeFile {
    handle: NonNull<c_void>,
    calls: &'static Calls,
}
impl Drop for NativeFile {
    fn drop(&mut self) {
        // SAFETY: exactly one owner; the storage outlives this temporary file.
        unsafe {
            (self.calls.file_close)(self.handle.as_ptr());
        }
    }
}
impl Drop for Archive {
    fn drop(&mut self) {
        // SAFETY: no files escape read(), unique live storage handle.
        unsafe {
            (self.calls.close)(self.handle.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_statuses_classify_distinctly() {
        assert_eq!(classify(1), ReadFailure::Missing);
        assert_eq!(classify(2), ReadFailure::Encrypted);
        assert_eq!(classify(3), ReadFailure::Unsupported);
        assert_eq!(classify(4), ReadFailure::Invalid);
        assert_eq!(classify(5), ReadFailure::Capacity);
        assert_eq!(classify(6), ReadFailure::Unavailable);
        assert_eq!(classify(7), ReadFailure::Truncated);
        assert_eq!(classify(8), ReadFailure::ShortRead);
        assert_eq!(classify(99), ReadFailure::Unavailable);
    }
    #[test]
    fn short_reads_keep_the_frame_cause() {
        // An encrypted frame after verified frames (the 70450 Item/ItemSparse/Map
        // case) is partial encryption, never corruption.
        assert_eq!(classify_read(2, 302_156), ReadFailure::EncryptedFrames);
        assert_eq!(classify_read(2, 0), ReadFailure::Encrypted);
        // A checksum mismatch stays a verification failure at any offset.
        assert_eq!(classify_read(4, 0), ReadFailure::Invalid);
        assert_eq!(classify_read(4, 4096), ReadFailure::Invalid);
        assert_eq!(classify_read(7, 4096), ReadFailure::Truncated);
        assert_eq!(classify_read(8, 4096), ReadFailure::ShortRead);
        assert_eq!(classify_read(6, 4096), ReadFailure::Unavailable);
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod boundary_tests;
