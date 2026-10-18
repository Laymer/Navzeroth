use azeroth_core::{local_data_inputs::InputTuple, local_data_inventory::*, release::DOMAINS};
#[test]
fn all_nine_domains_preserve_imported_zero_without_claiming_target_completeness() {
    let mut inventory = Inventory {
        format_version: 1,
        delivery_sha256: "a".repeat(64),
        tuple: InputTuple {
            product: "wow_classic_beta".into(),
            version: "1.60.1.70450".into(),
            build: 70450,
            locale: "enUS".into(),
        },
        authored_manifest_sha256: None,
        authored_catalog_sha256: None,
        domains: DOMAINS
            .into_iter()
            .map(|d| Domain {
                domain: d.into(),
                imported_count: 0,
                records: vec![],
                source_available: false,
                gaps: vec!["unavailable".into()],
                independently_required_count: None,
                independently_complete: false,
            })
            .collect(),
        local_only: true,
        acceptance_ready: false,
    };
    assert!(inventory.validate().is_ok());
    inventory.domains[0].independently_required_count = Some(0);
    assert!(inventory.validate().is_err());
    inventory.domains[0].independently_required_count = None;
    inventory.domains[0].independently_complete = true;
    assert!(inventory.validate().is_err());
    inventory.domains[0].independently_complete = false;
    inventory.format_version = 2;
    assert!(inventory.validate().is_err());
}
#[test]
fn six_classes_have_separate_solo_duo_drafts_and_no_invented_quest_sequence() {
    let designs = human_route_designs();
    assert_eq!(designs.len(), 12);
    for design in &designs {
        assert!(design.verified_quest_sequence.is_none());
        assert!(!design.eligibility_verified && !design.actionable_advice);
        assert!(!design.authored_model_reviews.is_empty());
        assert_eq!(
            design
                .required_facts
                .iter()
                .any(|v| v == "each_participant_progress"),
            design.mode == Mode::Duo
        );
    }
    assert_ne!(
        designs
            .iter()
            .find(|d| d.class == "mage")
            .unwrap()
            .authored_model_reviews,
        designs
            .iter()
            .find(|d| d.class == "warrior")
            .unwrap()
            .authored_model_reviews
    );
}
