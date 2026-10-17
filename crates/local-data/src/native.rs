//! The only archive FFI boundary. Handles never leave this module or move between
//! threads. The worker is the sole production caller. All native access is serialized.
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
    /// The archive reported end-of-data or a missing file part before the
    /// declared content size (local data incomplete, not asserted corrupt).
    Truncated,
    /// The read returned fewer bytes than declared with no recorded cause.
    ShortRead,
    Capacity,
}

pub(crate) struct Archive {
    handle: NonNull<c_void>,
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
        if unsafe { nav_casc_stats(self.handle.as_ptr(), &mut features, &mut local, &mut total) }
            != 0
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
        let serial = NATIVE.lock().map_err(|_| "Native reader unavailable")?;
        let path = native_text(path.as_os_str())?;
        let product = native_text(std::ffi::OsStr::new(product))?;
        let mut handle = std::ptr::null_mut();
        // SAFETY: both text buffers are NUL-terminated, live throughout the call;
        // shim initializes the output handle and copies selected strings internally.
        let error = unsafe {
            nav_casc_open(
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
    ) -> Result<Vec<u8>, ReadFailure> {
        if id == Some(0) {
            return Err(ReadFailure::Invalid);
        }
        let (mut file, mut size) = (std::ptr::null_mut(), 0);
        // SAFETY: owned live storage; valid output pointers; shim closes on failure.
        let mut error = if let Some(id) = id {
            unsafe { nav_casc_file_open(self.handle.as_ptr(), id, &mut file, &mut size) }
        } else {
            1
        };
        if error == 1 {
            let name = CString::new(format!("DBFilesClient\\{name}.{extension}"))
                .map_err(|_| ReadFailure::Invalid)?;
            // SAFETY: static virtual prefix and reviewed table identifier, NUL-terminated.
            error = unsafe {
                nav_casc_name_open(self.handle.as_ptr(), name.as_ptr(), &mut file, &mut size)
            };
        }
        if error != 0 {
            return Err(classify(error));
        }
        let file = NativeFile(NonNull::new(file).ok_or(ReadFailure::Invalid)?);
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
            nav_casc_read(
                file.0.as_ptr(),
                bytes.as_mut_ptr().cast(),
                size as u32,
                &mut done,
            )
        };
        if error != 0 {
            // The partial buffer is dropped here; only the cause is reported.
            return Err(classify_read(error, done));
        }
        Ok(bytes)
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
struct NativeFile(NonNull<c_void>);
impl Drop for NativeFile {
    fn drop(&mut self) {
        // SAFETY: exactly one owner; the storage outlives this temporary file.
        unsafe {
            nav_casc_file_close(self.0.as_ptr());
        }
    }
}
impl Drop for Archive {
    fn drop(&mut self) {
        // SAFETY: no files escape read(), unique live storage handle.
        unsafe {
            nav_casc_close(self.handle.as_ptr());
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
