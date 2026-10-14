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
pub(crate) fn decode<T: serde::de::DeserializeOwned + serde::Serialize>(
    text: &str,
    limit: usize,
) -> Result<T> {
    if text.len() > limit {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Local state exceeds byte bound",
            false,
        ));
    }
    let Strict(value) = serde_json::from_str(text)
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid local state JSON", false))?;
    let typed: T = serde_json::from_value(value.clone()).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Invalid local state document",
            false,
        )
    })?;
    // Reject ignored nested fields as well as unknown fields at the outer DTO.
    if serde_json::to_value(&typed).ok().as_ref() != Some(&value) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Noncanonical local state shape",
            false,
        ));
    }
    Ok(typed)
}
