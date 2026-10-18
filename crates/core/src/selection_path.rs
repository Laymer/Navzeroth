//! Lexical prefix rule shared by the selection boundaries (the pack builder's
//! `path_checked` and local-data's `checked_path`). Pure: no file-system access.
//! The callers still inspect every component for traversal, links and kind.
use std::path::{Component, Path, Prefix};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixRefusal {
    /// `\\.\pipe\x`, `\\.\C:\...`, `\\?\GLOBALROOT\...` or another prefix that is
    /// not a drive letter or network share.
    DeviceNamespace,
    /// `C:foo` or a bare `C:` (also verbatim): it names a volume or resolves
    /// against a per-drive current directory, not a rooted path.
    DriveRelative,
}

/// Accept a path with no prefix, or with a drive-letter or network-share prefix,
/// plain or verbatim. A drive prefix must be followed by a root. A share prefix
/// is always absolute (`\\?\UNC\server\share` with no trailing separator is the
/// share root).
pub fn check_prefix(path: &Path) -> Result<(), PrefixRefusal> {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Ok(());
    };
    match prefix.kind() {
        Prefix::Disk(_) | Prefix::VerbatimDisk(_) => match components.next() {
            Some(Component::RootDir) => Ok(()),
            _ => Err(PrefixRefusal::DriveRelative),
        },
        Prefix::UNC(..) | Prefix::VerbatimUNC(..) => Ok(()),
        Prefix::Verbatim(_) | Prefix::DeviceNS(_) => Err(PrefixRefusal::DeviceNamespace),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_without_a_prefix_pass() {
        for path in ["/", "/a/b", "a/b", ""] {
            assert_eq!(check_prefix(Path::new(path)), Ok(()), "{path}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn drive_and_share_prefixes_pass_devices_and_drive_relative_paths_are_refused() {
        for path in [
            r"C:\",
            r"C:\a",
            r"\\?\C:\a",
            r"\\server\share",
            r"\\server\share\a",
            r"\\?\UNC\server\share",
            r"\\?\UNC\server\share\a",
        ] {
            assert_eq!(check_prefix(Path::new(path)), Ok(()), "{path}");
        }
        for path in [r"\\.\pipe\x", r"\\.\C:\Windows", r"\\?\GLOBALROOT\Device\x"] {
            assert_eq!(
                check_prefix(Path::new(path)),
                Err(PrefixRefusal::DeviceNamespace),
                "{path}"
            );
        }
        for path in ["C:", "C:a", r"C:a\b", r"\\?\C:"] {
            assert_eq!(
                check_prefix(Path::new(path)),
                Err(PrefixRefusal::DriveRelative),
                "{path}"
            );
        }
    }
}
