use azeroth_core::{error::ErrorCode, types::*};
use serde_json::{from_str, json, to_string};

#[test]
fn ids_and_currency_preserve_values_beyond_javascript_precision() {
    let id: ItemId = from_str("\"9007199254740993\"").unwrap();
    assert_eq!(to_string(&id).unwrap(), "\"9007199254740993\"");
    let money = Copper::new(u64::MAX);
    assert_eq!(
        from_str::<Copper>(&to_string(&money).unwrap()).unwrap(),
        money
    );
    for bad in ["0", "-1", "\"-1\"", "\"01\"", "\"18446744073709551616\""] {
        assert!(from_str::<Copper>(bad).is_err());
    }
    assert!(from_str::<ItemId>("9007199254740993").is_err());
    assert!(ItemId::new(" ").is_err());
    assert!(ItemId::new("private\ncontent").is_err());
}
#[test]
fn units_and_schema_reject_unsafe_inputs() {
    assert!(Millis::new(MAX_SAFE_INTEGER).is_ok());
    assert!(Millis::new(MAX_SAFE_INTEGER + 1).is_err());
    assert!(UtcMillis::new(i64::MIN).is_err());
    assert!(Level::new(0).is_err());
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FiniteNumber::new(n).is_err());
        assert!(Normalized::new(n).is_err());
    }
    for n in [-0.01, 1.01] {
        assert!(Normalized::new(n).is_err());
    }
    for n in [0., 1.] {
        assert!(Normalized::new(n).is_ok());
    }
    assert_eq!(
        SchemaVersion::try_from(2).unwrap_err().code,
        ErrorCode::UnsupportedSchema
    );
    assert!(from_str::<SchemaVersion>("0").is_err());
    assert_eq!(to_string(&SchemaVersion::V1).unwrap(), "1");
}
#[test]
fn build_variant_and_namespace_are_part_of_identity() {
    let a: ItemKey = serde_json::from_value(json!({"context":{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},"item_id":"123","variant":"base"})).unwrap();
    let mut b = a.clone();
    b.context.build.edition = EditionId::new("wow_classic").unwrap();
    assert_ne!(a, b);
    assert!(a.context.ensure_compatible(&b.context).is_err());
    b = a.clone();
    b.variant = VariantId::new("other").unwrap();
    assert_ne!(a, b);
    b = a.clone();
    b.context.namespace = DatasetNamespace::Reference;
    assert!(a.context.ensure_compatible(&b.context).is_err());
}
