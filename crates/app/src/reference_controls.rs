//! Explicit ordinary-window requests. Setter success records dispatch acceptance,
//! never observed mouse/focus behavior. No game/global input listener belongs here.
use crate::reference::ReferenceWindowState;
use azeroth_core::error::Result;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    Interactive,
    PassThrough,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationDispatch {
    #[default]
    NotAttempted,
    Accepted,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlDispatch {
    Accepted,
    Refused,
    PartialFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlReason {
    MainNotFocused,
    FocusQueryFailed,
    ReferenceFocused,
    WindowAbsent,
    CreateFailed,
    FocusableFailed,
    PointerFailed,
    TopmostFailed,
    VisibilityFailed,
    StateQueryFailed,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct OperationReceipts {
    pub creation: OperationDispatch,
    pub focusability: OperationDispatch,
    pub pointer_events: OperationDispatch,
    pub topmost: OperationDispatch,
    pub visibility: OperationDispatch,
    pub state_query: OperationDispatch,
}
#[derive(Clone, Debug, Serialize)]
pub struct ControlReceipt {
    pub schema_version: u32,
    pub requested_visible: Option<bool>,
    pub requested_interaction: InteractionMode,
    pub dispatch: ControlDispatch,
    pub reason: Option<ControlReason>,
    pub operations: OperationReceipts,
    pub window: Option<ReferenceWindowState>,
}
/// The native adapter supplies actual fixed-window state and caller focus.
pub trait WindowOperations {
    fn main_focused(&mut self) -> Result<bool>;
    fn state(&mut self) -> Result<ReferenceWindowState>;
    fn reference_focused(&mut self) -> Result<bool>;
    fn create_hidden(&mut self) -> Result<()>;
    fn focusable(&mut self, value: bool) -> Result<()>;
    fn ignore_pointer(&mut self, value: bool) -> Result<()>;
    fn topmost(&mut self, value: bool) -> Result<()>;
    fn visible(&mut self, value: bool) -> Result<()>;
}
fn receipt(mode: InteractionMode) -> ControlReceipt {
    ControlReceipt {
        schema_version: 1,
        requested_visible: None,
        requested_interaction: mode,
        dispatch: ControlDispatch::Accepted,
        reason: None,
        operations: OperationReceipts::default(),
        window: None,
    }
}
fn fail(r: &mut ControlReceipt, reason: ControlReason) {
    if r.dispatch != ControlDispatch::Refused {
        r.dispatch = ControlDispatch::PartialFailure;
    }
    r.reason.get_or_insert(reason);
}
fn inspect(ops: &mut impl WindowOperations, r: &mut ControlReceipt) {
    match ops.state() {
        Ok(window) => {
            r.operations.state_query = OperationDispatch::Accepted;
            r.window = Some(window);
        }
        Err(_) => {
            r.operations.state_query = OperationDispatch::Failed;
            r.window = None;
            fail(r, ControlReason::StateQueryFailed);
        }
    }
}
fn focused(ops: &mut impl WindowOperations, r: &mut ControlReceipt) -> bool {
    match ops.main_focused() {
        Ok(true) => true,
        Ok(false) => {
            r.dispatch = ControlDispatch::Refused;
            r.reason = Some(ControlReason::MainNotFocused);
            false
        }
        Err(_) => {
            r.dispatch = ControlDispatch::Refused;
            r.reason = Some(ControlReason::FocusQueryFailed);
            false
        }
    }
}
fn apply_mode(ops: &mut impl WindowOperations, r: &mut ControlReceipt) -> bool {
    if r.requested_interaction == InteractionMode::PassThrough {
        match ops.reference_focused() {
            Ok(false) => {}
            Ok(true) => {
                r.dispatch = ControlDispatch::Refused;
                r.reason = Some(ControlReason::ReferenceFocused);
                return false;
            }
            Err(_) => {
                r.dispatch = ControlDispatch::Refused;
                r.reason = Some(ControlReason::FocusQueryFailed);
                return false;
            }
        }
    }
    if ops
        .focusable(r.requested_interaction == InteractionMode::Interactive)
        .is_err()
    {
        r.operations.focusability = OperationDispatch::Failed;
        fail(r, ControlReason::FocusableFailed);
        return false;
    }
    r.operations.focusability = OperationDispatch::Accepted;
    if ops
        .ignore_pointer(r.requested_interaction == InteractionMode::PassThrough)
        .is_err()
    {
        r.operations.pointer_events = OperationDispatch::Failed;
        fail(r, ControlReason::PointerFailed);
        return false;
    }
    r.operations.pointer_events = OperationDispatch::Accepted;
    true
}
/// Only the explicit focused-main shortcut/control calls this. Actual visibility
/// is sampled inside the caller's native controller lock.
pub fn toggle(
    ops: &mut impl WindowOperations,
    mode: InteractionMode,
    topmost: bool,
) -> ControlReceipt {
    let mut r = receipt(mode);
    if !focused(ops, &mut r) {
        inspect(ops, &mut r);
        return r;
    }
    let current = match ops.state() {
        Ok(s) => s,
        Err(_) => {
            fail(&mut r, ControlReason::StateQueryFailed);
            inspect(ops, &mut r);
            return r;
        }
    };
    let show = !current.visible;
    r.requested_visible = Some(show);
    if !show {
        // Hide is attempted first and remains reachable even if interaction reset fails.
        if ops.visible(false).is_ok() {
            r.operations.visibility = OperationDispatch::Accepted;
        } else {
            r.operations.visibility = OperationDispatch::Failed;
            fail(&mut r, ControlReason::VisibilityFailed);
        }
        if ops.topmost(false).is_ok() {
            r.operations.topmost = OperationDispatch::Accepted;
        } else {
            r.operations.topmost = OperationDispatch::Failed;
            fail(&mut r, ControlReason::TopmostFailed);
        }
        r.requested_interaction = InteractionMode::Interactive;
        apply_mode(ops, &mut r);
        inspect(ops, &mut r);
        return r;
    }
    if !current.exists {
        if ops.create_hidden().is_err() {
            r.operations.creation = OperationDispatch::Failed;
            fail(&mut r, ControlReason::CreateFailed);
            inspect(ops, &mut r);
            return r;
        }
        r.operations.creation = OperationDispatch::Accepted;
    }
    if !apply_mode(ops, &mut r) {
        inspect(ops, &mut r);
        return r;
    }
    if ops.topmost(topmost).is_err() {
        r.operations.topmost = OperationDispatch::Failed;
        fail(&mut r, ControlReason::TopmostFailed);
        inspect(ops, &mut r);
        return r;
    }
    r.operations.topmost = OperationDispatch::Accepted;
    if ops.visible(true).is_ok() {
        r.operations.visibility = OperationDispatch::Accepted;
    } else {
        r.operations.visibility = OperationDispatch::Failed;
        fail(&mut r, ControlReason::VisibilityFailed);
    }
    inspect(ops, &mut r);
    r
}
/// Does not create, show or focus a window. No observed-mode getter is invented.
pub fn set_interaction(ops: &mut impl WindowOperations, mode: InteractionMode) -> ControlReceipt {
    let mut r = receipt(mode);
    if !focused(ops, &mut r) {
        inspect(ops, &mut r);
        return r;
    }
    match ops.state() {
        Ok(s) if s.exists => {
            apply_mode(ops, &mut r);
        }
        Ok(_) => {
            r.dispatch = ControlDispatch::Refused;
            r.reason = Some(ControlReason::WindowAbsent);
        }
        Err(_) => fail(&mut r, ControlReason::StateQueryFailed),
    }
    inspect(ops, &mut r);
    r
}
