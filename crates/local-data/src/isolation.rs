//! Narrow OS boundary. Windows parent attaches a Job before sending work; Linux
//! worker applies an address-space ceiling before archive opening. Neither limit
//! is a promise about resident memory; decoded allocation bounds are separate.
use crate::Result;

// POSIX statvfs counter widths differ between supported native targets.
#[allow(clippy::unnecessary_cast)]
pub(crate) fn available_space(path: &std::path::Path) -> Result<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "Invalid storage path")?;
        let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: live NUL-terminated path and correctly sized output buffer.
        if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
            return Err("Free storage space unavailable");
        }
        // SAFETY: statvfs succeeded and initialized every output field.
        let info = unsafe { info.assume_init() };
        (info.f_bavail as u64)
            .checked_mul(info.f_frsize as u64)
            .ok_or("Free storage size overflow")
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut available = 0u64;
        // SAFETY: terminated UTF-16 selection and valid output pointer; optional
        // total-capacity outputs are explicitly null.
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                path.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err("Free storage space unavailable");
        }
        Ok(available)
    }
}

#[cfg(unix)]
pub(crate) fn worker_limit(bytes: u64) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: worker-only process control, no pointer arguments.
        if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } != 0
            || unsafe { libc::getppid() } == 1
        {
            return Err("Worker parent lifetime protection unavailable");
        }
    }
    let limit = libc::rlimit {
        rlim_cur: bytes,
        rlim_max: bytes,
    };
    // SAFETY: initialized rlimit pointer; affects only the already spawned worker.
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) } != 0 {
        return Err("Worker memory limit unavailable");
    }
    Ok(())
}
#[cfg(windows)]
pub(crate) fn worker_limit(_: u64) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(crate) struct ProcessLimit;
#[cfg(unix)]
impl ProcessLimit {
    pub(crate) fn attach(_: &std::process::Child, _: u64) -> Result<Self> {
        Ok(Self)
    }
}

#[cfg(windows)]
pub(crate) struct ProcessLimit(usize);
#[cfg(windows)]
impl ProcessLimit {
    pub(crate) fn attach(child: &std::process::Child, bytes: u64) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        // SAFETY: create unnamed job with default security, initialized info buffer,
        // borrowed child process handle. Ownership is retained until worker exits.
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err("Worker Job Object unavailable");
            }
            let job = Self(handle as usize);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY
                | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            info.BasicLimitInformation.ActiveProcessLimit = 1;
            info.ProcessMemoryLimit =
                usize::try_from(bytes).map_err(|_| "Worker memory exceeds platform capacity")?;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                return Err("Worker Job Object limit could not be applied");
            }
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessLimit {
    fn drop(&mut self) {
        // SAFETY: one owned Job handle, no use after close.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(
                self.0 as windows_sys::Win32::Foundation::HANDLE,
            );
        }
    }
}
