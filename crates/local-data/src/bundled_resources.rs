//! Bounded installed-resource I/O; no ancestor-race sandbox is claimed.
use crate::Result;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
pub const MAX_RESOURCE_BYTES: u64 = 576 * 1024 * 1024;
pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
fn checked<T>(
    path: &Path,
    maximum: u64,
    read: impl FnOnce(&mut fs::File) -> Result<T>,
) -> Result<T> {
    let selected = crate::checked_path(path)?;
    let metadata =
        fs::symlink_metadata(&selected).map_err(|_| "Cannot inspect bundled resource")?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum {
        return Err("Invalid bundled resource size/type");
    }
    let before = crate::stamp(&metadata)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut file = options
        .open(&selected)
        .map_err(|_| "Cannot open bundled resource")?;
    let opened = file
        .metadata()
        .map_err(|_| "Cannot inspect bundled resource handle")?;
    if !opened.is_file() || crate::linked(&opened) || crate::stamp(&opened)? != before {
        return Err("Bundled resource changed before reading");
    }
    let result = read(&mut file)?;
    if crate::checked_path(path)? != selected
        || crate::stamp(
            &file
                .metadata()
                .map_err(|_| "Cannot inspect bundled resource handle")?,
        )? != before
        || crate::stamp(
            &fs::symlink_metadata(&selected).map_err(|_| "Cannot inspect bundled resource")?,
        )? != before
    {
        return Err("Bundled resource changed while reading");
    }
    Ok(result)
}
pub fn read(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    if maximum > MAX_DOCUMENT_BYTES {
        return Err("Bundled document limit exceeds bound");
    }
    checked(path, maximum as u64, |file| {
        let mut bytes = Vec::new();
        file.take(maximum as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read bundled document")?;
        if bytes.len() > maximum {
            return Err("Bundled document grew beyond bounds");
        }
        Ok(bytes)
    })
}
pub fn verify(path: &Path, expected_bytes: u64, expected_sha256: &str) -> Result<()> {
    if expected_bytes == 0 || expected_bytes > MAX_RESOURCE_BYTES {
        return Err("Bundled resource limit exceeds bound");
    }
    checked(path, expected_bytes, |file| {
        let mut bytes = [0; 65536];
        let mut total = 0u64;
        let mut hash = Sha256::new();
        loop {
            let n = file
                .read(&mut bytes)
                .map_err(|_| "Cannot read bundled resource")?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > expected_bytes {
                return Err("Bundled resource grew beyond bounds");
            }
            hash.update(&bytes[..n]);
        }
        if total != expected_bytes || format!("{:x}", hash.finalize()) != expected_sha256 {
            return Err("Bundled resource digest/size differs");
        }
        Ok(())
    })
}
