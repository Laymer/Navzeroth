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
            serde_json::json!({"format_version":1,"error":"usage","usage":"local-data (select|inspect) --installation <directory> --product <product>; local-data (plan|extract) --installation <directory> --product <product> --destination <private-directory> [--preset small|balanced|large|max] [extract: --tables <comma-separated-names> --locales <sorted-comma-separated-installed-locales>]; local-data (verify|coverage|status) --destination <private-directory> --catalog <sha256>; local-data (cache-stage|cache-stage-fixture|observation-stage) --destination <private-directory> --catalog <sha256> --input <selected-file> --kind <hotfix|quest|creature|game-object|item|snapshot|session> --locale <locale> [--preset small|balanced|large|max]; local-data guide-facts-build --destination <private-directory> --projection <sha256> [--inputs <comma-separated-input-digests>] [--preset small|balanced|large|max]; local-data guide-facts-verify --destination <private-directory> --guide-facts <sha256>; local-data media-discover --destination <private-directory> --catalog <sha256> --locale <locale> [--offset <number> --limit <1-200>]; local-data media-extract --installation <directory> --destination <private-directory> --catalog <sha256> --locale <locale> --ids <sorted-comma-separated-IDs> [--preset small|balanced|large|max]; local-data media-verify --destination <private-directory> --media <sha256>; local-data media-page --destination <private-directory> --media <sha256> --offset <number> --limit <1-200>; local-data tooltip --destination <private-directory> --projection <sha256> --spell <id>; local-data delivery-build --destination <private-directory> --projection <sha256> [--media <sorted-digests>] [--inputs <sorted-digests>] [--guide-facts <sha256>]; local-data (delivery-verify|delivery-active) --destination <private-directory> --delivery <sha256>; local-data (delivery-activate|delivery-rollback) --destination <private-directory> --delivery <sha256> --expected <generation>; local-data delivery-diff --destination <private-directory> --before <sha256> --after <sha256>; local-data delivery-page --destination <private-directory> --delivery <sha256> --domain <domain-or-table> --offset <number> --limit <1-200>; local-data delivery-receipt-(sign|assemble|verify) <explicit receipt options, see LOCAL_DELIVERY.md>; local-data inventory-build --destination <private-directory> --delivery <sha256> [--authored-pack <verified-directory>]; local-data (inventory-verify|route-designs) --destination <private-directory> --inventory <sha256> [--authored-pack <verified-directory>]; local-data inventory-page --destination <private-directory> --inventory <sha256> --domain <domain> --offset <number> --limit <1-200> [--authored-pack <verified-directory>]; local-data bound-guide-review --destination <private-directory> --delivery <sha256> --guide <file> --race <Race> --class <Class> --mode solo|duo; local-data input-verify --destination <private-directory> --input <sha256>; local-data input-page --destination <private-directory> --input <sha256> --offset <number> --limit <1-200>; local-data project --destination <private-directory> --catalog <sha256> --locale <locale>; local-data projection-verify --destination <private-directory> --projection <sha256>; local-data (jobs|selections) --destination <private-directory>; local-data (status|cancel|resume|forget) --destination <private-directory> --job <id>; local-data active --destination <private-directory> --catalog <sha256>; local-data (activate|rollback) --destination <private-directory> --catalog <sha256> --expected <generation>; local-data receipt-public-key --secret-key <file>; local-data receipt-sign --payload <file> --policy <file> --key-id <id> --secret-key <file>; local-data receipt-assemble --payload <file> --operator <file> --reviewer <file>; local-data receipt-verify --receipt <file> --policy <file> --candidate <commit> --production <sha256>"})
        );
        ExitCode::from(2)
    };
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        return usage();
    };
    let allowed: &[&str] = match command {
        "inventory-build" => &["--destination", "--delivery", "--authored-pack", "--preset"],
        "inventory-verify" | "route-designs" => {
            &["--destination", "--inventory", "--authored-pack"]
        }
        "inventory-page" => &[
            "--destination",
            "--inventory",
            "--authored-pack",
            "--domain",
            "--offset",
            "--limit",
        ],
        "bound-guide-review" => &[
            "--destination",
            "--delivery",
            "--guide",
            "--race",
            "--class",
            "--mode",
        ],
        "delivery-receipt-sign" => &["--payload", "--policy", "--key-id", "--secret-key"],
        "delivery-receipt-assemble" => &["--payload", "--operator", "--reviewer"],
        "delivery-receipt-verify" => &[
            "--receipt",
            "--policy",
            "--candidate",
            "--production",
            "--delivery",
        ],
        "delivery-build" => &[
            "--destination",
            "--projection",
            "--media",
            "--inputs",
            "--guide-facts",
            "--preset",
        ],
        "delivery-verify" | "delivery-active" => &["--destination", "--delivery"],
        "delivery-activate" | "delivery-rollback" => &["--destination", "--delivery", "--expected"],
        "delivery-diff" => &["--destination", "--before", "--after"],
        "delivery-page" => &[
            "--destination",
            "--delivery",
            "--domain",
            "--offset",
            "--limit",
        ],
        "media-discover" => &[
            "--destination",
            "--catalog",
            "--locale",
            "--offset",
            "--limit",
        ],
        "media-extract" => &[
            "--destination",
            "--installation",
            "--catalog",
            "--locale",
            "--ids",
            "--preset",
        ],
        "media-verify" => &["--destination", "--media"],
        "media-page" => &["--destination", "--media", "--offset", "--limit"],
        "tooltip" => &["--destination", "--projection", "--spell"],
        "guide-facts-build" => &["--destination", "--projection", "--inputs", "--preset"],
        "guide-facts-verify" => &["--destination", "--guide-facts"],
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
            "--locales",
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
    if command.starts_with("inventory-")
        || matches!(command, "route-designs" | "bound-guide-review")
    {
        return inventory_command(command, &options);
    }
    if command.starts_with("delivery-receipt-") {
        return delivery_receipt_command(command, &options);
    }
    if command.starts_with("delivery-") {
        return delivery_command(command, &options);
    }
    if matches!(
        command,
        "media-discover" | "media-extract" | "media-verify" | "media-page" | "tooltip"
    ) {
        return media_command(command, &options);
    }
    if matches!(command, "guide-facts-build" | "guide-facts-verify") {
        return guide_facts_command(command, &options);
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
    let locales: Vec<String> = match options.get("--locales") {
        None => Vec::new(),
        Some(value) => match value.to_str() {
            Some(text) => text.split(',').map(str::to_owned).collect(),
            None => return usage(),
        },
    };
    if options.contains_key("--locales")
        && (locales.is_empty()
            || locales.len() > azeroth_core::local_data_workflow::CONTENT_LOCALES.len()
            || locales.iter().any(|locale| {
                !azeroth_core::local_data_workflow::CONTENT_LOCALES.contains(&locale.as_str())
            })
            || locales.windows(2).any(|pair| pair[0] >= pair[1]))
    {
        return usage();
    }
    let request = ExtractionRequest {
        protocol_version: if locales.is_empty() { 1 } else { 3 },
        locales,
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

fn guide_facts_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    emit((|| {
        let destination = Path::new(
            options
                .get("--destination")
                .ok_or("Missing private destination")?,
        );
        if command == "guide-facts-verify" {
            let digest = options
                .get("--guide-facts")
                .and_then(|s| s.to_str())
                .ok_or("Missing guide facts digest")?;
            return serde_json::to_value(azeroth_local_data::guide_facts::verify(
                destination,
                digest,
            )?)
            .map_err(|_| "Cannot encode guide facts verification");
        }
        let projection = options
            .get("--projection")
            .and_then(|s| s.to_str())
            .ok_or("Missing projection digest")?;
        let mut inputs = match options.get("--inputs") {
            Some(value) => value
                .to_str()
                .ok_or("Invalid observation input digests")?
                .split(',')
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            None => vec![],
        };
        inputs.sort();
        let preset = preset(options.get("--preset")).ok_or("Invalid preset")?;
        serde_json::to_value(azeroth_local_data::guide_facts::build(
            destination,
            projection,
            &inputs,
            preset,
        )?)
        .map_err(|_| "Cannot encode guide facts report")
    })())
}

fn media_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    let result = (|| {
        let value = |key: &str| {
            options
                .get(key)
                .and_then(|v| v.to_str())
                .ok_or("Missing media option")
        };
        let root = Path::new(
            options
                .get("--destination")
                .ok_or("Missing media destination")?,
        );
        match command {
            "media-discover" => serde_json::to_value(azeroth_local_data::media::discovery_page(
                root,
                value("--catalog")?,
                value("--locale")?,
                options.get("--offset").map_or(Ok(0), |v| {
                    v.to_str()
                        .ok_or("Invalid discovery offset")?
                        .parse()
                        .map_err(|_| "Invalid discovery offset")
                })?,
                options.get("--limit").map_or(Ok(200), |v| {
                    v.to_str()
                        .ok_or("Invalid discovery limit")?
                        .parse()
                        .map_err(|_| "Invalid discovery limit")
                })?,
            )?)
            .map_err(|_| "Media JSON failed"),
            "media-verify" => {
                serde_json::to_value(azeroth_local_data::media::verify(root, value("--media")?)?)
                    .map_err(|_| "Media JSON failed")
            }
            "media-page" => serde_json::to_value(azeroth_local_data::media::page(
                root,
                value("--media")?,
                value("--offset")?
                    .parse()
                    .map_err(|_| "Invalid media offset")?,
                value("--limit")?
                    .parse()
                    .map_err(|_| "Invalid media limit")?,
            )?)
            .map_err(|_| "Media JSON failed"),
            "tooltip" => serde_json::to_value(azeroth_local_data::media::tooltip(
                root,
                value("--projection")?,
                value("--spell")?.parse().map_err(|_| "Invalid spell ID")?,
            )?)
            .map_err(|_| "Tooltip JSON failed"),
            "media-extract" => {
                let ids = value("--ids")?
                    .split(',')
                    .map(|s| s.parse::<u32>().map_err(|_| "Invalid media IDs"))
                    .collect::<Result<Vec<_>, _>>()?;
                let request = azeroth_local_data::media::Request {
                    format_version: 1,
                    installation: Path::new(
                        options
                            .get("--installation")
                            .ok_or("Missing media installation")?,
                    )
                    .into(),
                    destination: root.into(),
                    catalog: value("--catalog")?.into(),
                    locale: value("--locale")?.into(),
                    ids,
                    preset: preset(options.get("--preset")).ok_or("Invalid media preset")?,
                };
                let exe = std::env::current_exe().map_err(|_| "Media executable unavailable")?;
                let revision = azeroth_local_data::media::extract_worker(&exe, &request)?;
                Ok(
                    serde_json::json!({"format_version":1,"media_sha256":revision,"local_only":true,"verified_for_gameplay":false}),
                )
            }
            _ => Err("Unsupported media operation"),
        }
    })();
    emit(result)
}

fn delivery_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    let result = (|| {
        let value = |key: &str| {
            options
                .get(key)
                .and_then(|s| s.to_str())
                .ok_or("Missing delivery option")
        };
        let root = Path::new(
            options
                .get("--destination")
                .ok_or("Missing delivery destination")?,
        );
        let json = |v| Ok(v);
        match command {
            "delivery-build" => {
                let list = |key: &str| -> Result<Vec<String>, &'static str> {
                    options.get(key).map_or(Ok(vec![]), |v| {
                        Ok(v.to_str()
                            .ok_or("Invalid delivery digests")?
                            .split(',')
                            .map(str::to_owned)
                            .collect())
                    })
                };
                let media = list("--media")?;
                let inputs = list("--inputs")?;
                let guide = options
                    .get("--guide-facts")
                    .map(|s| s.to_str().ok_or("Invalid guide facts identity"))
                    .transpose()?;
                let digest = azeroth_local_data::delivery::build(
                    root,
                    value("--projection")?,
                    &media,
                    &inputs,
                    guide,
                    preset(options.get("--preset")).ok_or("Invalid delivery preset")?,
                )?;
                json(
                    serde_json::json!({"format_version":1,"delivery_sha256":digest,"verified_for_planning":false}),
                )
            }
            "delivery-verify" => json(
                serde_json::to_value(azeroth_local_data::delivery::verify(
                    root,
                    value("--delivery")?,
                )?)
                .map_err(|_| "Delivery JSON failed")?,
            ),
            "delivery-active" => json(
                serde_json::to_value(azeroth_local_data::delivery::active(
                    root,
                    value("--delivery")?,
                )?)
                .map_err(|_| "Delivery JSON failed")?,
            ),
            "delivery-activate" | "delivery-rollback" => json(
                serde_json::to_value(azeroth_local_data::delivery::activate(
                    root,
                    value("--delivery")?,
                    value("--expected")?
                        .parse()
                        .map_err(|_| "Invalid delivery generation")?,
                    command == "delivery-rollback",
                )?)
                .map_err(|_| "Delivery JSON failed")?,
            ),
            "delivery-diff" => json(
                serde_json::to_value(azeroth_local_data::delivery::diff(
                    root,
                    value("--before")?,
                    value("--after")?,
                )?)
                .map_err(|_| "Delivery JSON failed")?,
            ),
            "delivery-page" => json(
                serde_json::to_value(azeroth_local_data::delivery::page(
                    root,
                    value("--delivery")?,
                    value("--domain")?,
                    value("--offset")?
                        .parse()
                        .map_err(|_| "Invalid delivery offset")?,
                    value("--limit")?
                        .parse()
                        .map_err(|_| "Invalid delivery limit")?,
                    None,
                )?)
                .map_err(|_| "Delivery JSON failed")?,
            ),
            _ => Err("Unsupported delivery command"),
        }
    })();
    emit(result)
}

