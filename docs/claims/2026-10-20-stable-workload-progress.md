# Bounded durable hosted workload progress

Owner desktop_review. Baseline d271bba67e8fa274e44953c9f78198b1d8fc5f6d.
Sole writer /tmp/navzeroth-stable-workload-progress branchfix/stable-workload-progress.
Scope Windows workload driver/progress helper/parent timeout diagnostics and tests;
workflow invocation only as needed, focused claim/handoff. No runtime/source/pack/
canonical/key changes. Preserve all assertions and20minute/60minute limits.

Actual V13package39466044590: four hosted cases passed; persistence-recovery failed
its per-workload20minute watchdog. Fresh958237ms and restart840007ms passed; recovery
report absent because parent killed Node before finally. Exact recovery operation
is UNLOCATED. Add bounded partial/nonpassing checkpoints before/after issued IPC,
phase and parent deadline/sampling evidence so forced kill retains useful cause.
Runtime optimization diagnosis belongs to separate benchmark, not this repair.
