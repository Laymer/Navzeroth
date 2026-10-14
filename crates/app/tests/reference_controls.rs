use azeroth_app::{reference::ReferenceWindowState, reference_controls::*};
use azeroth_core::error::{AppError, ErrorCode, Result};
struct Ops {
    exists: bool,
    visible: bool,
    topmost: bool,
    main: bool,
    reference: bool,
    fail: &'static str,
    calls: Vec<&'static str>,
}
impl Default for Ops {
    fn default() -> Self {
        Self {
            exists: false,
            visible: false,
            topmost: false,
            main: true,
            reference: false,
            fail: "",
            calls: vec![],
        }
    }
}
impl Ops {
    fn call(&mut self, name: &'static str) -> Result<()> {
        self.calls.push(name);
        if self.fail == name {
            Err(AppError::new(
                ErrorCode::ProviderUnavailable,
                "private-path-must-not-leak",
                false,
            ))
        } else {
            Ok(())
        }
    }
}
impl WindowOperations for Ops {
    fn main_focused(&mut self) -> Result<bool> {
        self.call("main_focus")?;
        Ok(self.main)
    }
    fn state(&mut self) -> Result<ReferenceWindowState> {
        self.call("state")?;
        Ok(ReferenceWindowState {
            schema_version: 1,
            exists: self.exists,
            visible: self.visible,
            always_on_top: self.topmost,
        })
    }
    fn reference_focused(&mut self) -> Result<bool> {
        self.call("reference_focus")?;
        Ok(self.reference)
    }
    fn create_hidden(&mut self) -> Result<()> {
        self.call("create_hidden")?;
        self.exists = true;
        Ok(())
    }
    fn focusable(&mut self, v: bool) -> Result<()> {
        self.call(if v { "focusable_yes" } else { "focusable_no" })
    }
    fn ignore_pointer(&mut self, v: bool) -> Result<()> {
        self.call(if v {
            "ignore_pointer"
        } else {
            "receive_pointer"
        })
    }
    fn topmost(&mut self, v: bool) -> Result<()> {
        self.call(if v { "topmost_yes" } else { "topmost_no" })?;
        self.topmost = v;
        Ok(())
    }
    fn visible(&mut self, v: bool) -> Result<()> {
        self.call(if v { "show" } else { "hide" })?;
        self.visible = v;
        Ok(())
    }
}
#[test]
fn opt_in_creation_applies_mode_before_show_and_hiding_resets_after_visibility() {
    let mut ops = Ops::default();
    let shown = toggle(&mut ops, InteractionMode::PassThrough, true);
    assert_eq!(shown.dispatch, ControlDispatch::Accepted);
    assert_eq!(
        ops.calls,
        vec![
            "main_focus",
            "state",
            "create_hidden",
            "reference_focus",
            "focusable_no",
            "ignore_pointer",
            "topmost_yes",
            "show",
            "state"
        ]
    );
    assert!(shown.window.unwrap().visible);
    ops.calls.clear();
    let hidden = toggle(&mut ops, InteractionMode::PassThrough, true);
    assert_eq!(hidden.requested_interaction, InteractionMode::Interactive);
    assert_eq!(hidden.requested_visible, Some(false));
    assert_eq!(
        ops.calls,
        vec![
            "main_focus",
            "state",
            "hide",
            "topmost_no",
            "focusable_yes",
            "receive_pointer",
            "state"
        ]
    );
    let observed = hidden.window.unwrap();
    assert!(!observed.visible);
    assert!(!observed.always_on_top);
}
#[test]
fn mode_only_never_creates_or_shows_and_unsafe_focus_is_refused() {
    let mut ops = Ops::default();
    let absent = set_interaction(&mut ops, InteractionMode::PassThrough);
    assert_eq!(absent.dispatch, ControlDispatch::Refused);
    assert_eq!(absent.reason, Some(ControlReason::WindowAbsent));
    assert!(!ops.exists);
    assert!(!ops.calls.contains(&"create_hidden"));
    ops.exists = true;
    ops.reference = true;
    ops.calls.clear();
    let refused = set_interaction(&mut ops, InteractionMode::PassThrough);
    assert_eq!(refused.reason, Some(ControlReason::ReferenceFocused));
    assert!(!ops.calls.contains(&"ignore_pointer"));
    ops.main = false;
    ops.calls.clear();
    let refused = toggle(&mut ops, InteractionMode::Interactive, false);
    assert_eq!(refused.reason, Some(ControlReason::MainNotFocused));
    assert_eq!(ops.calls, vec!["main_focus", "state"]);
}
#[test]
fn partial_setter_failures_never_show_or_invent_interaction_observations() {
    for failure in [
        "create_hidden",
        "reference_focus",
        "focusable_no",
        "ignore_pointer",
        "topmost_yes",
        "show",
    ] {
        let mut ops = Ops {
            fail: failure,
            ..Ops::default()
        };
        let r = toggle(&mut ops, InteractionMode::PassThrough, true);
        assert_ne!(r.dispatch, ControlDispatch::Accepted, "{failure}");
        assert!(!r.window.as_ref().unwrap().visible, "{failure}");
        assert!(!serde_json::to_string(&r).unwrap().contains("private-path"));
        if failure != "show" {
            assert!(!ops.calls.contains(&"show"));
        }
        if failure == "ignore_pointer" {
            assert_eq!(r.operations.focusability, OperationDispatch::Accepted);
            assert_eq!(r.operations.pointer_events, OperationDispatch::Failed);
        }
    }
}
#[test]
fn hide_remains_attempted_when_reset_fails_and_observed_state_is_queried() {
    let mut ops = Ops {
        exists: true,
        visible: true,
        topmost: true,
        fail: "focusable_yes",
        ..Ops::default()
    };
    let r = toggle(&mut ops, InteractionMode::PassThrough, true);
    assert_eq!(r.dispatch, ControlDispatch::PartialFailure);
    assert_eq!(r.operations.visibility, OperationDispatch::Accepted);
    assert!(!r.window.unwrap().visible);
    assert!(ops.calls.contains(&"hide"));
}
#[test]
fn unknown_focus_or_state_keeps_refusal_truth_and_no_window_mutation() {
    let mut ops = Ops {
        fail: "main_focus",
        ..Ops::default()
    };
    let r = toggle(&mut ops, InteractionMode::Interactive, false);
    assert_eq!(r.dispatch, ControlDispatch::Refused);
    assert_eq!(r.reason, Some(ControlReason::FocusQueryFailed));
    assert_eq!(r.operations.creation, OperationDispatch::NotAttempted);
    ops.main = false;
    ops.fail = "state";
    let r = set_interaction(&mut ops, InteractionMode::PassThrough);
    assert_eq!(r.dispatch, ControlDispatch::Refused);
    assert!(r.window.is_none());
    assert_eq!(r.operations.state_query, OperationDispatch::Failed);
}
#[test]
fn actual_reference_caller_gate_still_denies_all_new_commands() {
    for name in [
        "toggle_reference_window",
        "set_reference_interaction",
        "unknown_command",
    ] {
        assert!(azeroth_app::reference::permits_command("main", name));
        assert!(!azeroth_app::reference::permits_command("reference", name));
        assert!(!azeroth_app::reference::permits_command("other", name));
    }
    assert!(azeroth_app::reference::permits_command(
        "reference",
        "get_reference_view"
    ));
}
