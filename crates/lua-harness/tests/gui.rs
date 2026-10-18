//! In-game window behavior against the shared widget mock (`common::ui`). Everything is
//! synthetic: the mock models a client UI that does not exist in this environment, so
//! none of this says anything about real Forever templates, tooltips or taint. The
//! LIVE-GUI runbook rows stay unverified.
mod common;

use azeroth_lua_harness::Sandbox;
use common::ui::{ui_client, ui_client_unloaded};
use serde_json::{Value, json};

fn run(sandbox: &Sandbox, code: &str) {
    sandbox.run::<()>("gui", code).unwrap();
}

/// Wraps every action in a recording spy; the log holds dotted action paths.
const SPY: &str = r#"
function ui.spy(actions)
  local log = {}
  local function wrap(source, prefix)
    local out = {}
    for key, value in pairs(source) do
      if type(value) == 'table' then out[key] = wrap(value, prefix .. key .. '.')
      else out[key] = function(...) log[#log + 1] = prefix .. key; return value(...) end end
    end
    return out
  end
  return wrap(actions, ''), log
end
function ui.press(gui, log, id, expected)
  local widget = assert(gui.widgets[id], 'no widget ' .. id)
  local before = #log
  assert(ui.click(widget.frame), id .. ' was not clickable')
  if expected then
    assert(#log == before + 1 and log[#log] == expected,
      id .. ' logged ' .. tostring(log[before + 1]) .. ' expected ' .. expected)
  else
    assert(#log == before, id .. ' must not call an action')
  end
end
"#;

fn spy_window(build: &str, quest_event: bool) -> Sandbox {
    let sandbox = ui_client_unloaded(build);
    run(&sandbox, SPY);
    run(&sandbox, &format!("ui.quest_event = {quest_event}"));
    sandbox.load_toc_files().unwrap();
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      local actions, log = ui.spy(state.actions)
      ui.log = log
      ui.gui = ns.Gui.new(_G, state, actions)
      assert(ui.gui.open())
    "#,
    );
    sandbox
}

#[test]
fn no_frames_exist_at_load_and_login_creates_only_entry_points() {
    let sandbox = ui_client_unloaded("70374");
    run(
        &sandbox,
        "ui.baseline = #ui.frames; assert(ui.baseline == 2)",
    );
    sandbox.load_toc_files().unwrap();
    run(
        &sandbox,
        r#"
      -- the event frame is the only frame the addon creates at load
      assert(#ui.frames == ui.baseline + 1 and AzerothNavigatorFrame == nil)
      ui.login()
      assert(AzerothNavigatorFrame == nil)
      local s = ui.state()
      assert(s.gui == nil and s.minimap and s.options_panel)
      assert(s.gui_minimap_code == 'gui_minimap_registered')
      assert(s.gui_options_code == 'gui_options_registered')
    "#,
    );
}

#[test]
fn slash_anav_toggles_the_window_and_registers_escape_once() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      assert(SLASH_ANAV1 == '/anav' and type(SlashCmdList.ANAV) == 'function')
      local s = ui.state()
      SlashCmdList.ANAV('')
      local window = AzerothNavigatorFrame
      assert(window and window.shown and s.gui.isShown() and s.gui_code == 'gui_shown')
      assert(window.strata == 'DIALOG' and window.movable and window.parent == UIParent)
      assert(#UISpecialFrames == 1 and UISpecialFrames[1] == 'AzerothNavigatorFrame')
      -- never takes the keyboard or the focus
      for _, f in ipairs(ui.frames) do
        assert(not f.keyboard and not f.propagate and not f.focus)
        if f.kind == 'EditBox' then assert(f.auto_focus == false) end
      end
      SlashCmdList.ANAV(' ')
      assert(not window.shown and s.gui_code == 'gui_hidden')
      SlashCmdList.ANAV('')
      assert(window.shown)
      assert(ui.moves == 0)
      -- a reload attaches again; the Escape entry stays unique
      local again = ns.Diagnostics.attach(_G)
      again.onGuiSlash('')
      assert(#UISpecialFrames == 1 and AzerothNavigatorFrame ~= window)
      assert(again.gui.isShown() and s.gui.isShown())
    "#,
    );
}

#[test]
fn slash_anav_help_lists_every_command_and_unknown_input_prints_usage() {
    let sandbox = ui_client("70374");
    let before = sandbox.printed().len();
    run(
        &sandbox,
        r#"
      SlashCmdList.ANAV('help')
      SlashCmdList.ANAV('bogus')
      assert(AzerothNavigatorFrame == nil)
    "#,
    );
    let printed = sandbox.printed();
    let text = printed[before..].join("\n");
    for command in [
        "/anav ",
        "/anavdiag probe",
        "/anavdiag clock",
        "/anavplan paste",
        "/anavplan size",
        "/anavsession start",
        "/anavsession export",
    ] {
        assert!(text.contains(command), "{command} missing from {text}");
    }
    assert!(printed.last().unwrap().contains("usage: /anav [help]"));
}

#[test]
fn every_tab_button_calls_the_right_action_and_changes_state() {
    let sandbox = spy_window("70374", true);
    run(
        &sandbox,
        r#"
      local gui, log, state, press = ui.gui, ui.log, ui.state(), ui.press
      local W = gui.widgets
      -- Overview
      assert(gui.select('overview') and gui.tab == 'overview')
      state.report = nil
      press(gui, log, 'probe', 'probe')
      assert(state.report and state.stored)
      press(gui, log, 'hud_toggle', 'plan.hide')
      assert(state.settings.hud_visible == false and W.hud_toggle.frame.text == 'Show HUD')
      press(gui, log, 'hud_toggle', 'plan.show')
      assert(state.settings.hud_visible == true and W.hud_toggle.frame.text == 'Hide HUD')
      press(gui, log, 'refresh', nil)
      -- Plan
      gui.select('plan')
      ui.type(W.plan_paste.frame, 'ANAV1:{')
      press(gui, log, 'plan_import', 'plan.import')
      assert(state.plan_import_code and state.plan_import_code ~= 'plan_imported')
      assert(W.plan_paste:getText() == 'ANAV1:{', 'a refused import keeps the text')
      press(gui, log, 'plan_clear', 'plan.clear')
      press(gui, log, 'plan_hud_toggle', 'plan.hide')
      assert(state.settings.hud_visible == false)
      press(gui, log, 'plan_hud_toggle', 'plan.show')
      press(gui, log, 'plan_status', 'plan.status')
      assert(state.hud_model and state.hud_model.status == 'no_plan')
      -- HUD
      gui.select('hud')
      assert(W.hud_width:getText() == '420' and W.hud_height:getText() == '220')
      ui.type(W.hud_width.frame, '500'); ui.type(W.hud_height.frame, '300')
      press(gui, log, 'hud_size_apply', 'plan.size')
      assert(state.settings.hud_width == 500 and state.settings.hud_height == 300)
      ui.type(W.hud_width.frame, '5')
      assert(not ui.click(W.hud_size_apply.frame), 'an out of range width disables Apply')
      assert(state.settings.hud_width == 500)
      press(gui, log, 'hud_font', 'plan.font')
      assert(state.settings.hud_font_percent == 125)
      press(gui, log, 'hud_contrast', 'plan.contrast')
      assert(state.settings.hud_contrast == 'high' and W.hud_contrast.checked)
      press(gui, log, 'hud_contrast', 'plan.contrast')
      assert(state.settings.hud_contrast == 'default')
      press(gui, log, 'hud_reset', 'plan.reset')
      assert(state.settings.hud_width == 420 and state.settings.hud_height == 220)
      assert(state.settings.hud_font_percent == 100)
      assert(W.hud_width:getText() == '420', 'fields follow a reset')
      -- Capture
      gui.select('capture')
      assert(not ui.click(W.capture_snapshot.frame), 'needs a valid profile id')
      ui.type(W.capture_profile.frame, 'alpha_1')
      press(gui, log, 'capture_snapshot', 'snapshot')
      assert(AzerothNavigatorDB.snapshot_export and AzerothNavigatorDB.snapshot_sequence == 1)
      press(gui, log, 'capture_position', 'position')
      assert(state.position_diagnostics)
      press(gui, log, 'capture_copy_snapshot', 'copy')
      assert(state.copy_ui_code == 'copy_shown' and state.copy_view.title.text:find('snapshot'))
      press(gui, log, 'capture_copy_diagnostic', 'copy')
      assert(state.copy_view.title.text:find('diagnostic', 1, true))
      state.copy_view.close.scripts.OnClick()
      -- Session
      gui.select('session')
      assert(not ui.click(W.session_start.frame), 'needs a valid profile id')
      ui.type(W.session_profile.frame, 'run-1')
      assert(not ui.click(W.session_sample.frame), 'nothing to sample while idle')
      press(gui, log, 'session_start', 'session.start')
      assert(state.session_recorder:status().phase == 'recording')
      assert(not ui.click(W.session_start.frame) and not ui.click(W.session_resume.frame))
      ui.mono = ui.mono + 1; ui.wall = ui.wall + 1
      press(gui, log, 'session_sample', 'session.sample')
      press(gui, log, 'session_pause', 'session.pause')
      assert(state.session_recorder:status().phase == 'paused')
      press(gui, log, 'session_resume', 'session.resume')
      press(gui, log, 'session_stop', 'session.stop')
      assert(state.session_recorder:status().phase == 'stopped')
      assert(not ui.click(W.session_copy.frame), 'nothing stored yet')
      press(gui, log, 'session_export', 'session.export')
      assert(AzerothNavigatorDB.session_export)
      press(gui, log, 'session_copy', 'copy')
      assert(state.copy_view.title.text:find('session', 1, true))
      state.copy_view.close.scripts.OnClick()
      press(gui, log, 'session_start_quests', 'session.startQuests')
      assert(state.session_recorder:status().phase == 'recording')
      press(gui, log, 'session_stop', 'session.stop')
      -- Diagnostics
      gui.select('diagnostics')
      press(gui, log, 'diag_probe', 'probe')
      assert(not ui.click(W.diag_clock_trust.frame), 'trust needs a pending check')
      press(gui, log, 'diag_clock_check', 'clock.check')
      assert(state.clock.status().pending and not state.clock.status().trusted)
      press(gui, log, 'diag_clock_trust', 'clock.trust')
      assert(state.clock.status().trusted)
      press(gui, log, 'diag_clock_clear', 'clock.clear')
      assert(not state.clock.status().trusted and not state.clock.status().pending)
      -- Settings
      gui.select('settings')
      press(gui, log, 'set_auto_probe', 'setSetting')
      assert(state.settings.auto_probe == false and W.set_auto_probe.checked == false)
      press(gui, log, 'set_chat_summary', 'setSetting')
      assert(state.settings.chat_summary == false)
      press(gui, log, 'set_minimap_button', 'setSetting')
      assert(state.settings.minimap_button == false and state.minimap.frame.shown == false)
      press(gui, log, 'set_minimap_button', 'setSetting')
      assert(state.minimap.frame.shown == true)
      assert(AzerothNavigatorDB.settings.auto_probe == false)
    "#,
    );
}

#[test]
fn tabs_and_close_work_and_the_last_tab_is_remembered_across_reload() {
    let sandbox = spy_window("70374", false);
    run(
        &sandbox,
        r#"
      local gui, state = ui.gui, ui.state()
      for _, id in ipairs(ns.GuiText.tab_ids) do
        assert(ui.click(gui.widgets['tab_' .. id].frame))
        assert(gui.tab == id and gui.pages[id].shown)
        for other, page in pairs(gui.pages) do assert((other == id) == page.shown) end
        assert(state.settings.gui_last_tab == id)
        -- the active tab is marked in words, not by color alone
        assert(gui.widgets['tab_' .. id].frame.text:find('[', 1, true))
      end
      assert(AzerothNavigatorDB.settings.gui_last_tab == 'settings')
      assert(ui.click(gui.widgets.close.frame) and not AzerothNavigatorFrame.shown)
      local again = ns.Diagnostics.attach(_G)
      again.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      assert(again.settings.gui_last_tab == 'settings')
      again.onGuiSlash('')
      assert(again.gui.tab == 'settings')
    "#,
    );
}

#[test]
fn every_interactive_widget_has_a_tooltip_title_body_slash_and_gate() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      assert(state.actions.gui.open())
      local gui = state.gui
      local seen, expected = {}, 0
      for id, widget in pairs(gui.widgets) do
        local kind = widget.kind
        if kind == 'button' or kind == 'checkbox' or kind == 'editbox' then
          expected = expected + 1
        end
      end
      for _, f in ipairs(ui.interactive(AzerothNavigatorFrame)) do
        ui.enter(f)
        local tip = ui.tooltip
        assert(tip.shown and tip.owner == f and tip.anchor == 'ANCHOR_RIGHT')
        assert(type(tip.title) == 'string' and #tip.title > 0)
        assert(#tip.lines == 3)
        for _, line in ipairs(tip.lines) do assert(#line.text > 0 and line.wrap == true) end
        assert(tip.lines[2].text:find('Slash command: ', 1, true))
        assert(tip.lines[3].text:find('Gate: ', 1, true))
        ui.leave(f)
        assert(not tip.shown)
        seen[f] = true
      end
      for id, widget in pairs(gui.widgets) do
        local kind = widget.kind
        if kind == 'button' or kind == 'checkbox' or kind == 'editbox' then
          assert(seen[widget.frame], id .. ' is not in the walked tree or has no tooltip')
          assert(widget.tooltip_id == id)
        end
      end
      local count = 0
      for _ in pairs(seen) do count = count + 1 end
      assert(count == expected and count >= 43, count)
      -- the entry points carry tooltips too
      for _, f in ipairs({state.minimap.frame, state.options_panel.open.frame}) do
        ui.enter(f); assert(ui.tooltip.shown and #ui.tooltip.title > 0); ui.leave(f)
      end
      -- every text entry is complete
      for id, spec in pairs(ns.GuiText.widgets) do
        for _, key in ipairs({'label', 'tooltip_title', 'tooltip_body', 'slash_equivalent',
          'gate_hint'}) do
          assert(type(spec[key]) == 'string' and #spec[key] > 0, id .. '.' .. key)
        end
      end
    "#,
    );
}

#[test]
fn a_template_failure_falls_back_to_plain_frames_and_a_missing_tooltip_shows_inline_help() {
    for failing in [
        "UIPanelButtonTemplate",
        "UICheckButtonTemplate",
        "InputBoxTemplate",
        "UIPanelScrollFrameTemplate",
        "BackdropTemplate",
    ] {
        let sandbox = ui_client_unloaded("70374");
        run(&sandbox, &format!("ui.fail.{failing} = true"));
        sandbox.load_toc_files().unwrap();
        run(
            &sandbox,
            &format!(
                r#"
          ui.login()
          local state = ui.state()
          assert(state.actions.gui.open(), '{failing}')
          assert(ui.count_templates('{failing}') == 0)
          assert(AzerothNavigatorFrame.shown and state.gui_code == 'gui_shown')
          if '{failing}' == 'BackdropTemplate' then
            assert(AzerothNavigatorFrame.backdrop == nil)
          end
          -- the fallback widgets still work
          state.gui.select('settings')
          assert(ui.click(state.gui.widgets.set_auto_probe.frame))
          assert(state.settings.auto_probe == false)
          state.gui.select('overview')
          assert(ui.click(state.gui.widgets.probe.frame))
          assert(state.report)
        "#
            ),
        );
    }
    // Every template unknown at once.
    let sandbox = ui_client_unloaded("70374");
    run(&sandbox, "ui.templates = {}");
    sandbox.load_toc_files().unwrap();
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      assert(state.actions.gui.open())
      for _, f in ipairs(ui.interactive(AzerothNavigatorFrame)) do
        ui.enter(f); assert(ui.tooltip.shown); ui.leave(f)
      end
    "#,
    );
    // No GameTooltip: an inline help line appears and clears.
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      GameTooltip = nil
      local state = ui.state()
      assert(state.actions.gui.open())
      local widget = state.gui.widgets.probe
      local function joined() return table.concat(ui.texts(AzerothNavigatorFrame), '\n') end
      assert(not joined():find('Run capability probe', 1, true))
      ui.enter(widget.frame)
      assert(joined():find('Run capability probe', 1, true), joined())
      assert(joined():find('/anavdiag probe', 1, true))
      ui.leave(widget.frame)
      assert(not joined():find('Run capability probe', 1, true))
    "#,
    );
}

#[test]
fn combat_true_or_unknown_refuses_opening_and_every_action_with_no_mutation() {
    for combat in ["true", "'error'", "'nil'", "'number'"] {
        let sandbox = ui_client("70374");
        run(
            &sandbox,
            &format!(
                r#"
          local state = ui.state()
          local frames = #ui.frames
          local db_before = ns.Json.encode(AzerothNavigatorDB.settings)
          ui.combat = {combat}
          local printed_before = #state.messages
          assert(not state.actions.gui.open())
          assert(AzerothNavigatorFrame == nil and #ui.frames == frames)
          assert(state.gui_code == (ui.combat == true and 'gui_combat_deferred' or
            'gui_combat_unknown'), tostring(state.gui_code))
          assert(#state.messages == printed_before + 1)
          SlashCmdList.ANAV('')
          assert(AzerothNavigatorFrame == nil)
          -- the minimap click is the same gate
          ui.click(state.minimap.frame)
          assert(AzerothNavigatorFrame == nil)
          assert(ns.Json.encode(AzerothNavigatorDB.settings) == db_before)
        "#
            ),
        );
    }
    // A window that is already open refuses every action when combat starts.
    let sandbox = spy_window("70374", true);
    run(
        &sandbox,
        r#"
      local gui, log, state = ui.gui, ui.log, ui.state()
      gui.select('capture'); ui.type(gui.widgets.capture_profile.frame, 'alpha')
      gui.select('session'); ui.type(gui.widgets.session_profile.frame, 'alpha')
      local before = ns.Json.encode(AzerothNavigatorDB)
      local baseline = #log
      for _, combat in ipairs({true, 'error', 'nil'}) do
        ui.combat = combat
        for id, widget in pairs(gui.widgets) do
          if widget.kind == 'button' or widget.kind == 'checkbox' then
            if id ~= 'close' and id:sub(1, 4) ~= 'tab_' then
              widget.enabled = true
              widget.frame.enabled = true
              ui.fire(widget.frame, 'OnClick', 'LeftButton', false)
            end
          end
        end
        local last = state.messages[#state.messages]
        assert(last:find(combat == true and 'refused: you are in combat' or
          'refused: combat state unknown', 1, true), tostring(combat) .. ': ' .. last)
        for _, message in ipairs(state.messages) do
          assert(not message:find('private', 1, true))
        end
      end
      for index = baseline + 1, #log do
        assert(log[index] == 'note', 'only the refusal note ran: ' .. log[index])
      end
      assert(#log > baseline, 'the refusals were recorded')
      assert(ns.Json.encode(AzerothNavigatorDB) == before, 'saved variables changed')
      -- closing and switching tabs stay possible; the tab is not saved in combat
      ui.combat = true
      local saved = state.settings.gui_last_tab
      assert(gui.select('plan') and gui.tab == 'plan')
      assert(state.settings.gui_last_tab == saved)
      assert(gui.close())
    "#,
    );
}

#[test]
fn a_non_admitted_build_still_opens_and_explains_the_refusal() {
    let sandbox = ui_client("70375");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      assert(state.actions.gui.open() and AzerothNavigatorFrame.shown)
      local text = table.concat(ui.texts(AzerothNavigatorFrame), '\n')
      assert(text:find('Admission: not admitted', 1, true), text)
      assert(text:find('Build not admitted. Probe, status and copy work', 1, true))
      state.gui.select('capture')
      ui.type(state.gui.widgets.capture_profile.frame, 'alpha')
      assert(ui.click(state.gui.widgets.capture_snapshot.frame))
      assert(AzerothNavigatorDB.snapshot_export == nil)
      assert(state.messages[#state.messages]:find('not an enabled experimental target', 1, true))
      state.gui.select('plan')
      ui.type(state.gui.widgets.plan_paste.frame, 'ANAV1:{}')
      assert(ui.click(state.gui.widgets.plan_import.frame))
      assert(state.plan_import_code == 'plan_build_unrecorded')
      assert(state.plan_controls == nil)
      state.gui.select('session')
      ui.type(state.gui.widgets.session_profile.frame, 'alpha')
      assert(ui.click(state.gui.widgets.session_start.frame))
      assert(state.session_recorder:status().phase == 'idle')
      state.gui.select('diagnostics')
      assert(ui.click(state.gui.widgets.diag_clock_check.frame))
      assert(not state.clock.status().pending)
    "#,
    );
}

#[test]
fn build_70425_is_development_unverified_and_the_other_builds_are_labelled() {
    for (build, label, words) in [
        (
            "70425",
            "development_unverified",
            "Development build, UNVERIFIED",
        ),
        (
            "70450",
            "development_unverified",
            "Development build, UNVERIFIED",
        ),
        ("70374", "release_target", "Release target build"),
        ("70338", "previous_recorded", "Previously recorded build"),
    ] {
        let sandbox = ui_client(build);
        run(
            &sandbox,
            &format!(
                r#"
          local state = ui.state()
          assert(state.actions.gui.open())
          local text = table.concat(ui.texts(AzerothNavigatorFrame), '\n')
          assert(text:find('Admission: {label}', 1, true), text)
          assert(text:find('{words}', 1, true), text)
          assert(text:find('Build: 1.60.1 / {build} / interface 16001', 1, true), text)
        "#
            ),
        );
    }
    // 70425 is a development capture target: the same actions run, labelled as such.
    let sandbox = ui_client("70425");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      state.onSlash('snapshot dev-profile')
      assert(AzerothNavigatorDB.snapshot_export)
      state.actions.gui.open(); state.gui.select('overview')
      local text = table.concat(ui.texts(AzerothNavigatorFrame), '\n')
      assert(text:find('not the 1.0 release target', 1, true) or
        text:find('Not the 1.0 release target', 1, true))
    "#,
    );
}

#[test]
fn the_minimap_angle_is_dragged_persisted_and_survives_a_reload() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      local button = state.minimap
      local last = button.frame.points[#button.frame.points]
      -- default 225 degrees: cos/sin * 80 rounded
      assert(last.point == 'CENTER' and last.rel == Minimap and last.x == -57 and last.y == -57)
      assert(button.frame.parent == Minimap and button.frame.name == nil)
      ui.drag(button.frame, 500, 500) -- straight above the minimap center: 90 degrees
      assert(ui.moves == 1 and state.settings.minimap_angle == 90)
      assert(AzerothNavigatorDB.settings.minimap_angle == 90)
      last = button.frame.points[#button.frame.points]
      assert(last.x == 0 and last.y == 80)
      -- a drag ends a click; the click after it opens normally
      assert(AzerothNavigatorFrame == nil)
      ui.click(button.frame)
      assert(AzerothNavigatorFrame.shown)
      -- the saved value is applied after a reload
      local again = ns.Diagnostics.attach(_G)
      again.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      again.onEvent(nil, 'PLAYER_LOGIN')
      local rim = again.minimap.frame.points[#again.minimap.frame.points]
      assert(again.settings.minimap_angle == 90 and rim.x == 0 and rim.y == 80)
      -- an out of range stored angle falls back to the default instead of being used
      AzerothNavigatorDB.settings.minimap_angle = 999
      local third = ns.Diagnostics.attach(_G)
      third.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      assert(third.settings.minimap_angle == 225)
    "#,
    );
    // Dragging is refused in combat and the setting is unchanged.
    run(
        &sandbox,
        r#"
      local state = ui.state()
      ui.combat = true
      local before = state.settings.minimap_angle
      local moves = ui.moves
      ui.drag(state.minimap.frame, 400, 400)
      assert(ui.moves == moves and state.settings.minimap_angle == before)
    "#,
    );
}

#[test]
fn missing_minimap_or_options_apis_set_silent_status_codes_and_never_block_other_entries() {
    // No Minimap object and no options API at all.
    let sandbox = ui_client_unloaded("70374");
    run(
        &sandbox,
        "Minimap = nil; Settings = nil; InterfaceOptions_AddCategory = nil",
    );
    sandbox.load_toc_files().unwrap();
    run(&sandbox, "ui.login()");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      assert(state.gui_minimap_code == 'gui_minimap_unavailable')
      assert(state.gui_options_code == 'gui_options_unavailable')
      assert(state.minimap == nil and state.options_panel == nil)
      SlashCmdList.ANAV('')
      assert(AzerothNavigatorFrame.shown, '/anav needs neither entry point')
    "#,
    );
    assert_eq!(sandbox.printed().len(), 1, "{:?}", sandbox.printed());

    // Only the legacy options API: it is used, and Minimap alone fails silently.
    let sandbox = ui_client_unloaded("70374");
    run(&sandbox, "Minimap = nil; Settings = nil");
    sandbox.load_toc_files().unwrap();
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      assert(state.gui_minimap_code == 'gui_minimap_unavailable')
      assert(state.gui_options_code == 'gui_options_registered')
      assert(#ui.legacy == 1 and ui.legacy[1].name == 'Azeroth Navigator' and #ui.categories == 0)
    "#,
    );

    // A modern Settings table that raises: still silent, only a code.
    let sandbox = ui_client_unloaded("70374");
    run(
        &sandbox,
        "Settings.RegisterAddOnCategory = function() error('private options failure') end",
    );
    sandbox.load_toc_files().unwrap();
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      assert(state.gui_options_code == 'gui_options_unavailable')
      assert(state.gui_minimap_code == 'gui_minimap_registered')
    "#,
    );
    assert_eq!(sandbox.printed().len(), 1);
    assert!(!sandbox.printed()[0].contains("private"));

    // auto_probe=false still registers, and prints nothing at login.
    let sandbox = ui_client_unloaded("70374");
    run(
        &sandbox,
        "AzerothNavigatorDB = {db_version = 1, settings = {auto_probe = false}}",
    );
    sandbox.load_toc_files().unwrap();
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      assert(state.settings.auto_probe == false and state.report == nil)
      assert(state.gui_minimap_code == 'gui_minimap_registered')
      assert(state.gui_options_code == 'gui_options_registered')
      assert(#ui.categories == 1 and #ui.registered == 1)
      assert(ui.categories[1].name == 'Azeroth Navigator')
    "#,
    );
    assert!(sandbox.printed().is_empty(), "{:?}", sandbox.printed());
}

#[test]
fn the_options_page_and_the_minimap_button_open_the_window() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      local panel = state.options_panel
      assert(panel.frame.name == 'Azeroth Navigator' and ui.categories[1].panel == panel.frame)
      assert(ui.click(panel.open.frame) and AzerothNavigatorFrame.shown)
      ui.click(state.minimap.frame)
      assert(not AzerothNavigatorFrame.shown)
      ui.combat = true
      ui.click(panel.open.frame)
      assert(not AzerothNavigatorFrame.shown)
      local text = table.concat(ui.texts(panel.frame), '\n')
      assert(text:find('Not affiliated with or endorsed by Blizzard Entertainment', 1, true))
    "#,
    );
}

#[test]
fn the_profile_id_validator_matches_the_slash_commands() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      local samples = {'a', 'A-z_09', string.rep('x', 80), string.rep('x', 81), '', 'a b',
        'Thrall-Area52 ', 'caf\195\169', 'a.b', 'a/b', nil, 5, '-', '_'}
      samples[#samples + 1] = string.rep('y', 80) .. '-'
      for index = 1, 15 do
        local value = samples[index]
        local valid = ns.Gui.validProfile(value)
        AzerothNavigatorDB.snapshot_export = nil
        AzerothNavigatorDB.snapshot_sequence = 0
        local ok = state.actions.snapshot(value)
        assert((ok == true) == valid, tostring(value) .. ' snapshot')
        local phase = state.session_recorder:status().phase
        local started = state.actions.session.start(value)
        assert((started == true) == valid, tostring(value) .. ' session')
        if started then
          state.actions.session.stop()
          -- a recording needs a fresh id per start in one run
        end
      end
    "#,
    );
}

#[test]
fn feedback_does_not_depend_on_chat_and_the_message_ring_is_bounded() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      assert(#state.messages == 1) -- the login summary, also printed
      for index = 1, 50 do state.actions.note('message ' .. index) end
      assert(#state.messages == 20 and state.messages[20] == 'message 50')
      assert(state.messages[1] == 'message 31')
      local printed = #state.messages
      state.actions.note('only in the window')
      state.actions.gui.open()
      state.gui.select('overview')
      local text = table.concat(ui.texts(AzerothNavigatorFrame), '\n')
      assert(text:find('Latest messages:', 1, true) and text:find('only in the window', 1, true))
      assert(text:find('Latest message: only in the window', 1, true))
      -- at most the last five are listed on the overview
      assert(not text:find('message 40', 1, true))
    "#,
    );
    let printed = sandbox.printed();
    assert_eq!(printed.len(), 1, "note() must not print: {printed:?}");
}

#[test]
fn login_still_prints_exactly_one_line_with_the_entry_points_registered() {
    let sandbox = ui_client("70374");
    assert_eq!(sandbox.printed().len(), 1, "{:?}", sandbox.printed());
    assert!(sandbox.printed()[0].starts_with("AzerothNavigator: 16 capabilities ("));
    run(
        &sandbox,
        "local s = ui.state(); assert(s.gui_minimap_code and s.gui_options_code)",
    );
}

#[test]
fn private_error_text_is_never_printed_or_recorded() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      state.actions.gui.open()
      ns.Capabilities.probe = function() error('private failure text') end
      local gui = state.gui
      gui.select('overview')
      ui.click(gui.widgets.probe.frame)
      gui.select('capture')
      ui.type(gui.widgets.capture_profile.frame, 'alpha')
      ui.click(gui.widgets.capture_snapshot.frame)
      gui.select('session')
      ui.type(gui.widgets.session_profile.frame, 'alpha')
      ui.click(gui.widgets.session_start.frame)
      gui.select('plan')
      ui.click(gui.widgets.plan_status.frame)
      gui.select('diagnostics')
      ui.click(gui.widgets.diag_clock_check.frame)
      ui.click(gui.widgets.diag_probe.frame)
      for _, message in ipairs(state.messages) do
        assert(not message:find('private', 1, true), message)
      end
      local text = table.concat(ui.texts(AzerothNavigatorFrame), '\n')
      assert(not text:find('private', 1, true), text)
    "#,
    );
    for line in sandbox.printed() {
        assert!(!line.contains("private"), "{line}");
    }
}

#[test]
fn status_after_a_failed_store_reports_the_failure_and_session_copy_works() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      ns.Export.store = function() return false, 'json_output_too_large' end
      state.onSlash('probe')
      state.onSlash('status')
      local last = state.messages[#state.messages]
      assert(last:find('export not stored (json_output_too_large)', 1, true), last)
      assert(not last:find('export stored', 1, true))
    "#,
    );
}

