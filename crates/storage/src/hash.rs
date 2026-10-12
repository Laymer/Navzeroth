//! Snapshot identity (plan section 7.2, CASE-007).
//!
//! The identity is SHA-256 over the canonical JSON of the decoded, validated
//! envelope, stored as lowercase hex. The canonical form is compact JSON with
//! object keys in sorted order, which equals `serde_json::to_vec` of the
//! envelope because every core map is a `BTreeMap`; sorting here makes the
//! digest independent of serde_json feature unification. Whitespace and key
//! order in the imported text therefore never matter.
//!
//! Desktop-added receipt times (`received_at_utc_ms` inside observation
//! metadata) are removed before hashing, so re-reading the same export on a
//! later day is a duplicate. Changing this form requires a schema migration.
use crate::error::storage;
use azeroth_core::{
    contract::{Envelope, EnvelopeData},
    entities::CharacterSnapshot,
    error::Result,
    types::Sha256Hex,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const RECEIVED_AT: &str = "received_at_utc_ms";

/// Content hash of a validated character snapshot envelope.
pub fn snapshot_content_hash(envelope: &Envelope<CharacterSnapshot>) -> Result<Sha256Hex> {
    let mut value = serde_json::to_value::<&EnvelopeData<CharacterSnapshot>>(envelope)
        .map_err(|_| storage("Snapshot cannot be serialized"))?;
    strip_receipt_times(&mut value);
    let mut bytes = Vec::new();
    write_canonical(&value, &mut bytes)?;
    let digest = Sha256::digest(&bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Sha256Hex::new(hex)
}

/// Observation metadata always serializes `clock_trust` as a string; record
/// maps hold object values, so a map key named like the field is never touched.
fn strip_receipt_times(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if matches!(map.get("clock_trust"), Some(Value::String(_))) {
                map.remove(RECEIVED_AT);
            }
            map.values_mut().for_each(strip_receipt_times);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_receipt_times),
        _ => {}
    }
}

fn write_canonical(value: &Value, out: &mut Vec<u8>) -> Result<()> {
    let fail = |_| storage("Snapshot cannot be serialized");
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.push(b'{');
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key).map_err(fail)?;
                out.push(b':');
                write_canonical(item, out)?;
            }
            out.push(b'}');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_canonical(item, out)?;
            }
            out.push(b']');
        }
        scalar => serde_json::to_writer(&mut *out, scalar).map_err(fail)?,
    }
    Ok(())
}
