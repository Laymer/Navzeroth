//! Reject duplicate keys at every depth before decoding the typed local package.
use azeroth_core::error::{AppError, ErrorCode, Result};
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Value};
use std::fmt;
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded JSON data")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("Invalid number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::String(v.into())))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut values = Vec::new();
                while let Some(Strict(value)) = a.next_element()? {
                    values.push(value);
                }
                Ok(Strict(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut values = Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("Duplicate JSON key"));
                    }
                    let Strict(value) = a.next_value()?;
                    values.insert(key, value);
                }
                Ok(Strict(Value::Object(values)))
            }
        }
        d.deserialize_any(V)
    }
}
pub(crate) fn strict_value(text: &str) -> Result<Value> {
    let Strict(value) = serde_json::from_str(text).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Invalid or duplicate-key JSON",
            false,
        )
    })?;
    Ok(value)
}
pub(super) fn request(text: &str) -> Result<super::PreparationRequest> {
    if text.len() > super::MAX_REQUEST_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Preparation request exceeds 1 MiB",
            false,
        ));
    }
    let Strict(value) = serde_json::from_str(text)
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid preparation JSON", false))?;
    if value.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(AppError::new(
            ErrorCode::UnsupportedSchema,
            "Unsupported preparation schema",
            false,
        ));
    }
    serde_json::from_value(value).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Invalid preparation request",
            false,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preparation_json_preserves_values_and_refuses_unknown_schema_before_decoding() {
        let value = strict_value(r#"[false,0,-1,1.25,null,"inert"]"#).unwrap();
        assert_eq!(
            value,
            serde_json::json!([false, 0, -1, 1.25, null, "inert"])
        );
        for text in [
            "{}",
            r#"{"schema_version":0}"#,
            r#"{"schema_version":2}"#,
            r#"{"schema_version":false}"#,
        ] {
            assert_eq!(
                request(text).unwrap_err().code,
                ErrorCode::UnsupportedSchema
            );
        }
        use serde::de::value::{BytesDeserializer, Error};
        let error = Strict::deserialize(BytesDeserializer::<Error>::new(b"inert"))
            .err()
            .unwrap();
        assert!(error.to_string().contains("bounded JSON data"));
    }
}
