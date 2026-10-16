//! Clock trust is operator-confirmed; all callbacks here are synthetic.
mod common;
use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::UtcMillis;

#[test]
fn utc_is_uncertain_until_confirmed_and_jumps_resets_failures_revoke_trust() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "clock_replay",
            r#"
      local wall, mono = 1791792000, 100
      local clock = ns.Clock.new(function() return wall end, function() return mono end)
      assert(clock:capture().clock_trust == 'uncertain')
      assert(not clock:confirm())
      assert(clock:probe().utc_seconds == wall)
      wall, mono = wall + 1, mono + 1
      assert(clock:confirm().clock_trust == 'trusted')
      wall, mono = wall + 10, mono + 10
      local value = clock:capture()
      assert(value.clock_trust == 'trusted' and value.observed_at_utc_ms == wall * 1000)
      wall = wall + 100
      local code
      value, code = clock:capture()
      assert(value.clock_trust == 'uncertain' and code == 'clock_changed')
      assert(clock:capture().clock_trust == 'uncertain')
      assert(clock:probe()); assert(clock:confirm())
      mono = mono - 1
      assert(clock:capture().clock_trust == 'uncertain')
      mono = 200
      assert(clock:probe()); assert(clock:confirm())
      wall = wall - 1
      assert(clock:capture().clock_trust == 'uncertain')
      assert(clock:probe())
      wall, mono = wall + 121, mono + 121
      assert(not clock:confirm())
      assert(clock:probe()); assert(clock:confirm())
      wall = false
      value = clock:capture()
      assert(value.clock_trust == 'uncertain' and value.observed_at_utc_ms == nil)
      wall, mono = 1791793000, 300
      assert(clock:probe()); assert(clock:confirm())
      clock:clear(); assert(clock:capture().clock_trust == 'uncertain')
      local reloaded = ns.Clock.new(function() return wall end, function() return mono end)
      assert(reloaded:capture().clock_trust == 'uncertain')
      local noApi = ns.Clock.new(nil, nil)
      assert(not noApi:probe())
      local failed = ns.Clock.new(function() error('private error') end, function() return mono end)
      assert(failed:capture().observed_at_utc_ms == nil)
      local zero = ns.Clock.new(function() return 0 end, function() return 0 end)
      assert(zero:probe().utc_seconds == 0)
      assert(zero:confirm()); assert(zero:capture().observed_at_utc_ms == 0)
    "#,
        )
        .unwrap();
}

#[test]
fn slow_samples_fractional_utc_overflow_and_gradual_drift_are_rejected() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "bounds",
            r#"
      for _, wall in ipairs({-1, 0.5, '1791792000', 9007199254741, math.huge}) do
        local clock = ns.Clock.new(function() return wall end, function() return 100 end)
        assert(not clock:probe())
      end
      local ticks = 0
      local slow = ns.Clock.new(function() return 1791792000 end, function()
        ticks=ticks+3; return ticks
      end)
      assert(not slow:probe())
      local wall, mono = 1791792000, 100
      local clock = ns.Clock.new(function() return wall end, function() return mono end)
      assert(clock:probe()); assert(clock:confirm())
      for _=1,5 do
        wall, mono = wall+11, mono+10
        assert(clock:capture().clock_trust == 'trusted')
      end
      wall, mono = wall+11, mono+10
      assert(clock:capture().clock_trust == 'uncertain')
    "#,
        )
        .unwrap();
}

