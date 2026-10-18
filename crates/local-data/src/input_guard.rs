//! Process names only: no process memory, handles to games, or control actions.
use crate::Result;

pub(super) fn require_closed() -> Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_NO_MORE_FILES, GetLastError, INVALID_HANDLE_VALUE},
            System::Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
        };
        // SAFETY: owned snapshot handle, initialized fixed-size entry and bounded
        // UTF-16 executable-name field. No per-process access is requested.
        unsafe {
            let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if handle == INVALID_HANDLE_VALUE {
                return Err("Cannot establish closed-game precondition");
            }
            struct Snapshot(windows_sys::Win32::Foundation::HANDLE);
            impl Drop for Snapshot {
                fn drop(&mut self) {
                    unsafe {
                        CloseHandle(self.0);
                    }
                }
            }
            let _owned = Snapshot(handle);
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if Process32FirstW(handle, &mut entry) == 0 {
                return Err("Cannot establish closed-game precondition");
            }
            for _ in 0..65_536 {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .ok_or("Invalid process name")?;
                let name = String::from_utf16(&entry.szExeFile[..length])
                    .map_err(|_| "Invalid process name")?
                    .to_ascii_lowercase();
                if name.starts_with("wow") && name.ends_with(".exe") {
                    return Err("Mutable cache work waits until the game closes");
                }
                if Process32NextW(handle, &mut entry) == 0 {
                    return if GetLastError() == ERROR_NO_MORE_FILES {
                        Ok(())
                    } else {
                        Err("Cannot establish closed-game precondition")
                    };
                }
            }
            Err("Process inventory exceeds capacity")
        }
    }
    #[cfg(not(windows))]
    {
        Err("Reliable closed-game check unavailable on this platform; mutable caches deferred")
    }
}

pub(super) fn same_file(left: &std::fs::File, right: &std::fs::File) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, GetFileInformationByHandle,
        };
        fn identity(file: &std::fs::File) -> Result<(u32, u32, u32)> {
            // SAFETY: live owned file handle and correctly sized initialized output.
            unsafe {
                let mut info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
                if GetFileInformationByHandle(file.as_raw_handle(), &mut info) == 0
                    || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
                {
                    return Err("Cannot establish selected file identity");
                }
                Ok((
                    info.dwVolumeSerialNumber,
                    info.nFileIndexHigh,
                    info.nFileIndexLow,
                ))
            }
        }
        if identity(left)? != identity(right)? {
            return Err("Selected input path changed identity");
        }
    }
    #[cfg(not(windows))]
    {
        if crate::stamp(
            &left
                .metadata()
                .map_err(|_| "Cannot inspect selected handle")?,
        )? != crate::stamp(
            &right
                .metadata()
                .map_err(|_| "Cannot inspect selected handle")?,
        )? {
            return Err("Selected input path changed identity");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn native_process_inventory_establishes_a_closed_or_running_game_precondition() {
        // A real ToolHelp snapshot, executable names only. Running WoW must
        // remain a refusal; this test never asks for per-process handles.
        assert!(matches!(
            super::require_closed(),
            Ok(()) | Err("Mutable cache work waits until the game closes")
        ));
    }
    #[test]
    fn selected_handle_identity_rejects_a_different_same_size_file() {
        let root =
            std::env::temp_dir().join(format!("nav-input-file-identity-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("a"), b"same").unwrap();
        std::fs::write(root.join("b"), b"same").unwrap();
        let a = std::fs::File::open(root.join("a")).unwrap();
        let again = std::fs::File::open(root.join("a")).unwrap();
        let b = std::fs::File::open(root.join("b")).unwrap();
        assert!(super::same_file(&a, &again).is_ok());
        assert!(super::same_file(&a, &b).is_err());
        drop((a, again, b));
        std::fs::remove_dir_all(root).unwrap();
    }
}
