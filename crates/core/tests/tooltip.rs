use azeroth_core::tooltip::{Document, Gap, evaluate};
use std::collections::BTreeMap;
#[test]
fn inert_tokens_preserve_zero_unknown_and_unsupported_expressions() {
    let docs = BTreeMap::from([(
        1,
        Document {
            text: "Hit $s1 in $d; $m2; $$; ${evil}; |Hlink|h".into(),
            scalars: BTreeMap::from([("s1".into(), "0".into()), ("d".into(), "2 sec".into())]),
        },
    )]);
    let result = evaluate(&docs, 1).unwrap();
    assert_eq!(
        result.text,
        "Hit 0 in 2 sec; [unknown]; $; ${evil}; |Hlink|h"
    );
    assert_eq!(
        result.gaps,
        [Gap::MissingScalar, Gap::UnsupportedExpression]
    );
    assert!(!result.verified_for_gameplay);
}
#[test]
fn dependency_cycles_missing_targets_and_expansion_are_bounded() {
    let docs = BTreeMap::from([
        (
            1,
            Document {
                text: "$@spelldesc2 $@spelldesc3".into(),
                ..Default::default()
            },
        ),
        (
            2,
            Document {
                text: "$@spelldesc1".into(),
                ..Default::default()
            },
        ),
    ]);
    let result = evaluate(&docs, 1).unwrap();
    assert_eq!(result.text, "[cycle] [missing]");
    assert_eq!(result.gaps, [Gap::MissingDocument, Gap::Cycle]);
    let docs = (1..=20)
        .map(|id| {
            (
                id,
                Document {
                    text: format!("$@spelldesc{}", id + 1),
                    ..Default::default()
                },
            )
        })
        .collect();
    assert_eq!(evaluate(&docs, 1).unwrap().gaps, [Gap::Capacity]);
}
#[test]
fn scalar_substitution_cannot_inject_further_tokens_and_input_is_bounded() {
    let mut doc = Document {
        text: "$s1".into(),
        scalars: BTreeMap::from([("s1".into(), "$@spelldesc1".into())]),
    };
    assert!(evaluate(&BTreeMap::from([(1, doc.clone())]), 1).is_err());
    doc.scalars.clear();
    doc.text = "x".repeat(16_385);
    assert!(evaluate(&BTreeMap::from([(1, doc)]), 1).is_err());
}

#[test]
fn closure_references_honor_escaping_and_reject_oversized_text() {
    assert_eq!(
        azeroth_core::tooltip::references("$$@spelldesc9 $@spelldesc2 $@spelldesc1 $@spelldesc2")
            .unwrap(),
        [1, 2]
    );
    assert!(azeroth_core::tooltip::references(&"x".repeat(16_385)).is_err());
}
