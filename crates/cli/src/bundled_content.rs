//! Packaging verifier for the signed exact resource closure; it never activates.
use azeroth_local_data::bundled_receipts as receipts;
use std::{collections::BTreeMap, ffi::OsString, path::Path, process::ExitCode};
pub fn execute(args: &[OsString]) -> Result<Vec<u8>, &'static str> {
    if !matches!(args.len(), 4 | 6) {
        return Err("Expected --input DIR --identity FILE [--policy FILE]");
    }
    let mut options = BTreeMap::new();
    let (pairs, _) = args.as_chunks::<2>();
    for pair in pairs {
        let name = pair[0].to_str().ok_or("Invalid bundle option")?;
        if !["--input", "--identity", "--policy"].contains(&name)
            || pair[1].len() > 4096
            || options.insert(name, &pair[1]).is_some()
        {
            return Err("Invalid or duplicate bundle option");
        }
    }
    let root = Path::new(options.get("--input").ok_or("Missing bundle input")?);
    let identity = receipts::payload(&receipts::read_input(
        Path::new(options.get("--identity").ok_or("Missing build identity")?),
        receipts::MAX_BYTES,
    )?)?;
    let policy = match options.get("--policy") {
        Some(path) => receipts::read_input(Path::new(path), receipts::MAX_BYTES)?,
        None => include_bytes!("../../../content/bundled/signers-v1.json").to_vec(),
    };
    let policy = receipts::policy(&policy)?;
    let expected = receipts::Expected {
        app_version: &identity.app_version,
        target: &identity.target,
        bundle_manifest_sha256: identity.bundle_manifest_sha256.as_str(),
        production_sha256: identity.production_sha256.as_str(),
        tested_source_commit: &identity.tested_source_commit,
        review: &identity.review,
    };
    let verified = azeroth_app::bundled_content::verify(root, &policy, &expected)
        .map_err(|_| "Signed bundle resource verification failed")?;
    serde_json::to_vec(&serde_json::json!({"verified":true,"verification":"signed-resource-closure","app_version":verified.app_version,"pack_count":verified.packs.len(),"file_count":verified.files.len()})).map_err(|_| "Cannot encode bundle verification")
}
pub fn run(args: &[OsString]) -> ExitCode {
    use std::io::Write;
    match execute(args) {
        Ok(bytes) => {
            if std::io::stdout().lock().write_all(&bytes).is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(message) => {
            eprintln!("bundled-content: {message}");
            ExitCode::FAILURE
        }
    }
}
