use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::DatasetRevision;
use serde_json::{Value, json};
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
const CONTENT: &str = include_str!("../../../fixtures/desktop/content/demo-revision.json");
fn setup(
    build: &str,
    interface: &str,
    edit: impl FnOnce(&mut Value),
) -> (SnapshotApplication, String) {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = SNAPSHOT.replace("UNVERIFIED_DEMO", build);
    let mut snapshot: Value = serde_json::from_str(&snapshot).unwrap();
    snapshot["game"]["interface"] = json!(interface);
    snapshot["payload"]["context"]["build"]["interface"] = json!(interface);
    fn interfaces(v: &mut Value, interface: &str) {
        match v {
            Value::Object(m) => {
                if m.contains_key("interface") {
                    m.insert("interface".into(), json!(interface));
                }
                m.values_mut().for_each(|v| interfaces(v, interface));
            }
            Value::Array(a) => a.iter_mut().for_each(|v| interfaces(v, interface)),
            _ => {}
        }
    }
    interfaces(&mut snapshot, interface);
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), None)
        .unwrap()
        .profile_id;
    let mut content: Value =
        serde_json::from_str(&CONTENT.replace("UNVERIFIED_DEMO", build)).unwrap();
    interfaces(&mut content, interface);
    edit(&mut content);
    app.stage_content(&content.to_string()).unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    (app, profile)
}
#[test]
fn no_selection_is_unavailable_not_zero_and_records_only_narrow_historical_evidence() {
    let app = SnapshotApplication::in_memory().unwrap();
    let report = app.compatibility_report(None).unwrap();
    let value = serde_json::to_value(&report).unwrap();
    assert!(value["active"].is_null());
    assert!(value["selected"].is_null());
    let builds = value["recorded_builds"].as_array().unwrap();
    assert_eq!(builds.len(), 2);
    assert_eq!(builds[0]["build"]["build"], "70338");
    assert_eq!(builds[1]["basis"], "assumed_reference");
    let capabilities = builds[0]["capabilities"].as_array().unwrap();
    assert_eq!(
        capabilities
            .iter()
            .find(|c| c["capability"] == "C_Map.GetPlayerMapPosition")
            .unwrap()["state"],
        "function_present_only"
    );
    assert_eq!(
        capabilities
            .iter()
            .find(|c| c["capability"] == "CombatLogGetCurrentEventInfo")
            .unwrap()["state"],
        "reported_missing"
    );
    assert!(report.official_api.contains("not evidence"));
    assert_eq!(report.unverified_checks.len(), 4);
    assert!(
        serde_json::to_vec(&report).unwrap().len()
            < azeroth_app::compatibility::MAX_COMPATIBILITY_BYTES
    );
}
#[test]
fn exact_edition_build_interface_only_matches_its_historical_tuple() {
    for (build, interface, expected) in [
        ("70338", "16001", [true, false]),
        ("1.60.1.70205", "16001", [false, true]),
        ("70338", "future-interface", [false, false]),
        ("future-build", "16001", [false, false]),
    ] {
        let (app, profile) = setup(build, interface, |_| {});
        let report = app.compatibility_report(Some(&profile)).unwrap();
        assert!(report.selected.as_ref().unwrap().fixture);
        for (entry, expected) in report.recorded_builds.iter().zip(expected) {
            assert_eq!(entry.matches_selected_build, Some(expected));
        }
        let active = report.active.unwrap();
        assert_eq!(active.context.build.build.as_str(), build);
        assert_eq!(active.context.build.interface.as_str(), interface);
        assert!(active.fixture);
        assert_eq!(active.revision.as_str(), "demo-r1");
    }
}
#[test]
fn source_policy_values_and_unknown_statuses_are_counted_without_forwarding_private_strings() {
    let (app, profile) = setup("70338", "16001", |v| {
        v["sources"][0]["publisher"] = json!("private-publisher-do-not-forward");
        v["sources"][0]["location"] =
            json!({"status":"known","value":"private-url-do-not-forward"});
        v["sources"][0]["redistribution"] = json!({"status":"known","value":"conditional"});
        v["sources"][0]["offline_usable"] = json!({"status":"known","value":false});
        v["sources"][0]["retention"] = json!({"status":"restricted"});
        v["sources"][0]["attribution"] = json!({"status":"unsupported"});
        v["sources"][0]["transformations"] = json!({"status":"not_applicable"});
    });
    let report = app.compatibility_report(Some(&profile)).unwrap();
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("private-"));
    assert!(!text.contains("demo-character-paladin"));
    assert!(!text.contains("demo-snapshot-001"));
    let active = report.active.unwrap();
    let sources = active.sources;
    assert_eq!(sources.registered, 1);
    assert_eq!(sources.unregistered_distinct, 0);
    assert_eq!(sources.redistribution_values.conditional, 1);
    assert_eq!(sources.redistribution_values.permitted, 0);
    assert_eq!(sources.offline_values.known_false, 1);
    assert_eq!(sources.offline_values.known_true, 0);
    assert_eq!(sources.policy_statuses["retention"].restricted, 1);
    assert_eq!(sources.policy_statuses["attribution"].unsupported, 1);
    assert_eq!(sources.policy_statuses["license"].unknown, 1);
    assert_eq!(sources.policy_statuses["transformations"].not_applicable, 1);
    assert_eq!(
        active.counts.collections[&azeroth_core::coverage::Collection::MapImages].total,
        0
    );
    assert_eq!(
        active.counts.collections[&azeroth_core::coverage::Collection::Quests].total,
        1
    );
}
#[test]
fn a_selected_profile_without_active_content_has_no_fabricated_zero_counts() {
    let app = SnapshotApplication::in_memory().unwrap();
    let profile = app
        .import_text(&format!("ANAV1:{SNAPSHOT}"), None)
        .unwrap()
        .profile_id;
    let report = app.compatibility_report(Some(&profile)).unwrap();
    assert!(report.selected.is_some());
    assert!(report.active.is_none());
    for profile in ["", "../private-path", &"x".repeat(129)] {
        let error = app.compatibility_report(Some(profile)).unwrap_err();
        assert!(!error.to_string().contains("private-path"));
    }
}

#[test]
fn matching_numeric_build_from_another_edition_does_not_inherit_recorded_evidence() {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = SNAPSHOT
        .replace("UNVERIFIED_DEMO", "70338")
        .replace("wow_forever", "another_edition");
    let mut snapshot: Value = serde_json::from_str(&snapshot).unwrap();
    fn interface(v: &mut Value) {
        match v {
            Value::Object(m) => {
                if m.contains_key("interface") {
                    m.insert("interface".into(), json!("16001"));
                }
                m.values_mut().for_each(interface);
            }
            Value::Array(a) => a.iter_mut().for_each(interface),
            _ => {}
        }
    }
    interface(&mut snapshot);
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), None)
        .unwrap()
        .profile_id;
    let report = app.compatibility_report(Some(&profile)).unwrap();
    assert!(
        report
            .recorded_builds
            .iter()
            .all(|b| b.matches_selected_build == Some(false))
    );
}
