//! M5.2: bounded deterministic configuration enumeration. CASE-006 (two-handed
//! plus shield), CASE-005, CASE-015, unique groups, paired slots and baseline
//! matching. All identifiers and weights are SYNTHETIC.
mod common;
use azeroth_core::{
    entities::*, error::ErrorCode, evaluation::*, gear::*, observation::*, predicate::RuleLimits,
    types::*,
};
use common::*;
use serde_json::{Value, from_value, json};
use std::collections::BTreeSet;

fn known(v: Value) -> Value {
    json!({"status":"known","value":v})
}
fn na() -> Value {
    json!({"status":"not_applicable"})
}
fn observed(v: Value) -> Value {
    json!({"status":"known","value":v,"observed_at_utc_ms":100,"clock_trust":"trusted","source":"synthetic_fixture"})
}
fn demo() -> DataContext {
    from_value(context()).unwrap()
}
fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gear")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn parts(doc: &Value) -> (RoleProfile, SlotLayout, Vec<GearCandidate>) {
    (
        from_value(doc["profile"].clone()).unwrap(),
        from_value(doc["layout"].clone()).unwrap(),
        from_value(doc["candidates"].clone()).unwrap(),
    )
}
fn profile() -> RoleProfile {
    from_value(json!({"role":"synthetic-role","version":"gear-model-v1","weight_source":{"type":"synthetic_test"},"weights":{"synthetic-power":1000},"known_unmodeled":[]})).unwrap()
}
fn layout_of(slots: &[&str], hands: Option<(&str, &str)>) -> SlotLayout {
    from_value(
        json!({"slots":slots,"hands":hands.map(|(m,o)| json!({"main_hand":m,"off_hand":o}))}),
    )
    .unwrap()
}
struct Spec<'a> {
    id: &'a str,
    variant: &'a str,
    slots: &'a [&'a str],
    two_handed: Value,
    unique: Value,
    power: f64,
    effects: Value,
}
fn spec<'a>(id: &'a str, slots: &'a [&'a str], power: f64) -> Spec<'a> {
    Spec {
        id,
        variant: "base",
        slots,
        two_handed: na(),
        unique: na(),
        power,
        effects: known(json!([])),
    }
}
fn build(s: Spec<'_>) -> GearCandidate {
    from_value(json!({"item":{
        "key":{"context":context(),"item_id":s.id,"variant":s.variant},
        "slots":s.slots,"two_handed":s.two_handed,"unique_group":s.unique,
        "attributes":known(json!({"synthetic-power":s.power})),
        "requirements":known(json!({"op":"all","predicates":[]})),
        "effects":s.effects,"evidence":[]},
        "category":na()}))
    .unwrap()
}
fn scenario<'a>(
    context: &'a DataContext,
    profile: &'a RoleProfile,
    layout: &'a SlotLayout,
    candidates: &'a [GearCandidate],
) -> GearScenario<'a> {
    GearScenario {
        context,
        profile,
        layout,
        candidates,
    }
}
fn ids(items: &BTreeSet<&ItemKey>) -> Vec<String> {
    items
        .iter()
        .map(|k| k.item_id.as_str().to_string())
        .collect()
}
fn config_ids(config: &RankedConfiguration) -> Vec<String> {
    let keys: BTreeSet<&ItemKey> = config
        .configuration
        .items
        .values()
        .map(|p| &p.item)
        .collect();
    ids(&keys)
}
fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}
/// Constraint check shared by the loop-based invariant tests: the same rules
/// `GearRecommendationData::validate` enforces, plus no repeated candidate and
/// no known two-handed item next to an off-hand item.
fn assert_constraints(
    config: &RankedConfiguration,
    candidates: &[GearCandidate],
    hands: Option<(&str, &str)>,
) {
    let mut occupied = BTreeSet::new();
    let mut groups = BTreeSet::new();
    let mut used = BTreeSet::new();
    for (slot, placed) in &config.configuration.items {
        assert!(
            placed.occupied_slots.contains(slot),
            "item occupies its slot"
        );
        for s in &placed.occupied_slots {
            assert!(occupied.insert(s.clone()), "overlapping occupied slots");
        }
        assert!(used.insert(placed.item.clone()), "candidate used twice");
        if let Some(group) = placed.unique_group.value() {
            assert!(
                groups.insert(group.as_str().to_string()),
                "unique group repeated"
            );
        }
        let definition = candidates.iter().find(|c| c.key() == &placed.item).unwrap();
        assert!(
            definition.item.slots.contains(slot),
            "item placed in a slot it does not list"
        );
    }
    if let Some((main, off)) = hands
        && let Some(main_item) = config.configuration.items.get(&SlotId::new(main).unwrap())
    {
        let definition = candidates
            .iter()
            .find(|c| c.key() == &main_item.item)
            .unwrap();
        if definition.item.two_handed.value() == Some(&true) {
            assert!(
                !config
                    .configuration
                    .items
                    .contains_key(&SlotId::new(off).unwrap()),
                "two-handed item with an off-hand item"
            );
        }
    }
}