#[test]
fn the_reason_inspector_reflects_the_hud_model_after_a_pasted_import() {
    let sandbox = ui_client("70338");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      state.actions.gui.open(); state.gui.select('capture')
      ui.type(state.gui.widgets.capture_profile.frame, 'ui-profile')
      ui.click(state.gui.widgets.capture_snapshot.frame)
      ui.captured = AzerothNavigatorDB.snapshot_export
    "#,
    );
    let captured: String = sandbox.run("captured", "return ui.captured").unwrap();
    let snapshot: Value = serde_json::from_str(&captured[6..]).unwrap();
    let mut plan: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/route-plan.json"
    ))
    .unwrap();
    let reference = serde_json::json!({"id":snapshot["payload"]["id"],"character":snapshot["payload"]["character"],"context":snapshot["payload"]["context"]});
    plan["fixture"] = false.into();
    plan["namespace"] = "reference".into();
    plan["game"] = snapshot["game"].clone();
    plan["producer"] =
        serde_json::json!({"component":"desktop","version":"synthetic-protocol-test"});
    plan["provenance"] = serde_json::json!({"source":"manual_entry","clock_trust":"uncertain"});
    plan["payload"]["context"] = snapshot["payload"]["context"].clone();
    plan["payload"]["inputs"] = serde_json::json!([reference]);
    plan["dependencies"]["snapshots"] = plan["payload"]["inputs"].clone();
    plan["payload"]["steps"] = serde_json::json!([]);
    plan["payload"]["travel"] = serde_json::json!([]);
    plan["payload"]["capability_assumptions"] = serde_json::json!({});
    plan["payload"]["expires_at_utc_ms"] = Value::Null;
    plan["payload"]["goal"] = "Synthetic empty plan protocol test".into();
    sandbox
        .ns()
        .set("transport", format!("ANAV1:{plan}"))
        .unwrap();
    run(
        &sandbox,
        r#"
      local state = ui.state()
      local W = state.gui.widgets
      state.gui.select('plan')
      local function inspector() return W.plan_inspector.text end
      assert(inspector():find('No plan has been evaluated', 1, true), inspector())
      ui.type(W.plan_paste.frame, ns.transport)
      assert(ui.click(W.plan_import.frame))
      assert(state.plan_receipt and state.plan_receipt.id == 'demo-plan-001')
      assert(state.plan_receipt.steps == 0 and state.plan_import_code == 'plan_imported')
      assert(W.plan_paste:getText() == '', 'a successful import clears the box')
      local shown = table.concat(ui.texts(state.gui.pages.plan), '\n')
      assert(shown:find('Receipt: demo-plan-001, 0 steps, fixture false', 1, true), shown)
      assert(shown:find('Last import: imported', 1, true))
      assert(inspector():find('Status: no steps', 1, true), inspector())
      assert(inspector():find('Revalidation required.', 1, true))
      assert(inspector():find('steps never auto-complete', 1, true))
      -- refused imports keep the previous plan and say why in words
      ui.type(W.plan_paste.frame, 'ANAV1:{')
      assert(ui.click(W.plan_import.frame))
      assert(state.plan_receipt.id == 'demo-plan-001' and state.plan_import_code ~= 'plan_imported')
      -- an oversize paste is refused, not truncated into a different plan
      ui.type(W.plan_paste.frame, 'ANAV1:{' .. string.rep('x', 40000))
      assert(#W.plan_paste:getText() == 32769)
      assert(ui.click(W.plan_import.frame))
      assert(state.plan_import_code == 'transport_bytes')
      assert(table.concat(ui.texts(state.gui.pages.plan), '\n'):find('larger than 32 KiB', 1, true))
      assert(ui.click(W.plan_clear.frame))
      assert(state.plan_receipt == nil and inspector():find('no plan loaded', 1, true))
    "#,
    );
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(common::path(&format!("fixtures/contracts/v1/{name}"))).unwrap(),
    )
    .unwrap()
}

