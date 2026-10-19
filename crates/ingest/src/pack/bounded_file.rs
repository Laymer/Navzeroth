//! Descriptor-bound pack reads. Component inspection/revalidation refuses ordinary
//! changes; it is not an ancestor-race sandbox or an assurance of filesystem isolation.
use super::{Res, path_checked};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};
#[derive(PartialEq)]
struct Stamp {
    length: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
    #[cfg(windows)]
    identity: (u64, u32),
}
fn stamp(metadata: &fs::Metadata) -> Res<Stamp> {
    Ok(Stamp {
        length: metadata.len(),
        modified: metadata
            .modified()
            .map_err(|_| "Pack modification identity unavailable")?,
        #[cfg(unix)]
        identity: {
            use std::os::unix::fs::MetadataExt;
            (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            )
        },
        #[cfg(windows)]
        identity: {
            use std::os::windows::fs::MetadataExt;
            (metadata.creation_time(), metadata.file_attributes())
        },
    })
}
fn components(path: &Path) -> Res<Vec<(PathBuf, Stamp)>> {
    path_checked(path, false)?;
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| "Current directory unavailable")?
            .join(path)
    };
    let mut cursor = PathBuf::new();
    let mut result = Vec::new();
    for part in absolute.components() {
        if matches!(part, Component::CurDir) {
            continue;
        }
        cursor.push(part.as_os_str());
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&cursor).map_err(|_| "Pack component unavailable")?;
        let mut identity = stamp(&metadata)?;
        if metadata.is_dir() {
            // Unrelated children changing a shared ancestor must not reject a
            // read. Retain directory identity, not its mutable entry statistics.
            identity.length = 0;
            identity.modified = SystemTime::UNIX_EPOCH;
            #[cfg(unix)]
            {
                identity.identity.2 = 0;
                identity.identity.3 = 0;
            }
        }
        result.push((cursor.clone(), identity));
    }
    Ok(result)
}
fn regular(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    metadata.is_file() && !metadata.file_type().is_symlink()
}
pub(super) fn read(path: &Path, limit: usize) -> Res<Vec<u8>> {
    read_with(path, limit, |_| {})
}
fn read_with(path: &Path, limit: usize, mut checkpoint: impl FnMut(u8)) -> Res<Vec<u8>> {
    let before = components(path)?;
    let selected = before.last().ok_or("Pack path is empty")?;
    if selected.1.length > limit as u64 {
        return Err("Pack file exceeds size limit");
    }
    checkpoint(0);
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
        .open(path)
        .map_err(|_| "Pack file cannot be opened safely")?;
    let opened = file.metadata().map_err(|_| "Pack descriptor unavailable")?;
    if !regular(&opened) || stamp(&opened)? != selected.1 {
        return Err("Pack file changed before reading");
    }
    checkpoint(1);
    let mut bytes = Vec::new();
    (&mut file)
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| "Pack file cannot be read")?;
    if bytes.len() > limit {
        return Err("Pack file exceeds size limit");
    }
    checkpoint(2);
    if bytes.len() as u64 != opened.len()
        || stamp(&file.metadata().map_err(|_| "Pack descriptor unavailable")?)? != selected.1
        || components(path)? != before
    {
        return Err("Pack file changed while reading");
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "../../tests/support/pack_bounded_file.rs"]
mod tests;