#[test]
fn manifest_lists_every_gear_fixture_and_marks_it_synthetic() {
    let manifest = fixture("manifest.json");
    assert_eq!(manifest["synthetic"], json!(true));
    let listed: BTreeSet<String> = manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_string())
        .collect();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gear");
    let mut on_disk = BTreeSet::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        if name != "manifest.json" {
            on_disk.insert(name);
        }
    }
    assert_eq!(listed, on_disk);
    for name in listed {
        assert_eq!(fixture(&name)["synthetic"], json!(true), "{name}");
    }
}

#[test]
fn case_006_two_handed_plus_shield_is_never_enumerated_as_valid() {
    let doc = fixture("case-006-two-hand-shield.json");
    let (profile, layout, candidates) = parts(&doc);
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    let expected = &doc["expected"];

    // Individually the two-handed weapon and the shield each lead their slot.
    let single = |name: &str| {
        ranking
            .valid
            .iter()
            .find(|r| config_ids(r) == vec![name.to_string()])
            .unwrap()
            .score
            .total
    };
    assert!(single("synthetic-2h-sword") > single("synthetic-1h-sword"));
    assert!(single("synthetic-shield") > 0);

    // No valid or uncertain configuration holds both.
    for entry in ranking.valid.iter().chain(&ranking.uncertain) {
        let held = config_ids(entry);
        assert!(
            !(held.contains(&"synthetic-2h-sword".to_string())
                && held.contains(&"synthetic-shield".to_string()))
        );
        assert_constraints(
            entry,
            &candidates,
            Some(("synthetic-main", "synthetic-off")),
        );
    }
    // The rejection is reported with its reason.
    let rejected = &ranking.rejected;
    assert_eq!(rejected.len(), 1);
    let mut names: Vec<String> = rejected[0]
        .items
        .values()
        .map(|k| k.item_id.as_str().to_string())
        .collect();
    names.sort();
    assert_eq!(names, strings(&expected["rejected_items"]));
    assert!(matches!(
        rejected[0].reasons.as_slice(),
        [ConfigurationRejection::TwoHandedWithOffHand { main_hand_item, off_hand_item }]
            if main_hand_item.item_id.as_str() == "synthetic-2h-sword"
                && off_hand_item.item_id.as_str() == "synthetic-shield"
    ));
    let code = serde_json::to_value(&rejected[0].reasons[0]).unwrap()["code"].clone();
    assert_eq!(code, expected["rejected_code"]);

    // Best valid is 1H + shield (12) rather than the 2H alone (10).
    let best = &ranking.valid[0];
    assert_eq!(config_ids(best), strings(&expected["best_valid_items"]));
    assert_eq!(
        best.score.total,
        expected["best_valid_total"].as_i64().unwrap()
    );
    assert_eq!(best.rank, 1);
    assert_eq!(best.configuration.validity, ConfigurationValidity::Valid);
    assert!(ranking.uncertain.is_empty());
    assert_eq!(ranking.ranking, Ranking::Theoretical);
    assert_eq!(ranking.label, "Best among verified candidates");
    assert!(ranking.top_is_definitive);
}

#[test]
fn case_006_two_handed_item_occupies_the_off_hand_slot() {
    let doc = fixture("case-006-two-hand-shield.json");
    let (profile, layout, candidates) = parts(&doc);
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    let two_handed = ranking
        .valid
        .iter()
        .find(|r| config_ids(r) == vec!["synthetic-2h-sword".to_string()])
        .unwrap();
    let placed = &two_handed.configuration.items[&SlotId::new("synthetic-main").unwrap()];
    let slots: Vec<&str> = placed.occupied_slots.iter().map(SlotId::as_str).collect();
    assert_eq!(slots, vec!["synthetic-main", "synthetic-off"]);
}

#[test]
fn case_006_unknown_two_handed_is_never_assumed_one_handed() {
    let doc = fixture("case-006-unknown-two-handed.json");
    let (profile, layout, candidates) = parts(&doc);
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    let expected = &doc["expected"];

    // The mystery weapon with the shield is uncertain, excluded from valid.
    assert_eq!(ranking.uncertain.len(), 1);
    let uncertain = &ranking.uncertain[0];
    assert_eq!(config_ids(uncertain), strings(&expected["uncertain_items"]));
    assert_eq!(
        uncertain.configuration.validity,
        ConfigurationValidity::Uncertain
    );
    assert!(matches!(
        uncertain.configuration.uncertainties.as_slice(),
        [CertaintyReason::UnknownTwoHanded { item }] if item.item_id.as_str() == "synthetic-mystery-sword"
    ));
    assert_ne!(uncertain.score.certainty, Certainty::Complete);
    for entry in &ranking.valid {
        assert!(
            !(config_ids(entry).contains(&"synthetic-mystery-sword".to_string())
                && config_ids(entry).contains(&"synthetic-shield".to_string()))
        );
    }
    // Its raw total is higher than any valid one, yet it does not lead and the
    // top result is not called definitive.
    assert!(uncertain.score.total > ranking.valid[0].score.total);
    assert_eq!(
        config_ids(&ranking.valid[0]),
        strings(&expected["best_valid_items"])
    );
    assert_eq!(
        ranking.valid[0].score.total,
        expected["best_valid_total"].as_i64().unwrap()
    );
    assert!(!ranking.top_is_definitive);
    // The mystery weapon alone (empty off-hand) is fine either way.
    assert!(
        ranking
            .valid
            .iter()
            .any(|r| config_ids(r) == vec!["synthetic-mystery-sword".to_string()])
    );
}

