//! Development and diagnostic CLI for Azeroth Navigator.
//!
//! The no-argument banner identifies the current capability. `export-schemas`
//! writes the generated exchange schema; `doctor` reports selected path kinds;
//! `build-content`/`verify-pack` handle local reference packs; `prepare-atlas` prepares map imagery
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

const USAGE: &str = "Unsupported command or options. Implemented: bundle-receipt-(sign|assemble|verify) <explicit files, see BUNDLED_CONTENT_RECEIPTS.md>; local-data (inspect|plan) <options, see docs/data/LOCAL_DATA.md>; inspect-capture --input <SavedVariables-file>; doctor [--json] [--saved-variables <file>] [--addons <dir>] [--content <dir>]; build-content --input <dir> --output <dir>; build-content-v2 --input <dir> --output <dir>; verify-pack --input <dir>; export-schemas --output <dir>; \
    prepare-atlas --manifest <file> --content-root <dir> [--output <dir> | --check-pointer]; \
    validate-fixtures; validate-content --input <file-or-dir> [--json]; replay --fixture <file>; \
    inventory-report --root <repository> --input <relative-file>; \
    (release-readiness|candidate-readiness|release-build-readiness|release-draft-readiness) --root <repository> --version <version> [--evidence <relative-file>]; \
    release-materials --root <repository> --version <version> [--evidence <relative-file>] [--output <new-directory>]; \
    stage-questiedb <raw-file options>; stage-foreverdb --input <directory> --manifest <file>; \
    guide verify --guide <file> --facts <file> [--race <Race>] [--class <Class>]; \
    verify-world-data --input <dir> [--manifest <file>]. See README.md.";

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

