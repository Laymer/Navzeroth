//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! The no-argument banner identifies the current capability. `export-schemas`
//! writes the generated exchange schema; `prepare-atlas` prepares map imagery
//! (ADR 0002); `validate-fixtures` checks the contract and transport fixture
//! manifests. The remaining plan section 19.3 commands land in later
//! milestones. Unsupported arguments fail explicitly without echoing input.
use azeroth_cli::atlas::{self, Paths};
use azeroth_core::contract::schema::{SCHEMA_FILE, exchange_schema_text};
use azeroth_ingest::fixtures::{self, ManifestReport, Mismatch};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::ExitCode,
};

const USAGE: &str = "Unsupported command or options. Implemented: export-schemas --output <dir>; \
    prepare-atlas --manifest <file> --content-root <dir> [--output <dir> | --check-pointer]; \
    validate-fixtures; validate-content --input <file-or-dir> [--json]. See README.md.";

/// Fixture directories checked by `validate-fixtures`, relative to the repository root.
/// This list is explicit on purpose: the command never globs `fixtures/**`.
const CONTRACT_FIXTURES: &str = "fixtures/contracts/v1";
const TRANSPORT_FIXTURES: &str = "fixtures/transport";
/// Overrides the repository root for `validate-fixtures` (tests, relocated binaries).
const ROOT_ENV: &str = "AZEROTH_REPO_ROOT";

/// The repository root: `AZEROTH_REPO_ROOT` when set, otherwise the checkout this
/// binary was built from (`CARGO_MANIFEST_DIR/../..`, resolved at compile time).
fn repository_root() -> PathBuf {
    std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn report_manifest(label: &str, outcome: Result<ManifestReport, Vec<Mismatch>>) -> bool {
    match outcome {
        Ok(report) => {
            println!(
                "{label}: {} fixtures checked ({} valid, {} rejected as declared)",
                report.checked, report.valid, report.rejected
            );
            true
        }
        Err(problems) => {
            for problem in problems {
                if problem.file.is_empty() {
                    eprintln!("{label}: {}", problem.problem.message());
                } else {
                    eprintln!("{label}: {}: {}", problem.file, problem.problem.message());
                }
            }
            false
        }
    }
}

/// Checks exactly the two fixture manifests; fixture content is never printed.
fn validate_fixtures() -> ExitCode {
    let root = repository_root();
    let contracts = report_manifest(
        CONTRACT_FIXTURES,
        fixtures::check_contract_fixtures(&root.join(CONTRACT_FIXTURES)),
    );
    let transport = report_manifest(
        TRANSPORT_FIXTURES,
        fixtures::check_transport_fixtures(&root.join(TRANSPORT_FIXTURES)),
    );
    if contracts && transport {
        ExitCode::SUCCESS
    } else {
        eprintln!("validate-fixtures failed");
        ExitCode::FAILURE
    }
}

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
                 <file> --content-root <dir> [--output <dir> | --check-pointer]; validate-fixtures. \
                 Other commands are planned; see README.md and plan section 19.3."
            );
            ExitCode::SUCCESS
        }
        [command, flag, dir] if command == "export-schemas" && flag == "--output" => {
            export_schemas(Path::new(dir))
        }
        [command] if command == "validate-fixtures" => validate_fixtures(),
        [command, flag, input, rest @ ..]
            if command == "validate-content"
                && flag == "--input"
                && (rest.is_empty() || rest == [OsString::from("--json")]) =>
        {
            match azeroth_cli::content::reports(Path::new(input)) {
                Ok(reports) => {
                    if rest.is_empty() {
                        print!("{}", azeroth_cli::content::human(&reports));
                    } else {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&reports).expect("report serialization")
                        );
                    }
                    if reports.iter().all(|report| report.passes) {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(message) => {
                    eprintln!("validate-content: {message}");
                    ExitCode::FAILURE
                }
            }
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
