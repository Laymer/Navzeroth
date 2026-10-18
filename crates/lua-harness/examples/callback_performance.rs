//! Repository-owned synthetic HUD refresh bursts; no live client API measurement.
#[path = "../tests/common/mod.rs"]
mod common;
use std::time::Instant;
fn main() {
    if cfg!(debug_assertions) {
        eprintln!("Use --release for measurements");
        std::process::exit(2);
    }
    let sandbox = common::strict_core();
    sandbox
        .ns()
        .set(
            "plan_text",
            include_str!("../../../fixtures/contracts/v1/route-plan.json"),
        )
        .unwrap();
    sandbox
        .ns()
        .set(
            "snapshot_text",
            include_str!("../../../fixtures/contracts/v1/character-snapshot.json"),
        )
        .unwrap();
    sandbox
        .run::<()>(
            "measurement-setup",
            r#"
  local envelope=assert(ns.Decode.decode(ns.plan_text))
  ns.plans=ns.Plan.new({context=envelope.payload.context,fixture=true})
  assert(ns.plans:importText(ns.plan_text))
  local envelopeSnapshot=assert(ns.Decode.decode(ns.snapshot_text))
  ns.context={snapshot=envelopeSnapshot.payload,fixture=true,
    freshness={now_utc_ms=1791792000000,clock_trust='trusted',max_age_ms=120000}}
  ns.hud=ns.Hud.new(ns.plans,function() return ns.context end,function() end)
 "#,
        )
        .unwrap();
    let burst = || {
        sandbox
            .run::<()>("synthetic-hud-burst", "for i=1,100 do ns.hud:refresh() end")
            .unwrap()
    };
    for _ in 0..10 {
        burst();
    }
    sandbox.lua().gc_collect().unwrap();
    let before = sandbox.lua().used_memory();
    let mut samples = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        burst();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    sandbox.lua().gc_collect().unwrap();
    let raw_samples = samples.clone();
    samples.sort_by(f64::total_cmp);
    println!(
        "synthetic-hud-refresh-burst-v1 build=release warmup=10 samples=100 callbacks_per_sample=100 median_ms={:.6} p95_ms={:.6} max_ms={:.6} lua_retained_before_bytes={} lua_retained_after_bytes={} (after explicit GC; no native/client event claims)",
        (samples[49] + samples[50]) / 2.0,
        samples[94],
        samples[99],
        before,
        sandbox.lua().used_memory()
    );
    println!(
        "{}",
        serde_json::json!({"format_version":1,"fixture":"synthetic-hud-refresh-burst-v1","synthetic":true,"warmup":10,"samples":100,"callbacks_per_sample":100,"samples_ms":raw_samples,"min_ms":samples[0],"median_ms":(samples[49]+samples[50])/2.0,"p95_ms":samples[94],"max_ms":samples[99],"lua_retained_before_bytes":before,"lua_retained_after_bytes":sandbox.lua().used_memory(),"explicit_gc":true,"live_client_verified":false})
    );
}
