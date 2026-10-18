//! Explicit synthetic measurement; run release mode, never a timed CI test.
mod support;
use azeroth_core::{
    error::{AppError, ErrorCode},
    search_control::*,
};
use std::{hint::black_box, time::Instant};
#[derive(Default)]
struct Counts {
    checkpoints: u64,
    cancel_at: Option<u64>,
}
impl SearchControl for Counts {
    fn checkpoint(&mut self, _phase: SearchPhase) -> azeroth_core::error::Result<()> {
        self.checkpoints += 1;
        if self.cancel_at == Some(self.checkpoints) {
            return Err(AppError::new(
                ErrorCode::OperationCanceled,
                "Search canceled",
                false,
            ));
        }
        Ok(())
    }
}
fn rss_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
fn main() {
    if cfg!(debug_assertions) {
        eprintln!("Use --release for measurements");
        std::process::exit(2);
    }
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/performance/search-v1.json")).unwrap();
    println!(
        "fixture={} version={} warmup=10 samples=100 build=release timings include candidate/request setup; fixture construction excluded",
        manifest["identity"],
        env!("CARGO_PKG_VERSION")
    );
    for (name, n, e, members) in [
        ("focused-solo", 256, 512, 1),
        ("larger-solo", 1024, 1024, 1),
        ("focused-joint-2", 256, 512, 2),
        ("focused-joint-40", 256, 512, 40),
    ] {
        let fixture = support::Fixture::new(n, e, members);
        let run = |control: &mut dyn SearchControl| -> azeroth_core::error::Result<(usize, usize)> {
            if members == 1 {
                let r = fixture.solo(128, control)?;
                Ok((r.selected.len(), r.candidates_evaluated))
            } else {
                let r = fixture.joint(128, control)?;
                Ok((r.plan.selected.len(), r.plan.candidates_evaluated))
            }
        };
        for _ in 0..10 {
            black_box(run(&mut NoopSearchControl).unwrap());
        }
        let before = rss_kib();
        let mut times = Vec::new();
        let mut checkpoints = 0;
        let mut rss_samples = Vec::new();
        let mut result = (0, 0);
        for sample in 0..100 {
            let mut control = Counts::default();
            let start = Instant::now();
            result = black_box(run(&mut control).unwrap());
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            checkpoints = control.checkpoints;
            if (sample + 1) % 10 == 0 {
                rss_samples.push(rss_kib());
            }
        }
        let samples_ms = times.clone();
        times.sort_by(f64::total_cmp);
        let mut control = Counts {
            cancel_at: Some(1000),
            ..Default::default()
        };
        let start = Instant::now();
        let canceled = run(&mut control).unwrap_err();
        assert_eq!(canceled.code, ErrorCode::OperationCanceled);
        println!(
            "{}",
            serde_json::json!({"workload":name,"nodes":n,"edges":e,"participants":members,"candidates":128,"selected":result.0,"candidates_evaluated":result.1,"samples_ms":samples_ms,"median_ms":(times[49]+times[50])/2.0,"p95_ms":times[94],"max_ms":times[99],"checkpoints":checkpoints,"cancel_after_checkpoints":control.checkpoints,"cancel_ms":start.elapsed().as_secs_f64()*1000.0,"rss_samples_each_10_runs_kib":rss_samples,"rss_before_kib":before,"rss_after_kib":rss_kib()})
        );
    }
}
