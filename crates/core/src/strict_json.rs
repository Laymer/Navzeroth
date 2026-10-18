//! Bounded JSON decoding that refuses duplicate members, including nested maps.
use serde::Deserialize;
use serde_json::Value;
/// Refuse duplicate JSON members before typed decoding (including nested maps).
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique-key JSON data")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|v| Unique(Value::Number(v)))
                    .ok_or_else(|| E::custom("Invalid number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.to_owned())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<Unique>()? {
                    values.push(v.0);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("Duplicate JSON member"));
                    }
                    values.insert(key, a.next_value::<Unique>()?.0);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
pub fn value(bytes: &[u8], maximum_bytes: usize) -> Result<Value, &'static str> {
    if bytes.len() > maximum_bytes {
        return Err("JSON input exceeds byte bound");
    }
    serde_json::from_slice::<Unique>(bytes)
        .map(|v| v.0)
        .map_err(|_| "Invalid or duplicate-key pack JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::{BytesDeserializer, Error, F64Deserializer, StringDeserializer};

    #[test]
    fn owned_strings_preserve_embedded_zero_and_inert_text() {
        // Exercise Deserialize's owned-string path as well as the borrowed JSON
        // path: owning a string must not interpret Lua-looking text or erase NUL.
        let text = "literal\0os.execute('never run')".to_owned();
        let decoded = Unique::deserialize(StringDeserializer::<Error>::new(text.clone())).unwrap();
        assert_eq!(decoded.0, Value::String(text));
    }

    #[test]
    fn generic_deserializer_rejects_nonfinite_numbers_and_unsupported_byte_values() {
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = match Unique::deserialize(F64Deserializer::<Error>::new(number)) {
                Ok(_) => panic!("Nonfinite number became JSON"),
                Err(error) => error,
            };
            assert_eq!(error.to_string(), "Invalid number");
        }
        let error = match Unique::deserialize(BytesDeserializer::<Error>::new(b"synthetic bytes")) {
            Ok(_) => panic!("Byte array became JSON without declared encoding"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("unique-key JSON data"));
    }
}
