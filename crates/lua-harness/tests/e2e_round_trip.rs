//! Chained addon <-> desktop round trips (G2). Every hop runs the real code on both
//! sides: the production addon Lua under the vendored Lua 5.1, and the real app
//! services over in-memory or temporary SQLite. The client is a synthetic stub, so
//! nothing here says anything about live Forever behaviour; live checks stay
//! unverified in the handoff.
mod common;

use azeroth_app::snapshots::{FileInputFormat, SnapshotApplication};
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Builds the addon admits for capture. 70425 joins this list when its admission
/// lands; this test deliberately does not assert anything about unadmitted builds.
const ADMITTED_BUILDS: [&str; 2] = ["70374", "70338"];
const WALL_START: i64 = 1_791_792_000;

/// Synthetic frames rich enough for the copy window and the HUD.
const UI_STUB: &str = r#"
  frames = {}; messages = {}; combat = false
  function InCombatLockdown() return combat end
  function print(value) messages[#messages + 1] = value end
  function CreateFrame(kind)
    local f = {kind = kind, scripts = {}, shown = false, text = '', focus = false}
    frames[#frames + 1] = f
    local noop = function() end
    for _, name in ipairs({'SetSize', 'SetPoint', 'ClearAllPoints', 'SetMultiLine',
      'SetFontObject', 'SetFrameStrata', 'EnableMouse', 'SetAllPoints', 'SetColorTexture',
      'RegisterEvent', 'SetMovable', 'RegisterForDrag', 'SetJustifyH', 'StartMoving',
      'StopMovingOrSizing', 'SetTextColor'}) do f[name] = noop end
    function f:SetScript(name, callback) self.scripts[name] = callback end
    function f:SetText(value)
      self.text = value
      if self.scripts.OnTextChanged then self.scripts.OnTextChanged(self, false) end
    end
    function f:GetText() return self.text end
    function f:SetAutoFocus(value) self.autoFocus = value end
    function f:SetMaxLetters(value)
      self.max = value
      if #self.text > value then self:SetText(self.text:sub(1, value)) end
    end
    function f:SetFocus() self.focus = true end
    function f:ClearFocus() self.focus = false end
    function f:HighlightText() self.highlight = self.text end
    function f:Show() self.shown = true end
    function f:Hide() self.shown = false end
    function f:SetScrollChild(child) self.child = child end
    function f:GetFont() return 'synthetic-font', 12, '' end
    function f:SetFont() return true end
    function f:GetTextColor() return 1, 1, 1, 1 end
    function f:CreateFontString() return CreateFrame('FontString') end
    function f:CreateTexture() return CreateFrame('Texture') end
    return f
  end
"#;

/// Writes `AzerothNavigatorDB` the way WoW does on logout: one assignment per saved
/// variable, tab indentation, `["key"] = value,` entries and `-- [n]` array comments.
const WRITER: &str = r#"
  local function quote(s)
    return '"' .. s:gsub('[%c"\\]', function(c)
      if c == '\n' then return '\\n' elseif c == '\r' then return '\\r'
      elseif c == '"' then return '\\"' elseif c == '\\' then return '\\\\' end
      return string.format('\\%03d', c:byte())
    end) .. '"'
  end
  local function scalar(v)
    if type(v) == 'string' then return quote(v) end
    if type(v) == 'boolean' then return tostring(v) end
    if v == math.floor(v) then return string.format('%d', v) end
    return string.format('%.17g', v)
  end
  local function write(v, depth, out)
    if type(v) ~= 'table' then out[#out + 1] = scalar(v); return end
    out[#out + 1] = '{\n'
    local pad = string.rep('\t', depth + 1)
    local n = #v
    for i = 1, n do
      out[#out + 1] = pad
      write(v[i], depth + 1, out)
      out[#out + 1] = ', -- [' .. i .. ']\n'
    end
    local keys = {}
    for k in pairs(v) do
      if not (type(k) == 'number' and k >= 1 and k <= n and k == math.floor(k)) then
        keys[#keys + 1] = k
      end
    end
    table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
    for _, k in ipairs(keys) do
      out[#out + 1] = pad .. '[' .. (type(k) == 'string' and quote(k) or ('' .. k)) .. '] = '
      write(v[k], depth + 1, out)
      out[#out + 1] = ',\n'
    end
    out[#out + 1] = string.rep('\t', depth) .. '}'
  end
  function writeSavedVariables(name, value)
    local out = {name, ' = '}
    write(value, 0, out)
    out[#out + 1] = '\n'
    return table.concat(out)
  end
"#;

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        Scratch(std::env::temp_dir().join(format!("anav-e2e-{}-{name}", std::process::id())))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(self.0.with_extension("sqlite3"));
    }
}

fn client(build: &str) -> azeroth_lua_harness::Sandbox {
    let sandbox = common::stub_client("1.60.1", build, 16001);
    sandbox
        .run::<()>(
            "e2e-client",
            &format!(
                "{UI_STUB}\n{WRITER}\n\
                 wall, mono, level = {WALL_START}, 100, 12\n\
                 function GetServerTime() return wall end\n\
                 function GetTime() return mono end\n\
                 function UnitLevel() return level end"
            ),
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
}

fn now_ms(offset_s: i64) -> UtcMillis {
    UtcMillis::new((WALL_START + offset_s) * 1000).unwrap()
}

fn disposition(receipt: &azeroth_app::snapshots::ImportReceipt) -> Value {
    serde_json::to_value(&receipt.disposition).unwrap()
}

/// Reads the saved variable through the WoW-format writer into a temp file.
fn write_saved_variables(sandbox: &azeroth_lua_harness::Sandbox, scratch: &Scratch) {
    let text: String = sandbox
        .run(
            "write-saved-variables",
            "return writeSavedVariables('OtherAddonDB', {profile='demo', ratios={0.5, -3, 1e15}}) \
             .. writeSavedVariables('AzerothNavigatorDB', AzerothNavigatorDB)",
        )
        .unwrap();
    assert!(text.contains("AzerothNavigatorDB = {\n"));
    assert!(text.contains("\t[\"snapshot_export\"] = \"ANAV1:"));
    std::fs::write(&scratch.0, text).unwrap();
}

#[test]
fn variant_a_full_addon_capture_reaches_the_desktop_and_back() {
    for build in ADMITTED_BUILDS {
        full_addon_round_trip(build);
    }
}

fn full_addon_round_trip(build: &str) {
    let sandbox = client(build);
    sandbox
        .run::<()>(
            "trust-clock-and-capture",
            r#"
      local state = ns.Diagnostics.state
      state.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      state.onSlash('clock')
      state.onSlash('clock trust')
      state.onSlash('snapshot e2e-profile')
      assert(AzerothNavigatorDB.snapshot_sequence == 1)
      state.onSlash('copy snapshot')
      assert(state.copy_ui_code == 'copy_shown', tostring(state.copy_ui_code))
      assert(state.copy_view.input:GetText() == AzerothNavigatorDB.snapshot_export)
    "#,
        )
        .unwrap();
    let saved: String = sandbox
        .run("saved", "return AzerothNavigatorDB.snapshot_export")
        .unwrap();
    let copied: String = sandbox
        .run(
            "copied",
            "return ns.Diagnostics.state.copy_view.input:GetText()",
        )
        .unwrap();
    assert_eq!(
        saved, copied,
        "the copy window shows exactly the saved export"
    );

    // Hop 1: SavedVariables file in WoW's writer format, and the copied text.
    let scratch = Scratch::new(&format!("{build}.lua"));
    write_saved_variables(&sandbox, &scratch);
    let app = SnapshotApplication::in_memory().unwrap();
    let from_file = app
        .import_file(
            &scratch.0,
            &FileInputFormat::SavedVariables {
                global: "AzerothNavigatorDB".into(),
                key: "snapshot_export".into(),
            },
            Some(now_ms(1)),
        )
        .unwrap();
    let from_copy = app.import_text(&copied, Some(now_ms(1))).unwrap();
    assert_eq!(from_file.snapshot_id, from_copy.snapshot_id);
    assert_eq!(from_file.profile_id, from_copy.profile_id);
    assert_eq!(disposition(&from_file)["outcome"], "stored");
    assert_eq!(disposition(&from_copy)["outcome"], "duplicate");
    assert!(!from_file.fixture);
    assert_eq!(
        serde_json::to_value(from_file.namespace).unwrap(),
        "reference"
    );
    assert_eq!(
        serde_json::to_value(from_file.freshness).unwrap()["status"],
        "fresh",
        "a confirmed-trusted clock yields fresh, not uncertain"
    );
    let profile = from_file.profile_id.clone();
    let first_id = from_file.snapshot_id.clone();

    // Unknown, restricted, unsupported and known stay distinct on both sides.
    let envelope: Value = serde_json::from_str(&saved[6..]).unwrap();
    assert_eq!(envelope["game"]["build"], format!("1.60.1.{build}"));
    let stored = app.current(&profile, Some(now_ms(1))).unwrap().unwrap();
    let stored_payload = serde_json::to_value(&stored.envelope.payload).unwrap();
    assert_eq!(stored_payload["level"]["status"], "known");
    assert_eq!(stored_payload["level"]["value"], 12);
    for field in ["role", "location", "equipment", "quests", "objectives"] {
        assert_eq!(
            semantic(&stored_payload[field]),
            semantic(&envelope["payload"][field]),
            "{field} changed between the addon export and the desktop store"
        );
    }
    assert_eq!(envelope["payload"]["role"]["status"], "unknown");

    // Hop 2: an empty, explicitly synthetic protocol plan bound to the snapshot.
    let plan = synthetic_empty_plan(&envelope);
    sandbox
        .ns()
        .set("plan_text", format!("ANAV1:{plan}"))
        .unwrap();
    sandbox
        .run::<()>(
            "import-plan",
            r#"
      local state = ns.Diagnostics.state
      state.onPlanSlash('import ' .. ns.plan_text)
      assert(state.hud_model.status == 'no_steps', state.hud_model.status)
      assert(state.hud_model.plan_id == 'demo-plan-001')
    "#,
        )
        .unwrap();

    // Honest status: objective mappings are unavailable and nothing auto-advances.
    sandbox
        .run::<()>(
            "mapping-status",
            r#"
      local state = ns.Diagnostics.state
      assert(state.hud_model.objective_mapping_available == false)
      assert(state.plan_controls:describe():find(
        'Objective completion mapping unavailable; steps do not auto-complete', 1, true))
      assert(state.hud_model.step_id == nil, 'an empty plan has no step to advance')
    "#,
        )
        .unwrap();

    // Hop 3: recapture supersedes the loaded plan's snapshot; desktop keeps history.
    sandbox
        .run::<()>(
            "recapture",
            r#"
      wall = wall + 30; mono = mono + 30
      local state = ns.Diagnostics.state
      state.onSlash('snapshot e2e-profile')
      assert(AzerothNavigatorDB.snapshot_sequence == 2)
      assert(state.hud_model.status == 'snapshot_superseded', state.hud_model.status)
    "#,
        )
        .unwrap();
    let second: String = sandbox
        .run("second", "return AzerothNavigatorDB.snapshot_export")
        .unwrap();
    let receipt = app.import_text(&second, Some(now_ms(31))).unwrap();
    assert_eq!(disposition(&receipt)["outcome"], "stored");
    assert_eq!(
        disposition(&receipt)["previous_current"],
        json!(first_id.as_str())
    );
    let history = app.history(&profile, 10, Some(now_ms(31))).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history.iter().filter(|h| h.is_current).count(), 1);
    let current = app.current(&profile, Some(now_ms(31))).unwrap().unwrap();
    assert_eq!(current.envelope.payload.id, receipt.snapshot_id);
    // Replaying the older export is history only and never replaces current.
    let replay = app.import_text(&saved, Some(now_ms(31))).unwrap();
    assert_eq!(disposition(&replay)["outcome"], "duplicate");

    // Refusals on the desktop: a tampered payload and a flipped fixture flag.
    let tampered = second.replacen("\"value\":12", "\"value\":\"12\"", 1);
    assert_ne!(tampered, second);
    assert!(app.import_text(&tampered, Some(now_ms(31))).is_err());
    let flipped = second.replacen("\"fixture\":false", "\"fixture\":true", 1);
    assert_ne!(flipped, second);
    assert!(app.import_text(&flipped, Some(now_ms(31))).is_err());
    assert_eq!(
        app.history(&profile, 10, Some(now_ms(31))).unwrap().len(),
        2
    );

    // Refusals in the addon: a plan for another fixture flag or build keeps the
    // loaded plan untouched.
    for (name, edit) in [
        ("fixture", ("fixture", json!(true))),
        ("build", ("build", json!("1.60.1.99999"))),
    ] {
        let mut other = plan.clone();
        other["payload"]["id"] = json!("refused-plan");
        if edit.0 == "fixture" {
            other["fixture"] = edit.1;
        } else {
            other["game"]["build"] = edit.1;
        }
        sandbox
            .ns()
            .set("refused_plan", format!("ANAV1:{other}"))
            .unwrap();
        sandbox
            .run::<()>(
                name,
                r#"
          local state = ns.Diagnostics.state
          local before = state.plan_controls:status().plan_id
          state.onPlanSlash('import ' .. ns.refused_plan)
          assert(before == 'demo-plan-001')
          assert(state.plan_controls:status().plan_id == before, 'refused plan replaced the loaded one')
        "#,
            )
            .unwrap();
    }

    // Control: an otherwise identical plan with a new id is accepted, so the refusals
    // above are caused by the fixture flag and build, not by the id.
    let mut accepted = plan.clone();
    accepted["payload"]["id"] = json!("accepted-plan");
    sandbox
        .ns()
        .set("accepted_plan", format!("ANAV1:{accepted}"))
        .unwrap();
    sandbox
        .run::<()>(
            "control",
            r#"
      local state = ns.Diagnostics.state
      state.onPlanSlash('import ' .. ns.accepted_plan)
      assert(state.plan_controls:status().plan_id == 'accepted-plan')
      state.onPlanSlash('import ' .. ns.plan_text)
      assert(state.plan_controls:status().plan_id == 'demo-plan-001')
    "#,
        )
        .unwrap();

    // Hop 4: manual session recording, export and desktop analytics.
    session_hop(&sandbox, &app, &profile, build);

    // Hop 5: a reload loses runtime trust and forgets the loaded plan.
    sandbox
        .run::<()>(
            "reload",
            r#"
      local reloaded = ns.Diagnostics.attach(ns.Diagnostics.state.env)
      reloaded.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      reloaded.onSlash('snapshot e2e-profile')
      local envelope = assert(ns.Decode.decode(AzerothNavigatorDB.snapshot_export:sub(7)))
      assert(envelope.payload.capture.clock_trust == 'uncertain')
    "#,
        )
        .unwrap();
    // A cached capture from one admitted build is never accepted on another.
    let other_build = ADMITTED_BUILDS.iter().find(|b| **b != build).unwrap();
    sandbox
        .run::<()>(
            "cross-build",
            &format!(
                r#"
      function GetBuildInfo() return '1.60.1', '{other_build}', '', 16001 end
      local reloaded = ns.Diagnostics.attach(ns.Diagnostics.state.env)
      reloaded.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      reloaded.onPlanSlash('import ' .. ns.plan_text)
      local model = reloaded.plan_controls and reloaded.plan_controls:status() or {{}}
      assert(model.status ~= 'ready' and not model.cached_snapshot,
        'build ' .. tostring(model.status) .. ' accepted a capture from {build}')
      assert(reloaded.plan_controls == nil or reloaded.plan_controls:status().plan_id == nil,
        'a plan for {build} loaded on {other_build}')
      function GetBuildInfo() return '1.60.1', '{build}', '', 16001 end
    "#
            ),
        )
        .unwrap();
    let after_reload: String = sandbox
        .run("after-reload", "return AzerothNavigatorDB.snapshot_export")
        .unwrap();
    let receipt = app.import_text(&after_reload, Some(now_ms(200))).unwrap();
    assert_eq!(
        serde_json::to_value(receipt.freshness).unwrap()["status"],
        "uncertain",
        "unconfirmed clock must not read as fresh or stale"
    );
}

/// Observation metadata (source, clock trust) may be defaulted on either side; the
/// status and the value it guards must never change in transit.
fn semantic(value: &Value) -> Value {
    match value {
        Value::Object(map) if map.contains_key("status") => {
            let mut kept = serde_json::Map::new();
            for key in ["status", "value"] {
                if let Some(inner) = map.get(key) {
                    kept.insert(key.into(), semantic(inner));
                }
            }
            Value::Object(kept)
        }
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), semantic(v))).collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(semantic).collect()),
        other => other.clone(),
    }
}

fn synthetic_empty_plan(snapshot: &Value) -> Value {
    let mut plan: Value = serde_json::from_str(
        &std::fs::read_to_string(common::path("fixtures/contracts/v1/route-plan.json")).unwrap(),
    )
    .unwrap();
    let reference = json!({"id": snapshot["payload"]["id"],
        "character": snapshot["payload"]["character"],
        "context": snapshot["payload"]["context"]});
    plan["fixture"] = false.into();
    plan["namespace"] = "reference".into();
    plan["game"] = snapshot["game"].clone();
    plan["producer"] = json!({"component": "desktop", "version": "synthetic-protocol-test"});
    plan["provenance"] = json!({"source": "manual_entry", "clock_trust": "uncertain"});
    plan["payload"]["context"] = snapshot["payload"]["context"].clone();
    plan["payload"]["inputs"] = json!([reference]);
    plan["dependencies"]["snapshots"] = plan["payload"]["inputs"].clone();
    plan["payload"]["steps"] = json!([]);
    plan["payload"]["travel"] = json!([]);
    plan["payload"]["capability_assumptions"] = json!({});
    plan["payload"]["expires_at_utc_ms"] = Value::Null;
    plan["payload"]["goal"] = "Synthetic empty plan protocol test".into();
    plan
}

fn session_hop(
    sandbox: &azeroth_lua_harness::Sandbox,
    app: &SnapshotApplication,
    profile: &str,
    build: &str,
) {
    sandbox
        .run::<()>(
            "session",
            r#"
      local state = ns.Diagnostics.state
      mono = mono + 10
      state.onSessionSlash('start e2e-profile')
      mono = mono + 10
      state.onSessionSlash('sample')
      mono = mono + 10
      state.onSessionSlash('pause')
      mono = mono + 10
      state.onSessionSlash('resume')
      level = 13
      mono = mono + 10
      state.onSessionSlash('sample')
      mono = mono + 10
      state.onSessionSlash('stop')
      state.onSessionSlash('export')
      assert(AzerothNavigatorDB.session_export, 'no session export')
    "#,
        )
        .unwrap();
    let text: String = sandbox
        .run("session-text", "return AzerothNavigatorDB.session_export")
        .unwrap();
    let receipt = app.import_session_text(profile, &text);
    let receipt = receipt.unwrap_or_else(|e| panic!("{build}: {e:?}"));
    assert_eq!(receipt.inserted, 1);
    let rows = app.list_sessions(profile).unwrap();
    assert_eq!(rows.len(), 1);
    let detail = app
        .get_session_detail(profile, &rows[0].session_id)
        .unwrap();
    let analytics = serde_json::to_value(&detail.analytics).unwrap();
    // Zero is a measurement, not an absence: observed time is 0 while pause time is a gap.
    assert_eq!(analytics["timing"]["totals"]["observed_ms"], 0);
    assert_eq!(analytics["timing"]["totals"]["unclassified_ms"], 40_000);
    assert_eq!(analytics["timing"]["totals"]["gap_ms"], 10_000);
    assert_eq!(analytics["timing"]["covered_ms"], 50_000);
    assert_eq!(analytics["timing"]["uncovered_ms"], 0);
    assert_eq!(analytics["progression"]["observed_level_change"], 1);
    assert_eq!(analytics["utc"]["status"], "uncertain");
    assert_eq!(analytics["summary"]["context"]["namespace"], "reference");
    assert_eq!(
        analytics["summary"]["context"]["build"]["build"],
        format!("1.60.1.{build}")
    );
    assert!(!detail.fixture);
    assert_eq!(
        serde_json::to_value(&rows[0].session_id).unwrap(),
        "recording-e2e-profile-1"
    );
}

fn fixture_plan_export() -> (String, Value, UtcMillis) {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let value: Value = serde_json::from_str(snapshot).unwrap();
    let now = UtcMillis::new(
        value["payload"]["capture"]["observed_at_utc_ms"]
            .as_i64()
            .unwrap()
            + 100,
    )
    .unwrap();
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    // The objective needs level 20, so the HUD has a real prerequisite to evaluate.
    content["envelope"]["payload"]["objectives"][0]["requirements"] =
        json!({"status": "known", "value": {"op": "level_at_least", "level": 20}});
    app.stage_content(&content.to_string()).unwrap();
    app.activate_content(
        &profile,
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    let request = json!({"profile_id": profile, "snapshot_id": value["payload"]["id"],
        "revision": "synthetic-planning-v1", "goal_profile": "manual-v1",
        "origin": "demo-node-1", "stopping_node": "demo-node-1", "budget_ms": 100000,
        "stopping_service_ms": 0, "candidates": [{"objective": "synthetic-service",
        "node": "demo-node-2", "utility": 10, "completion_ms": 1000, "service_ms": 0}],
        "excluded": []});
    let report = app.plan_session(&request.to_string(), Some(now)).unwrap();
    let planned = serde_json::to_value(&report).unwrap();
    let text = app
        .export_plan(&request.to_string(), Some(now))
        .unwrap()
        .text;
    (text, planned, now)
}

#[test]
fn variant_b_planner_steps_reach_a_ready_hud_with_distinct_failure_states() {
    let (text, planned, now) = fixture_plan_export();
    let exported: Value = serde_json::from_str(&text[6..]).unwrap();
    let first_planned = planned["result"]["selected"][0]["candidate"].clone();
    assert_eq!(first_planned, "synthetic-service");
    let first_step = exported["payload"]["steps"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(exported["fixture"], true);

    let sandbox = common::strict_core();
    sandbox.ns().set("exported", text.clone()).unwrap();
    sandbox
        .ns()
        .set(
            "snapshot_text",
            include_str!("../../../fixtures/contracts/v1/character-snapshot.json"),
        )
        .unwrap();
    sandbox.ns().set("now_ms", now.get()).unwrap();
    sandbox
        .ns()
        .set("exported_step", first_step.as_str())
        .unwrap();
    sandbox
        .run::<()>(
            "hud",
            r#"
      local envelope = assert(ns.Decode.decode(ns.exported:sub(7)))
      ns.snapshot = assert(ns.Decode.decode(ns.snapshot_text)).payload
      ns.plans = ns.Plan.new({context = envelope.payload.context, fixture = true})
      assert(ns.plans:importTransport(ns.exported))
      ns.rendered = nil
      ns.controls = ns.PlanControls.new(ns.plans, function()
        return {snapshot = ns.snapshot, fixture = true,
          freshness = {clock_trust = 'trusted', now_utc_ms = ns.now_ms, max_age_ms = 1800000}}
      end, function(model) ns.rendered = model end)
    "#,
        )
        .unwrap();
    let ready: Value = serde_json::from_str(
        &sandbox
            .run::<String>(
                "ready",
                "return ns.Json.encode({status=ns.controls:status().status,                  step=ns.controls:status().step_id, mapping=ns.controls:status().objective_mapping_available})",
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["step"], first_step.as_str());
    assert_eq!(ready["mapping"], false);

    // Blocked/invalidated, unknown (restricted) and ready are three different answers.
    let statuses: Vec<String> = sandbox
        .run(
            "states",
            r#"
      local out = {}
      ns.snapshot.level.value = 1
      out[#out + 1] = ns.controls:status().status
      ns.snapshot.level = {status = 'restricted'}
      out[#out + 1] = ns.controls:status().status
      ns.snapshot.level = {status = 'unknown'}
      out[#out + 1] = ns.controls:status().status
      ns.snapshot = assert(ns.Decode.decode(ns.snapshot_text)).payload
      out[#out + 1] = ns.controls:status().status
      return out
    "#,
        )
        .unwrap();
    assert_eq!(statuses[0], "invalidated");
    assert_eq!(statuses[1], "uncertain");
    assert_eq!(statuses[2], "uncertain");
    assert_eq!(statuses[3], "ready");

    // Refusals: wrong fixture flag, wrong build, wrong dependency, tampered payload.
    for (label, edit) in [
        ("fixture", 0),
        ("build", 1),
        ("dependency", 2),
        ("tampered", 3),
    ] {
        let mut value = exported.clone();
        match edit {
            0 => value["fixture"] = json!(false),
            1 => value["game"]["build"] = json!("different"),
            2 => value["dependencies"]["dataset_revision"] = json!("changed"),
            _ => value["payload"]["steps"][0]["id"] = json!(7),
        }
        sandbox
            .ns()
            .set("candidate", format!("ANAV1:{value}"))
            .unwrap();
        assert!(
            sandbox
                .run::<bool>(
                    label,
                    "local before = ns.plans:get().payload.id;                      return ns.plans:importTransport(ns.candidate) == nil                      and ns.plans:get().payload.id == before                      and ns.plans:get().payload.steps[1].id == ns.exported_step",
                )
                .unwrap(),
            "{label} replacement was accepted or disturbed the loaded plan"
        );
    }
}
