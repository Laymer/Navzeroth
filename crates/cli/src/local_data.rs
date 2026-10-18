//! Explicit local-data CLI dispatch. No operation implicitly activates or signs.
use azeroth_core::local_data::PerformancePreset;
use azeroth_local_data::worker::{ExtractionRequest, ExtractionWorker, WorkerEvent};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::Path,
    process::ExitCode,
};

fn emit<T: serde::Serialize>(result: Result<T, &'static str>) -> ExitCode {
    match result {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("local-data JSON")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", serde_json::json!({"format_version":1,"error":error}));
            ExitCode::from(1)
        }
    }
}
fn preset(value: Option<&&OsStr>) -> Option<PerformancePreset> {
    match value.map(|s| s.to_str()) {
        None | Some(Some("max")) => Some(PerformancePreset::Max),
        Some(Some("small")) => Some(PerformancePreset::Small),
        Some(Some("balanced")) => Some(PerformancePreset::Balanced),
        Some(Some("large")) => Some(PerformancePreset::Large),
        _ => None,
    }
}

pub fn run(args: &[OsString]) -> ExitCode {
    let usage = || {
        println!(
            "{}",
            serde_json::json!({"format_version":1,"error":"usage","usage":"local-data (select|inspect) --installation <directory> --product <product>; local-data (plan|extract) --installation <directory> --product <product> --destination <private-directory> [--preset small|balanced|large|max] [extract: --tables <comma-separated-names>]; local-data (verify|coverage|status) --destination <private-directory> --catalog <sha256>; local-data (cache-stage|cache-stage-fixture|observation-stage) --destination <private-directory> --catalog <sha256> --input <selected-file> --kind <hotfix|quest|creature|game-object|item|snapshot|session> --locale <locale> [--preset small|balanced|large|max]; local-data input-verify --destination <private-directory> --input <sha256>; local-data input-page --destination <private-directory> --input <sha256> --offset <number> --limit <1-200>; local-data project --destination <private-directory> --catalog <sha256> --locale <locale>; local-data projection-verify --destination <private-directory> --projection <sha256>; local-data (jobs|selections) --destination <private-directory>; local-data (status|cancel|resume|forget) --destination <private-directory> --job <id>; local-data active --destination <private-directory> --catalog <sha256>; local-data (activate|rollback) --destination <private-directory> --catalog <sha256> --expected <generation>; local-data receipt-public-key --secret-key <file>; local-data receipt-sign --payload <file> --policy <file> --key-id <id> --secret-key <file>; local-data receipt-assemble --payload <file> --operator <file> --reviewer <file>; local-data receipt-verify --receipt <file> --policy <file> --candidate <commit> --production <sha256>"})
        );
        ExitCode::from(2)
    };
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        return usage();
    };
    let allowed: &[&str] = match command {
        "cache-stage" | "cache-stage-fixture" | "observation-stage" => &[
            "--destination",
            "--catalog",
            "--input",
            "--kind",
            "--locale",
            "--preset",
        ],
        "input-verify" => &["--destination", "--input"],
        "input-page" => &["--destination", "--input", "--offset", "--limit"],
        "inspect" => &["--installation", "--product"],
        "plan" | "select" => &["--installation", "--product", "--destination", "--preset"],
        "extract" => &[
            "--installation",
            "--product",
            "--destination",
            "--preset",
            "--tables",
            "--job",
            "--selection",
        ],
        "verify" | "coverage" => &["--destination", "--catalog"],
        "status" => &["--destination", "--catalog", "--job"],
        "jobs" | "selections" => &["--destination"],
        "cancel" | "resume" | "forget" => &["--destination", "--job"],
        "active" => &["--destination", "--catalog"],
        "activate" | "rollback" => &["--destination", "--catalog", "--expected"],
        "project" => &["--destination", "--catalog", "--locale"],
        "projection-verify" => &["--destination", "--projection"],
        "receipt-public-key" => &["--secret-key"],
        "receipt-sign" => &["--payload", "--policy", "--key-id", "--secret-key"],
        "receipt-assemble" => &["--payload", "--operator", "--reviewer"],
        "receipt-verify" => &["--receipt", "--policy", "--candidate", "--production"],
        _ => return usage(),
    };
    let (pairs, remainder) = args[1..].as_chunks::<2>();
    let mut options = BTreeMap::new();
    if !remainder.is_empty() {
        return usage();
    }
    for pair in pairs {
        let Some(key) = pair[0].to_str() else {
            return usage();
        };
        if !allowed.contains(&key)
            || pair[1].is_empty()
            || pair[1].to_str().is_some_and(|s| s.starts_with("--"))
            || options.insert(key, pair[1].as_os_str()).is_some()
        {
            return usage();
        }
    }
    if matches!(
        command,
        "cache-stage" | "cache-stage-fixture" | "observation-stage" | "input-verify" | "input-page"
    ) {
        return input_command(command, &options);
    }
    if matches!(
        command,
        "jobs" | "selections" | "cancel" | "resume" | "forget" | "active" | "activate" | "rollback"
    ) || (command == "status" && options.contains_key("--job"))
    {
        return lifecycle_command(command, &options);
    }
    if command.starts_with("receipt-") {
        if options.len() != allowed.len() {
            return usage();
        }
        return receipt(command, &options);
    }
    if matches!(command, "project" | "projection-verify") {
        if options.len() != allowed.len() {
            return usage();
        }
        return projection(command, &options);
    }
    if matches!(command, "verify" | "coverage" | "status") {
        let (Some(destination), Some(digest)) = (
            options.get("--destination"),
            options.get("--catalog").and_then(|s| s.to_str()),
        ) else {
            return usage();
        };
        let result = azeroth_local_data::verify_catalog(Path::new(destination), digest);
        if command == "coverage" {
            return emit(result.map(|catalog|{
                let mut dispositions=BTreeMap::<String,u32>::new();
                for table in &catalog.tables {let key=serde_json::to_value(table.disposition).unwrap().as_str().unwrap().to_owned();*dispositions.entry(key).or_default()+=1;}
                serde_json::json!({"format_version":1,"catalog_sha256":digest,"build":catalog.installation.build,"dispositions":dispositions,"gaps":catalog.gaps,"gameplay_projection_available":false,"complete_target_inventory":null})
            }));
        }
        return emit(result);
    }
    let saved = if command == "extract" {
        match options.get("--selection") {
            Some(id) => {
                if options.contains_key("--installation") || options.contains_key("--product") {
                    return usage();
                }
                let Some(destination) = options.get("--destination") else {
                    return usage();
                };
                match azeroth_local_data::lifecycle::index(Path::new(destination)).and_then(|db| {
                    db.selection(id.to_str().ok_or("Invalid selection")?)
                        .map_err(|_| "Unknown selection")
                }) {
                    Ok(s) => Some(s),
                    Err(e) => return emit::<()>(Err(e)),
                }
            }
            None => None,
        }
    } else {
        None
    };
    let installation_saved = saved.as_ref().map(|s| s.installation.as_os_str());
    let product_saved = saved.as_ref().map(|s| s.product.as_str());
    let (Some(installation), Some(product)) = (
        options
            .get("--installation")
            .copied()
            .or(installation_saved),
        options
            .get("--product")
            .and_then(|s| s.to_str())
            .or(product_saved),
    ) else {
        return usage();
    };

    if command == "inspect" {
        return emit(azeroth_local_data::inspect(
            Path::new(installation),
            product,
        ));
    }
    let (Some(destination), Some(preset)) = (
        options.get("--destination"),
        preset(options.get("--preset")).map(|p| {
            if options.contains_key("--preset") {
                p
            } else {
                saved.as_ref().map_or(p, |s| s.preset)
            }
        }),
    ) else {
        return usage();
    };
    if command == "select" {
        return emit(azeroth_local_data::lifecycle::select(
            Path::new(destination),
            Path::new(installation),
            product,
            preset,
        ));
    }
    if command == "plan" {
        return match azeroth_local_data::plan(
            Path::new(installation),
            product,
            Path::new(destination),
            preset,
        ) {
            Ok(plan) => {
                let _ = emit(Ok(plan));
                ExitCode::from(3)
            }
            Err(error) => emit::<()>(Err(error)),
        };
    }
    let tables = match options.get("--tables") {
        None => Vec::new(),
        Some(value) => match value.to_str() {
            Some(text) => text.split(',').map(str::to_owned).collect(),
            None => return usage(),
        },
    };
    let request_id = match options.get("--job") {
        Some(id) => match id.to_str() {
            Some(id) => id.into(),
            None => return usage(),
        },
        None => match azeroth_local_data::lifecycle::new_job_id() {
            Ok(id) => id,
            Err(e) => return emit::<()>(Err(e)),
        },
    };
    let request = ExtractionRequest {
        protocol_version: 1,
        resume: false,
        request_id,
        installation: Path::new(installation).to_path_buf(),
        product: product.into(),
        destination: Path::new(destination).to_path_buf(),
        preset,
        tables,
    };
    run_worker(&request)
}
fn run_worker(request: &ExtractionRequest) -> ExitCode {
    let mut worker = match ExtractionWorker::start(request) {
        Ok(worker) => worker,
        Err(error) => return emit::<()>(Err(error)),
    };
    loop {
        match worker.next_event() {
            Ok(WorkerEvent::Progress { .. }) => {}
            Ok(WorkerEvent::Complete { summary, .. }) => {
                let mut value = serde_json::to_value(summary).expect("extraction summary");
                value
                    .as_object_mut()
                    .expect("summary object")
                    .insert("job_id".into(), request.request_id.clone().into());
                let _ = emit(Ok(value));
                return ExitCode::from(3);
            }
            Ok(WorkerEvent::Failed { code, .. }) => {
                println!("{}", serde_json::json!({"format_version":1,"error":code}));
                return ExitCode::from(1);
            }
            Err(error) => return emit::<()>(Err(error)),
        }
    }
}

