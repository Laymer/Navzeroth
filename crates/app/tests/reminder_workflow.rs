use azeroth_app::{
    reminders::{ReminderAction, ReminderRequest},
    snapshots::SnapshotApplication,
};
use azeroth_core::types::SnapshotId;
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
#[test]
fn guarded_preferences_and_invalid_acknowledgment_are_atomic() {
    let app = SnapshotApplication::in_memory().unwrap();
    let id = app
        .import_text(&format!("ANAV1:{SNAPSHOT}"), None)
        .unwrap()
        .profile_id;
    let view = app.reminder_view(&id, None).unwrap();
    assert!(view.enabled);
    assert!(view.rules.is_empty());
    let mut request = ReminderRequest {
        profile_id: id.clone(),
        snapshot_id: view.snapshot_id,
        revision: view.revision,
        action: ReminderAction::Preferences { enabled: false },
    };
    app.mutate_reminders(&serde_json::to_string(&request).unwrap(), None)
        .unwrap();
    assert!(!app.reminder_view(&id, None).unwrap().enabled);
    request.snapshot_id = Some(SnapshotId::new("obsolete").unwrap());
    request.action = ReminderAction::Preferences { enabled: true };
    assert!(
        app.mutate_reminders(&serde_json::to_string(&request).unwrap(), None)
            .is_err()
    );
    assert!(!app.reminder_view(&id, None).unwrap().enabled);
    request.snapshot_id = app.reminder_view(&id, None).unwrap().snapshot_id;
    request.action = ReminderAction::Acknowledge { rules: vec![] };
    assert!(
        app.mutate_reminders(&serde_json::to_string(&request).unwrap(), None)
            .is_err()
    );
    assert!(app.mutate_reminders(&" ".repeat(32769), None).is_err());
}
#[test]
fn explicit_presentation_dismiss_restore_and_refresh_guards() {
    use azeroth_core::types::{DatasetRevision, RuleId, UtcMillis};
    let app = SnapshotApplication::in_memory().unwrap();
    let id = app
        .import_text(&format!("ANAV1:{SNAPSHOT}"), None)
        .unwrap()
        .profile_id;
    let bundle = include_str!("../../../fixtures/desktop/content/demo-revision.json");
    app.stage_content(bundle).unwrap();
    app.activate_content(&id, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    let now = Some(UtcMillis::new(1791791990000).unwrap());
    let view = app.reminder_view(&id, now).unwrap();
    assert_eq!(
        view.plan.as_ref().unwrap().mode,
        azeroth_core::reminders::PresentationMode::NonIntrusive
    );
    assert_eq!(view.plan.as_ref().unwrap().queue.len(), 1);
    let mut req = ReminderRequest {
        profile_id: id.clone(),
        snapshot_id: view.snapshot_id,
        revision: view.revision,
        action: ReminderAction::Acknowledge {
            rules: vec![RuleId::new("invented").unwrap()],
        },
    };
    assert!(
        app.mutate_reminders(&serde_json::to_string(&req).unwrap(), now)
            .is_err()
    );
    assert_eq!(
        app.reminder_view(&id, now)
            .unwrap()
            .plan
            .unwrap()
            .queue
            .len(),
        1
    );
    req.action = ReminderAction::Dismiss {
        rule: RuleId::new("demo-rule-1").unwrap(),
    };
    let hidden = app
        .mutate_reminders(&serde_json::to_string(&req).unwrap(), now)
        .unwrap();
    assert_eq!(hidden.dismissed.len(), 1);
    assert!(hidden.plan.unwrap().queue.is_empty());
    req.action = ReminderAction::Restore {
        rule: RuleId::new("demo-rule-1").unwrap(),
    };
    assert_eq!(
        app.mutate_reminders(&serde_json::to_string(&req).unwrap(), now)
            .unwrap()
            .plan
            .unwrap()
            .queue
            .len(),
        1
    );
    req.action = ReminderAction::Acknowledge {
        rules: vec![RuleId::new("demo-rule-1").unwrap()],
    };
    assert!(
        app.mutate_reminders(&serde_json::to_string(&req).unwrap(), now)
            .unwrap()
            .plan
            .unwrap()
            .queue
            .is_empty()
    );
    assert!(
        app.reminder_view(&id, now)
            .unwrap()
            .plan
            .unwrap()
            .queue
            .is_empty()
    );
    let mut refreshed: serde_json::Value = serde_json::from_str(SNAPSHOT).unwrap();
    refreshed["payload"]["id"] = serde_json::json!("refreshed");
    refreshed["payload"]["capture"]["sequence"] = serde_json::json!(13);
    refreshed["producer"]["sequence"] = serde_json::json!(13);
    app.import_text(&format!("ANAV1:{refreshed}"), now).unwrap();
    assert!(
        app.reminder_view(&id, now)
            .unwrap()
            .plan
            .unwrap()
            .queue
            .is_empty()
    );
    assert!(
        app.mutate_reminders(&serde_json::to_string(&req).unwrap(), now)
            .is_err()
    );
    req.revision = Some(DatasetRevision::new("obsolete").unwrap());
    req.action = ReminderAction::Preferences { enabled: false };
    assert!(
        app.mutate_reminders(&serde_json::to_string(&req).unwrap(), now)
            .is_err()
    );
}
