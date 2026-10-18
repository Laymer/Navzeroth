//! Private media delivery contracts. No coordinates or availability are inferred.
use crate::local_data::is_sha256;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub table: String,
    pub row: u32,
    pub field: String,
    pub file_data_id: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Discovery {
    pub format_version: u32,
    pub catalog_sha256: String,
    pub locale: String,
    pub candidates: Vec<Candidate>,
    pub gaps: Vec<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Decoded,
    Unsupported,
    Invalid,
    Capacity,
    Missing,
    Encrypted,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadRefusal {
    Missing,
    Encrypted,
    EncryptedFrames,
    Unsupported,
    Invalid,
    Truncated,
    ShortRead,
    Capacity,
    Unavailable,
}
impl ReadRefusal {
    pub fn disposition(self) -> Disposition {
        match self {
            Self::Missing => Disposition::Missing,
            Self::Encrypted | Self::EncryptedFrames => Disposition::Encrypted,
            Self::Unsupported => Disposition::Unsupported,
            Self::Invalid | Self::Truncated => Disposition::Invalid,
            Self::Capacity => Disposition::Capacity,
            Self::ShortRead | Self::Unavailable => Disposition::Unavailable,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub file_data_id: u32,
    pub disposition: Disposition,
    pub source_sha256: Option<String>,
    pub read_refusal: Option<ReadRefusal>,
    pub png_sha256: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaManifest {
    pub format_version: u32,
    pub catalog_sha256: String,
    pub locale: String,
    pub discovery_sha256: String,
    pub profile_sha256: String,
    pub fixture: bool,
    pub local_only: bool,
    pub calibrated: bool,
    pub verified_for_gameplay: bool,
    pub assets: Vec<Asset>,
}
impl MediaManifest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != 1
            || !is_sha256(&self.catalog_sha256)
            || !is_sha256(&self.discovery_sha256)
            || !is_sha256(&self.profile_sha256)
            || !self.local_only
            || self.calibrated
            || self.verified_for_gameplay
            || self.assets.is_empty()
            || self.assets.len() > 128
            || self.locale.len() != 4
        {
            return Err("Invalid media manifest");
        }
        let mut last = 0;
        for a in &self.assets {
            if a.file_data_id <= last
                || [&a.source_sha256, &a.png_sha256]
                    .into_iter()
                    .flatten()
                    .any(|h| !is_sha256(h))
            {
                return Err("Invalid media asset");
            }
            last = a.file_data_id;
            match (&a.source_sha256, a.read_refusal) {
                (None, Some(reason)) if reason.disposition() == a.disposition && !self.fixture => {}
                (Some(_), None)
                    if matches!(
                        a.disposition,
                        Disposition::Decoded
                            | Disposition::Unsupported
                            | Disposition::Invalid
                            | Disposition::Capacity
                    ) => {}
                _ => return Err("Media read/decode provenance mismatch"),
            }
            if a.disposition == Disposition::Decoded {
                if a.source_sha256.is_none()
                    || a.png_sha256.is_none()
                    || a.width
                        .zip(a.height)
                        .is_none_or(|(w, h)| w == 0 || h == 0 || w as u64 * h as u64 > 4_194_304)
                {
                    return Err("Invalid decoded media asset");
                }
            } else if a.png_sha256.is_some() || a.width.is_some() || a.height.is_some() {
                return Err("Undecoded asset has decoded metadata");
            }
        }
        Ok(())
    }
}