/// Research-only projections (task V2.2). `project` seals a projection and exits
/// 3 like extraction: the output is never verified for planning or activated.
fn projection(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    use azeroth_local_data::projection;
    let destination = Path::new(options["--destination"]);
    let text = |key: &str| options[key].to_str().ok_or("Invalid option value");
    if command == "project" {
        let result =
            (|| projection::project_catalog(destination, text("--catalog")?, text("--locale")?))();
        return match result {
            Ok(summary) => {
                let _ = emit(Ok(summary));
                ExitCode::from(3)
            }
            Err(error) => emit::<()>(Err(error)),
        };
    }
    // Exit 4: integrity checked, but a different projector sealed it, so the
    // records were not re-derived and the projection is not verified.
    match (|| projection::verify_projection(destination, text("--projection")?))() {
        Ok(report) => {
            let verified = report.verified;
            let _ = emit(Ok(report));
            if verified {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(4)
            }
        }
        Err(error) => emit::<()>(Err(error)),
    }
}

/// Bounded regular-file read for explicitly named receipt inputs.
fn read_small(path: &OsStr, limit: u64) -> Result<Vec<u8>, &'static str> {
    use std::io::Read;
    let named = std::fs::symlink_metadata(path).map_err(|_| "Receipt input cannot be read")?;
    if !named.is_file() {
        return Err("Receipt input must be a small regular file");
    }
    // Check the opened handle too and bound the actual read, so a file swapped
    // after the first check can't grow, block or redirect the read.
    let file = std::fs::File::open(path).map_err(|_| "Receipt input cannot be read")?;
    let opened = file
        .metadata()
        .map_err(|_| "Receipt input cannot be read")?;
    if !opened.is_file() || opened.len() > limit {
        return Err("Receipt input must be a small regular file");
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Receipt input cannot be read")?;
    if bytes.len() as u64 > limit {
        return Err("Receipt input must be a small regular file");
    }
    Ok(bytes)
}
/// Explicit receipt steps (task V1.2). Signing uses a caller-supplied key file
/// and a policy that already trusts it; nothing is generated or activated.
fn receipt(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    use azeroth_local_data::receipts;
    const LIMIT: u64 = 128 * 1024;
    let file = |key: &str| read_small(options[key], LIMIT);
    match command {
        "receipt-public-key" => emit((|| {
            let secret = receipts::read_secret_key(&read_small(options["--secret-key"], 1024)?)?;
            Ok(
                serde_json::json!({"format_version":1,"public_key_hex":receipts::public_key_hex(&secret)}),
            )
        })()),
        "receipt-sign" => emit((|| {
            let payload = receipts::read_payload(&file("--payload")?)?;
            let policy = receipts::policy(&file("--policy")?)?;
            let secret = receipts::read_secret_key(&read_small(options["--secret-key"], 1024)?)?;
            let key_id = options["--key-id"].to_str().ok_or("Invalid key id")?;
            receipts::sign(&payload, &policy, key_id, &secret)
        })()),
        "receipt-assemble" => {
            let result = (|| {
                let payload = receipts::read_payload(&file("--payload")?)?;
                let operator = receipts::read_signature(&file("--operator")?)?;
                let reviewer = receipts::read_signature(&file("--reviewer")?)?;
                receipts::assemble(payload, operator, reviewer)
            })();
            match result {
                // Canonical compact bytes: `verify` refuses any other encoding.
                Ok(bytes) => {
                    use std::io::Write;
                    // Exact bytes, no trailing newline, so `> file` verifies.
                    let mut out = std::io::stdout().lock();
                    if out.write_all(&bytes).and_then(|()| out.flush()).is_err() {
                        return ExitCode::from(1);
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => emit::<()>(Err(error)),
            }
        }
        _ => emit((|| {
            let bytes = file("--receipt")?;
            let policy = receipts::policy(&file("--policy")?)?;
            let candidate = options["--candidate"].to_str().ok_or("Invalid candidate")?;
            let production = options["--production"]
                .to_str()
                .ok_or("Invalid production")?;
            receipts::verify_hybrid(&bytes, &policy, candidate, production)
                .map(|r| serde_json::json!({"format_version":1,"verified":true,"private_catalog_sha256":r.payload.private_catalog_sha256}))
        })()),
    }
}

fn lifecycle_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    use azeroth_local_data::lifecycle;
    let Some(destination) = options.get("--destination") else {
        return emit::<()>(Err("Destination is required"));
    };
    let root = Path::new(destination);
    if command == "jobs" && options.len() == 1 {
        return emit(lifecycle::jobs(root));
    }
    if command == "selections" && options.len() == 1 {
        return emit(lifecycle::selections(root));
    }
    if let Some(id) = options.get("--job").and_then(|s| s.to_str()) {
        if options.len() != 2 {
            return emit::<()>(Err("Expected destination and job only"));
        }
        return match command {
            "status" => emit(lifecycle::status(root, id)),
            "cancel" => emit(lifecycle::cancel(root, id)),
            "forget" => emit(lifecycle::index(root).and_then(|mut db| {
                db.forget(id)
                    .map_err(|_| "Cannot forget active or unknown job")
            })),
            "resume" => match lifecycle::resume_request(root, id) {
                Ok(request) => run_worker(&request),
                Err(e) => emit::<()>(Err(e)),
            },
            _ => emit::<()>(Err("Unsupported lifecycle command")),
        };
    }
    let Some(digest) = options.get("--catalog").and_then(|s| s.to_str()) else {
        return emit::<()>(Err("Catalog identity is required"));
    };
    if command == "active" && options.len() == 2 {
        return emit(lifecycle::active(root, digest));
    }
    let expected = options
        .get("--expected")
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|s| s.parse::<u64>().ok());
    match expected {
        Some(expected) if options.len() == 3 => emit(lifecycle::activate(
            root,
            digest,
            expected,
            command == "rollback",
        )),
        _ => emit::<()>(Err("Expected activation generation is required")),
    }
}

