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
            serde_json::json!({"format_version":1,"error":"usage","usage":"local-data inspect --installation <directory> --product <product>; local-data (plan|extract) --installation <directory> --product <product> --destination <private-directory> [--preset small|balanced|large|max] [extract: --tables <comma-separated-names>]; local-data (verify|coverage|status) --destination <private-directory> --catalog <sha256>; local-data receipt-public-key --secret-key <file>; local-data receipt-sign --payload <file> --policy <file> --key-id <id> --secret-key <file>; local-data receipt-assemble --payload <file> --operator <file> --reviewer <file>; local-data receipt-verify --receipt <file> --policy <file> --candidate <commit> --production <sha256>"})
        );
        ExitCode::from(2)
    };
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        return usage();
    };
    let allowed: &[&str] = match command {
        "inspect" => &["--installation", "--product"],
        "plan" => &["--installation", "--product", "--destination", "--preset"],
        "extract" => &[
            "--installation",
            "--product",
            "--destination",
            "--preset",
            "--tables",
        ],
        "verify" | "coverage" | "status" => &["--destination", "--catalog"],
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
    if command.starts_with("receipt-") {
        if options.len() != allowed.len() {
            return usage();
        }
        return receipt(command, &options);
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
    let (Some(installation), Some(product)) = (
        options.get("--installation"),
        options.get("--product").and_then(|s| s.to_str()),
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
        preset(options.get("--preset")),
    ) else {
        return usage();
    };
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
    let request = ExtractionRequest {
        protocol_version: 1,
        request_id: "cli-extract".into(),
        installation: Path::new(installation).to_path_buf(),
        product: product.into(),
        destination: Path::new(destination).to_path_buf(),
        preset,
        tables,
    };
    let mut worker = match ExtractionWorker::start(&request) {
        Ok(worker) => worker,
        Err(error) => return emit::<()>(Err(error)),
    };
    loop {
        match worker.next_event() {
            Ok(WorkerEvent::Progress { .. }) => {}
            Ok(WorkerEvent::Complete { summary, .. }) => {
                let _ = emit(Ok(summary));
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