#[test]
fn paired_slots_are_filled_without_reusing_a_candidate() {
    let slots = ["synthetic-finger-1", "synthetic-finger-2"];
    let candidates = vec![
        build(spec("synthetic-ring-a", &slots, 5.0)),
        build(spec("synthetic-ring-b", &slots, 4.0)),
        build(spec("synthetic-ring-c", &slots, 3.0)),
    ];
    let layout = layout_of(&slots, None);
    let profile = profile();
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    // Subsets of size one and two: 3 + 3, each arrangement counted once.
    assert_eq!(ranking.valid.len(), 6);
    assert!(ranking.rejected.is_empty());
    assert_eq!(
        config_ids(&ranking.valid[0]),
        vec!["synthetic-ring-a", "synthetic-ring-b"]
    );
    assert_eq!(ranking.valid[0].score.total, 9000);
    for entry in &ranking.valid {
        assert_constraints(entry, &candidates, None);
    }
    // Ranks are consecutive and totals never increase down the list.
    for (i, entry) in ranking.valid.iter().enumerate() {
        assert_eq!(entry.rank, i + 1);
    }
    assert!(
        ranking
            .valid
            .windows(2)
            .all(|w| w[0].score.total >= w[1].score.total)
    );
}

#[test]
fn items_listing_only_one_of_the_paired_slots_are_still_placed() {
    let both = ["synthetic-finger-1", "synthetic-finger-2"];
    let first_only = ["synthetic-finger-1"];
    // The first-slot ring sorts after the flexible one, so a canonical
    // "ascending order" shortcut would lose the only valid pair.
    let candidates = vec![
        build(spec("synthetic-ring-a-flexible", &both, 1.0)),
        build(spec("synthetic-ring-z-first-only", &first_only, 9.0)),
    ];
    let layout = layout_of(&both, None);
    let profile = profile();
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    let best = &ranking.valid[0];
    assert_eq!(best.score.total, 10000);
    assert_eq!(
        best.configuration.items[&SlotId::new("synthetic-finger-1").unwrap()]
            .item
            .item_id
            .as_str(),
        "synthetic-ring-z-first-only"
    );
    assert_eq!(
        best.configuration.items[&SlotId::new("synthetic-finger-2").unwrap()]
            .item
            .item_id
            .as_str(),
        "synthetic-ring-a-flexible"
    );
}

#[test]
fn known_unique_group_blocks_the_pair_and_is_reported() {
    let slots = ["synthetic-finger-1", "synthetic-finger-2"];
    let mut a = spec("synthetic-ring-a", &slots, 5.0);
    a.unique = known(json!("synthetic-group-x"));
    let mut b = spec("synthetic-ring-b", &slots, 4.0);
    b.unique = known(json!("synthetic-group-x"));
    let c = spec("synthetic-ring-c", &slots, 1.0);
    let candidates = vec![build(a), build(b), build(c)];
    let layout = layout_of(&slots, None);
    let profile = profile();
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    assert_eq!(ranking.rejected.len(), 1);
    assert!(matches!(
        ranking.rejected[0].reasons.as_slice(),
        [ConfigurationRejection::UniqueGroupConflict { first, second }]
            if first.item_id.as_str() == "synthetic-ring-a" && second.item_id.as_str() == "synthetic-ring-b"
    ));
    // The best valid pair takes one unique ring plus the unrestricted one.
    assert_eq!(
        config_ids(&ranking.valid[0]),
        vec!["synthetic-ring-a", "synthetic-ring-c"]
    );
    for entry in &ranking.valid {
        assert_constraints(entry, &candidates, None);
    }
}

