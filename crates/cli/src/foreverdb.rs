//! Explicit local diagnostic staging; manifest integrity is not source admission.
use azeroth_ingest::foreverdb;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, ffi::OsString, path::PathBuf};

pub const USAGE: &str = "stage-foreverdb --input <extracted-addon-root> --manifest <member-hashes.json>; read-only quest staging, no content activation";
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const FILES: [&str; 3] = [
    "ForeverDB/Data/Meta.lua",
    "ForeverDB/Data/QuestForever.lua",
    "ForeverDB/Data/QuestClassic.lua",
];
#[derive(Debug)]
pub struct Options {
    input: PathBuf,
    manifest: PathBuf,
}
impl Options {
    pub fn parse(args: &[OsString]) -> Option<Self> {
        if args.len() != 4 {
            return None;
        }
        let mut input = None;
        let mut manifest = None;
        for pair in args.as_chunks::<2>().0 {
            if pair[1].is_empty()
                || pair[1].as_encoded_bytes().len() > 4096
                || pair[1].as_encoded_bytes().starts_with(b"--")
            {
                return None;
            }
            let slot = match pair[0].to_str()? {
                "--input" => &mut input,
                "--manifest" => &mut manifest,
                _ => return None,
            };
            if slot.replace(PathBuf::from(&pair[1])).is_some() {
                return None;
            }
        }
        Some(Self {
            input: input?,
            manifest: manifest?,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    bytes: u64,
    sha256: String,
}
#[derive(Serialize)]
pub struct Report {
    pub manifest_sha256: String,
    pub integrity: &'static str,
    pub archive_identity: &'static str,
    pub staging: foreverdb::Report,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// The selected manifest is an untrusted claim. Verify complete selected files
/// against its hashes; never treat a match as an authentic publisher signature.
pub fn report(options: &Options) -> Result<Report, &'static str> {
    let manifest = crate::questiedb::read_local_source(&options.manifest)
        .map_err(|_| "Cannot safely read selected member manifest")?;
    if manifest.len() > MAX_MANIFEST_BYTES {
        return Err("Member manifest exceeds its size limit");
    }
    // Strict duplicate-field rejection, including repeated path/hash keys.
    let members: Vec<Member> =
        serde_json::from_slice(&manifest).map_err(|_| "Malformed member manifest")?;
    if members.is_empty() || members.len() > 1024 {
        return Err("Member manifest count exceeds its limits");
    }
    let mut seen = BTreeSet::new();
    for m in &members {
        if m.path.len() > 512
            || azeroth_core::types::AssetPath::new(m.path.to_ascii_lowercase()).is_err()
            || !seen.insert(m.path.to_ascii_lowercase())
            || m.bytes > foreverdb::MAX_FILE_BYTES as u64
            || m.sha256.len() != 64
            || !m
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Member manifest contains an invalid or duplicate declaration");
        }
    }
    let mut sources = Vec::new();
    for path in FILES {
        let m = members
            .iter()
            .find(|m| m.path == path)
            .ok_or("Required ForeverDB file is not declared")?;
        let data = crate::questiedb::read_local_source(&options.input.join(path))
            .map_err(|_| "Cannot safely read selected ForeverDB file")?;
        if data.len() as u64 != m.bytes || digest(&data) != m.sha256 {
            return Err("ForeverDB file differs from selected manifest");
        }
        sources.push(data);
    }
    let staged = foreverdb::stage(&sources[0], &sources[1], &sources[2])
        .map_err(|_| "ForeverDB source does not match the supported inert schema")?;
    Ok(Report {
        manifest_sha256: digest(&manifest),
        integrity: "selected file bytes match selected member manifest",
        archive_identity: "not established from extracted files",
        staging: staged.report(),
    })
}
