# Bounded hosted Windows recovery engineering probe

Owner tooling_review. Baseline b601bb8f9e2b4323cf8fc30036e88a4b1708bcda,
including reviewed recovery repair2659d8c2 / PR275 final749977a0.
Isolated branch verify/hosted-recovery-probe, worktree /tmp/navzeroth-hosted-recovery-probe.
Own a new opt-in workflow, ignored Rust full-pack engineering harness, bounded
probe wrapper/fixtures/tests and handoff. Existing package/download workflows and
runtime are other owners' files. Preserve 1200-second whole workload including
startup, strict backup/inspect/restore/state/integrity/reopen. Explicit historical
installer/resources/source metadata are engineering fixtures, not a new installer
acceptance or release result. No dispatch/push-trigger until independent review.
