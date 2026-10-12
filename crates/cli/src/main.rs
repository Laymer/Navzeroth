//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! Placeholder created by task M1.1 to prove azeroth-cli links against
//! azeroth-core inside the Cargo workspace. Real subcommands (doctor,
//! validate-fixtures, ...; plan section 19.3) land in later milestones.
//! Do not report any subcommand as implemented until it actually exists.

fn main() {
    println!(
        "{} {} - {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        azeroth_core::placeholder()
    );
    println!(
        "No subcommands are implemented yet. Planned commands are listed in \
         README.md and plan section 19.3."
    );
}
