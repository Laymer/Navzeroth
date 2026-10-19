//! V2.4: the committed authored slice catalog decodes, passes the authored-content
//! rules, defines exactly the expected canonical records, and invented game facts
//! are refused. The catalog is original authoring, never verified game data.
use azeroth_core::{
    content_v2::{self, authored::validate_authored, records::content_records},
    types::SourceId,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const CATALOG: &[u8] = include_bytes!("../../../content/authored/slice-v1/catalog.json");
const REV: &str = "navzeroth-authored-slice-r3-stable-coverage";

fn source() -> SourceId {
    SourceId::new("navzeroth-authored-v1").unwrap()
}
fn value() -> Value {
    serde_json::from_slice(CATALOG).unwrap()
}
fn id(parts: Value) -> String {
    let mut all = vec![json!(REV)];
    all.extend(parts.as_array().unwrap().iter().cloned());
    serde_json::to_string(&all).unwrap()
}

#[test]
fn committed_catalog_is_authored_only_and_defines_the_expected_records() {
    let content = content_v2::decode(CATALOG).unwrap();
    validate_authored(&content, &source()).unwrap();
    assert!(!content.reference.fixture);
    assert_eq!(content.reference.payload.revision.as_str(), REV);
    let records = content_records(&content).unwrap();
    let domain = |d: &str| -> BTreeSet<String> {
        records
            .get(d)
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    };
    let models: BTreeSet<_> = [
        "warrior-melee-damage",
        "warrior-tank",
        "paladin-melee-damage",
        "paladin-healer",
        "paladin-tank",
        "hunter-ranged-damage",
        "shaman-melee-damage",
        "shaman-ranged-damage",
        "shaman-healer",
        "druid-melee-damage",
        "druid-ranged-damage",
        "druid-healer",
        "druid-tank",
        "rogue-melee-damage",
        "priest-healer",
        "priest-ranged-damage",
        "mage-ranged-damage",
        "warlock-ranged-damage",
    ]
    .iter()
    .flat_map(|m| {
        [(1, 20), (21, 40), (41, 60)]
            .into_iter()
            .map(move |(lo, hi)| id(json!(["models", format!("model:{m}-{lo}-{hi}")])))
    })
    .collect();
    assert_eq!(domain("class_roles"), models);
    assert_eq!(
        domain("dungeons"),
        BTreeSet::from([id(json!(["dungeons", "dungeon:the-deadmines"]))])
    );
    let mut rules: BTreeSet<_> = [
        "train-after-level-up",
        "deadmines-repair-restock",
        "deadmines-hearthstone",
        "learn-flight-points",
        "heuristic-level-boundary",
        "duo-compare-progress",
    ]
    .iter()
    .map(|r| id(json!(["rules", format!("reminder:{r}")])))
    .collect();
    for role in ["tank", "healer", "melee_damage", "ranged_damage"] {
        for (lo, hi) in [(1, 20), (21, 40), (41, 60)] {
            rules.insert(id(json!([
                "rules",
                format!("reminder:role-{role}-{lo}-{hi}")
            ])));
        }
    }
    assert_eq!(domain("reminders"), rules);
    // The only geography record is the authored placeholder the dungeon names.
    assert_eq!(
        domain("geography"),
        BTreeSet::from([id(json!([
            "map_spaces",
            "authored:the-deadmines",
            "authored:pending",
            null
        ]))])
    );
    for d in ["quests", "equipment", "acquisition", "travel"] {
        assert!(domain(d).is_empty(), "{d} must have no authored records");
    }
    for sources in records.values().flat_map(|r| r.values()) {
        assert_eq!(
            sources,
            &BTreeSet::from(["navzeroth-authored-v1".to_owned()])
        );
    }
    // Every admitted single-class quest restriction has a heuristic model.
    let classes: BTreeSet<_> = content.models.iter().map(|m| m.class.as_str()).collect();
    assert_eq!(
        classes,
        BTreeSet::from([
            "class:druid",
            "class:hunter",
            "class:shaman",
            "class:mage",
            "class:paladin",
            "class:priest",
            "class:rogue",
            "class:warlock",
            "class:warrior"
        ])
    );
    assert_eq!(content.models.len(), 54);
    for class in classes {
        for level in 1..=60 {
            assert!(content.models.iter().any(|m| m.class.as_str() == class
                && m.minimum_level.get() <= level
                && m.maximum_level.get() >= level));
        }
    }
}

fn refused(edit: impl FnOnce(&mut Value)) {
    let mut v = value();
    edit(&mut v);
    let bytes = serde_json::to_vec(&v).unwrap();
    // Each edit is still a valid v2 graph: only the authored rules refuse it.
    let content = content_v2::decode(&bytes)
        .unwrap_or_else(|e| panic!("edited catalog must still decode: {e:?} {v}"));
    assert!(validate_authored(&content, &source()).is_err());
}

#[test]
fn invented_game_facts_are_refused() {
    let known = |v: Value| json!({"status": "known", "value": v});
    // Even an empty known list claims the dungeon has no preparation quests.
    refused(|v| v["dungeons"][0]["preparation_quests"] = known(json!([])));
    refused(|v| v["dungeons"][0]["travel_edges"] = known(json!([])));
    refused(|v| v["dungeons"][0]["rewards"] = known(json!([])));
    refused(|v| v["dungeons"][0]["access"] = known(json!({"op": "level_at_least", "level": 15})));
    // A client map ID instead of the authored placeholder.
    refused(|v| {
        v["reference"]["payload"]["map_spaces"][0]["key"]["map_id"] = json!("36");
        v["dungeons"][0]["map"]["map_id"] = json!("36");
    });
    refused(|v| v["models"][0]["class"] = json!("1"));
    // Loot tables are game facts even without any item record.
    refused(|v| {
        v["loot_tables"] = json!([{"id":"loot:creature:1","evidence":["ev-authored-v1"],
            "table":"creature","source":{"status":"unknown"},
            "rows":[{"item":"1","min_count":1,"max_count":1}]}])
    });
    refused(|v| {
        v["models"][0]["layout"] = known(
            json!({"slots": ["main", "off"], "hands": {"main_hand": "main", "off_hand": "off"}}),
        )
    });
    refused(|v| {
        v["reference"]["payload"]["evidence"][0]["confidence"] = json!("verified_on_build")
    });
    // Predicates may read declared state only, not another participant's facts.
    refused(|v| {
        v["reference"]["payload"]["rules"][0]["predicate"] = json!({"op": "participant_state",
            "participant": "someone", "predicate": {"op": "level_at_least", "level": 2}})
    });
    refused(|v| {
        v["dungeons"][0]["checklist"]["value"][2]["condition"] =
            known(json!({"op": "participant_state",
            "participant": "someone", "predicate": {"op": "level_at_least", "level": 15}}))
    });
    let content = content_v2::decode(CATALOG).unwrap();
    assert!(validate_authored(&content, &SourceId::new("another-source").unwrap()).is_err());
    let mut fixture = content.clone();
    let mut reference = fixture.reference.into_inner();
    reference.fixture = true;
    reference.provenance.source = azeroth_core::observation::StateSource::SyntheticFixture;
    fixture.reference = azeroth_core::error::Validated::try_new(reference).unwrap();
    assert_eq!(
        validate_authored(&fixture, &source()).unwrap_err(),
        "Authored content is not a fixture"
    );
}

#[test]
fn game_fact_fields_stay_unknown_not_only_unclaimed() {
    // `not_applicable`, `unsupported` or `restricted` also states something about
    // the game (no access rule, no rewards, no layout), so authoring refuses them.
    for status in ["not_applicable", "unsupported", "restricted"] {
        let pending = || json!({"status": status});
        for field in [
            "rewards",
            "access",
            "preparation_quests",
            "entrance",
            "travel_edges",
        ] {
            refused(|v| v["dungeons"][0][field] = pending());
        }
        for field in ["proficiency", "layout"] {
            refused(|v| v["models"][0][field] = pending());
        }
        refused(|v| v["reference"]["payload"]["map_spaces"][0]["transform"] = pending());
    }
}

#[test]
fn authored_role_reminders_cover_each_band_without_guessing_unknown_or_stale_roles() {
    use azeroth_core::{
        contract::Envelope,
        entities::CharacterSnapshot,
        evaluation::{EvaluationContext, StalePolicy, Truth, evaluate_rule},
        observation::{ClockTrust, FreshnessContext},
        predicate::RuleLimits,
        types::{Millis, UtcMillis},
    };
    let content = content_v2::decode(CATALOG).unwrap();
    let envelope: Envelope<CharacterSnapshot> = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let mut raw = serde_json::to_value(&envelope.payload).unwrap();
    raw["location"] = json!({"status":"unknown"});
    raw["context"] = serde_json::to_value(&content.reference.payload.rules[0].context).unwrap();
    for field in ["equipment", "quests", "objectives"] {
        raw[field] = json!({});
    }
    let now = 1791791990100i64;
    let known = |value: Value| json!({"status":"known","value":value,"clock_trust":"trusted","observed_at_utc_ms":now-100,"source":"synthetic_fixture"});
    let clock = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(now).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(1000).unwrap(),
    };
    let rules: Vec<_> = content
        .reference
        .payload
        .rules
        .iter()
        .filter(|r| r.id.as_str().starts_with("reminder:role-"))
        .collect();
    assert_eq!(rules.len(), 12);
    for model in &content.models {
        for level in [model.minimum_level.get(), model.maximum_level.get()] {
            raw["level"] = known(json!(level));
            raw["role"] = known(json!(model.profile.role));
            let snapshot: CharacterSnapshot = serde_json::from_value(raw.clone()).unwrap();
            let context = EvaluationContext {
                snapshot: &snapshot,
                party: None,
                freshness: &clock,
                stale_policy: StalePolicy::Reject,
                limits: RuleLimits::default(),
            };
            let truths: Vec<_> = rules
                .iter()
                .filter(|r| evaluate_rule(r, &context).unwrap().truth == Truth::True)
                .collect();
            assert_eq!(truths.len(), 1, "{} level {level}", model.id.as_str());
            assert!(
                truths[0]
                    .result_template
                    .as_str()
                    .contains("Heuristic preparation")
            );
        }
    }
    raw["level"] = known(json!(20));
    for role in [
        json!({"status":"unknown"}),
        json!({"status":"restricted"}),
        json!({"status":"unsupported"}),
        json!({"status":"known","value":"healer","observed_at_utc_ms":now-2000,"clock_trust":"trusted","source":"synthetic_fixture"}),
    ] {
        raw["role"] = role;
        let snapshot: CharacterSnapshot = serde_json::from_value(raw.clone()).unwrap();
        let context = EvaluationContext {
            snapshot: &snapshot,
            party: None,
            freshness: &clock,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        };
        assert!(
            rules
                .iter()
                .all(|r| evaluate_rule(r, &context).unwrap().truth != Truth::True)
        );
    }
}
