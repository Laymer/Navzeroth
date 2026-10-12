//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! The no-argument banner identifies the current capability. `export-schemas`
//! writes the generated exchange schema; `prepare-atlas` prepares map imagery
//! (ADR 0002). The remaining plan section 19.3 commands land in later
//! milestones. Unsupported arguments fail explicitly without echoing input.
use azeroth_cli::atlas::{self, Paths};
use azeroth_core::contract::schema::{SCHEMA_FILE, exchange_schema_text};
use std::{ffi::OsString, path::Path, process::ExitCode};

const USAGE: &str = "Unsupported command or options. Implemented: export-schemas --output <dir>; \
    prepare-atlas --manifest <file> --content-root <dir> [--output <dir> | --check-pointer]. See README.md.";

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

struct AtlasArgs {
    manifest: OsString,
    content_root: OsString,
    output: Option<OsString>,
    check_pointer: bool,
}
/// Exact option parsing: every option at most once, values never start with
/// `--`, no positional arguments. Any other shape is a usage error.
fn parse_atlas_args(args: &[OsString]) -> Option<AtlasArgs> {
    let (mut manifest, mut content_root, mut output, mut check_pointer) = (None, None, None, false);
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].to_str()?;
        let mut value = |slot: &mut Option<OsString>| -> Option<()> {
            let v = args.get(i + 1)?;
            if slot.is_some() || v.to_str().is_some_and(|s| s.starts_with("--")) {
                return None;
            }
            *slot = Some(v.clone());
            i += 2;
            Some(())
        };
        match flag {
            "--manifest" => value(&mut manifest)?,
            "--content-root" => value(&mut content_root)?,
            "--output" => value(&mut output)?,
            "--check-pointer" if !check_pointer => {
                check_pointer = true;
                i += 1;
            }
            _ => return None,
        }
    }
    if check_pointer && output.is_some() {
        return None;
    }
    Some(AtlasArgs {
        manifest: manifest?,
        content_root: content_root?,
        output,
        check_pointer,
    })
}
fn prepare_atlas(args: &AtlasArgs) -> ExitCode {
    let paths = Paths {
        manifest: Path::new(&args.manifest),
        content_root: Path::new(&args.content_root),
        output: args.output.as_deref().map(Path::new),
    };
    if args.check_pointer {
        return match atlas::check_pointers(&paths) {
            Ok(count) => {
                println!(
                    "Checked {count} Git LFS pointer(s) against the manifest and generated record"
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("prepare-atlas --check-pointer failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    match atlas::prepare(&paths) {
        Ok(generated) => {
            for layer in &generated.layers {
                println!(
                    "Wrote {} ({}x{}, divisor {}, sha256 {})",
                    layer.file.as_str(),
                    layer.width_px,
                    layer.height_px,
                    layer.frame_divisor,
                    layer.sha256.as_str()
                );
            }
            if let Some(results) = &generated.alignment {
                for r in results {
                    println!(
                        "Alignment {}: offset ({}, {}) residual {:.2}",
                        r.layer.as_str(),
                        r.offset_px.x,
                        r.offset_px.y,
                        r.residual
                    );
                }
            }
            println!("Wrote {}", atlas::GENERATED_FILE);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("prepare-atlas failed: {error}");
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
                "Implemented subcommands: export-schemas --output <dir>; prepare-atlas --manifest \
                 <file> --content-root <dir> [--output <dir> | --check-pointer]. Other commands \
                 are planned; see README.md and plan section 19.3."
            );
            ExitCode::SUCCESS
        }
        [command, flag, dir] if command == "export-schemas" && flag == "--output" => {
            export_schemas(Path::new(dir))
        }
        [command, rest @ ..] if command == "prepare-atlas" => match parse_atlas_args(rest) {
            Some(parsed) => prepare_atlas(&parsed),
            None => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