/// The synthetic HUD replay of plan_hud.rs: a route plan, one snapshot and a live HUD.
fn replay() -> Sandbox {
    let sandbox = common::strict_core();
    sandbox
        .ns()
        .set(
            "plan_text",
            serde_json::to_string(&fixture("route-plan.json")).unwrap(),
        )
        .unwrap();
    let mut snapshot = fixture("character-snapshot.json")["payload"].clone();
    snapshot["objectives"] = json!({});
    sandbox
        .ns()
        .set("snapshot_text", snapshot.to_string())
        .unwrap();
    run(
        &sandbox,
        r#"
      local envelope = assert(ns.Decode.decode(ns.plan_text))
      ns.plans = ns.Plan.new({context=envelope.payload.context, fixture=true})
      assert(ns.plans:importText(ns.plan_text))
      envelope = ns.plans:get()
      local first = envelope.payload.steps[1]
      first.prerequisite = {status='not_applicable'}
      first.invalidation = {status='not_applicable'}
      first.alternatives = {'alternative-a', 'alternative-b', 'undefined-alternative'}
      local function step(id, objective)
        return {id=id, objective=objective, participants=first.participants,
          alternatives={}, prerequisite={status='not_applicable'},
          invalidation={status='not_applicable'}, required_fields={},
          explanation='Synthetic replay', evidence={},
          expected_ms={status='not_applicable'}, estimate={status='not_applicable'}}
      end
      envelope.payload.steps = {first, step('step-a', 'alternative-a'),
        step('step-b', 'alternative-b')}
      ns.document = envelope
      local function plain(value)
        if value == ns.Decode.null then return nil end
        if type(value) ~= 'table' then return value end
        local copy = {}
        for k, v in pairs(value) do copy[k] = plain(v) end
        return copy
      end
      ns.encode = function() return assert(ns.Json.encode(plain(ns.document), 16384)) end
      assert(ns.plans:importText(ns.encode()))
      ns.context = {snapshot=assert(ns.Decode.decode(ns.snapshot_text)), fixture=true,
        stale_policy='allow_stale', freshness={clock_trust='trusted',
          now_utc_ms=1791792000000, max_age_ms=60000}}
      ns.known = function(value)
        return {status='known', value=value, clock_trust='trusted',
          observed_at_utc_ms=1791792000000}
      end
      ns.hud = ns.Hud.new(ns.plans, function() return ns.context end, function() end)
      ns.primary = first.objective
    "#,
    );
    sandbox
}

