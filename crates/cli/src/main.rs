//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! The no-argument banner identifies the current capability. `export-schemas`
//! writes the generated exchange schema. The remaining plan section 19.3
//! commands land in later milestones. Unsupported arguments fail explicitly.
use azeroth_core::contract::schema::{SCHEMA_FILE, exchange_schema_text};
use std::{path::Path, process::ExitCode};

fn export_schemas(output: &Path) -> ExitCode {
    let path = output.join(SCHEMA_FILE);
    match std::fs::create_dir_all(output)
        .and_then(|()| std::fs::write(&path, exchange_schema_text()))
    {
        Ok(()) => {
            println!("Wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Could not write the schema file: {}", error.kind());
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!(
                "{} {} - domain and exchange contracts available",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            );
            println!(
                "Implemented subcommand: export-schemas --output <dir>. Other commands \
                 are planned; see README.md and plan section 19.3."
            );
            ExitCode::SUCCESS
        }
        [command, flag, dir] if command == "export-schemas" && flag == "--output" => {
            export_schemas(Path::new(dir))
        }
        _ => {
            eprintln!(
                "Unsupported command or options. Only export-schemas --output <dir> is implemented. See README.md."
            );
            ExitCode::from(2)
        }
    }
}