fn input_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    use azeroth_core::local_data_inputs::InputKind;
    use azeroth_local_data::inputs;
    let result = (|| {
        let value = |name| {
            options
                .get(name)
                .and_then(|v| v.to_str())
                .ok_or("Missing or invalid input option")
        };
        let destination = Path::new(
            options
                .get("--destination")
                .ok_or("Missing private destination")?,
        );
        let input = options.get("--input").ok_or("Missing selected input")?;
        if command == "input-verify" {
            return serde_json::to_value(inputs::verify(destination, value("--input")?)?)
                .map_err(|_| "Cannot encode input report");
        }
        if command == "input-page" {
            let offset = value("--offset")?
                .parse()
                .map_err(|_| "Invalid input offset")?;
            let limit = value("--limit")?
                .parse()
                .map_err(|_| "Invalid input limit")?;
            return serde_json::to_value(inputs::cache_page(
                destination,
                value("--input")?,
                offset,
                limit,
            )?)
            .map_err(|_| "Cannot encode input page");
        }
        let kind = match value("--kind")? {
            "hotfix" => InputKind::HotfixCache,
            "quest" => InputKind::QuestCache,
            "creature" => InputKind::CreatureCache,
            "game-object" => InputKind::GameObjectCache,
            "item" => InputKind::ItemCache,
            "snapshot" => InputKind::CharacterSnapshot,
            "session" => InputKind::SessionEvents,
            _ => return Err("Unsupported selected input kind"),
        };
        let preset = preset(options.get("--preset")).ok_or("Invalid preset")?;
        let catalog = value("--catalog")?;
        let locale = value("--locale")?;
        let staged = match command {
            "cache-stage" => {
                inputs::stage_cache(destination, catalog, Path::new(input), kind, locale, preset)
            }
            "cache-stage-fixture" => inputs::stage_cache_fixture(
                destination,
                catalog,
                Path::new(input),
                kind,
                locale,
                preset,
            ),
            _ => inputs::stage_observation(
                destination,
                catalog,
                Path::new(input),
                kind,
                locale,
                preset,
            ),
        }?;
        serde_json::to_value(staged).map_err(|_| "Cannot encode input report")
    })();
    emit(result)
}