#[test]
fn the_inspector_lists_prerequisite_reasons_and_rejected_alternatives_in_words() {
    let sandbox = replay();
    run(
        &sandbox,
        r#"
      local function text(model) return table.concat(ns.Gui.inspect(model), '\n') end
      assert(text(nil):find('No plan has been evaluated', 1, true))
      -- A restricted level is shown as restricted, never as false or zero.
      ns.document.payload.steps[1].prerequisite = {status='known',
        value={op='level_at_least', level=100}}
      assert(ns.plans:importText(ns.encode()))
      ns.context.snapshot.level = {status='restricted'}
      local uncertain = ns.hud:refresh()
      assert(uncertain.status == 'uncertain')
      local shown = text(uncertain)
      assert(shown:find('Status: uncertain (step demo-step-1)', 1, true), shown)
      assert(shown:find('Prerequisite: unknown', 1, true), shown)
      assert(shown:find('field level: unavailable / restricted', 1, true), shown)
      assert(shown:find('Invalidation: true', 1, true) == nil)
      assert(shown:find('Objective completion mapping unavailable', 1, true))
      -- A known low level blocks the primary, so a ready alternative is selected.
      ns.context.snapshot.level = ns.known(10)
      local function setObjective(id, value)
        ns.context.snapshot.objectives[id] = ns.known(value)
      end
      for _, id in ipairs({ns.primary, 'alternative-a', 'alternative-b'}) do
        setObjective(id, false)
      end
      local selected = ns.hud:refresh()
      assert(selected.status == 'ready' and selected.selected_alternative)
      shown = text(selected)
      assert(shown:find('Status: ready (step step-a)', 1, true), shown)
      assert(shown:find('An alternative objective is selected', 1, true), shown)
      -- Once the level is known and enough, the primary is back and rejected
      -- alternatives stay listed with their reasons.
      ns.context.snapshot.level = ns.known(100)
      setObjective('alternative-a', true)
      setObjective('alternative-b', true)
      local primary = ns.hud:refresh()
      assert(primary.status == 'ready' and primary.step_id == 'demo-step-1')
      shown = text(primary)
      assert(shown:find('Prerequisite: true', 1, true), shown)
      assert(shown:find('Rejected alternatives:', 1, true), shown)
      assert(shown:find('undefined-alternative: readiness_not_declared', 1, true), shown)
      -- Imported ids cannot start a client escape sequence or break the layout.
      local hostile = {status = 'ready', step_id = 'a|cffff0000b\nc', objective = 'x|r'}
      shown = text(hostile)
      assert(shown:find('a||cffff0000b c', 1, true) and shown:find('x||r', 1, true), shown)
      assert(not shown:find('|c', 1, true) or shown:find('||c', 1, true))
      -- every reason line is bounded
      local many = {status = 'blocked', prerequisite = {truth = 'false', reasons = {}}}
      for index = 1, 30 do
        many.prerequisite.reasons[index] = {input = {input = 'field',
          field = {field = 'level'}}, reason = {reason = 'stale'}}
      end
      assert(#ns.Gui.inspect(many) <= 14)
    "#,
    );
}

#[test]
fn a_stored_session_is_copied_by_slash_and_by_the_window_without_a_new_capture() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      local state = ui.state()
      state.onSessionSlash('start copy-profile')
      ui.mono = ui.mono + 2; ui.wall = ui.wall + 2
      state.onSessionSlash('sample'); state.onSessionSlash('stop')
      state.onSessionSlash('export')
      local original = AzerothNavigatorDB.session_export
      assert(original and original:sub(1, 13) == 'ANAVSESSION1:')
      local levelReads = 0
      function UnitLevel() levelReads = levelReads + 1; return 12 end
      state.onSlash('copy session')
      local view = state.copy_view
      assert(state.copy_ui_code == 'copy_shown' and view.frame.shown)
      assert(view.input:GetText() == original and view.input.max == ns.Session.MAX_BYTES)
      assert(view.title.text:find('Stored session export', 1, true))
      assert(AzerothNavigatorDB.session_export == original and levelReads == 0)
      view.close.scripts.OnClick()
      -- the window button reuses the same view
      state.actions.gui.open(); state.gui.select('session')
      local frames = #ui.frames
      assert(ui.click(state.gui.widgets.session_copy.frame))
      assert(#ui.frames == frames and view.frame.shown and levelReads == 0)
      -- a damaged stored session is refused with a reason and nothing is shown
      view.close.scripts.OnClick()
      AzerothNavigatorDB.session_export = 'ANAVSESSION1:{"format":"other"}'
      state.onSlash('copy session')
      assert(not view.frame.shown)
      assert(state.messages[#state.messages]:find('copy_export_invalid', 1, true))
      state.gui.refresh()
      assert(not ui.click(state.gui.widgets.session_copy.frame))
      state.onSlash('copy sessions')
      assert(state.messages[#state.messages]:find('usage: /anavdiag copy [snapshot|session]',
        1, true))
    "#,
    );
}

#[test]
fn reset_and_apply_resync_the_size_fields_even_when_the_saved_request_is_unchanged() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      state.actions.gui.open()
      local gui, W = state.gui, state.gui.widgets
      gui.select('hud')
      local width = W.hud_width:getText()
      -- invalid text, saved request still the default
      ui.type(W.hud_width.frame, '5000')
      assert(not ui.click(W.hud_size_apply.frame))
      assert(ui.click(W.hud_reset.frame))
      assert(W.hud_width:getText() == width, 'Reset restores the field')
      assert(ui.click(W.hud_size_apply.frame) == true, 'Apply is enabled again')
      -- typing is still never replaced by a plain refresh
      ui.type(W.hud_width.frame, '4000')
      gui.refresh()
      assert(W.hud_width:getText() == '4000')
    "#,
    );
}

