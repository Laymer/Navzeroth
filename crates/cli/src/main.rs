//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! The no-argument banner identifies the current capability. Real subcommands
//! (doctor, validate-fixtures, ...; plan section 19.3) land in later milestones.
//! Unsupported arguments fail explicitly.

fn main() -> std::process::ExitCode {
    if std::env::args_os().nth(1).is_some() {
        eprintln!("No subcommands or options are implemented yet. See README.md.");
        return std::process::ExitCode::from(2);
    }
    println!(
        "{} {} - domain foundation available; no CLI commands implemented",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "No subcommands are implemented yet. Planned commands are listed in \
         README.md and plan section 19.3."
    );
    std::process::ExitCode::SUCCESS
}