#[test]
fn unknown_unique_group_lowers_certainty_instead_of_being_non_unique() {
    let slots = ["synthetic-finger-1", "synthetic-finger-2"];
    let mut a = spec("synthetic-ring-a", &slots, 5.0);
    a.unique = json!({"status":"unknown"});
    let b = spec("synthetic-ring-b", &slots, 4.0);
    let candidates = vec![build(a), build(b)];
    let layout = layout_of(&slots, None);
    let profile = profile();
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    let pair = ranking
        .valid
        .iter()
        .find(|r| r.configuration.items.len() == 2)
        .unwrap();
    assert_eq!(pair.score.certainty, Certainty::Reduced);
    assert!(pair.score.certainty_reasons.iter().any(|r| matches!(
        r,
        CertaintyReason::UnknownUniqueGroup { item } if item.item_id.as_str() == "synthetic-ring-a"
    )));
    // A lone ring has nothing to conflict with, so its certainty is not reduced
    // by the unknown group (its other fields are complete).
    let alone = ranking
        .valid
        .iter()
        .find(|r| config_ids(r) == vec!["synthetic-ring-a"])
        .unwrap();
    assert_eq!(alone.score.certainty, Certainty::Complete);
    assert!(!ranking.top_is_definitive);
}

#[test]
fn search_space_and_candidate_limits_return_a_safe_error() {
    let profile = profile();
    let context = demo();
    let make = |n: usize| {
        let names: Vec<String> = (0..n).map(|i| format!("synthetic-slot-{i:02}")).collect();
        let candidates: Vec<GearCandidate> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                build(spec(
                    &format!("synthetic-item-{i:02}"),
                    &[name.as_str()],
                    1.0,
                ))
            })
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        (layout_of(&refs, None), candidates)
    };
    // 12 independent slots = exactly MAX_CONFIGURATIONS assignments: allowed.
    let (layout12, candidates12) = make(12);
    let ranking =
        rank_theoretical(&scenario(&context, &profile, &layout12, &candidates12)).unwrap();
    assert_eq!(ranking.valid.len(), MAX_CONFIGURATIONS - 1);
    // 13 would need 8192: refused, not truncated.
    let (layout13, candidates13) = make(13);
    let err =
        rank_theoretical(&scenario(&context, &profile, &layout13, &candidates13)).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    // More than 1024 candidates are refused before any search.
    let many: Vec<GearCandidate> = (0..1025)
        .map(|i| {
            build(spec(
                &format!("synthetic-many-{i:04}"),
                &["synthetic-slot-00"],
                1.0,
            ))
        })
        .collect();
    let single = layout_of(&["synthetic-slot-00"], None);
    let err = rank_theoretical(&scenario(&context, &profile, &single, &many)).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn invalid_scenarios_are_errors() {
    let profile = profile();
    let context = demo();
    let layout = layout_of(&["synthetic-slot"], None);
    // A slot outside the layout.
    let stray = vec![build(spec(
        "synthetic-stray",
        &["synthetic-elsewhere"],
        1.0,
    ))];
    assert_eq!(
        rank_theoretical(&scenario(&context, &profile, &layout, &stray))
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    // The same full key twice.
    let twin = vec![
        build(spec("synthetic-twin", &["synthetic-slot"], 1.0)),
        build(spec("synthetic-twin", &["synthetic-slot"], 2.0)),
    ];
    assert_eq!(
        rank_theoretical(&scenario(&context, &profile, &layout, &twin))
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    // A malformed layout cannot be constructed.
    assert!(from_value::<SlotLayout>(json!({"slots":["a","a"],"hands":null})).is_err());
    assert!(
        from_value::<SlotLayout>(
            json!({"slots":["a","b"],"hands":{"main_hand":"a","off_hand":"c"}})
        )
        .is_err()
    );
    assert!(
        from_value::<SlotLayout>(
            json!({"slots":["a","b"],"hands":{"main_hand":"a","off_hand":"a"}})
        )
        .is_err()
    );
}

#[test]
fn case_005_contexts_and_variants_stay_distinct() {
    let doc = fixture("case-005-context-isolation.json");
    let (profile, layout, candidates) = parts(&doc);
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    let expected = &doc["expected"];

    // Two variants of one item_id are two candidates; both fit the two trinket
    // slots together, and each keeps its own variant in the output.
    let best = &ranking.valid[0];
    assert_eq!(
        best.score.total,
        expected["best_valid_total"].as_i64().unwrap()
    );
    let variants: BTreeSet<&str> = best
        .configuration
        .items
        .values()
        .map(|p| p.item.variant.as_str())
        .collect();
    assert_eq!(
        variants.len() as u64,
        expected["distinct_variants"].as_u64().unwrap()
    );
    assert!(
        best.configuration
            .items
            .values()
            .all(|p| p.item.item_id.as_str() == "synthetic-charm")
    );

    // A candidate from another build with the same item_id is a BuildMismatch.
    let foreign: GearCandidate = from_value(doc["foreign_candidate"].clone()).unwrap();
    let mut mixed = candidates.clone();
    mixed.push(foreign);
    let err = rank_theoretical(&scenario(&context, &profile, &layout, &mixed)).unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
}

#[test]
fn case_015_demo_candidates_with_a_reference_scenario_are_rejected() {
    let doc = fixture("case-006-two-hand-shield.json");
    let (profile, layout, candidates) = parts(&doc);
    let mut reference = context();
    reference["namespace"] = json!("reference");
    let reference: DataContext = from_value(reference).unwrap();
    let err = rank_theoretical(&scenario(&reference, &profile, &layout, &candidates)).unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
}

#[test]
fn rankings_do_not_depend_on_input_order() {
    let doc = fixture("case-006-unknown-two-handed.json");
    let (profile, layout, candidates) = parts(&doc);
    let context = demo();
    let forward = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    for rotation in 0..candidates.len() {
        let mut shuffled = candidates.clone();
        shuffled.rotate_left(rotation);
        let again = rank_theoretical(&scenario(&context, &profile, &layout, &shuffled)).unwrap();
        assert_eq!(forward, again);
    }
    let mut reversed = candidates.clone();
    reversed.reverse();
    assert_eq!(
        forward,
        rank_theoretical(&scenario(&context, &profile, &layout, &reversed)).unwrap()
    );
    // Equal scores are ordered by item keys, not by input order.
    let slots = ["synthetic-finger-1", "synthetic-finger-2"];
    let equal = vec![
        build(spec("synthetic-ring-2", &slots, 3.0)),
        build(spec("synthetic-ring-1", &slots, 3.0)),
    ];
    let layout = layout_of(&slots, None);
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &equal)).unwrap();
    assert_eq!(
        config_ids(&ranking.valid[0]),
        vec!["synthetic-ring-1", "synthetic-ring-2"]
    );
    assert_eq!(config_ids(&ranking.valid[1]), vec!["synthetic-ring-1"]);
    assert_eq!(config_ids(&ranking.valid[2]), vec!["synthetic-ring-2"]);
}

