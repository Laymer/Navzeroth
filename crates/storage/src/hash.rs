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

pub(crate) fn write_canonical(value: &Value, out: &mut Vec<u8>) -> Result<()> {
    write_canonical_to(value, out)
}

/// Same canonical bytes for buffers and streaming digest sinks.
pub(crate) fn write_canonical_to(value: &Value, out: &mut impl std::io::Write) -> Result<()> {
    fn fail<T>(_: T) -> azeroth_core::error::AppError {
        storage("Snapshot cannot be serialized")
    }
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.write_all(b"{").map_err(fail)?;
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.write_all(b",").map_err(fail)?;
                }
                serde_json::to_writer(&mut *out, key).map_err(fail)?;
                out.write_all(b":").map_err(fail)?;
                write_canonical_to(item, out)?;
            }
            out.write_all(b"}").map_err(fail)?;
        }
        Value::Array(items) => {
            out.write_all(b"[").map_err(fail)?;
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.write_all(b",").map_err(fail)?;
                }
                write_canonical_to(item, out)?;
            }
            out.write_all(b"]").map_err(fail)?;
        }
        scalar => serde_json::to_writer(&mut *out, scalar).map_err(fail)?,
    }
    Ok(())
}

#[cfg(test)]
mod canonical_stream_tests {
    use super::*;
    use std::io::{self, Write};

    #[test]
    fn buffered_and_streaming_writers_match_fixed_historical_canonical_bytes() {
        // Deliberately unsorted struct fields and maps, scalar escaping, false,
        // zero, null and finite floats. Expected bytes are independent of writer.
        #[derive(serde::Serialize)]
        struct Input {
            z: Vec<Value>,
            a: Value,
        }
        let input = Input {
            z: vec![
                serde_json::json!(false),
                serde_json::json!(0),
                serde_json::json!(1.25),
                serde_json::json!("line\n\\\"é"),
                Value::Null,
            ],
            a: serde_json::json!({"z":-2,"a":true}),
        };
        let expected =
            "{\"a\":{\"a\":true,\"z\":-2},\"z\":[false,0,1.25,\"line\\n\\\\\\\"é\",null]}"
                .as_bytes();
        let value = serde_json::to_value(&input).unwrap();
        let mut buffered = Vec::new();
        write_canonical(&value, &mut buffered).unwrap();
        assert_eq!(buffered, expected);
        let expected_digest = format!("{:x}", Sha256::digest(expected));
        assert_eq!(
            crate::content::hash(&input).unwrap().as_str(),
            expected_digest
        );

        struct ShortWriter(Vec<u8>);
        impl Write for ShortWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                let n = bytes.len().min(1);
                self.0.extend_from_slice(&bytes[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut short = ShortWriter(Vec::new());
        write_canonical_to(&value, &mut short).unwrap();
        short.flush().unwrap();
        assert_eq!(short.0, expected, "partial sink writes must not lose bytes");
    }

    #[test]
    fn canonical_stream_propagates_sink_failures_at_every_byte_boundary() {
        struct Limited(usize);
        impl Write for Limited {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if bytes.len() > self.0 {
                    return Err(io::Error::other("controlled sink failure"));
                }
                self.0 -= bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let value = serde_json::json!({"b":[false,0,null,{"é":"escaped\n\""}],"a":true});
        let mut expected = Vec::new();
        write_canonical(&value, &mut expected).unwrap();
        for budget in 0..expected.len() {
            assert!(
                write_canonical_to(&value, &mut Limited(budget)).is_err(),
                "budget {budget}"
            );
        }
        write_canonical_to(&value, &mut Limited(expected.len())).unwrap();
    }
}
