//! Explicit adapter admission is synthetic engineering evidence, never live verification.
mod common;

#[test]
fn only_reported_exact_tuples_admit_experimental_capture() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "target-allowlist",
            r#"
      local check=ns.Capabilities.isCaptureTarget
      for _, build in ipairs({'70338','70374'}) do
        assert(check({version='1.60.1',build=build,interface='16001'}))
      end
      for _, build in ipairs({'70205','70339','70375','unknown','70374 '}) do
        assert(not check({version='1.60.1',build=build,interface='16001'}))
      end
      assert(not check({version='1.60.2',build='70374',interface='16001'}))
      assert(not check({version='1.60.1',build='70374',interface='16002'}))
      assert(not check({version='1.60.1',build='70374',interface=16001}))
      assert(not check(nil) and not check(false) and not check({}))
      assert(not check(setmetatable({}, {__index=function() error('unreadable') end})))
    "#,
        )
        .unwrap();
}

#[test]
fn new_target_preserves_combat_and_unknown_hud_gates() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "target-hud",
            r#"
      local build={version='1.60.1',build='70374',interface='16001'}
      local gate=ns.HudAppearance.gate
      local allowed,code=gate(build,function() return false end)
      assert(allowed and code=='hud_gate_allowed')
      allowed,code=gate(build,function() return true end)
      assert(not allowed and code=='hud_combat_deferred')
      for _, getter in ipairs({function() return nil end,function() return 0 end,
          function() error('restricted') end}) do
        allowed,code=gate(build,getter)
        assert(not allowed and code=='hud_combat_unknown')
      end
      allowed,code=gate(build,nil)
      assert(not allowed and code=='hud_combat_unknown')
    "#,
        )
        .unwrap();
}

#[test]
fn a_supported_target_does_not_promote_probe_capabilities() {
    let sandbox = common::stub_client("1.60.1", "70374", 16001);
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "target-probe",
            r#"
      local report=ns.Capabilities.probe(_G)
      assert(ns.Capabilities.isCaptureTarget(report.build))
      for _, capability in ipairs(report.capabilities) do
        assert(capability.status=='untested')
      end
    "#,
        )
        .unwrap();
}