#[test]
fn every_enumerated_configuration_satisfies_the_constraints() {
    let main_off = ("synthetic-main", "synthetic-off");
    let slots = [
        "synthetic-main",
        "synthetic-off",
        "synthetic-finger-1",
        "synthetic-finger-2",
    ];
    let mut two = spec("synthetic-2h", &["synthetic-main"], 9.0);
    two.two_handed = known(json!(true));
    let mut one = spec("synthetic-1h", &["synthetic-main"], 3.0);
    one.two_handed = known(json!(false));
    let mut mystery = spec("synthetic-mystery", &["synthetic-main"], 6.0);
    mystery.two_handed = json!({"status":"unknown"});
    let mut shield = spec("synthetic-shield", &["synthetic-off"], 4.0);
    shield.two_handed = known(json!(false));
    let mut ring_a = spec(
        "synthetic-ring-a",
        &["synthetic-finger-1", "synthetic-finger-2"],
        2.0,
    );
    ring_a.unique = known(json!("synthetic-group"));
    let mut ring_b = spec(
        "synthetic-ring-b",
        &["synthetic-finger-1", "synthetic-finger-2"],
        2.0,
    );
    ring_b.unique = known(json!("synthetic-group"));
    let ring_c = spec(
        "synthetic-ring-c",
        &["synthetic-finger-1", "synthetic-finger-2"],
        1.0,
    );
    let candidates: Vec<GearCandidate> = [two, one, mystery, shield, ring_a, ring_b, ring_c]
        .into_iter()
        .map(build)
        .collect();
    let layout = layout_of(&slots, Some(main_off));
    let profile = profile();
    let context = demo();
    let ranking = rank_theoretical(&scenario(&context, &profile, &layout, &candidates)).unwrap();
    assert!(ranking.valid.len() > 10);
    for entry in ranking.valid.iter().chain(&ranking.uncertain) {
        assert_constraints(entry, &candidates, Some(main_off));
        assert!(!entry.configuration.items.is_empty());
    }
    assert!(!ranking.rejected.is_empty());
    for r in &ranking.rejected {
        assert!(!r.reasons.is_empty());
    }
    // Uncertain entries are never valid: the mystery weapon never sits next to
    // a shield in the valid list.
    for entry in &ranking.valid {
        let held = config_ids(entry);
        assert!(
            !(held.contains(&"synthetic-mystery".to_string())
                && held.contains(&"synthetic-shield".to_string()))
        );
    }
    assert!(ranking.uncertain.iter().all(|e| {
        let held = config_ids(e);
        held.contains(&"synthetic-mystery".to_string())
            && held.contains(&"synthetic-shield".to_string())
    }));
}

fn eligibility_fixture(
    level: Value,
) -> (
    CharacterSnapshot,
    FreshnessContext,
    ProficiencyTable,
    Observation<SourceId>,
) {
    let mut s = snapshot();
    s["level"] = level;
    (
        from_value(s).unwrap(),
        FreshnessContext {
            now_utc_ms: Some(UtcMillis::new(150).unwrap()),
            clock_trust: ClockTrust::Trusted,
            session: None,
            max_age_ms: Millis::new(100).unwrap(),
        },
        from_value(json!({"classes":{"synthetic-class-a":{"allowed":[],"disallowed":[]}}}))
            .unwrap(),
        from_value(known(json!("synthetic-class-a"))).unwrap(),
    )
}
fn gated(id: &str, slot: &str, power: f64, requirement: Value) -> GearCandidate {
    from_value(json!({"item":{
        "key":{"context":context(),"item_id":id,"variant":"base"},
        "slots":[slot],"two_handed":na(),"unique_group":na(),
        "attributes":known(json!({"synthetic-power":power})),
        "requirements":requirement,
        "effects":known(json!([])),"evidence":[]},
        "category":na()}))
    .unwrap()
}