fn pack_result(result: Result<azeroth_cli::pack::Report, &'static str>) -> ExitCode {
    match result {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("safe report serialization")
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("content-pack: {message}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.as_slice()
        == [std::ffi::OsString::from(
            azeroth_local_data::worker::WORKER_ARGUMENT,
        )]
    {
        return ExitCode::from(azeroth_local_data::worker::entry() as u8);
    }
    if args.as_slice()
        == [std::ffi::OsString::from(
            azeroth_local_data::media::WORKER_ARGUMENT,
        )]
    {
        return ExitCode::from(azeroth_local_data::media::worker_entry() as u8);
    }
    match args.as_slice() {
        [command, rest @ ..]
            if command == "bundle-receipt-sign"
                || command == "bundle-receipt-assemble"
                || command == "bundle-receipt-verify" =>
        {
            azeroth_cli::bundled_receipts::run(
                command.to_str().expect("ASCII receipt command"),
                rest,
            )
        }
        [command, rest @ ..] if command == "local-data" => azeroth_cli::local_data::run(rest),
        [command, flag, input] if command == "inspect-capture" && flag == "--input" => {
            match azeroth_cli::capture::inspect_file(Path::new(input)) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report).expect("report serialization")
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("inspect-capture: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [] => {
            println!(
                "{} {} - domain and exchange contracts available",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            );
            println!(
                "Implemented subcommands: inspect-capture --input <SavedVariables-file>; doctor; build-content --input <dir> --output <dir>; build-content-v2 --input <dir> --output <dir>; verify-pack --input <dir>; export-schemas --output <dir>; prepare-atlas --manifest \
                 <file> --content-root <dir> [--output <dir> | --check-pointer]; validate-fixtures; replay --fixture <file>. \
                 (release-readiness|candidate-readiness|release-build-readiness|release-draft-readiness) --root <repository> --version <version> [--evidence <relative-file>]. \
                 release-materials --root <repository> --version <version> [--evidence <relative-file>] [--output <new-directory>]. \
                 stage-questiedb <raw-file options> (see docs/data/QUESTIEDB_STAGING.md); stage-foreverdb --input <dir> --manifest <file> (see docs/data/FOREVERDB_STAGING.md); guide verify --guide <file> --facts <file> (see docs/data/GUIDE_VERIFIER.md); verify-world-data --input <dir> [--manifest <file>] (see docs/data/WORLD_DATA.md). \
                 Other commands are planned; see README.md and plan section 19.3."
            );
            ExitCode::SUCCESS
        }
        [command, flag, dir] if command == "export-schemas" && flag == "--output" => {
            export_schemas(Path::new(dir))
        }
        [command, input_flag, input, output_flag, output]
            if command == "build-content"
                && input_flag == "--input"
                && output_flag == "--output" =>
        {
            pack_result(azeroth_cli::pack::build(
                Path::new(input),
                Path::new(output),
            ))
        }
        [command, input_flag, input, output_flag, output]
            if command == "build-content-v2"
                && input_flag == "--input"
                && output_flag == "--output" =>
        {
            pack_result(azeroth_cli::pack::v2::build(
                Path::new(input),
                Path::new(output),
            ))
        }
        [command, flag, input] if command == "verify-pack" && flag == "--input" => {
            pack_result(azeroth_cli::pack::verify(Path::new(input)))
        }
        [command, rest @ ..] if command == "doctor" => {
            let Some(options) = azeroth_cli::doctor::Options::parse(rest) else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            let report = azeroth_cli::doctor::report(&options);
            if options.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("report serialization")
                );
            } else {
                print!("{}", report.human());
            }
            if report.passes() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        [command, rest @ ..] if command == "inventory-report" => {
            let Some(options) = azeroth_cli::release_inventory::Options::parse(rest) else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            match azeroth_cli::release_inventory::report(&options) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .expect("inventory report serialization")
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("inventory-report: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..]
            if command == "release-readiness"
                || command == "candidate-readiness"
                || command == "release-build-readiness"
                || command == "release-draft-readiness" =>
        {
            let Some(options) = azeroth_cli::release::Options::parse(rest) else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            match azeroth_cli::release::report(&options) {
                Ok(mut report) => {
                    if command == "release-build-readiness" || command == "release-draft-readiness"
                    {
                        let phase = if command == "release-build-readiness" {
                            azeroth_core::release::BuildPhase::Prebuild
                        } else {
                            azeroth_core::release::BuildPhase::Draft
                        };
                        let prepared = azeroth_cli::release::BuildReport::new(report, phase);
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&prepared)
                                .expect("build report serialization")
                        );
                        return if prepared.build_ready {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::FAILURE
                        };
                    }
                    let candidate = command == "candidate-readiness";
                    let allowed = if candidate {
                        report.readiness.candidate_ready
                    } else {
                        report.readiness.publication_allowed
                    };
                    if candidate {
                        report.readiness.publication_allowed = false;
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .expect("release report serialization")
                    );
                    if allowed {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(message) => {
                    eprintln!("release-readiness: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..] if command == "enumerate-release-packs" => {
            use azeroth_cli::release::enumeration;
            let Some(args) = enumeration::Arguments::parse(rest) else {
                eprintln!(
                    "enumerate-release-packs --root <dir> --pack <relative-dir> [--pack <relative-dir>] --output <new-relative-json>"
                );
                return ExitCode::from(2);
            };
            match enumeration::run(&args) {
                Ok(summary) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&summary).expect("enumeration summary")
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("enumerate-release-packs: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..] if command == "release-materials" => {
            use azeroth_cli::release::materials;
            let Some(options) = materials::Arguments::parse(rest) else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            let result = match &options.output {
                Some(output) => materials::stage(&options.release, output),
                None => materials::plan(&options.release),
            };
            match result.and_then(|manifest| manifest.bytes()) {
                Ok(bytes) => {
                    use std::io::Write;
                    if std::io::stdout().lock().write_all(&bytes).is_ok() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(message) => {
                    eprintln!("release-materials: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..] if command == "source-sync" || command == "source-build" => {
            let build = command == "source-build";
            let Some(options) = azeroth_cli::source_pipeline::Options::parse(rest, build) else {
                eprintln!("{}", azeroth_cli::source_pipeline::USAGE);
                return ExitCode::from(2);
            };
            let result = if build {
                azeroth_cli::source_pipeline::build(&options).and_then(|r| {
                    serde_json::to_string_pretty(&r)
                        .map_err(|_| "Cannot encode source build report")
                })
            } else {
                azeroth_cli::source_pipeline::sync(&options).and_then(|r| {
                    serde_json::to_string_pretty(&r)
                        .map_err(|_| "Cannot encode source closure report")
                })
            };
            match result {
                Ok(report) => {
                    println!("{report}");
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::from(1)
                }
            }
        }
        [command, rest @ ..] if command == "stage-foreverdb" => {
            let Some(options) = azeroth_cli::foreverdb::Options::parse(rest) else {
                eprintln!("Unsupported options. {}", azeroth_cli::foreverdb::USAGE);
                return ExitCode::from(2);
            };
            match azeroth_cli::foreverdb::report(&options) {
                Ok(report) => {
                    use std::io::Write;
                    let mut bytes =
                        serde_json::to_vec_pretty(&report).expect("staging report serialization");
                    bytes.push(b'\n');
                    if std::io::stdout().lock().write_all(&bytes).is_ok() {
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("Staging report cannot be written");
                        ExitCode::FAILURE
                    }
                }
                Err(message) => {
                    eprintln!("stage-foreverdb: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..] if command == "stage-questiedb" => {
            let Some(options) = azeroth_cli::questiedb::Options::parse(rest) else {
                eprintln!("Unsupported options. {}", azeroth_cli::questiedb::USAGE);
                return ExitCode::from(2);
            };
            match azeroth_cli::questiedb::report(&options) {
                Ok(report) => {
                    use std::io::Write;
                    let mut bytes =
                        serde_json::to_vec_pretty(&report).expect("staging report serialization");
                    bytes.push(b'\n');
                    match std::io::stdout().lock().write_all(&bytes) {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(_) => {
                            eprintln!("Staging report cannot be written");
                            ExitCode::FAILURE
                        }
                    }
                }
                Err(message) => {
                    eprintln!("stage-questiedb: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..] if command == "guide" => {
            let Some(options) = azeroth_cli::guide::Options::parse(rest) else {
                eprintln!("Unsupported options. {}", azeroth_cli::guide::USAGE);
                return ExitCode::from(2);
            };
            match azeroth_cli::guide::verify(&options) {
                Ok(report) => {
                    use std::io::Write;
                    let mut bytes =
                        serde_json::to_vec_pretty(&report).expect("guide report serialization");
                    bytes.push(b'\n');
                    if std::io::stdout().lock().write_all(&bytes).is_err() {
                        eprintln!("Guide report cannot be written");
                        return ExitCode::FAILURE;
                    }
                    // 0: clean replay against facts that claim evidence; 3: clean
                    // replay against synthetic facts (never a pass); 1: not clean.
                    if report.passes {
                        ExitCode::SUCCESS
                    } else if report.replay_clean {
                        ExitCode::from(3)
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(message) => {
                    eprintln!("guide verify: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, rest @ ..] if command == "verify-world-data" => {
            use azeroth_cli::world_data;
            let Some(options) = world_data::Options::parse(rest) else {
                eprintln!("Unsupported options. {}", world_data::USAGE);
                return ExitCode::from(2);
            };
            match world_data::verify(&options) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .expect("world-data report serialization")
                    );
                    if report.passes {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(message) => {
                    eprintln!("verify-world-data: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        [command] if command == "validate-fixtures" => validate_fixtures(),
        [command, flag, path]
            if command == "replay"
                && flag == "--fixture"
                && !path.to_str().is_some_and(|p| p.starts_with("--")) =>
        {
            match azeroth_cli::replay::file(Path::new(path)) {
                Ok(bytes) => {
                    use std::io::Write;
                    match std::io::stdout().lock().write_all(&bytes) {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(_) => {
                            eprintln!("Replay output cannot be written");
                            ExitCode::FAILURE
                        }
                    }
                }
                Err(message) => {
                    eprintln!("replay: {message}");
                    ExitCode::FAILURE
                }
            }
        }
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
