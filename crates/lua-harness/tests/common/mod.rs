//! Shared helpers for the addon harness tests. Everything here is synthetic: stubs
//! stand in for a client that does not exist in this environment, so no test result
//! says anything about real Forever behaviour.
#![allow(dead_code)]

pub mod ui;

use azeroth_lua_harness::{EnvMode, Sandbox, repo_root};
use serde_json::Value;
use std::path::PathBuf;

/// Lua helper installed into stub clients. `define("A.B")` creates a stub function at
/// that path that records its own name in `calls`.
const STUB_PRELUDE: &str = r#"
calls = {}
local function record(name)
  return function() calls[#calls + 1] = name end
end
function define(path)
  local owner, name = path:match("^(.-)%.?([^.]+)$")
  local target = _G
  for segment in owner:gmatch("[^.]+") do
    target[segment] = target[segment] or {}
    target = target[segment]
  end
  target[name] = record(path)
end
function install_frame_stub()
  frame = { events = {}, scripts = {} }
  function CreateFrame(kind)
    calls[#calls + 1] = "CreateFrame"
    frame.kind = kind
    function frame:RegisterEvent(event)
      self.events[#self.events + 1] = event
    end
    function frame:SetScript(name, handler)
      self.scripts[name] = handler
    end
    return frame
  end
  SlashCmdList = {}
end
function install_build_info(version, build, interface)
  function GetBuildInfo()
    calls[#calls + 1] = "GetBuildInfo"
    return version, build, "Jan 1 2000", interface
  end
end
"#;

pub fn path(relative: &str) -> PathBuf {
    repo_root().join(relative)
}

/// Capability ids and API names from the committed machine-readable template.
pub fn template() -> Value {
    let text = std::fs::read_to_string(path(
        "docs/compatibility/capability-observations.template.json",
    ))
    .expect("template exists");
    serde_json::from_str(&text).expect("template is valid JSON")
}

pub fn template_api_names() -> Vec<String> {
    let mut names: Vec<String> = template()["capabilities"]
        .as_array()
        .expect("capabilities array")
        .iter()
        .flat_map(|row| row["api_names"].as_array().expect("api_names").clone())
        .map(|name| name.as_str().expect("string").to_owned())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// A permissive sandbox with the synthetic client installed: every probed API exists as
/// a recording stub, `GetBuildInfo` returns the given values, and `CreateFrame` and
/// `SlashCmdList` capture what the addon registers. Nothing is loaded yet.
pub fn stub_client(version: &str, build: &str, interface: i64) -> Sandbox {
    let sandbox = Sandbox::new(EnvMode::Client).expect("sandbox");
    sandbox
        .run::<()>("stub-prelude", STUB_PRELUDE)
        .expect("prelude");
    for name in template_api_names() {
        if name != "GetBuildInfo" && name != "CreateFrame" {
            sandbox
                .run::<()>("stub-define", &format!("define({name:?})"))
                .expect("define stub");
        }
    }
    sandbox
        .run::<()>(
            "stub-core",
            &format!(
                "install_frame_stub(); install_build_info({version:?}, {build:?}, {interface}); C_QuestLog.GetNumQuestLogEntries = function() return 0 end"
            ),
        )
        .expect("core stubs");
    sandbox
}

/// The synthetic build every committed fixture uses.
pub fn synthetic_client() -> Sandbox {
    stub_client("0.0.0", "SYNTHETIC", 99999)
}

/// [`synthetic_client`] with every TOC file already loaded (the addon has attached itself).
pub fn loaded_client() -> Sandbox {
    let sandbox = synthetic_client();
    sandbox.load_toc_files().expect("addon loads");
    sandbox
}

/// A strict sandbox with only the `Core/` files loaded; any client-global access raises.
pub fn strict_core() -> Sandbox {
    let sandbox = Sandbox::new(EnvMode::Strict).expect("sandbox");
    let toc = azeroth_lua_harness::Toc::read().expect("toc");
    for file in toc.files.iter().filter(|file| file.starts_with("Core/")) {
        sandbox.load_addon_file(file).expect("core file loads");
    }
    sandbox
}

/// Runs a snippet that returns `(string|nil, string|nil)` and splits the result.
pub fn encode_pair(sandbox: &Sandbox, code: &str) -> (Option<String>, Option<String>) {
    sandbox
        .run::<(Option<String>, Option<String>)>("encode", code)
        .expect("snippet runs")
}

/// Largest container nesting depth of a JSON value (a scalar has depth 0).
pub fn json_depth(value: &Value) -> usize {
    match value {
        Value::Array(items) => 1 + items.iter().map(json_depth).max().unwrap_or(0),
        Value::Object(map) => 1 + map.values().map(json_depth).max().unwrap_or(0),
        _ => 0,
    }
}