#[test]
fn obtainable_ranking_excludes_future_progression_and_lists_unknowns() {
    let slots = ["synthetic-head", "synthetic-chest", "synthetic-legs"];
    let none = known(json!({"op":"all","predicates":[]}));
    let candidates = vec![
        gated("synthetic-head-now", "synthetic-head", 2.0, none.clone()),
        gated(
            "synthetic-head-later",
            "synthetic-head",
            9.0,
            known(json!({"op":"level_at_least","level":40})),
        ),
        gated(
            "synthetic-chest-opaque",
            "synthetic-chest",
            5.0,
            json!({"status":"unknown"}),
        ),
        gated("synthetic-legs-now", "synthetic-legs", 1.0, none),
    ];
    let layout = layout_of(&slots, None);
    let profile = profile();
    let context = demo();
    let (snap, clock, table, class) = eligibility_fixture(observed(json!(10)));
    let evaluation = EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let eligibility = GearEligibilityContext {
        evaluation: &evaluation,
        class: &class,
        proficiency: &table,
    };
    let scenario = scenario(&context, &profile, &layout, &candidates);
    let theoretical = rank_theoretical(&scenario).unwrap();
    let obtainable = rank_obtainable(&scenario, &eligibility).unwrap();

    // Theoretical ignores eligibility; the high-level head piece leads it.
    assert!(
        theoretical.valid[0]
            .configuration
            .items
            .values()
            .any(|p| p.item.item_id.as_str() == "synthetic-head-later")
    );
    assert_eq!(obtainable.ranking, Ranking::Obtainable);
    assert_ne!(obtainable.ranking, Ranking::Practical);
    for entry in &obtainable.valid {
        let held = config_ids(entry);
        assert!(!held.contains(&"synthetic-head-later".to_string()));
        assert!(!held.contains(&"synthetic-chest-opaque".to_string()));
    }
    assert_eq!(
        config_ids(&obtainable.valid[0]),
        vec!["synthetic-head-now", "synthetic-legs-now"]
    );

    // Excluded candidates keep their distinct reasons.
    let excluded: Vec<(&str, &GearEligibility)> = obtainable
        .excluded
        .iter()
        .map(|e| (e.item.item_id.as_str(), &e.eligibility))
        .collect();
    assert_eq!(excluded.len(), 2);
    assert!(matches!(
        excluded[0],
        ("synthetic-chest-opaque", GearEligibility::Unknown { .. })
    ));
    assert!(matches!(
        excluded[1],
        (
            "synthetic-head-later",
            GearEligibility::FutureProgression { .. }
        )
    ));
    assert!(theoretical.excluded.is_empty());
    assert!(obtainable.scope_note.contains("acquisition"));
}

#[test]
fn obtainable_ranking_rejects_a_snapshot_from_another_context() {
    let candidates = vec![gated(
        "synthetic-a",
        "synthetic-head",
        1.0,
        known(json!({"op":"all","predicates":[]})),
    )];
    let layout = layout_of(&["synthetic-head"], None);
    let profile = profile();
    let context = demo();
    let mut s = snapshot();
    s["context"]["namespace"] = json!("reference");
    s["level"] = observed(json!(10));
    let snap: CharacterSnapshot = from_value(s).unwrap();
    let (_, clock, table, class) = eligibility_fixture(observed(json!(10)));
    let evaluation = EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let err = rank_obtainable(
        &scenario(&context, &profile, &layout, &candidates),
        &GearEligibilityContext {
            evaluation: &evaluation,
            class: &class,
            proficiency: &table,
        },
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::BuildMismatch);
}

fn equipped(item_id: &str, variant: Value) -> Value {
    json!({"state":"item","item":{"context":context(),"item_id":item_id,"variant":variant,"instance":unknown(),"modifiers":unknown()}})
}