#[test]
fn clock_trust_state_is_re_evaluated_on_every_refresh_and_on_open() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      state.actions.gui.open()
      local gui, W = state.gui, state.gui.widgets
      gui.select('diagnostics')
      assert(ui.click(W.diag_clock_check.frame))
      assert(state.clock.status().confirmable and W.diag_clock_trust.enabled)
      -- no timers exist; time passes and the next refresh re-evaluates the clock
      ui.mono = ui.mono + 130
      ui.wall = ui.wall + 130
      assert(not state.clock.status().confirmable)
      gui.refresh()
      assert(not W.diag_clock_trust.enabled, 'Trust is disabled after the window expires')
      assert(table.concat(ui.texts(AzerothNavigatorFrame), '\n'):find('expired', 1, true))
      -- reopening re-evaluates a fresh pending check too
      assert(ui.click(W.diag_clock_check.frame))
      assert(W.diag_clock_trust.enabled)
      gui.close()
      ui.mono = ui.mono + 130
      ui.wall = ui.wall + 130
      state.actions.gui.open()
      assert(not W.diag_clock_trust.enabled, 'open re-evaluates the expired window')
    "#,
    );
}

#[test]
fn a_failed_build_releases_the_partial_frame_and_a_retry_starts_clean() {
    let sandbox = ui_client("70374");
    run(
        &sandbox,
        r#"
      ui.login()
      local state = ui.state()
      local real, count = CreateFrame, 0
      CreateFrame = function(...)
        count = count + 1
        if count >= 4 then error('boom') end
        return real(...)
      end
      state.actions.gui.open()
      assert(state.gui_code == 'gui_ui_unavailable', tostring(state.gui_code))
      assert(AzerothNavigatorFrame == nil, 'the orphan is no longer reachable by name')
      for _, name in ipairs(UISpecialFrames) do assert(name ~= 'AzerothNavigatorFrame') end
      CreateFrame = real
      state.actions.gui.open()
      assert(state.gui_code == 'gui_shown', tostring(state.gui_code))
      assert(AzerothNavigatorFrame ~= nil)
      local seen = 0
      for _, name in ipairs(UISpecialFrames) do
        if name == 'AzerothNavigatorFrame' then seen = seen + 1 end
      end
      assert(seen == 1)
    "#,
    );
}
