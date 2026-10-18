//! The packaged executable already gets Tauri's manifest. Library/bin test
//! harnesses need the same Common Controls v6 dependency before the Windows
//! loader resolves dialog imports; no window/dialog is displayed by this setup.
#[cfg(all(windows, target_env = "msvc"))]
#[used]
// SAFETY: fixed ASCII linker metadata in the documented COFF directive section;
// no executable data or symbols are placed in a runtime code/data section.
#[unsafe(link_section = ".drectve")]
static COMMON_CONTROLS_V6: [u8; 168] = *br#" /MANIFESTDEPENDENCY:"type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'" "#;