#[test]
fn baseline_matches_by_full_key_and_keeps_unknown_variants_unresolved() {
    let slots = [
        "synthetic-a",
        "synthetic-b",
        "synthetic-c",
        "synthetic-d",
        "synthetic-e",
        "synthetic-f",
    ];
    let mut base = spec("synthetic-gear", &["synthetic-a"], 2.0);
    base.variant = "v1";
    let mut other_slot = spec("synthetic-gear", &["synthetic-b"], 6.0);
    other_slot.variant = "v2";
    let candidates = vec![
        build(base),
        build(other_slot),
        build(spec("synthetic-unrelated", &["synthetic-c"], 1.0)),
    ];
    let layout = layout_of(&slots, None);
    let profile = profile();
    let context = demo();
    let mut s = snapshot();
    s["level"] = observed(json!(10));
    s["equipment"] = json!({
        // Same item_id, variant that exists in the catalog: matches v1.
        "synthetic-a": observed(equipped("synthetic-gear", observed(json!("v1")))),
        // Same item_id but a variant the catalog lacks: never merged with v1/v2.
        "synthetic-b": observed(equipped("synthetic-gear", observed(json!("v3")))),
        // Unknown variant: cannot be matched.
        "synthetic-c": observed(equipped("synthetic-unrelated", unknown())),
        "synthetic-d": observed(json!({"state":"empty"})),
        // synthetic-e is unrecorded; synthetic-f is restricted.
        "synthetic-f": json!({"status":"restricted"})
    });
    let snap: CharacterSnapshot = from_value(s).unwrap();
    let clock = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    };
    let evaluation = EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let scenario = scenario(&context, &profile, &layout, &candidates);
    let baseline = resolve_baseline(&scenario, &evaluation).unwrap();
    let slot = |name: &str| &baseline.slots[&SlotId::new(name).unwrap()];
    assert!(
        matches!(slot("synthetic-a"), BaselineSlot::Matched { item } if item.variant.as_str() == "v1")
    );
    assert!(matches!(
        slot("synthetic-b"),
        BaselineSlot::Unresolved { gap: BaselineGap::NotInCatalog { variant, .. } } if variant.as_str() == "v3"
    ));
    assert!(matches!(
        slot("synthetic-c"),
        BaselineSlot::Unresolved {
            gap: BaselineGap::VariantUnavailable {
                status: ObservationStatus::Unknown,
                ..
            }
        }
    ));
    assert_eq!(slot("synthetic-d"), &BaselineSlot::Empty);
    assert!(matches!(
        slot("synthetic-e"),
        BaselineSlot::Unresolved {
            gap: BaselineGap::SlotNotRecorded
        }
    ));
    assert!(matches!(
        slot("synthetic-f"),
        BaselineSlot::Unresolved {
            gap: BaselineGap::SlotUnavailable {
                status: ObservationStatus::Restricted
            }
        }
    ));
    assert!(!baseline.is_complete());
    assert_eq!(baseline.matched().len(), 1);

    // Unresolved slots lower the baseline's certainty; the matched item scores.
    let score = score_baseline(&scenario, &baseline).unwrap();
    assert_eq!(score.total, 2000);
    assert_eq!(score.certainty, Certainty::Reduced);
    assert_eq!(
        score
            .certainty_reasons
            .iter()
            .filter(|r| matches!(r, CertaintyReason::BaselineSlotUnresolved { .. }))
            .count(),
        4
    );
}

#[test]
fn baseline_refuses_a_stale_slot_and_a_foreign_context() {
    let candidates = vec![build(spec("synthetic-gear", &["synthetic-a"], 2.0))];
    let layout = layout_of(&["synthetic-a"], None);
    let profile = profile();
    let context = demo();
    let mut s = snapshot();
    s["equipment"] = json!({"synthetic-a": {
        "status":"known","value":equipped("synthetic-gear", observed(json!("base"))),
        "observed_at_utc_ms":0,"clock_trust":"trusted","source":"synthetic_fixture"}});
    let snap: CharacterSnapshot = from_value(s).unwrap();
    let clock = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    };
    let make = |policy| EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: policy,
        limits: RuleLimits::default(),
    };
    let scenario = scenario(&context, &profile, &layout, &candidates);
    let stale = resolve_baseline(&scenario, &make(StalePolicy::Reject)).unwrap();
    assert!(matches!(
        stale.slots.values().next().unwrap(),
        BaselineSlot::Unresolved {
            gap: BaselineGap::SlotStale
        }
    ));
    let allowed = resolve_baseline(&scenario, &make(StalePolicy::AllowStale)).unwrap();
    assert!(matches!(
        allowed.slots.values().next().unwrap(),
        BaselineSlot::Matched { .. }
    ));

    let mut reference = context.clone();
    reference.namespace = DatasetNamespace::Reference;
    let mismatched = GearScenario {
        context: &reference,
        profile: &profile,
        layout: &layout,
        candidates: &candidates,
    };
    assert_eq!(
        resolve_baseline(&mismatched, &make(StalePolicy::Reject))
            .unwrap_err()
            .code,
        ErrorCode::BuildMismatch
    );
}

