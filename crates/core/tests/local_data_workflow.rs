use azeroth_core::local_data_workflow::*;
#[test]
fn locale_choices_are_explicit_sorted_installed_and_preserve_legacy_order() {
    let declared = vec!["enUS".into(), "deDE".into()];
    assert_eq!(select_locales(&declared, &[]).unwrap(), declared);
    assert_eq!(
        select_locales(&declared, &["deDE".into()]).unwrap(),
        ["deDE"]
    );
    assert!(select_locales(&declared, &["frFR".into()]).is_err());
    assert!(select_locales(&declared, &["enUS".into(), "deDE".into()]).is_err());
}
#[test]
fn change_detection_debounces_deduplicates_and_bounds_retries() {
    let baseline = "a".repeat(64);
    let next = "b".repeat(64);
    let mut detector = ChangePoller::new(&baseline).unwrap();
    assert_eq!(detector.poll(0, Some(&baseline)).unwrap(), PollStatus::Idle);
    assert_eq!(
        detector.poll(500, Some(&next)).unwrap(),
        PollStatus::WaitingForStableFile
    );
    assert_eq!(
        detector.poll(1000, Some(&next)).unwrap(),
        PollStatus::WaitingForStableFile
    );
    assert_eq!(detector.poll(1500, Some(&next)).unwrap(), PollStatus::Ready);
    detector.staged();
    assert_eq!(
        detector.poll(2000, Some(&next)).unwrap(),
        PollStatus::Staged
    );
    assert_eq!(detector.poll(2500, None).unwrap(), PollStatus::Missing);
    assert_eq!(
        detector.poll(3000, Some(&next)).unwrap(),
        PollStatus::WaitingForStableFile
    );
    for now in [4000, 4500, 5000] {
        assert_eq!(detector.poll(now, Some(&next)).unwrap(), PollStatus::Ready);
    }
    assert_eq!(
        detector.poll(5500, Some(&next)).unwrap(),
        PollStatus::RetryExhausted
    );
    assert!(detector.poll(5000, Some(&next)).is_err());
    assert_eq!(
        detector.poll(6000, Some(&baseline)).unwrap(),
        PollStatus::Idle
    );
}
#[test]
fn explicit_delivery_choices_reject_unknown_versions_duplicates_and_unbounded_inputs() {
    use azeroth_core::local_data::PerformancePreset;
    let mut choices = DeliveryChoices {
        format_version: 1,
        media: vec![],
        inputs: vec!["a".repeat(64)],
        guide_facts: true,
        preset: PerformancePreset::Small,
    };
    assert!(choices.validate().is_ok());
    choices.inputs.push("a".repeat(64));
    assert!(choices.validate().is_err());
    choices.inputs = (0..17).map(|n| format!("{n:064x}")).collect();
    assert!(choices.validate().is_err());
    choices.inputs.clear();
    choices.format_version = 2;
    assert!(choices.validate().is_err());
}
