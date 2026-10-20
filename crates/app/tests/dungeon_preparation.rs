use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{
    error::ErrorCode,
    types::{DatasetRevision, PartyId, UtcMillis},
};
use serde_json::{Value, json};
const NOW: i64 = 1791792000000;
const PACKAGE: &str = include_str!("../../../fixtures/desktop/preparation/synthetic-package.json");
fn setup(
    count: usize,
    change: impl Fn(usize, &mut Value),
    stage: bool,
) -> (SnapshotApplication, Value) {
    let app = SnapshotApplication::in_memory().unwrap();
    let now = Some(UtcMillis::new(NOW).unwrap());
    let mut profiles = Vec::new();
    for index in 0..count {
        let mut s: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v1/character-snapshot.json"
        ))
        .unwrap();
        s["payload"]["character"] = json!(format!("member-{index}"));
        s["payload"]["id"] = json!(format!("snapshot-{index}"));
        s["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW - 100);
        s["payload"]["level"]["observed_at_utc_ms"] = json!(NOW - 100);
        s["payload"]["level"]["clock_trust"] = json!("trusted");
        s["payload"]["preferences"]["preparation-test"] = json!({"status":"known","value":false,"clock_trust":"trusted","observed_at_utc_ms":NOW-100});
        change(index, &mut s);
        profiles.push(
            app.import_text(&format!("ANAV1:{s}"), now)
                .unwrap()
                .profile_id,
        );
    }
    if stage {
        app.stage_content(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        app.activate_content(&profiles[0], &DatasetRevision::new("demo-r1").unwrap())
            .unwrap();
    }
    let subject = if count == 1 {
        json!({"kind":"character","profile_id":profiles[0]})
    } else {
        let id = PartyId::new("preparation-party").unwrap();
        app.save_party(&id, &profiles).unwrap();
        json!({"kind":"party","party_id":id})
    };
    (
        app,
        json!({"schema_version":1,"subject":subject,"package":serde_json::from_str::<Value>(PACKAGE).unwrap(),"origin":"demo-node-1"}),
    )
}
fn report(app: &SnapshotApplication, request: &Value) -> Value {
    serde_json::to_value(
        app.dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn local_reference_retains_false_zero_unknown_restricted_and_evidence_categories() {
    let (app, mut request) = setup(1, |_, _| {}, true);
    request["package"]["formation_ms"]["observation"] =
        json!({"status":"known","value":{"min_ms":0,"max_ms":0}});
    request["package"]["access"]["observation"] = json!({"status":"restricted"});
    let value = report(&app, &request);
    assert_eq!(value["schema_version"], 1);
    assert_eq!(
        value["package"]["formation_ms"]["observation"]["value"]["max_ms"],
        0
    );
    assert_eq!(value["members"][0]["access"]["availability"], "restricted");
    assert!(value["members"][0]["access"]["evaluation"].is_null());
    assert_eq!(
        value["members"][0]["checks"][0]["condition"]["evaluation"]["truth"],
        "false"
    );
    assert_eq!(
        value["members"][0]["rewards"][0]["item_requirement"]["evaluation"]["truth"],
        "true"
    );
    assert_eq!(
        value["members"][0]["rewards"][0]["acquisition_requirement"]["availability"],
        "unknown"
    );
    assert_eq!(value["evidence"][0]["confidence"], "limited_sample");
    assert!(
        value["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["code"] == "local_package_unverified")
    );
    assert!(!value.as_object().unwrap().contains_key("ready"));
}
#[test]
fn no_package_or_content_returns_explicit_gaps_instead_of_default_requirements() {
    let (app, mut request) = setup(1, |_, _| {}, false);
    request["package"] = Value::Null;
    let value = report(&app, &request);
    assert!(value["revision"].is_null());
    assert!(value["package"].is_null());
    assert!(value["route"].is_null());
    for code in ["package_missing", "active_content_missing"] {
        assert!(
            value["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == code)
        );
    }
    assert!(value["members"][0]["access"].is_null());
}
#[test]
fn stale_and_uncertain_members_return_gaps_without_quest_or_route_recommendations() {
    for stale in [true, false] {
        let (app, request) = setup(
            2,
            |index, s| {
                if index == 1 {
                    s["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW - 1_800_001);
                    if !stale {
                        s["payload"]["capture"]["clock_trust"] = json!("uncertain");
                    }
                }
            },
            true,
        );
        let value = report(&app, &request);
        assert_eq!(
            value["members"][1]["freshness"]["status"],
            if stale { "stale" } else { "uncertain" }
        );
        assert!(value["members"][1]["quests"][0]["assessment"].is_null());
        assert!(value["members"][1]["checks"][0]["condition"]["evaluation"].is_null());
        assert!(value["route"].is_null());
        assert!(
            value["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == "route_needs_fresh_members")
        );
    }
}
#[test]
fn exact_context_revision_sources_and_references_are_required() {
    let (app, request) = setup(1, |_, _| {}, true);
    for (field, value, code) in [
        (
            "revision",
            json!("other-revision"),
            ErrorCode::SnapshotOutdated,
        ),
        ("fixture", json!(false), ErrorCode::InvalidInput),
    ] {
        let mut bad = request.clone();
        bad["package"][field] = value;
        assert_eq!(
            app.dungeon_preparation(&bad.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .unwrap_err()
                .code,
            code
        );
    }
    let mut bad = request.clone();
    bad["package"]["context"]["build"]["build"] = json!("different-build");
    assert_eq!(
        app.dungeon_preparation(&bad.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap_err()
            .code,
        ErrorCode::BuildMismatch
    );
    for patch in ["evidence", "quest", "source", "entrance"] {
        let mut bad = request.clone();
        match patch {
            "evidence" => bad["package"]["access"]["evidence"] = json!(["unregistered-evidence"]),
            "quest" => bad["package"]["quests"]["observation"]["value"] = json!(["missing-quest"]),
            "source" => {
                bad["package"]["rewards"]["observation"]["value"][0]["source"] =
                    json!("missing-source")
            }
            _ => bad["package"]["entrance"]["observation"]["value"]["id"] = json!("missing-node"),
        }
        assert_eq!(
            app.dungeon_preparation(&bad.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
#[test]
fn strict_future_duplicate_unknown_malformed_and_oversize_data_fail_without_echo() {
    let (app, request) = setup(1, |_, _| {}, true);
    let now = Some(UtcMillis::new(NOW).unwrap());
    let cases = [
        request
            .to_string()
            .replacen("\"schema_version\":1", "\"schema_version\":3", 1),
        request.to_string().replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1,
        ),
        request
            .to_string()
            .replacen("\"title\":", "\"title\":\"PLANTED_SECRET\",\"title\":", 1),
        "{PLANTED_SECRET".to_owned(),
        " ".repeat(1024 * 1024 + 1),
    ];
    for text in cases {
        let error = app.dungeon_preparation(&text, now).unwrap_err();
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("PLANTED_SECRET")
        );
    }
    let mut bad = request.clone();
    bad["package"]["hidden_code"] = json!("PLANTED_SECRET");
    assert!(app.dungeon_preparation(&bad.to_string(), now).is_err());
    let mut bad = request.clone();
    bad["package"]["schema_version"] = json!(3);
    assert_eq!(
        app.dungeon_preparation(&bad.to_string(), now)
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedSchema
    );
    let mut bad = request.clone();
    bad["package"]["quests"]["observation"]["value"] = json!(vec!["demo-quest-1"; 33]);
    assert!(app.dungeon_preparation(&bad.to_string(), now).is_err());
}
#[test]
fn expanded_actual_evaluation_report_is_refused_before_publication() {
    let (app, mut request) = setup(40, |_, _| {}, true);
    let predicate = json!({"op":"all","predicates":(0..64).map(|i|json!({"op":"equals","field":{"field":"preference","key":format!("missing-{i}")},"value":{"type":"bool","value":true}})).collect::<Vec<_>>()});
    request["package"]["checks"]["observation"]["value"]=json!((0..32).map(|i|json!({"id":format!("many-gaps-{i}"),"label":"Synthetic missing observations","category":"equipment","requirement":{"observation":{"status":"known","value":predicate},"evidence":["demo-evidence-1"]}})).collect::<Vec<_>>());
    let Err(error) =
        app.dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
    else {
        panic!("Oversized evaluated report must fail")
    };
    assert_eq!(error.code, ErrorCode::ImportTooLarge);
    assert_eq!(app.profiles().unwrap().len(), 40);
}

#[test]
fn shared_route_checks_second_member_requirements_and_preserves_false_gate() {
    for allowed in [true, false] {
        let (app, mut request) = setup(
            2,
            |index, s| {
                s["payload"]["preferences"]["route-ready"] = json!({"status":"known","value":index==0||allowed,"clock_trust":"trusted","observed_at_utc_ms":NOW-100});
            },
            true,
        );
        let mut content: Value = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        content["envelope"]["payload"]["revision"] = json!("preparation-route-test");
        content["envelope"]["payload"]["edges"][0]["requirements"] = json!({"status":"known","value":{"op":"equals","field":{"field":"preference","key":"route-ready"},"value":{"type":"bool","value":true}}});
        app.stage_content(&content.to_string()).unwrap();
        let profile = app.profiles().unwrap()[0].id.clone();
        app.activate_content(
            &profile,
            &DatasetRevision::new("preparation-route-test").unwrap(),
        )
        .unwrap();
        request["package"]["revision"] = json!("preparation-route-test");
        let value = report(&app, &request);
        if allowed {
            assert_eq!(value["route"]["outcome"]["status"], "complete");
            assert_eq!(value["route"]["outcome"]["route"]["total_ms"], 30000);
        } else {
            assert_ne!(value["route"]["outcome"]["status"], "complete");
            assert!(
                value["route_exclusions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["character"] == "member-1"
                        && e["condition"]["evaluation"]["truth"] == "false")
            );
        }
    }
}

#[test]
fn report_wire_fixture_retains_stale_member_and_declared_zero_timing() {
    let (app, mut request) = setup(
        2,
        |index, s| {
            if index == 1 {
                s["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW - 1_800_001);
            }
        },
        true,
    );
    request["package"]["formation_ms"]["observation"] =
        json!({"status":"known","value":{"min_ms":0,"max_ms":0}});
    let actual = report(&app, &request);
    if std::env::var_os("NAVZEROTH_UPDATE_PREPARATION_FIXTURE").is_some() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/desktop/preparation/report.json"
            ),
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
    }
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/desktop/preparation/report.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(actual, expected);
}
