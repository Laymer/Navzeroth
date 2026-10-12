//! Synthetic engineering cases only (demo namespace, fictional IDs). They are not
//! game data and are not verified against a live client.
mod common;
use azeroth_core::{
    eligibility::*, entities::*, error::ErrorCode, evaluation::*, observation::*, predicate::*,
    types::*,
};
use common::*;
use serde_json::{Value, from_value, json, to_value};
use std::collections::BTreeMap;

fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"observed_at_utc_ms":100,"clock_trust":"trusted","source":"synthetic_fixture"})
}
fn stale(value: Value) -> Value {
    json!({"status":"known","value":value,"observed_at_utc_ms":1,"clock_trust":"trusted","source":"synthetic_fixture"})
}
fn undated(value: Value) -> Value {
    json!({"status":"known","value":value})
}
fn yes() -> Value {
    known(json!(true))
}
fn no() -> Value {
    known(json!(false))
}
fn clock() -> FreshnessContext {
    FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(150).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(100).unwrap(),
    }
}
fn turned_in(quest: &str) -> Value {
    json!({"op":"quest_state","quest":quest,"state":"turned_in","value":true})
}
fn level(n: u16) -> Value {
    json!({"op":"level_at_least","level":n})
}
fn all(predicates: &[Value]) -> Value {
    json!({"op":"all","predicates":predicates})
}
fn any(predicates: &[Value]) -> Value {
    json!({"op":"any","predicates":predicates})
}
fn requirement(predicate: Value) -> Value {
    known(predicate)
}
fn record(quest: &str, accepted: Value, completed: Value, turned_in: Value) -> Value {
    json!({"quest":quest,"context":context(),"accepted":accepted,"completed":completed,"turned_in":turned_in,"objectives":{"s1":no(),"s2":no()}})
}
fn untouched(quest: &str) -> Value {
    record(quest, no(), no(), no())
}
fn done(quest: &str) -> Value {
    record(quest, no(), yes(), yes())
}
fn character(name: &str, level_value: Value, records: Vec<Value>) -> Value {
    let mut s = snapshot();
    s["id"] = json!(format!("snap_{name}"));
    s["character"] = json!(name);
    s["level"] = level_value;
    for r in records {
        let key = r["quest"].as_str().unwrap().to_string();
        s["quests"][key] = r;
    }
    s
}
fn definition(id: &str, eligibility: Value, prerequisites: Value, completion: Value) -> Value {
    json!({"id":id,"context":context(),"eligibility":eligibility,"prerequisites":prerequisites,
        "stages":["s1","s2"],"locations":unknown(),"rewards":unknown(),"completion":completion,
        "shared_progress":unknown(),"evidence":[]})
}
fn chain_b(prerequisites: Value) -> Value {
    definition(
        "q_chain_b",
        requirement(all(&[])),
        requirement(prerequisites),
        known(json!("unrepeatable")),
    )
}
#[derive(Default)]
struct Setup {
    access: Option<Value>,
    groups: Vec<Value>,
    facts: Vec<(&'static str, Value, Value, Value)>,
    stale_policy: Option<StalePolicy>,
}
fn facts_map(setup: &Setup) -> BTreeMap<CharacterId, ParticipantFacts> {
    setup
        .facts
        .iter()
        .map(|(who, class, race, faction)| {
            (
                CharacterId::new(*who).unwrap(),
                ParticipantFacts {
                    class: from_value(class.clone()).unwrap(),
                    race: from_value(race.clone()).unwrap(),
                    faction: from_value(faction.clone()).unwrap(),
                },
            )
        })
        .collect()
}
fn with_inputs<T>(def: &Value, setup: &Setup, f: impl FnOnce(&EligibilityInputs<'_>) -> T) -> T {
    let definition: QuestDefinition = from_value(def.clone()).unwrap();
    let access: Option<AccessRequirements> = setup.access.clone().map(|a| from_value(a).unwrap());
    let groups: Vec<ExclusiveGroup> = setup
        .groups
        .iter()
        .map(|g| from_value(g.clone()).unwrap())
        .collect();
    let facts = facts_map(setup);
    let freshness = clock();
    f(&EligibilityInputs {
        definition: &definition,
        access: access.as_ref(),
        exclusive_groups: &groups,
        facts: &facts,
        freshness: &freshness,
        stale_policy: setup.stale_policy.unwrap_or_default(),
        limits: RuleLimits::default(),
    })
}
fn solo_with(
    def: &Value,
    snap: &Value,
    setup: &Setup,
) -> Result<ParticipantEligibility, ErrorCode> {
    let snapshot: CharacterSnapshot = from_value(snap.clone()).unwrap();
    with_inputs(def, setup, |i| {
        evaluate_participant(i, &snapshot, None).map_err(|e| e.code)
    })
}
fn solo(def: &Value, snap: &Value) -> ParticipantEligibility {
    solo_with(def, snap, &Setup::default()).unwrap()
}
fn party_of(members: Vec<Value>) -> PartySnapshot {
    from_value(json!({"id":"party1","context":context(),"participants":members})).unwrap()
}
fn party_with(def: &Value, members: Vec<Value>, setup: &Setup) -> PartyEligibility {
    let party = party_of(members);
    with_inputs(def, setup, |i| evaluate_party(i, &party).unwrap())
}
fn codes(p: &ParticipantEligibility) -> Vec<Value> {
    p.reasons.iter().map(|r| to_value(r).unwrap()).collect()
}
fn has_code(reasons: &[QuestEligibilityReason], code: &str) -> bool {
    reasons
        .iter()
        .any(|r| to_value(r).unwrap()["code"] == json!(code))
}
fn quest_unmet(quest: &str, state: QuestState) -> Unmet {
    Unmet::Single {
        requirement: Requirement::Quest {
            quest: QuestId::new(quest).unwrap(),
            state,
        },
    }
}

#[test]
fn case_001_unknown_history_is_never_false_blocked_or_complete() {
    let def = chain_b(turned_in("q_chain_a"));
    let snap = character("paladin", known(json!(22)), vec![]);
    let result = solo(&def, &snap);
    assert_eq!(result.acceptance, AcceptanceOutcome::Unknown);
    assert_eq!(result.progress, ProgressOutcome::Unknown);
    assert_eq!(result.completion, CompletionOutcome::Unknown);
    assert_eq!(result.turn_in, TurnInOutcome::Unknown);
    assert!(result.blockers.is_empty() && result.catch_up.is_none());
    let predecessor = result.reasons.iter().any(|r| {
        r.phase == Phase::Acceptance
            && matches!(&r.code, QuestEligibilityCode::QuestHistoryUnknown { history_quest, state, status: None }
                if history_quest.as_str() == "q_chain_a" && *state == QuestState::TurnedIn)
    });
    assert!(predecessor, "{:?}", codes(&result));
    assert!(result.reasons.iter().any(|r| r.phase == Phase::Completion
        && matches!(&r.code, QuestEligibilityCode::QuestHistoryUnknown { history_quest, .. }
            if history_quest.as_str() == "q_chain_b")));
}

#[test]
fn case_001_unknown_record_status_is_reported_with_its_status() {
    let def = chain_b(turned_in("q_chain_a"));
    let unknown_record = record("q_chain_a", unknown(), unknown(), unknown());
    let snap = character("paladin", known(json!(22)), vec![unknown_record]);
    let result = solo(&def, &snap);
    assert_eq!(result.acceptance, AcceptanceOutcome::Unknown);
    assert!(result.reasons.iter().any(|r| matches!(
        &r.code,
        QuestEligibilityCode::QuestHistoryUnknown {
            status: Some(ObservationStatus::Unknown),
            ..
        }
    )));
}

#[test]
fn case_001_not_of_unknown_history_stays_unknown() {
    let negated = json!({"op":"not","predicate":turned_in("q_chain_a")});
    let snap = character("paladin", known(json!(22)), vec![untouched("q_chain_b")]);
    let result = solo(&chain_b(negated), &snap);
    assert_eq!(result.acceptance, AcceptanceOutcome::Unknown);
}

fn case_002_party(mage_a: Value) -> PartyEligibility {
    let def = chain_b(turned_in("q_chain_a"));
    let paladin = character(
        "paladin",
        known(json!(22)),
        vec![done("q_chain_a"), untouched("q_chain_b")],
    );
    let mage = character(
        "mage",
        known(json!(22)),
        vec![mage_a, untouched("q_chain_b")],
    );
    party_with(&def, vec![paladin, mage], &Setup::default())
}

#[test]
fn case_002_one_participant_behind_gives_explicit_catch_up() {
    let party = case_002_party(untouched("q_chain_a"));
    let PartyOutcome::EligibleForSome {
        eligible,
        behind,
        catch_up,
    } = &party.outcome
    else {
        panic!("{:?}", party.outcome);
    };
    assert_eq!(eligible, &[CharacterId::new("paladin").unwrap()]);
    assert_eq!(behind, &[CharacterId::new("mage").unwrap()]);
    assert_eq!(catch_up.len(), 1);
    assert_eq!(catch_up[0].participant.as_str(), "mage");
    assert_eq!(catch_up[0].quest.as_str(), "q_chain_b");
    assert_eq!(
        catch_up[0].code,
        QuestEligibilityCode::CatchUpRequired {
            unmet: quest_unmet("q_chain_a", QuestState::TurnedIn)
        }
    );
    assert_eq!(
        party.participants[0].acceptance,
        AcceptanceOutcome::Eligible
    );
    assert_eq!(party.participants[1].acceptance, AcceptanceOutcome::Behind);
}

#[test]
fn case_002_unknown_mage_history_is_unknown_not_catch_up() {
    for mage_a in [
        record("q_chain_a", no(), no(), unknown()),
        record("q_chain_a", no(), no(), json!({"status":"restricted"})),
    ] {
        let party = case_002_party(mage_a);
        let PartyOutcome::Unknown { missing } = &party.outcome else {
            panic!("{:?}", party.outcome);
        };
        assert!(has_code(missing, "quest_history_unknown"));
        assert!(missing.iter().all(|r| r.participant.as_str() == "mage"));
    }
}

#[test]
fn case_002_missing_mage_record_is_unknown_not_catch_up() {
    let def = chain_b(turned_in("q_chain_a"));
    let paladin = character(
        "paladin",
        known(json!(22)),
        vec![done("q_chain_a"), untouched("q_chain_b")],
    );
    let mage = character("mage", known(json!(22)), vec![untouched("q_chain_b")]);
    let party = party_with(&def, vec![paladin, mage], &Setup::default());
    assert!(matches!(party.outcome, PartyOutcome::Unknown { .. }));
}

#[test]
fn case_002_exclusive_choice_made_by_one_participant_blocks_the_party() {
    let def = definition(
        "q_choice_right",
        requirement(all(&[])),
        requirement(all(&[])),
        known(json!("unrepeatable")),
    );
    let setup = Setup {
        groups: vec![json!({"quests":["q_choice_left","q_choice_right"]})],
        ..Setup::default()
    };
    let paladin = character(
        "paladin",
        known(json!(22)),
        vec![untouched("q_choice_left"), untouched("q_choice_right")],
    );
    let mage = character(
        "mage",
        known(json!(22)),
        vec![done("q_choice_left"), untouched("q_choice_right")],
    );
    let party = party_with(&def, vec![paladin, mage], &setup);
    let PartyOutcome::Blocked { reasons } = &party.outcome else {
        panic!("{:?}", party.outcome);
    };
    assert_eq!(reasons.len(), 1);
    assert_eq!(reasons[0].participant.as_str(), "mage");
    assert_eq!(reasons[0].quest.as_str(), "q_choice_right");
    assert_eq!(
        reasons[0].code,
        QuestEligibilityCode::ExclusiveChoiceTaken {
            other_quest: QuestId::new("q_choice_left").unwrap()
        }
    );
    assert_eq!(
        party.participants[0].acceptance,
        AcceptanceOutcome::Eligible
    );
    assert_eq!(party.participants[1].acceptance, AcceptanceOutcome::Blocked);
}

#[test]
fn exclusive_choice_unknown_or_missing_is_unknown_and_accepted_mode_blocks_earlier() {
    let def = definition(
        "q_choice_right",
        requirement(all(&[])),
        requirement(all(&[])),
        known(json!("repeatable")),
    );
    let mut setup = Setup {
        groups: vec![json!({"quests":["q_choice_left","q_choice_right"]})],
        ..Setup::default()
    };
    let missing = character("paladin", known(json!(22)), vec![]);
    let result = solo_with(&def, &missing, &setup).unwrap();
    assert_eq!(result.acceptance, AcceptanceOutcome::Unknown);
    assert!(has_code(&result.reasons, "exclusive_choice_unknown"));
    let accepted_only = record("q_choice_left", yes(), no(), no());
    let snap = character("paladin", known(json!(22)), vec![accepted_only]);
    assert_eq!(
        solo_with(&def, &snap, &setup).unwrap().acceptance,
        AcceptanceOutcome::Eligible
    );
    setup.groups =
        vec![json!({"quests":["q_choice_left","q_choice_right"],"taken_when":"accepted"})];
    assert_eq!(
        solo_with(&def, &snap, &setup).unwrap().acceptance,
        AcceptanceOutcome::Blocked
    );
    let group_elsewhere = Setup {
        groups: vec![json!({"quests":["q_other_x","q_other_y"]})],
        ..Setup::default()
    };
    let none = character("paladin", known(json!(22)), vec![]);
    assert_eq!(
        solo_with(&def, &none, &group_elsewhere).unwrap().acceptance,
        AcceptanceOutcome::Eligible
    );
}

#[test]
fn alternative_prerequisites_follow_three_valued_or() {
    let def = chain_b(any(&[turned_in("q_alt_x"), turned_in("q_chain_a")]));
    let outcome = |x: Value, a: Value| {
        let records = vec![
            record("q_alt_x", no(), no(), x),
            record("q_chain_a", no(), no(), a),
            untouched("q_chain_b"),
        ];
        solo(&def, &character("paladin", known(json!(22)), records))
    };
    assert_eq!(
        outcome(yes(), unknown()).acceptance,
        AcceptanceOutcome::Eligible
    );
    assert_eq!(
        outcome(no(), unknown()).acceptance,
        AcceptanceOutcome::Unknown
    );
    let both_false = outcome(no(), no());
    assert_eq!(both_false.acceptance, AcceptanceOutcome::Behind);
    assert!(has_code(
        &both_false.reasons,
        "alternative_prerequisite_unmet"
    ));
    assert_eq!(
        both_false.catch_up,
        Some(Unmet::OneOf {
            alternatives: vec![
                quest_unmet("q_alt_x", QuestState::TurnedIn),
                quest_unmet("q_chain_a", QuestState::TurnedIn)
            ]
        })
    );
}

#[test]
fn required_chain_false_dominates_unknown() {
    let def = chain_b(all(&[turned_in("q_alt_x"), turned_in("q_chain_a")]));
    let outcome = |x: Value, a: Value| {
        let records = vec![
            record("q_alt_x", no(), no(), x),
            record("q_chain_a", no(), no(), a),
            untouched("q_chain_b"),
        ];
        solo(&def, &character("paladin", known(json!(22)), records))
    };
    let one_false = outcome(no(), unknown());
    assert_eq!(one_false.acceptance, AcceptanceOutcome::Behind);
    assert_eq!(
        one_false.catch_up,
        Some(quest_unmet("q_alt_x", QuestState::TurnedIn))
    );
    assert_eq!(
        outcome(unknown(), no()).acceptance,
        AcceptanceOutcome::Behind
    );
    assert_eq!(
        outcome(yes(), unknown()).acceptance,
        AcceptanceOutcome::Unknown
    );
    assert_eq!(
        outcome(yes(), yes()).acceptance,
        AcceptanceOutcome::Eligible
    );
    let both = outcome(no(), no());
    assert_eq!(
        both.catch_up,
        Some(Unmet::AllOf {
            items: vec![
                quest_unmet("q_alt_x", QuestState::TurnedIn),
                quest_unmet("q_chain_a", QuestState::TurnedIn)
            ]
        })
    );
}

#[test]
fn not_catch_up_able_requirements_block() {
    let def = chain_b(json!({"op":"not","predicate":turned_in("q_alt_x")}));
    let snap = character(
        "paladin",
        known(json!(22)),
        vec![done("q_alt_x"), untouched("q_chain_b")],
    );
    let result = solo(&def, &snap);
    assert_eq!(result.acceptance, AcceptanceOutcome::Blocked);
    assert!(result.catch_up.is_none());
    assert!(!result.blockers.is_empty());
}

#[test]
fn minimum_level_known_unknown_and_below() {
    let def = definition(
        "q_chain_b",
        requirement(level(20)),
        requirement(all(&[])),
        known(json!("repeatable")),
    );
    let at = |l: Value| solo(&def, &character("paladin", l, vec![]));
    assert_eq!(at(known(json!(20))).acceptance, AcceptanceOutcome::Eligible);
    let below = at(known(json!(19)));
    assert_eq!(below.acceptance, AcceptanceOutcome::Behind);
    assert!(below.reasons.iter().any(|r| matches!(&r.code,
        QuestEligibilityCode::LevelBelowMinimum { level } if level.get() == 20)));
    let unknown_level = at(unknown());
    assert_eq!(unknown_level.acceptance, AcceptanceOutcome::Unknown);
    assert!(has_code(&unknown_level.reasons, "level_unknown"));
}

fn facts(class: Value, race: Value, faction: Value) -> Vec<(&'static str, Value, Value, Value)> {
    vec![("paladin", class, race, faction)]
}
#[test]
fn class_race_and_faction_allow_lists() {
    let def = definition(
        "q_chain_b",
        requirement(all(&[])),
        requirement(all(&[])),
        known(json!("repeatable")),
    );
    let snap = character("paladin", known(json!(22)), vec![]);
    let run = |class: Value, race: Value, faction: Value| {
        let setup = Setup {
            access: Some(
                json!({"classes":["paladin_c"],"races":["race_a","race_b"],"factions":["side_x"]}),
            ),
            facts: facts(class, race, faction),
            ..Setup::default()
        };
        solo_with(&def, &snap, &setup).unwrap()
    };
    let ok = run(
        known(json!("paladin_c")),
        known(json!("race_b")),
        known(json!("side_x")),
    );
    assert_eq!(ok.acceptance, AcceptanceOutcome::Eligible);
    let wrong_class = run(
        known(json!("mage_c")),
        known(json!("race_b")),
        known(json!("side_x")),
    );
    assert_eq!(wrong_class.acceptance, AcceptanceOutcome::Blocked);
    assert!(has_code(&wrong_class.blockers, "access_class_mismatch"));
    let wrong_race = run(
        known(json!("paladin_c")),
        known(json!("race_z")),
        known(json!("side_x")),
    );
    assert!(has_code(&wrong_race.blockers, "access_race_mismatch"));
    let wrong_faction = run(
        known(json!("paladin_c")),
        known(json!("race_a")),
        known(json!("side_y")),
    );
    assert!(has_code(&wrong_faction.blockers, "access_faction_mismatch"));
    for hidden in [
        unknown(),
        json!({"status":"restricted"}),
        json!({"status":"unsupported"}),
    ] {
        let result = run(hidden, known(json!("race_b")), known(json!("side_x")));
        assert_eq!(result.acceptance, AcceptanceOutcome::Unknown);
        assert!(has_code(&result.reasons, "access_fact_unknown"));
        assert!(result.blockers.is_empty());
    }
    let mismatch_beats_unknown = run(known(json!("mage_c")), unknown(), known(json!("side_x")));
    assert_eq!(
        mismatch_beats_unknown.acceptance,
        AcceptanceOutcome::Blocked
    );
}

#[test]
fn missing_participant_facts_are_unknown_and_unrestricted_access_needs_none() {
    let def = definition(
        "q_chain_b",
        requirement(all(&[])),
        requirement(all(&[])),
        known(json!("repeatable")),
    );
    let snap = character("paladin", known(json!(22)), vec![]);
    let restricted = Setup {
        access: Some(json!({"classes":["paladin_c"]})),
        ..Setup::default()
    };
    let result = solo_with(&def, &snap, &restricted).unwrap();
    assert_eq!(result.acceptance, AcceptanceOutcome::Unknown);
    let only_class = Setup {
        facts: facts(known(json!("paladin_c")), unknown(), unknown()),
        ..restricted
    };
    assert_eq!(
        solo_with(&def, &snap, &only_class).unwrap().acceptance,
        AcceptanceOutcome::Eligible
    );
    assert_eq!(
        solo_with(&def, &snap, &Setup::default())
            .unwrap()
            .acceptance,
        AcceptanceOutcome::Eligible
    );
}

#[test]
fn participant_facts_come_from_the_profile_and_the_caller() {
    let profile: CharacterProfile = from_value(json!({"id":"paladin","context":context(),
        "region":unknown(),"realm":unknown(),"race":known(json!("race_a")),"class":known(json!("paladin_c")),
        "level":known(json!(22)),"role":unknown(),"preferences":{}}))
    .unwrap();
    let faction: Observation<SourceId> = Observation::unknown();
    let facts = ParticipantFacts::from_profile(&profile, faction);
    assert_eq!(facts.class.value().unwrap().as_str(), "paladin_c");
    assert_eq!(facts.race.value().unwrap().as_str(), "race_a");
    assert_eq!(facts.faction.status(), ObservationStatus::Unknown);
}

#[test]
fn local_input_types_are_validated_and_bounded() {
    let bad = |v: Value| from_value::<AccessRequirements>(v).is_err();
    assert!(bad(json!({"classes":[]})));
    assert!(bad(json!({"races":[]})));
    let many: Vec<String> = (0..1025).map(|i| format!("c{i}")).collect();
    assert!(bad(json!({"factions":many})));
    assert!(from_value::<AccessRequirements>(json!({})).is_ok());
    let group = |v: Value| from_value::<ExclusiveGroup>(v);
    assert!(group(json!({"quests":["q_choice_left"]})).is_err());
    assert!(group(json!({"quests":[]})).is_err());
    let quests: Vec<String> = (0..1025).map(|i| format!("q{i}")).collect();
    assert!(group(json!({"quests":quests})).is_err());
    assert!(group(json!({"quests":["q_choice_left","q_choice_right"]})).is_ok());
}

#[test]
fn phases_are_separate() {
    let def = chain_b(all(&[]));
    let mid = record("q_chain_b", yes(), no(), no());
    let in_progress = solo(&def, &character("paladin", known(json!(22)), vec![mid]));
    assert_eq!(in_progress.progress, ProgressOutcome::InProgress);
    assert_eq!(in_progress.completion, CompletionOutcome::Incomplete);
    assert_eq!(in_progress.turn_in, TurnInOutcome::NotReady);
    let finished = record("q_chain_b", yes(), yes(), no());
    let ready = solo(
        &def,
        &character("paladin", known(json!(22)), vec![finished]),
    );
    assert_eq!(ready.completion, CompletionOutcome::Complete);
    assert_eq!(ready.turn_in, TurnInOutcome::Ready);
    assert_eq!(ready.progress, ProgressOutcome::AllStagesDone);
    let turned = solo(
        &def,
        &character("paladin", known(json!(22)), vec![done("q_chain_b")]),
    );
    assert_eq!(turned.turn_in, TurnInOutcome::AlreadyTurnedIn);
    assert_eq!(turned.acceptance, AcceptanceOutcome::Blocked);
    assert!(has_code(&turned.blockers, "already_turned_in"));
    let fresh = solo(
        &def,
        &character("paladin", known(json!(22)), vec![untouched("q_chain_b")]),
    );
    assert_eq!(fresh.progress, ProgressOutcome::NotStarted);
    assert_eq!(fresh.acceptance, AcceptanceOutcome::Eligible);
}

#[test]
fn stage_state_comes_from_the_quest_record() {
    let def = chain_b(all(&[]));
    let mut rec = record("q_chain_b", yes(), no(), no());
    rec["objectives"] = json!({"s1":yes(),"s2":yes()});
    let all_done = solo(
        &def,
        &character("paladin", known(json!(22)), vec![rec.clone()]),
    );
    assert_eq!(all_done.progress, ProgressOutcome::AllStagesDone);
    assert_eq!(all_done.completion, CompletionOutcome::Incomplete);
    rec["objectives"] = json!({"s1":yes(),"s2":unknown()});
    let partial = solo(
        &def,
        &character("paladin", known(json!(22)), vec![rec.clone()]),
    );
    assert_eq!(partial.progress, ProgressOutcome::Unknown);
    assert!(has_code(&partial.reasons, "stage_unknown"));
    rec["objectives"] = json!({"s1":yes()});
    let absent = solo(
        &def,
        &character("paladin", known(json!(22)), vec![rec.clone()]),
    );
    assert_eq!(absent.progress, ProgressOutcome::Unknown);
    rec["objectives"] = json!({"s1":no(),"s2":unknown()});
    let one_false = solo(&def, &character("paladin", known(json!(22)), vec![rec]));
    assert_eq!(one_false.progress, ProgressOutcome::InProgress);
}

#[test]
fn repeatability_semantics() {
    let with = |completion: Value| {
        definition(
            "q_chain_b",
            requirement(all(&[])),
            requirement(all(&[])),
            completion,
        )
    };
    let snap = character("paladin", known(json!(22)), vec![done("q_chain_b")]);
    for semantics in ["repeatable", "reset_bound"] {
        let r = solo(&with(known(json!(semantics))), &snap);
        assert_eq!(r.acceptance, AcceptanceOutcome::Eligible, "{semantics}");
        assert_eq!(r.turn_in, TurnInOutcome::AlreadyTurnedIn);
    }
    for hidden in [
        unknown(),
        json!({"status":"restricted"}),
        json!({"status":"unsupported"}),
        json!({"status":"not_applicable"}),
    ] {
        let r = solo(&with(hidden), &snap);
        assert_eq!(r.acceptance, AcceptanceOutcome::Unknown);
        assert!(has_code(&r.reasons, "repeatability_unknown"));
    }
    let unknown_turn_in = record("q_chain_b", no(), no(), unknown());
    let r = solo(
        &with(known(json!("unrepeatable"))),
        &character("paladin", known(json!(22)), vec![unknown_turn_in]),
    );
    assert_eq!(r.acceptance, AcceptanceOutcome::Unknown);
}

#[test]
fn unavailable_requirement_data_is_unknown_with_a_reason() {
    let snap = character("paladin", known(json!(22)), vec![untouched("q_chain_b")]);
    for status in ["unknown", "unsupported", "restricted", "not_applicable"] {
        let hidden = json!({"status":status});
        let in_eligibility = definition(
            "q_chain_b",
            hidden.clone(),
            requirement(all(&[])),
            known(json!("repeatable")),
        );
        let in_prerequisites = definition(
            "q_chain_b",
            requirement(all(&[])),
            hidden,
            known(json!("repeatable")),
        );
        for def in [in_eligibility, in_prerequisites] {
            let r = solo(&def, &snap);
            assert_eq!(r.acceptance, AcceptanceOutcome::Unknown, "{status}");
            assert!(has_code(&r.reasons, "requirement_data_unknown"));
        }
    }
}

#[test]
fn stale_and_clock_uncertain_observations_are_unknown_under_reject() {
    let def = chain_b(turned_in("q_chain_a"));
    let old = record("q_chain_a", no(), no(), stale(json!(true)));
    let snap = character(
        "paladin",
        known(json!(22)),
        vec![old, untouched("q_chain_b")],
    );
    let rejected = solo(&def, &snap);
    assert_eq!(rejected.acceptance, AcceptanceOutcome::Unknown);
    assert!(has_code(&rejected.reasons, "observation_stale"));
    let allow = Setup {
        stale_policy: Some(StalePolicy::AllowStale),
        ..Setup::default()
    };
    let allowed = solo_with(&def, &snap, &allow).unwrap();
    assert_eq!(allowed.acceptance, AcceptanceOutcome::Eligible);
    assert!(has_code(&allowed.warnings, "observation_stale"));
    let undated_record = record("q_chain_a", no(), no(), undated(json!(true)));
    let snap = character(
        "paladin",
        known(json!(22)),
        vec![undated_record, untouched("q_chain_b")],
    );
    let uncertain = solo(&def, &snap);
    assert_eq!(uncertain.acceptance, AcceptanceOutcome::Unknown);
    assert!(has_code(&uncertain.reasons, "clock_uncertain"));
}

#[test]
fn stale_stage_and_access_observations_are_reported() {
    let def = chain_b(all(&[]));
    let mut rec = record("q_chain_b", yes(), no(), no());
    rec["objectives"] = json!({"s1":yes(),"s2":stale(json!(true))});
    let r = solo(&def, &character("paladin", known(json!(22)), vec![rec]));
    assert_eq!(r.progress, ProgressOutcome::Unknown);
    assert!(has_code(&r.reasons, "observation_stale"));
    let setup = Setup {
        access: Some(json!({"classes":["paladin_c"]})),
        facts: facts(undated(json!("paladin_c")), unknown(), unknown()),
        ..Setup::default()
    };
    let snap = character("paladin", known(json!(22)), vec![untouched("q_chain_b")]);
    let r = solo_with(&def, &snap, &setup).unwrap();
    assert_eq!(r.acceptance, AcceptanceOutcome::Unknown);
    assert!(has_code(&r.reasons, "clock_uncertain"));
}

#[test]
fn data_context_mismatch_is_an_error_not_a_truth() {
    let mut def = chain_b(all(&[]));
    def["context"]["namespace"] = json!("reference");
    for key in ["eligibility", "prerequisites"] {
        def[key] = unknown();
    }
    let snap = character("paladin", known(json!(22)), vec![]);
    assert_eq!(
        solo_with(&def, &snap, &Setup::default()).unwrap_err(),
        ErrorCode::BuildMismatch
    );
    let mut other_build = chain_b(all(&[]));
    other_build["context"]["build"]["build"] = json!("OTHER");
    assert_eq!(
        solo_with(&other_build, &snap, &Setup::default()).unwrap_err(),
        ErrorCode::BuildMismatch
    );
    let party = party_of(vec![snap]);
    let result = with_inputs(&def, &Setup::default(), |i| evaluate_party(i, &party));
    assert_eq!(result.unwrap_err().code, ErrorCode::BuildMismatch);
}

#[test]
fn evaluation_errors_propagate_and_are_never_unknown() {
    let def = chain_b(all(&[
        turned_in("q_chain_a"),
        level(1),
        turned_in("q_alt_x"),
    ]));
    let snap = character("paladin", known(json!(22)), vec![untouched("q_chain_b")]);
    let snapshot: CharacterSnapshot = from_value(snap).unwrap();
    let definition: QuestDefinition = from_value(def).unwrap();
    let facts = BTreeMap::new();
    let freshness = clock();
    let inputs = EligibilityInputs {
        definition: &definition,
        access: None,
        exclusive_groups: &[],
        facts: &facts,
        freshness: &freshness,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::new(32, 1024, 2).unwrap(),
    };
    assert!(evaluate_participant(&inputs, &snapshot, None).is_err());
}

#[test]
fn party_snapshot_must_match_the_member_snapshot() {
    let def = chain_b(all(&[]));
    let in_party = character("paladin", known(json!(22)), vec![]);
    let party = party_of(vec![in_party]);
    let diverged: CharacterSnapshot =
        from_value(character("paladin", known(json!(30)), vec![])).unwrap();
    let result = with_inputs(&def, &Setup::default(), |i| {
        evaluate_participant(i, &diverged, Some(&party))
    });
    assert!(result.is_err());
}

#[test]
fn party_outcomes_cover_all_behind_and_jointly_eligible() {
    let def = chain_b(turned_in("q_chain_a"));
    let behind = |name: &str| {
        character(
            name,
            known(json!(22)),
            vec![untouched("q_chain_a"), untouched("q_chain_b")],
        )
    };
    let ready = |name: &str| {
        character(
            name,
            known(json!(22)),
            vec![done("q_chain_a"), untouched("q_chain_b")],
        )
    };
    let none = party_with(
        &def,
        vec![behind("paladin"), behind("mage")],
        &Setup::default(),
    );
    let PartyOutcome::AllBehind { catch_up } = &none.outcome else {
        panic!("{:?}", none.outcome);
    };
    assert_eq!(catch_up.len(), 2);
    let both = party_with(
        &def,
        vec![ready("paladin"), ready("mage")],
        &Setup::default(),
    );
    assert_eq!(both.outcome, PartyOutcome::JointlyEligible);
}

#[test]
fn blocked_outranks_unknown_and_unknown_outranks_catch_up() {
    let def = chain_b(turned_in("q_chain_a"));
    let blocked = character(
        "paladin",
        known(json!(22)),
        vec![done("q_chain_a"), done("q_chain_b")],
    );
    let unknown_one = character("mage", known(json!(22)), vec![]);
    let party = party_with(&def, vec![blocked, unknown_one], &Setup::default());
    assert!(matches!(party.outcome, PartyOutcome::Blocked { .. }));
    let eligible = character(
        "paladin",
        known(json!(22)),
        vec![done("q_chain_a"), untouched("q_chain_b")],
    );
    let behind = character(
        "rogue",
        known(json!(22)),
        vec![untouched("q_chain_a"), untouched("q_chain_b")],
    );
    let unknown_one = character("mage", known(json!(22)), vec![]);
    let party = party_with(&def, vec![eligible, behind, unknown_one], &Setup::default());
    assert!(matches!(party.outcome, PartyOutcome::Unknown { .. }));
}

#[test]
fn shared_progress_never_changes_or_implies_party_eligibility() {
    let outcome = |shared: Value| {
        let mut def = chain_b(turned_in("q_chain_a"));
        def["shared_progress"] = shared;
        let paladin = character(
            "paladin",
            known(json!(22)),
            vec![done("q_chain_a"), untouched("q_chain_b")],
        );
        let mage = character(
            "mage",
            known(json!(22)),
            vec![untouched("q_chain_a"), untouched("q_chain_b")],
        );
        party_with(&def, vec![paladin, mage], &Setup::default())
    };
    let unknown_shared = outcome(unknown());
    assert_eq!(unknown_shared, outcome(yes()));
    assert_eq!(unknown_shared, outcome(no()));
    assert!(matches!(
        unknown_shared.outcome,
        PartyOutcome::EligibleForSome { .. }
    ));
}

#[test]
fn unknown_required_inputs_never_become_a_definite_affirmative() {
    // Five required inputs, each known-true, known-false or unknown (243 combinations).
    let def = definition(
        "q_chain_b",
        requirement(level(20)),
        requirement(turned_in("q_chain_a")),
        known(json!("unrepeatable")),
    );
    let states = [Some(true), Some(false), None];
    let mut checked = 0;
    for (lvl, prereq, class, choice, own) in combinations(states) {
        let level_obs = match lvl {
            Some(true) => known(json!(25)),
            Some(false) => known(json!(5)),
            None => unknown(),
        };
        let obs = |v: Option<bool>| v.map_or_else(unknown, |b| known(json!(b)));
        let records = vec![
            record("q_chain_a", no(), no(), obs(prereq)),
            record("q_choice_left", no(), no(), obs(choice.map(|taken| !taken))),
            record("q_chain_b", no(), no(), obs(own.map(|done| !done))),
        ];
        let class_obs = class.map_or_else(unknown, |ok| {
            known(json!(if ok { "paladin_c" } else { "mage_c" }))
        });
        let setup = Setup {
            access: Some(json!({"classes":["paladin_c"]})),
            groups: vec![json!({"quests":["q_choice_left","q_chain_b"]})],
            facts: facts(class_obs, unknown(), unknown()),
            stale_policy: None,
        };
        let result = solo_with(&def, &character("paladin", level_obs, records), &setup).unwrap();
        // Each input is "requirement satisfied"; choice and own are satisfied while not turned in.
        let inputs = [lvl, prereq, class, choice, own];
        let any_false = inputs.contains(&Some(false));
        let any_unknown = inputs.contains(&None);
        let expected_eligible = !any_false && !any_unknown;
        assert_eq!(
            result.acceptance == AcceptanceOutcome::Eligible,
            expected_eligible,
            "{inputs:?}"
        );
        if any_unknown && !any_false {
            assert_eq!(result.acceptance, AcceptanceOutcome::Unknown, "{inputs:?}");
        }
        if any_false {
            assert!(
                matches!(
                    result.acceptance,
                    AcceptanceOutcome::Behind | AcceptanceOutcome::Blocked
                ),
                "{inputs:?}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 243);
}
type Combo = (
    Option<bool>,
    Option<bool>,
    Option<bool>,
    Option<bool>,
    Option<bool>,
);
fn combinations(states: [Option<bool>; 3]) -> Vec<Combo> {
    let mut out = vec![];
    for a in states {
        for b in states {
            for c in states {
                for d in states {
                    for e in states {
                        out.push((a, b, c, d, e));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn output_is_deterministic_and_serializes_stably() {
    let build = || {
        let def = chain_b(all(&[
            turned_in("q_chain_a"),
            turned_in("q_alt_x"),
            level(30),
        ]));
        let paladin = character(
            "paladin",
            known(json!(22)),
            vec![done("q_chain_a"), untouched("q_chain_b")],
        );
        let mage = character("mage", unknown(), vec![untouched("q_chain_a")]);
        party_with(&def, vec![paladin, mage], &Setup::default())
    };
    let (first, second) = (build(), build());
    assert_eq!(first, second);
    assert_eq!(
        to_value(&first).unwrap().to_string(),
        to_value(&second).unwrap().to_string()
    );
    let order: Vec<&str> = first
        .participants
        .iter()
        .map(|p| p.participant.as_str())
        .collect();
    assert_eq!(order, ["paladin", "mage"]);
}

#[test]
fn reason_serialization_is_a_closed_snake_case_shape() {
    let def = chain_b(turned_in("q_chain_a"));
    let snap = character("paladin", known(json!(22)), vec![]);
    let reasons = codes(&solo(&def, &snap));
    let first = &reasons[0];
    assert_eq!(first["code"], "quest_history_unknown");
    assert_eq!(first["participant"], "paladin");
    assert_eq!(first["quest"], "q_chain_b");
    assert_eq!(first["phase"], "acceptance");
    assert_eq!(first["history_quest"], "q_chain_a");
    assert_eq!(first["state"], "turned_in");
    let catch_up = QuestEligibilityReason {
        participant: CharacterId::new("mage").unwrap(),
        quest: QuestId::new("q_chain_b").unwrap(),
        phase: Phase::Acceptance,
        code: QuestEligibilityCode::CatchUpRequired {
            unmet: quest_unmet("q_chain_a", QuestState::TurnedIn),
        },
    };
    assert_eq!(
        to_value(&catch_up).unwrap(),
        json!({"participant":"mage","quest":"q_chain_b","phase":"acceptance","code":"catch_up_required",
            "unmet":{"kind":"single","requirement":{"kind":"quest","quest":"q_chain_a","state":"turned_in"}}})
    );
}
