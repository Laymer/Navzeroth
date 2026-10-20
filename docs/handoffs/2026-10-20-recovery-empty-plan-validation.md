# Reuse validation of an unchanged reconstructed database

Owner `/root/world_review`; claim6dd00b58, isolated
`fix/stable-recovery-empty-plan-validation` from d271bba6. Root measured a123727ms
full-pack inspection versus409ms copy in the separate ongoing benchmark; this
repair makes no elapsed-speedup or Windows recovery completion claim.

Only production change: retain the existing first complete validate(&store), read
current_plan.guard_json, and run the second complete validate only AFTER the Some
branch updates recovered provenance. With None the store has not changed since
its first validation. Typed current_plan already rejects malformed/partial tuples;
None is empty, not ignored unknown/corrupt state. Guarded plans retain both full
validation passes and recovered-reference demotion, including repeated recovery.
All source read-only/schema/migration/history/integrity/row/capacity checks, full
historical catalog/snapshot validation, destination cleanup and later application/
destructive restore-boundary validations remain unchanged. No cache/trust/schema/
budget/producer/canonical/receipt changes.

Meaningful regressions: a test-only per-thread validation counter proves no-plan
reconstruction actually executes one full scan, not zero/two. Another test imports
older and newer snapshots, corrupts ONLY the noncurrent historical hash while
current reads still succeed and plan is empty, then proves reconstruction refuses,
removes destination and preserves source bytes. Existing app populated pinned-plan
reconstruction/reconstruction-again still yields Historical, never Current.

Actual final checks:
- Storage library + recovery/local_state_recovery/local_state_foundation targets:
  51passed,0failed,0ignored across4 binaries; same instrumented targets pass.
- App recovery host unit tests:13passed,0failed; actual app local-state recovery
  integration:1passed,0failed, including pinned plan demotion and repeated recovery.
- Clippy storage/app all targets with warnings denied, formatting, diffcheck:pass.
- Combined actual instrumented storage/app recovery caller coverage for changed
  storage/src/recovery.rs:482/525 executable lines=91.8095%, unchanged per-file90%.
  Test-only counter has no production runtime overhead. No timing assertion.

Logs /tmp/navzeroth-empty-plan-{storage-final,app,host-recovery,clippy-final,
storage-coverage,app-coverage}.log. Exact combined LLVM report
/tmp/navzeroth-empty-plan-combined-coverage.json. Reused owned target, no unrelated
source/report/profile deletion. No keys, root-worktree mutation or evidence promotion.

Next independent root/desktop review, draft repair PR and normal integration; then
independent optimized full-pack recovery benchmark and genuine fresh functional/
Windows acceptance with renewed exact source/production/review/receipt bindings.
Observed V13 recovery timeout remains failed/unlocated until new actual evidence.
