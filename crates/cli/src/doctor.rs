//! Read-only host/path diagnostics. No imported file is opened or executed.
use serde::Serialize;
use std::{ffi::OsString, path::Path};

const TOOLCHAIN: &str = include_str!("../../../rust-toolchain.toml");
const NODE: &str = include_str!("../../../.node-version");
const PACKAGE: &str = include_str!("../../../package.json");

#[derive(Default)]
pub struct Options {
    pub json: bool,
    pub saved_variables: Option<OsString>,
    pub addons: Option<OsString>,
    pub content: Option<OsString>,
}
impl Options {
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let mut options = Self::default();
        let mut i = 0;
        while i < args.len() {
            let slot = match args[i].to_str()? {
                "--json" if !options.json => {
                    options.json = true;
                    i += 1;
                    continue;
                }
                "--saved-variables" => &mut options.saved_variables,
                "--addons" => &mut options.addons,
                "--content" => &mut options.content,
                _ => return None,
            };
            let value = args.get(i + 1)?;
            if slot.is_some()
                || value.is_empty()
                || value.to_str().is_some_and(|v| v.starts_with("--"))
            {
                return None;
            }
            *slot = Some(value.clone());
            i += 2;
        }
        Some(options)
    }
}
#[derive(Serialize)]
pub struct ToolRequirement {
    pub tool: &'static str,
    pub development_pin: String,
    pub available_on_path: bool,
    /// Presence never establishes compatibility with a version pin.
    pub runtime_version_verified: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathState {
    NotSelected,
    RegularFile,
    Directory,
    Missing,
    WrongKind,
    SymlinkRejected,
    Unreadable,
}
#[derive(Serialize)]
pub struct SelectedPath {
    pub integration: &'static str,
    pub state: PathState,
    pub content_verified: bool,
}
#[derive(Serialize)]
pub struct Report {
    pub report_version: u32,
    pub app_version: &'static str,
    pub os: &'static str,
    pub architecture: &'static str,
    pub development_tools: Vec<ToolRequirement>,
    pub selected_paths: Vec<SelectedPath>,
    pub native_prerequisites_verified: bool,
    pub live_client_verified: bool,
}
fn tool_available(name: &str) -> bool {
    let Some(search) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&search).any(|dir| {
        #[cfg(windows)]
        let candidates = [
            dir.join(format!("{name}.exe")),
            dir.join(format!("{name}.cmd")),
            dir.join(format!("{name}.bat")),
        ];
        #[cfg(not(windows))]
        let candidates = [dir.join(name)];
        candidates.iter().any(|p| {
            let Ok(meta) = std::fs::metadata(p) else {
                return false;
            };
            if !meta.is_file() {
                return false;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                true
            }
        })
    })
}
fn selected(integration: &'static str, path: Option<&OsString>, file: bool) -> SelectedPath {
    let state = match path {
        None => PathState::NotSelected,
        Some(path) => match std::fs::symlink_metadata(Path::new(path)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => PathState::Missing,
            Err(_) => PathState::Unreadable,
            Ok(meta) if meta.file_type().is_symlink() => PathState::SymlinkRejected,
            Ok(meta) if file && meta.is_file() => PathState::RegularFile,
            Ok(meta) if !file && meta.is_dir() => PathState::Directory,
            Ok(_) => PathState::WrongKind,
        },
    };
    SelectedPath {
        integration,
        state,
        content_verified: false,
    }
}
pub fn report(options: &Options) -> Report {
    let rust = TOOLCHAIN
        .lines()
        .find_map(|line| {
            line.strip_prefix("channel = ")
                .map(|v| v.trim_matches('"').to_owned())
        })
        .expect("committed Rust toolchain has a channel");
    let package: serde_json::Value = serde_json::from_str(PACKAGE).expect("committed package JSON");
    let pnpm = package["packageManager"]
        .as_str()
        .expect("committed pnpm pin")
        .strip_prefix("pnpm@")
        .expect("pnpm manager")
        .split('+')
        .next()
        .expect("version");
    let pins = [
        ("rustc", rust.clone()),
        ("cargo", rust),
        ("node", NODE.trim().to_owned()),
        ("pnpm", pnpm.to_owned()),
    ];
    Report {
        report_version: 1,
        app_version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        development_tools: pins
            .into_iter()
            .map(|(tool, development_pin)| ToolRequirement {
                tool,
                development_pin,
                available_on_path: tool_available(tool),
                runtime_version_verified: false,
            })
            .collect(),
        selected_paths: vec![
            selected("saved_variables", options.saved_variables.as_ref(), true),
            selected("addons", options.addons.as_ref(), false),
            selected("content", options.content.as_ref(), false),
        ],
        native_prerequisites_verified: false,
        live_client_verified: false,
    }
}
impl Report {
    /// Unselected paths and missing development tools are information; explicit
    /// invalid integrations fail. The packaged desktop needs no development tools.
    pub fn passes(&self) -> bool {
        self.selected_paths.iter().all(|p| {
            matches!(
                p.state,
                PathState::NotSelected | PathState::RegularFile | PathState::Directory
            )
        })
    }
    pub fn human(&self) -> String {
        let mut text = format!(
            "Azeroth Navigator {}: {} / {}\nDevelopment requirements (presence only; runtime versions unverified):\n",
            self.app_version, self.os, self.architecture
        );
        for tool in &self.development_tools {
            text.push_str(&format!(
                "  {}: pin {}; {}\n",
                tool.tool,
                tool.development_pin,
                if tool.available_on_path {
                    "present on PATH"
                } else {
                    "not found on PATH"
                }
            ));
        }
        for path in &self.selected_paths {
            text.push_str(&format!(
                "  {}: {:?}; contents unverified\n",
                path.integration, path.state
            ));
        }
        text.push_str("No selected file contents read. Native prerequisites and live client unverified.\nSee README.md and docs/compatibility/WINDOWS_INSTALL.md for setup.\n");
        text
    }
}
