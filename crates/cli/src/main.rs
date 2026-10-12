//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! Placeholder created by task M1.1 to prove azeroth-cli links against
//! azeroth-core inside the Cargo workspace. Real subcommands (doctor,
//! validate-fixtures, ...; plan section 19.3) land in later milestones.
//! Do not report any subcommand as implemented until it actually exists.

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
