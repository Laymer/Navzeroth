//! An explicitly requested ordinary application window; never a game hook.
use azeroth_core::error::{AppError, ErrorCode};
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
pub const LABEL: &str = azeroth_app::reference::REFERENCE_LABEL;
pub const PATH: &str = azeroth_app::reference::REFERENCE_PATH;
#[derive(Default)]
pub struct Controller(pub Mutex<()>);
fn unavailable() -> AppError {
    AppError::new(
        ErrorCode::ProviderUnavailable,
        "Ordinary reference window operation is unavailable",
        false,
    )
}
/// Actual invoke metadata is supplied by Tauri, not by frontend arguments.
pub fn permits(caller: &str, command: &str) -> bool {
    azeroth_app::reference::permits_command(caller, command)
}
pub fn main_caller(caller: &str) -> Result<(), AppError> {
    if azeroth_app::reference::permits_management(caller) {
        Ok(())
    } else {
        Err(AppError::new(
            ErrorCode::ProviderUnavailable,
            "Reference window management requires the main desktop",
            false,
        ))
    }
}
pub fn state(
    app: &tauri::AppHandle,
) -> Result<azeroth_app::reference::ReferenceWindowState, AppError> {
    match app.get_webview_window(LABEL) {
        Some(window) => Ok(azeroth_app::reference::ReferenceWindowState {
            schema_version: 1,
            exists: true,
            visible: window.is_visible().map_err(|_| unavailable())?,
            always_on_top: window.is_always_on_top().map_err(|_| unavailable())?,
        }),
        None => Ok(azeroth_app::reference::ReferenceWindowState {
            schema_version: 1,
            exists: false,
            visible: false,
            always_on_top: false,
        }),
    }
}
fn create_hidden(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, AppError> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(PATH.into()))
        .title("Azeroth Navigator — imported reference")
        .inner_size(460.0, 620.0)
        .min_inner_size(320.0, 300.0)
        .decorations(true)
        .resizable(true)
        .focused(false)
        .focusable(true)
        .visible(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .build()
        .map_err(|_| unavailable())
}
pub struct Operations {
    app: tauri::AppHandle,
    main: tauri::WebviewWindow,
    reference: Option<tauri::WebviewWindow>,
}
impl Operations {
    pub fn new(app: tauri::AppHandle, main: tauri::WebviewWindow) -> Self {
        let reference = app.get_webview_window(LABEL);
        Self {
            app,
            main,
            reference,
        }
    }
    fn window(&self) -> Result<&tauri::WebviewWindow, AppError> {
        self.reference.as_ref().ok_or_else(unavailable)
    }
}
impl azeroth_app::reference_controls::WindowOperations for Operations {
    fn main_focused(&mut self) -> Result<bool, AppError> {
        self.main.is_focused().map_err(|_| unavailable())
    }
    fn state(&mut self) -> Result<azeroth_app::reference::ReferenceWindowState, AppError> {
        state(&self.app)
    }
    fn reference_focused(&mut self) -> Result<bool, AppError> {
        self.window()?.is_focused().map_err(|_| unavailable())
    }
    fn create_hidden(&mut self) -> Result<(), AppError> {
        self.reference = Some(create_hidden(&self.app)?);
        Ok(())
    }
    fn focusable(&mut self, value: bool) -> Result<(), AppError> {
        self.window()?
            .set_focusable(value)
            .map_err(|_| unavailable())
    }
    fn ignore_pointer(&mut self, value: bool) -> Result<(), AppError> {
        self.window()?
            .set_ignore_cursor_events(value)
            .map_err(|_| unavailable())
    }
    fn topmost(&mut self, value: bool) -> Result<(), AppError> {
        self.window()?
            .set_always_on_top(value)
            .map_err(|_| unavailable())
    }
    fn visible(&mut self, value: bool) -> Result<(), AppError> {
        if value {
            self.window()?.show()
        } else {
            self.window()?.hide()
        }
        .map_err(|_| unavailable())
    }
}
pub fn set(
    app: &tauri::AppHandle,
    visible: bool,
    always_on_top: bool,
) -> Result<azeroth_app::reference::ReferenceWindowState, AppError> {
    if visible {
        let window = match app.get_webview_window(LABEL) {
            Some(w) => w,
            None => create_hidden(app)?,
        };
        window.set_focusable(true).map_err(|_| unavailable())?;
        window
            .set_ignore_cursor_events(false)
            .map_err(|_| unavailable())?;
        window
            .set_always_on_top(always_on_top)
            .map_err(|_| unavailable())?;
        window.show().map_err(|_| unavailable())?;
    } else if let Some(window) = app.get_webview_window(LABEL) {
        window.hide().map_err(|_| unavailable())?;
        window.set_always_on_top(false).map_err(|_| unavailable())?;
        window
            .set_ignore_cursor_events(false)
            .map_err(|_| unavailable())?;
        window.set_focusable(true).map_err(|_| unavailable())?;
    }
    state(app)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_caller_policy_has_one_read_only_reference_command() {
        for command in [
            "get_reference_view",
            "set_reference_window",
            "import_snapshot_file",
            "unknown",
        ] {
            assert!(permits("main", command));
            assert_eq!(permits(LABEL, command), command == "get_reference_view");
            assert!(!permits("unregistered", command));
        }
        assert!(main_caller("main").is_ok());
        assert!(main_caller(LABEL).is_err());
    }
    #[test]
    fn fixed_bundled_path_retains_reference_fragment() {
        for base in [
            "http://localhost:1420/",
            "http://tauri.localhost/",
            "tauri://localhost/",
        ] {
            let url = tauri::Url::parse(base).unwrap().join(PATH).unwrap();
            assert_eq!(url.fragment(), Some("reference"));
            assert_eq!(url.path(), "/index.html");
        }
    }
}
