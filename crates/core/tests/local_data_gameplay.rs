use azeroth_core::{
    local_data_gameplay::{budget_index, class_mask, race_mask},
    local_data_projection::Fact,
};
#[test]
fn masks_preserve_zero_signed_bits_unknown_and_unsupported_boundaries() {
    assert_eq!(class_mask(&Fact::Known(0), 1, 16), Fact::Known(false));
    assert_eq!(class_mask(&Fact::Known(-1), 16, 16), Fact::Known(true));
    assert_eq!(class_mask(&Fact::Known(-32768), 16, 16), Fact::Known(true));
    assert_eq!(class_mask(&Fact::Known(-32768), 1, 16), Fact::Known(false));
    assert_eq!(class_mask(&Fact::Known(1), 0, 16), Fact::Unsupported);
    assert_eq!(class_mask(&Fact::Known(1), 17, 16), Fact::Unsupported);
    assert_eq!(class_mask(&Fact::Known(65536), 1, 16), Fact::Unsupported);
    assert_eq!(class_mask(&Fact::Unknown, 1, 32), Fact::Unknown);
    assert_eq!(class_mask(&Fact::Missing, 1, 32), Fact::Missing);
    assert_eq!(class_mask(&Fact::Unsupported, 1, 32), Fact::Unsupported);
    assert_eq!(class_mask(&Fact::Known(1), 1, 64), Fact::Unsupported);
    assert_eq!(race_mask(&Fact::Known(vec![0, 1]), 32), Fact::Known(true));
    assert_eq!(
        race_mask(&Fact::Known(vec![0, -2147483648]), 63),
        Fact::Known(true)
    );
    assert_eq!(race_mask(&Fact::Known(vec![0, 0]), 0), Fact::Known(false));
    assert_eq!(race_mask(&Fact::Known(vec![1]), 0), Fact::Unsupported);
    assert_eq!(
        race_mask(&Fact::Known(vec![i64::MIN, 0]), 0),
        Fact::Unsupported
    );
    assert_eq!(race_mask(&Fact::Unknown, 1), Fact::Unknown);
    assert_eq!(race_mask(&Fact::Missing, 1), Fact::Missing);
    assert_eq!(race_mask(&Fact::Unsupported, 1), Fact::Unsupported);
    assert_eq!(race_mask(&Fact::Known(vec![1, 0]), 64), Fact::Unsupported);
}
#[test]
fn budget_buckets_are_explicit_and_wands_need_a_known_subclass() {
    for (inventory, bucket) in [
        (1, 0),
        (4, 0),
        (17, 0),
        (3, 1),
        (12, 1),
        (2, 2),
        (23, 2),
        (13, 3),
        (22, 3),
        (28, 4),
    ] {
        assert_eq!(budget_index(inventory, None), Some(bucket));
    }
    assert_eq!(budget_index(26, None), None);
    assert_eq!(budget_index(26, Some(19)), Some(3));
    assert_eq!(budget_index(26, Some(2)), Some(0));
    assert_eq!(budget_index(0, None), None);
    assert_eq!(budget_index(999, None), None);
}
#[test]
fn budget_model_is_bounded_and_does_not_invent_missing_socket_or_budget_values() {
    use azeroth_core::local_data_gameplay::base_budget_amount as amount;
    assert_eq!(
        amount(&Fact::Known(5000), &Fact::Known(100.0), &Fact::Known(0.0)),
        Fact::Known(50.0)
    );
    assert_eq!(
        amount(&Fact::Known(0), &Fact::Known(100.0), &Fact::Known(0.0)),
        Fact::Known(0.0)
    );
    assert_eq!(
        amount(&Fact::Known(100), &Fact::Missing, &Fact::Known(0.0)),
        Fact::Missing
    );
    assert_eq!(
        amount(&Fact::Unknown, &Fact::Known(10.0), &Fact::Known(0.0)),
        Fact::Unknown
    );
    assert_eq!(
        amount(&Fact::Known(-1), &Fact::Known(10.0), &Fact::Known(0.0)),
        Fact::Unsupported
    );
    assert_eq!(
        amount(&Fact::Known(1), &Fact::Known(10.0), &Fact::Known(0.5)),
        Fact::Unsupported
    );
    assert_eq!(
        amount(&Fact::Known(1), &Fact::Known(f64::NAN), &Fact::Known(0.0)),
        Fact::Unsupported
    );
    assert_eq!(
        amount(
            &Fact::Known(i64::MAX),
            &Fact::Known(f64::MAX),
            &Fact::Known(0.0)
        ),
        Fact::Unsupported
    );
    assert_eq!(
        amount(&Fact::Known(1), &Fact::Known(-1.0), &Fact::Known(0.0)),
        Fact::Unsupported
    );
    assert_eq!(
        amount(&Fact::Unsupported, &Fact::Known(1.0), &Fact::Known(0.0)),
        Fact::Unsupported
    );
}
