// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    if std::env::args_os().skip(1).collect::<Vec<_>>()
        == [std::ffi::OsString::from(
            azeroth_local_data::worker::WORKER_ARGUMENT,
        )]
    {
        return exit_code(azeroth_local_data::worker::entry());
    }
    if std::env::args_os().skip(1).collect::<Vec<_>>()
        == [std::ffi::OsString::from(
            azeroth_local_data::media::WORKER_ARGUMENT,
        )]
    {
        return exit_code(azeroth_local_data::media::worker_entry());
    }
    exit_code(azeroth_desktop_lib::run())
}

fn exit_code(code: i32) -> std::process::ExitCode {
    std::process::ExitCode::from(u8::try_from(code).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exit_status_preserves_success_and_worker_refusals() {
        assert_eq!(exit_code(0), std::process::ExitCode::SUCCESS);
        assert_eq!(exit_code(2), std::process::ExitCode::from(2));
        for code in [-1, 256, i32::MAX] {
            assert_eq!(exit_code(code), std::process::ExitCode::FAILURE);
        }
    }
}
