//! Explicit receipt operations; all inputs are selected files, no implicit keys.
use azeroth_local_data::{bundled_receipts as receipt, receipts::read_secret_key};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::Path,
    process::ExitCode,
};

pub fn execute(command: &str, args: &[OsString]) -> Result<Vec<u8>, &'static str> {
    let required: &[&str] = match command {
        "bundle-receipt-sign" => &["--payload", "--policy", "--key-id", "--secret-key"],
        "bundle-receipt-assemble" => &["--payload", "--operator", "--review-evidence"],
        "bundle-receipt-verify" => &["--receipt", "--policy", "--expected"],
        _ => return Err("Unsupported bundled-content receipt operation"),
    };
    if args.len() != required.len() * 2 {
        return Err("Missing bundled-content receipt options");
    }
    let mut options: BTreeMap<&str, &OsStr> = BTreeMap::new();
    let (pairs, _) = args.as_chunks::<2>();
    for pair in pairs {
        let key = pair[0].to_str().ok_or("Invalid bundled-content option")?;
        if !required.contains(&key)
            || pair[1].len() > 4096
            || options.insert(key, &pair[1]).is_some()
        {
            return Err("Unknown or duplicate bundled-content receipt option");
        }
    }
    let file = |key: &str, maximum| {
        receipt::read_input(
            Path::new(options.get(key).ok_or("Missing bundled-content input")?),
            maximum,
        )
    };
    let read = |key: &str| file(key, receipt::MAX_BYTES);
    match command {
        "bundle-receipt-sign" => {
            let payload = receipt::payload(&read("--payload")?)?;
            let policy = receipt::policy(&read("--policy")?)?;
            let secret = read_secret_key(&file("--secret-key", 1024)?)?;
            let signature = receipt::sign(
                &payload,
                &policy,
                options["--key-id"]
                    .to_str()
                    .ok_or("Invalid signing key identity")?,
                &secret,
            )?;
            receipt::encode(&signature)
        }
        "bundle-receipt-assemble" => receipt::assemble(
            receipt::payload(&read("--payload")?)?,
            receipt::signature(&read("--operator")?)?,
            receipt::signature(&read("--review-evidence")?)?,
        ),
        _ => {
            let policy = receipt::policy(&read("--policy")?)?;
            let expected = receipt::payload(&read("--expected")?)?;
            let verified = receipt::verify(
                &read("--receipt")?,
                &policy,
                &receipt::Expected {
                    app_version: &expected.app_version,
                    target: &expected.target,
                    bundle_manifest_sha256: expected.bundle_manifest_sha256.as_str(),
                    production_sha256: expected.production_sha256.as_str(),
                    tested_source_commit: &expected.tested_source_commit,
                    review: &expected.review,
                },
            )?;
            receipt::encode(&verified)
        }
    }
}
pub fn run(command: &str, args: &[OsString]) -> ExitCode {
    use std::io::Write;
    match execute(command, args) {
        Ok(bytes) => {
            if std::io::stdout().lock().write_all(&bytes).is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(message) => {
            eprintln!("bundled-content receipt: {message}");
            ExitCode::FAILURE
        }
    }
}
