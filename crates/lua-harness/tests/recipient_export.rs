//! Actual app-produced agreed recipient plans consumed by production Lua.
mod common;
use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, PartyId, UtcMillis};
use serde_json::{Value, json};
const NOW: i64 = 1791792000000;
fn exported(remote: bool) -> (String, String, Vec<String>) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshots = Vec::new();
    let mut profiles = Vec::new();
    for who in ["one", "two"] {
        let mut snapshot: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v1/character-snapshot.json"
        ))
        .unwrap();
        snapshot["payload"]["id"] = json!(format!("snapshot-{who}"));
        snapshot["payload"]["character"] = json!(who);
        snapshot["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW);
        fn stamp(value: &mut Value) {
            match value {
                Value::Object(m) => {
                    if m.get("status") == Some(&json!("known")) {
                        m.insert("clock_trust".into(), json!("trusted"));
                        m.insert("observed_at_utc_ms".into(), json!(NOW));
                    }
                    m.values_mut().for_each(stamp);
                }
                Value::Array(v) => v.iter_mut().for_each(stamp),
                _ => {}
            }
        }
        stamp(&mut snapshot["payload"]);
        profiles.push(
            app.import_text(
                &format!("ANAV1:{snapshot}"),
                Some(UtcMillis::new(NOW).unwrap()),
            )
            .unwrap()
            .profile_id,
        );
        snapshots.push(snapshot["payload"].to_string());
    }
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    let dataset = &mut content["envelope"]["payload"];
    dataset["objectives"][0]["participants"] = json!(["one", "two"]);
    dataset["objectives"][0]["targets"] = json!([{"kind":"service","node":{"id":"demo-node-1","map":dataset["nodes"][0]["position"]["map"]}}]);
    let requirement = if remote {
        json!({"op":"participant_state","participant":"two","predicate":{"op":"level_at_least","level":22}})
    } else {
        json!({"op":"level_at_least","level":22})
    };
    dataset["objectives"][0]["requirements"] = json!({"status":"known","value":requirement});
    app.stage_content(&content.to_string()).unwrap();
    app.activate_content(&profiles[0], &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    let party = PartyId::new("duo").unwrap();
    app.save_party(&party, &profiles).unwrap();
    let catalog = app
        .joint_planning_catalog(&party, Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    let request = json!({"party_id":"duo","participants":catalog.participants.iter().map(|p|json!({"profile_id":p.profile_id,"snapshot_id":p.snapshot_id})).collect::<Vec<_>>(),"revision":"demo-r1","goal_profile":"manual-same-unit-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":600000,"stopping_service_ms":0,"excluded":[],"candidates":[{"objective":"demo-objective-1","node":"demo-node-1","utilities":profiles.iter().map(|p|(p.clone(),json!(10))).collect::<serde_json::Map<_,_>>(),"completion_ms":60000,"service_ms":0}]});
    let mut p = app
        .propose_party_plan(
            &json!({"planning":request,"proposer_profile_id":profiles[0],"expected":null})
                .to_string(),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap();
    for profile in &profiles {
        p = app
            .accept_party_plan(
                &json!({"party_id":"duo","profile_id":profile,"expected":p.guard,"accepted":true})
                    .to_string(),
                Some(UtcMillis::new(NOW).unwrap()),
            )
            .unwrap();
    }
    let exports = profiles
        .iter()
        .map(|profile| {
            app.export_agreed_party_plan(
                &json!({"party_id":"duo","expected":p.guard,"recipient_profile_id":profile})
                    .to_string(),
                Some(UtcMillis::new(NOW).unwrap()),
            )
            .unwrap()
            .text
        })
        .collect::<Vec<_>>();
    (exports[0].clone(), exports[1].clone(), snapshots)
}
#[test]
fn produced_joint_choices_advance_only_the_explicit_recipient_and_bad_replacements_preserve_plan() {
    let (one, two, snapshots) = exported(false);
    let sandbox = common::strict_core();
    sandbox.ns().set("one", one.clone()).unwrap();
    sandbox.ns().set("two", two).unwrap();
    sandbox.ns().set("s1", snapshots[0].clone()).unwrap();
    sandbox.ns().set("s2", snapshots[1].clone()).unwrap();
    sandbox
        .ns()
        .set("oversize", format!("{one}{}", " ".repeat(32769)))
        .unwrap();
    sandbox.run::<()>("recipient_progress",r#"
      local s1=assert(ns.Decode.decode(ns.s1));local s2=assert(ns.Decode.decode(ns.s2))
      local plans=ns.Plan.new({context=s1.context,fixture=true});assert(plans:importTransport(ns.one))
      local plan=plans:get().payload;assert(#plan.inputs==2 and #plan.steps[1].participants==1 and plan.steps[1].participants[1]=='one')
      local context={snapshot=s1,fixture=true,freshness={clock_trust='trusted',now_utc_ms=1791792000000,max_age_ms=1800000}}
      local hud=ns.Hud.new(plans,function() return context end,function() end)
      assert(hud:refresh().status=='ready')
      context.snapshot=s2;assert(hud:refresh().status=='no_steps')
      s2.objectives[plan.steps[1].objective]={status='known',value=true,clock_trust='trusted',observed_at_utc_ms=1791792000000}
      assert(hud:refresh().status=='no_steps')
      context.snapshot=s1;assert(hud:refresh().status=='ready')
      s1.objectives[plan.steps[1].objective]={status='known',value=true,clock_trust='trusted',observed_at_utc_ms=1791792000000}
      assert(hud:refresh().status=='complete')
      local original=plan.id;assert(not plans:importTransport(ns.oversize));assert(plans:get().payload.id==original)
      assert(plans:importTransport(ns.two));context.snapshot=s1;assert(hud:refresh().status=='no_steps')
      context.snapshot=s2;assert(hud:refresh().status=='complete')
      context.freshness.now_utc_ms=plans:get().payload.expires_at_utc_ms;assert(hud:refresh().status=='expired')
    "#).unwrap();
}
#[test]
fn explicit_remote_predicates_remain_unknown_without_party_facts() {
    let (one, _, snapshots) = exported(true);
    let sandbox = common::strict_core();
    sandbox.ns().set("one", one).unwrap();
    sandbox.ns().set("s1", snapshots[0].clone()).unwrap();
    sandbox.ns().set("s2", snapshots[1].clone()).unwrap();
    sandbox.run::<()>("remote_readiness",r#"
      local s1=assert(ns.Decode.decode(ns.s1));local s2=assert(ns.Decode.decode(ns.s2))
      local plans=ns.Plan.new({context=s1.context,fixture=true});assert(plans:importTransport(ns.one))
      local context={snapshot=s1,fixture=true,freshness={clock_trust='trusted',now_utc_ms=1791792000000,max_age_ms=1800000}}
      local hud=ns.Hud.new(plans,function() return context end,function() end)
      assert(hud:refresh().status=='uncertain')
      context.party={id='duo',context=s1.context,participants={s1,s2}};assert(hud:refresh().status=='ready')
      s2.level={status='restricted'};assert(hud:refresh().status=='uncertain')
    "#).unwrap();
}
