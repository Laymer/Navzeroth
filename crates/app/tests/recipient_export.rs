mod support {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/recipient_export.rs"
    ));
}
use azeroth_core::{contract::decode_envelope, types::UtcMillis};
use azeroth_storage::{SqliteStore, content::ContentRevision};
use serde_json::{Value, json};
use support::*;
#[test]
fn exact_recipients_share_joint_choices_but_have_distinct_stable_identities() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let p = agree(&db);
    let one = export(&db, &p, 0).unwrap();
    let again = export(&db, &p, 0).unwrap();
    let two = export(&db, &p, 1).unwrap();
    assert_eq!(one.plan_id, again.plan_id);
    assert_ne!(one.plan_id, two.plan_id);
    for (index, exported) in [(0, one), (1, two)] {
        assert!(exported.byte_count <= 32768);
        let value: Value = serde_json::from_str(&exported.text[6..]).unwrap();
        let payload = &value["payload"];
        assert_eq!(payload["inputs"].as_array().unwrap().len(), 2);
        assert_eq!(payload["inputs"], value["dependencies"]["snapshots"]);
        let recipient = payload["inputs"][index]["character"].clone();
        assert_eq!(payload["steps"][0]["participants"], json!([recipient]));
        assert_eq!(
            payload["steps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["objective"].as_str().unwrap())
                .collect::<Vec<_>>(),
            p.selected_objectives
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(payload["ruleset_version"], "joint-greedy-v1");
    }
}
#[test]
fn every_local_acceptance_and_exact_cas_are_required() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let proposed = propose(&db, None);
    assert!(export(&db, &proposed, 0).is_err());
    let partial = accept(&db, &proposed, 0, true).unwrap();
    assert!(export(&db, &partial, 0).is_err());
    let agreed = accept(&db, &partial, 1, true).unwrap();
    assert!(export(&db, &partial, 0).is_err());
    assert!(export(&db, &agreed, 0).is_ok());
    let withdrawn = accept(&db, &agreed, 0, false).unwrap();
    assert!(export(&db, &agreed, 0).is_err());
    assert!(export(&db, &withdrawn, 0).is_err());
    let replacement = propose(&db, Some(withdrawn.guard));
    assert!(export(&db, &replacement, 0).is_err());
}
#[test]
fn recipient_and_request_bounds_are_checked_without_echoing_input() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let p = agree(&db);
    let mut request: Value = serde_json::from_str(&export_request(&p, 0)).unwrap();
    request["recipient_profile_id"] = json!("private-input-not-a-member");
    let error = db
        .app
        .export_agreed_party_plan(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap_err();
    assert!(!error.to_string().contains("private-input"));
    request["extra"] = json!(true);
    assert!(
        db.app
            .export_agreed_party_plan(&request.to_string(), None)
            .is_err()
    );
    assert!(
        db.app
            .export_agreed_party_plan(&" ".repeat(4097), None)
            .is_err()
    );
    assert!(
        db.app
            .export_agreed_party_plan("secret malformed", None)
            .unwrap_err()
            .to_string()
            .find("secret")
            .is_none()
    );
    assert!(
        db.app
            .export_agreed_party_plan(&export_request(&p, 0), None)
            .is_err()
    );
}
#[test]
fn refreshed_snapshot_cohort_and_content_invalidate_export() {
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let p = agree(&db);
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    snapshot["payload"]["id"] = json!("new-one");
    snapshot["payload"]["capture"]["sequence"] = json!(13);
    snapshot["producer"]["sequence"] = json!(13);
    snapshot["payload"]["character"] = json!("one");
    snapshot["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW + 100);
    db.app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW + 100).unwrap()),
        )
        .unwrap();
    assert!(export(&db, &p, 0).is_err());
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let p = agree(&db);
    let first = p.participants[0].profile_id.clone();
    db.app.save_party(&id(), &[first]).unwrap();
    assert!(export(&db, &p, 0).is_err());
    let db = setup(|_| {});
    members(&db, 23, NOW);
    let p = agree(&db);
    let mut store = SqliteStore::open(&db.path).unwrap();
    let mut revision: ContentRevision = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    let mut value = serde_json::to_value(&revision.envelope).unwrap();
    value["payload"]["revision"] = json!("demo-r2");
    revision.envelope = decode_envelope(&value.to_string()).unwrap();
    store.stage_content_revision(&revision).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    assert!(export(&db, &p, 0).is_err());
}
#[test]
fn valid_joint_readiness_that_exceeds_addon_rule_depth_is_refused() {
    let db = setup(|v| {
        let mut predicate = json!({"op":"level_at_least","level":22});
        for _ in 0..18 {
            predicate = json!({"op":"all","predicates":[predicate]});
        }
        v["payload"]["objectives"][0]["requirements"] = json!({"status":"known","value":predicate});
    });
    members(&db, 23, NOW);
    let p = agree(&db);
    assert!(export(&db, &p, 0).is_err());
    let still = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(still.guard, p.guard);
    assert!(matches!(
        still.state,
        azeroth_app::party_proposals::AgreementState::Agreed
    ));
}
#[test]
fn independently_expiring_member_field_bounds_export_and_invalidates_later_advice() {
    let db = setup(|_| {});
    let one = import_edit(&db.app, "one", 22, NOW, |v| {
        v["level"]["observed_at_utc_ms"] = json!(NOW - 1_790_000)
    });
    let two = import(&db.app, "two", 23, NOW);
    db.app.save_party(&id(), &[one, two]).unwrap();
    let p = agree(&db);
    let exported = export(&db, &p, 1).unwrap();
    assert_eq!(exported.expires_at_utc_ms.get(), NOW + 10_000);
    assert!(
        db.app
            .export_agreed_party_plan(
                &export_request(&p, 1),
                Some(UtcMillis::new(NOW + 20_000).unwrap())
            )
            .is_err()
    );
}
#[test]
fn joint_minimum_member_choice_is_not_replaced_with_recipient_solo_preference() {
    let db = setup(|v| {
        let mut second = v["payload"]["objectives"][0].clone();
        second["id"] = json!("balanced-objective");
        v["payload"]["objectives"]
            .as_array_mut()
            .unwrap()
            .push(second);
    });
    members(&db, 23, NOW);
    let mut planning = request(&db);
    planning["budget_ms"] = json!(60000);
    let one = planning["participants"][0]["profile_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let two = planning["participants"][1]["profile_id"]
        .as_str()
        .unwrap()
        .to_owned();
    planning["candidates"][0]["utilities"] = json!({one.clone():100,two.clone():1});
    let mut second = planning["candidates"][0].clone();
    second["objective"] = json!("balanced-objective");
    second["utilities"] = json!({one.clone():3,two:3});
    planning["candidates"].as_array_mut().unwrap().push(second);
    let p = db
        .app
        .propose_party_plan(
            &json!({"planning":planning,"proposer_profile_id":one,"expected":null}).to_string(),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap();
    let p = accept(&db, &p, 0, true).unwrap();
    let p = accept(&db, &p, 1, true).unwrap();
    assert_eq!(p.selected_objectives[0].as_str(), "balanced-objective");
    let text = export(&db, &p, 0).unwrap().text;
    let value: Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(
        value["payload"]["steps"][0]["objective"],
        "balanced-objective"
    );
    assert!(
        value["payload"]["steps"][0]["explanation"]
            .as_str()
            .unwrap()
            .contains("minimum-member group utility 3")
    );
}
#[test]
fn oversized_valid_joint_plan_is_refused_without_mutating_acceptances() {
    let db = setup(|v| {
        let mut base = v["payload"]["objectives"][0].clone();
        base["requirements"] = json!({"status":"known","value":{"op":"all","predicates":vec![json!({"op":"user_preference","key":"p".repeat(128),"value":true});100]}});
        v["payload"]["objectives"] = json!(
            (0..1)
                .map(|i| {
                    let mut o = base.clone();
                    o["id"] = json!(format!("wide-objective-{i}"));
                    o
                })
                .collect::<Vec<_>>()
        );
    });
    let members=["one","two"].iter().map(|who| import_edit(&db.app,who,23,NOW,|v| {v["preferences"]["p".repeat(128)]=json!({"status":"known","value":true,"clock_trust":"trusted","observed_at_utc_ms":NOW});})).collect::<Vec<_>>();
    db.app.save_party(&id(), &members).unwrap();
    let mut planning = request(&db);
    let base = planning["candidates"][0].clone();
    planning["candidates"] = json!(
        (0..1)
            .map(|i| {
                let mut c = base.clone();
                c["objective"] = json!(format!("wide-objective-{i}"));
                c["completion_ms"] = json!(1000);
                c
            })
            .collect::<Vec<_>>()
    );
    let p=db.app.propose_party_plan(&json!({"planning":planning,"proposer_profile_id":planning["participants"][0]["profile_id"],"expected":null}).to_string(),Some(UtcMillis::new(NOW).unwrap())).unwrap();
    let p = accept(&db, &p, 0, true).unwrap();
    let p = accept(&db, &p, 1, true).unwrap();
    assert_eq!(p.selected_objectives.len(), 1);
    assert!(export(&db, &p, 0).is_err());
    let still = db
        .app
        .party_plan_proposal(&id(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(still.guard, p.guard);
    assert!(still.participants.iter().all(|p| p.accepted));
}
