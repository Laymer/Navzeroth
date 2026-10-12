//! Bounded domain JSON decoding. This is not a snapshot transport/envelope importer.
use crate::{
    error::{AppError, ErrorCode, Result, Validate, invalid},
    types::SchemaVersion,
};
use serde::{Deserialize, Deserializer, de::DeserializeOwned};

pub const MAX_DOMAIN_JSON_BYTES: usize = 1_048_576;
fn declared_version<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<u32>, D::Error> {
    u32::deserialize(d).map(Some)
}
#[derive(Deserialize)]
struct Header {
    #[serde(default, deserialize_with = "declared_version")]
    schema_version: Option<u32>,
}
/// Decode a validated domain record. Limits bytes before parsing and retains
/// serde_json's recursion limit. Unknown metadata is ignored; declared schema
/// versions are checked even for records that rely on an outer envelope in M1.3.
/// Imported input is never included in the application error message.
pub fn decode_json<T: DeserializeOwned + Validate>(input: &str) -> Result<T> {
    if input.len() > MAX_DOMAIN_JSON_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Domain JSON exceeds 1 MiB",
            false,
        ));
    }
    let header: Header =
        serde_json::from_str(input).map_err(|_| invalid("Invalid domain JSON header"))?;
    if let Some(version) = header.schema_version {
        SchemaVersion::try_from(version)?;
    }
    let record: T =
        serde_json::from_str(input).map_err(|_| invalid("Invalid JSON or domain invariant"))?;
    record.validate()?;
    Ok(record)
}
