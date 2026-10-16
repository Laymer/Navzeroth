//! Explicit local-data CLI dispatch. Only implemented operations are accepted.
use azeroth_core::local_data::PerformancePreset;
use std::{ffi::OsString, path::Path, process::ExitCode};

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

pub fn run(args: &[OsString]) -> ExitCode {
    let usage = || {
        println!(
            "{}",
            serde_json::json!({"format_version":1,"error":"usage","usage":"local-data inspect --installation <directory> --product <product>; local-data plan --installation <directory> --product <product> --destination <existing-directory> [--preset small|balanced|large|max]"})
        );
        ExitCode::from(2)
    };
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        return usage();
    };
    if !matches!(command, "inspect" | "plan") {
        return usage();
    }
    let (mut installation, mut product, mut destination, mut preset) = (None, None, None, None);
    let (pairs, remainder) = args[1..].as_chunks::<2>();
    for pair in pairs {
        let slot = match pair[0].to_str() {
            Some("--installation") => &mut installation,
            Some("--product") => &mut product,
            Some("--destination") if command == "plan" => &mut destination,
            Some("--preset") if command == "plan" => &mut preset,
            _ => return usage(),
        };
        if slot.is_some()
            || pair[1].is_empty()
            || pair[1].to_str().is_some_and(|s| s.starts_with("--"))
        {
            return usage();
        }
        *slot = Some(pair[1].as_os_str());
    }
    if !remainder.is_empty() {
        return usage();
    }
    let (Some(installation), Some(product)) = (installation, product.and_then(|s| s.to_str()))
    else {
        return usage();
    };
    if command == "inspect" {
        return emit(azeroth_local_data::inspect(
            Path::new(installation),
            product,
        ));
    }
    let Some(destination) = destination else {
        return usage();
    };
    let preset = match preset.map(|s| s.to_str()) {
        None | Some(Some("max")) => PerformancePreset::Max,
        Some(Some("small")) => PerformancePreset::Small,
        Some(Some("balanced")) => PerformancePreset::Balanced,
        Some(Some("large")) => PerformancePreset::Large,
        _ => return usage(),
    };
    // A valid preliminary plan is useful structured output but is not an
    // executable extraction plan. Exit 3 explicitly distinguishes this gap.
    match azeroth_local_data::plan(
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
    }
}
