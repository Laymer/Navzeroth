// Prevents an extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os().skip(1).collect::<Vec<_>>()
        == [std::ffi::OsString::from(
            azeroth_local_data::worker::WORKER_ARGUMENT,
        )]
    {
        std::process::exit(azeroth_local_data::worker::entry());
    }
    if std::env::args_os().skip(1).collect::<Vec<_>>()
        == [std::ffi::OsString::from(
            azeroth_local_data::media::WORKER_ARGUMENT,
        )]
    {
        std::process::exit(azeroth_local_data::media::worker_entry());
    }
    azeroth_desktop_lib::run()
}