#[test]
fn generated_output_is_accepted_by_the_gear_recommendation_entity() {
    let doc = fixture("case-006-two-hand-shield.json");
    let (profile, layout, candidates) = parts(&doc);
    let context = demo();
    let scenario = scenario(&context, &profile, &layout, &candidates);
    let ranking = rank_theoretical(&scenario).unwrap();
    let snapshot_ref = SnapshotRef {
        id: SnapshotId::new("synthetic-snapshot").unwrap(),
        character: CharacterId::new("synthetic-character").unwrap(),
        context: context.clone(),
    };
    let data = recommendation_data(
        &scenario,
        &ranking,
        RecommendationInputs {
            snapshot: snapshot_ref.clone(),
            dataset_revision: DatasetRevision::new("synthetic-r1").unwrap(),
            evidence: vec![],
            baseline: None,
        },
    )
    .unwrap();
    assert_eq!(data.ranking, Ranking::Theoretical);
    assert_eq!(data.proposed_score.value().unwrap().get(), 12.0);
    assert_eq!(data.proposed.len(), 2);
    assert!(data.baseline_score.value().is_none());
    let recommendation = GearRecommendation::try_new(data).unwrap();
    assert!(
        recommendation
            .assumptions
            .iter()
            .any(|a| a.as_str().contains("not measured"))
    );

    // Every enumerated valid configuration passes the entity's checks.
    for entry in &ranking.valid {
        let mut data = recommendation_data(
            &scenario,
            &ranking,
            RecommendationInputs {
                snapshot: snapshot_ref.clone(),
                dataset_revision: DatasetRevision::new("synthetic-r1").unwrap(),
                evidence: vec![],
                baseline: None,
            },
        )
        .unwrap();
        data.proposed = RecordMap::new(entry.configuration.items.clone()).unwrap();
        assert!(GearRecommendation::try_new(data).is_ok());
    }

    // The entity does reject what the generator must never emit.
    let mut conflicting = recommendation_data(
        &scenario,
        &ranking,
        RecommendationInputs {
            snapshot: snapshot_ref,
            dataset_revision: DatasetRevision::new("synthetic-r1").unwrap(),
            evidence: vec![],
            baseline: None,
        },
    )
    .unwrap();
    let rejected = &ranking.rejected[0];
    let mut proposed = std::collections::BTreeMap::new();
    for (slot, key) in &rejected.items {
        let two_handed = key.item_id.as_str() == "synthetic-2h-sword";
        proposed.insert(
            slot.clone(),
            ConfigurationItem {
                item: key.clone(),
                occupied_slots: if two_handed {
                    vec![
                        SlotId::new("synthetic-main").unwrap(),
                        SlotId::new("synthetic-off").unwrap(),
                    ]
                } else {
                    vec![slot.clone()]
                },
                unique_group: from_value(na()).unwrap(),
            },
        );
    }
    conflicting.proposed = RecordMap::new(proposed).unwrap();
    assert!(GearRecommendation::try_new(conflicting).is_err());
}

#[test]
fn obtainable_recommendation_data_requires_and_carries_known_eligibility() {
    let slots = ["synthetic-head"];
    let candidates = vec![gated(
        "synthetic-head-now",
        "synthetic-head",
        2.0,
        known(json!({"op":"all","predicates":[]})),
    )];
    let layout = layout_of(&slots, None);
    let profile = profile();
    let context = demo();
    let (snap, clock, table, class) = eligibility_fixture(observed(json!(10)));
    let evaluation = EvaluationContext {
        snapshot: &snap,
        party: None,
        freshness: &clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let scenario = scenario(&context, &profile, &layout, &candidates);
    let ranking = rank_obtainable(
        &scenario,
        &GearEligibilityContext {
            evaluation: &evaluation,
            class: &class,
            proficiency: &table,
        },
    )
    .unwrap();
    let baseline = resolve_baseline(&scenario, &evaluation).unwrap();
    let baseline_score = score_baseline(&scenario, &baseline).unwrap();
    let data = recommendation_data(
        &scenario,
        &ranking,
        RecommendationInputs {
            snapshot: SnapshotRef {
                id: SnapshotId::new("synthetic-snapshot").unwrap(),
                character: CharacterId::new("synthetic-character").unwrap(),
                context: context.clone(),
            },
            dataset_revision: DatasetRevision::new("synthetic-r1").unwrap(),
            evidence: vec![],
            baseline: Some((&baseline, &baseline_score)),
        },
    )
    .unwrap();
    assert_eq!(data.eligibility.value(), Some(&true));
    // The snapshot recorded no equipment, so the baseline is incomplete.
    assert!(data.baseline_score.value().is_none());
    assert!(GearRecommendation::try_new(data).is_ok());

    // Nothing to recommend is an error, not an empty recommendation.
    let empty = ConfigurationRanking {
        valid: vec![],
        ..ranking
    };
    assert!(
        recommendation_data(
            &scenario,
            &empty,
            RecommendationInputs {
                snapshot: SnapshotRef {
                    id: SnapshotId::new("synthetic-snapshot").unwrap(),
                    character: CharacterId::new("synthetic-character").unwrap(),
                    context: context.clone(),
                },
                dataset_revision: DatasetRevision::new("synthetic-r1").unwrap(),
                evidence: vec![],
                baseline: None,
            },
        )
        .is_err()
    );
}