fn delivery_receipt_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    use azeroth_local_data::{delivery_receipts as v3, receipts};
    let text = |key: &str| {
        options
            .get(key)
            .and_then(|s| s.to_str())
            .ok_or("Missing format 3 receipt option")
    };
    let file = |key: &str| -> Result<Vec<u8>, &'static str> {
        let path = options.get(key).ok_or("Missing receipt input")?;
        azeroth_local_data::checked_path(Path::new(path))?;
        read_small(path, 128 * 1024)
    };
    if command == "delivery-receipt-assemble" {
        let result = (|| {
            v3::assemble(
                v3::read_payload(&file("--payload")?)?,
                receipts::read_signature(&file("--operator")?)?,
                receipts::read_signature(&file("--reviewer")?)?,
            )
        })();
        return match result {
            Ok(bytes) => {
                use std::io::Write;
                match std::io::stdout().lock().write_all(&bytes) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::from(1),
                }
            }
            Err(e) => emit::<()>(Err(e)),
        };
    }
    let result = (|| match command {
        "delivery-receipt-sign" => {
            let payload = v3::read_payload(&file("--payload")?)?;
            let policy = v3::policy(&file("--policy")?)?;
            let secret = receipts::read_secret_key(&file("--secret-key")?)?;
            serde_json::to_value(v3::sign(&payload, &policy, text("--key-id")?, &secret)?)
                .map_err(|_| "Receipt JSON failed")
        }
        "delivery-receipt-verify" => {
            let policy = v3::policy(&file("--policy")?)?;
            serde_json::to_value(v3::verify_current_release(
                &file("--receipt")?,
                &policy,
                text("--candidate")?,
                text("--production")?,
                text("--delivery")?,
            )?)
            .map_err(|_| "Receipt JSON failed")
        }
        _ => Err("Unsupported format 3 receipt operation"),
    })();
    emit(result)
}