#[test]
fn confirmed_capture_is_fresh_in_desktop_and_reload_requires_new_confirmation() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>(
            "client",
            r#"
      wall, mono = 1791792000, 100
      function GetServerTime() return wall end
      function GetTime() return mono end
      function UnitLevel() return 12 end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let uncertain: String = sandbox
        .run(
            "unconfirmed",
            r#"
      local state = ns.Diagnostics.state
      state.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      state.onSlash('snapshot clock-profile')
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let app = SnapshotApplication::in_memory().unwrap();
    let now = UtcMillis::new(1791792001000).unwrap();
    let receipt = app.import_text(&uncertain, Some(now)).unwrap();
    assert_eq!(
        serde_json::to_value(receipt.freshness).unwrap()["status"],
        "uncertain"
    );
    let trusted: String = sandbox
        .run(
            "confirmed",
            r#"
      local state = ns.Diagnostics.state
      state.onSlash('clock')
      state.onSlash('clock trust')
      state.onSlash('snapshot clock-profile')
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let receipt = app.import_text(&trusted, Some(now)).unwrap();
    assert_eq!(
        serde_json::to_value(receipt.freshness).unwrap()["status"],
        "fresh"
    );
    let value: serde_json::Value = serde_json::from_str(&trusted[6..]).unwrap();
    assert_eq!(value["payload"]["level"]["clock_trust"], "trusted");
    assert!(
        value["payload"]["capture"]
            .get("session_elapsed_ms")
            .is_none()
    );
    let receipt = app
        .import_text(&trusted, Some(UtcMillis::new(1791793900000).unwrap()))
        .unwrap();
    assert_eq!(
        serde_json::to_value(receipt.freshness).unwrap()["status"],
        "stale"
    );
    let receipt = app
        .import_text(&trusted, Some(UtcMillis::new(1791791999000).unwrap()))
        .unwrap();
    assert_eq!(
        serde_json::to_value(receipt.freshness).unwrap()["status"],
        "uncertain"
    );
    sandbox
        .run::<()>(
            "reload",
            r#"
      local newState = ns.Diagnostics.attach(ns.Diagnostics.state.env)
      newState.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      newState.onSlash('snapshot clock-profile')
      local envelope = assert(ns.Decode.decode(AzerothNavigatorDB.snapshot_export:sub(7)))
      assert(envelope.payload.capture.clock_trust == 'uncertain')
      assert(AzerothNavigatorDB.snapshot_sequence == 3)
    "#,
        )
        .unwrap();
}

#[test]
fn status_is_read_only_and_reports_pending_age_and_trust_without_changing_either() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "status",
            r#"
      local wall, mono = 1791792000, 100
      local clock = ns.Clock.new(function() return wall end, function() return mono end)
      local s = clock:status()
      assert(s.available and not s.pending and not s.trusted and not s.confirmable)
      assert(s.age_seconds == nil and s.pending_utc_seconds == nil)
      assert(clock:probe())
      wall, mono = wall + 30, mono + 30
      for _ = 1, 3 do -- repeated reads change nothing
        s = clock:status()
        assert(s.pending and s.age_seconds == 30 and s.confirmable and not s.trusted)
        assert(s.pending_utc_seconds == 1791792000)
      end
      wall, mono = wall + 91, mono + 91 -- 121 seconds old: pending, no longer confirmable
      s = clock:status()
      assert(s.pending and s.age_seconds == 121 and not s.confirmable)
      wall, mono = 1791792030, 130
      assert(clock:status().confirmable)
      assert(clock:confirm().clock_trust == 'trusted')
      s = clock:status()
      assert(s.trusted and not s.pending and not s.confirmable)
      -- a jump hides trust in the read-only view without clearing it; undoing it restores
      wall = wall + 100
      assert(not clock:status().trusted)
      wall = wall - 100
      assert(clock:status().trusted)
      assert(clock:capture().clock_trust == 'trusted')
      -- capture() is the only call that revokes trust on a jump
      wall = wall + 100
      assert(clock:status().trusted == false)
      assert(clock:capture().clock_trust == 'uncertain')
      wall = wall - 100
      assert(clock:status().trusted == false)
      -- an incoherent pending probe is not confirmable and has no age
      assert(clock:probe())
      mono = mono - 5
      s = clock:status()
      assert(s.pending and s.age_seconds == nil and not s.confirmable)
      -- unavailable sources are reported, not guessed
      s = ns.Clock.new(nil, nil):status()
      assert(not s.available and not s.pending and not s.trusted)
      s = ns.Clock.new(function() error('private error') end, function() return 1 end):status()
      assert(not s.available)
      -- zero is a reading, not a missing value
      local zero = ns.Clock.new(function() return 0 end, function() return 0 end)
      assert(zero:probe())
      s = zero:status()
      assert(s.available and s.pending and s.pending_utc_seconds == 0 and s.age_seconds == 0)
      assert(s.confirmable)
      zero:clear()
      assert(not zero:status().pending)
    "#,
        )
        .unwrap();
}
