//! Original source authoring with disclosed manual engineering character inputs.
use azeroth_app::{reminders::RuleDelivery, snapshots::SnapshotApplication};
use azeroth_core::{
    reminders::ReminderKind,
    types::{DatasetRevision, UtcMillis},
};
use serde_json::{Value, json};
fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"source":"manual_entry","clock_trust":"trusted","observed_at_utc_ms":1000})
}
fn setup(level: Value, role: Value) -> (SnapshotApplication, String) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshot: Value =
        serde_json::from_str(include_str!("support/stable_engineering_snapshot.json")).unwrap();
    snapshot["payload"]["level"] = level;
    snapshot["payload"]["role"] = role;
    let profile = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(1100).unwrap()),
        )
        .unwrap()
        .profile_id;
    let content: Value = serde_json::from_str(include_str!(
        "../../../content/authored/slice-v1/catalog.json"
    ))
    .unwrap();
    let revision = content["reference"]["payload"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let sources: Value = serde_json::from_str(include_str!(
        "../../../content/authored/slice-v1/sources.json"
    ))
    .unwrap();
    app.stage_content(
        &json!({"content":content,"sources":sources["sources"],"conflicts":[]}).to_string(),
    )
    .unwrap();
    app.activate_content(&profile, &DatasetRevision::new(revision).unwrap())
        .unwrap();
    (app, profile)
}
#[test]
fn authored_advice_is_explicitly_heuristic_and_global_optout_still_applies() {
    let (app, profile) = setup(known(json!(61)), json!({"status":"unknown"}));
    let view = app
        .reminder_view(&profile, Some(UtcMillis::new(1100).unwrap()))
        .unwrap();
    assert_eq!(view.rules.len(), 18);
    assert!(
        view.rules
            .iter()
            .all(|r| r.delivery == RuleDelivery::AuthoredHeuristic)
    );
    let plan = view.plan.unwrap();
    let item = plan
        .queue
        .iter()
        .find(|i| i.rule.as_str() == "reminder:heuristic-level-boundary")
        .unwrap();
    let ReminderKind::Advice { text } = &item.kind else {
        panic!("known engineering level yields advice")
    };
    assert!(text.as_str().starts_with("Authored heuristic: "));
    assert!(text.as_str().contains("not a verified game level cap"));
    let disabled=app.mutate_reminders(&json!({"profile_id":profile,"snapshot_id":view.snapshot_id,"revision":view.revision,"action":{"kind":"preferences","enabled":false}}).to_string(),Some(UtcMillis::new(1100).unwrap())).unwrap();
    assert!(!disabled.enabled && disabled.plan.is_none());
}
#[test]
fn heuristic_status_never_resolves_unknown_restricted_stale_false_or_wrong_band_conditions() {
    for level in [
        json!({"status":"unknown"}),
        json!({"status":"restricted"}),
        json!({"status":"unsupported"}),
        known(json!(60)),
    ] {
        let (app, profile) = setup(level, json!({"status":"unknown"}));
        let view = app
            .reminder_view(&profile, Some(UtcMillis::new(1100).unwrap()))
            .unwrap();
        assert!(
            !view
                .plan
                .unwrap()
                .queue
                .iter()
                .any(|i| i.rule.as_str() == "reminder:heuristic-level-boundary"
                    && matches!(i.kind, ReminderKind::Advice { .. }))
        );
    }
    for now in [1100, 9_000_000] {
        let (app, profile) = setup(known(json!(20)), known(json!("tank")));
        let view = app
            .reminder_view(&profile, Some(UtcMillis::new(now).unwrap()))
            .unwrap();
        let roles: Vec<_> = view
            .plan
            .unwrap()
            .queue
            .into_iter()
            .filter(|i| {
                i.rule.as_str().starts_with("reminder:role-")
                    && matches!(i.kind, ReminderKind::Advice { .. })
            })
            .collect();
        if now == 1100 {
            assert_eq!(roles.len(), 1);
            assert_eq!(roles[0].rule.as_str(), "reminder:role-tank-1-20");
        } else {
            assert!(roles.is_empty());
        }
    }
    let (app, profile) = setup(known(json!(20)), json!({"status":"unknown"}));
    let view = app
        .reminder_view(&profile, Some(UtcMillis::new(1100).unwrap()))
        .unwrap();
    assert!(
        !view
            .plan
            .unwrap()
            .queue
            .iter()
            .any(|i| i.rule.as_str().starts_with("reminder:role-")
                && matches!(i.kind, ReminderKind::Advice { .. }))
    );
}