fn inventory_command(command: &str, options: &BTreeMap<&str, &OsStr>) -> ExitCode {
    let result = (|| {
        let text = |key: &str| {
            options
                .get(key)
                .and_then(|v| v.to_str())
                .ok_or("Missing research inventory option")
        };
        let root = Path::new(
            options
                .get("--destination")
                .ok_or("Missing inventory destination")?,
        );
        let authored = options.get("--authored-pack").map(Path::new);
        match command {
            "inventory-build" => {
                let digest = azeroth_local_data::inventory::build(
                    root,
                    text("--delivery")?,
                    authored,
                    preset(options.get("--preset")).ok_or("Invalid inventory preset")?,
                )?;
                Ok(
                    serde_json::json!({"format_version":1,"inventory_sha256":digest,"acceptance_ready":false}),
                )
            }
            "inventory-verify" => serde_json::to_value(azeroth_local_data::inventory::summary(
                root,
                text("--inventory")?,
                authored,
            )?)
            .map_err(|_| "Inventory report encoding failed"),
            "inventory-page" => serde_json::to_value(azeroth_local_data::inventory::page(
                root,
                text("--inventory")?,
                text("--domain")?,
                text("--offset")?
                    .parse()
                    .map_err(|_| "Invalid inventory offset")?,
                text("--limit")?
                    .parse()
                    .map_err(|_| "Invalid inventory page limit")?,
                authored,
            )?)
            .map_err(|_| "Inventory page encoding failed"),
            "route-designs" => serde_json::to_value(azeroth_local_data::inventory::designs(
                root,
                text("--inventory")?,
                authored,
            )?)
            .map_err(|_| "Route design encoding failed"),
            "bound-guide-review" => {
                use azeroth_ingest::rxp::{Character, Class, Race};
                let character = Character {
                    race: Race::from_token(text("--race")?).ok_or("Unsupported guide race")?,
                    class: Class::from_token(text("--class")?).ok_or("Unsupported guide class")?,
                };
                let mode = match text("--mode")? {
                    "solo" => azeroth_core::local_data_inventory::Mode::Solo,
                    "duo" => azeroth_core::local_data_inventory::Mode::Duo,
                    _ => return Err("Unsupported guide review mode"),
                };
                serde_json::to_value(azeroth_local_data::guide_review::verify(
                    root,
                    text("--delivery")?,
                    Path::new(options.get("--guide").ok_or("Missing guide selection")?),
                    character,
                    mode,
                )?)
                .map_err(|_| "Bound guide report encoding failed")
            }
            _ => Err("Unsupported inventory operation"),
        }
    })();
    let clean_review = result
        .as_ref()
        .ok()
        .and_then(|value| value.get("report"))
        .and_then(|report| report.get("replay_clean"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let exit = emit(result);
    if command == "bound-guide-review" && exit == ExitCode::SUCCESS {
        ExitCode::from(if clean_review { 3 } else { 1 })
    } else {
        exit
    }
}
