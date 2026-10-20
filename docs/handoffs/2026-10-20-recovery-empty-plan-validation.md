# Reuse validation of an unchanged reconstructed database

Owner `/root/world_review`; claim6dd00b58, isolated
`fix/stable-recovery-empty-plan-validation` from d271bba6. Tooling executed the baseline diagnostic (inspection123727ms versus copy409ms);
root independently reviewed its raw report. Baseline diagnostic now completed
PASS721.39s. This repair makes no elapsed-speedup or Windows recovery completion
claim; optimized repeat remains separate actual work.

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
- Actual affected-file PRODUCTION coverage uses existing inventory.rustTestRanges
  and gate.lcovLines on only the exact current source path:414/449=92.2049%,
  unchanged exact90% threshold. Both numerator/denominator exclude cfg(test) ranges
  312–315,319–320,578–624. Older cached travel-worktree paths are not current evidence.
  Raw unique LCOV is448/483; initial LLVM JSON482/525 was a raw summary including
  tests and is NOT the production gate metric. Test counter has no production
  runtime overhead; no timing assertion or full/native gate pass is claimed.

Logs /tmp/navzeroth-empty-plan-{storage-final,app,host-recovery,clippy-final,
storage-coverage,app-coverage}.log. Exact combined LLVM report
/tmp/navzeroth-empty-plan-combined-coverage.json. Exact production derivation
/tmp/navzeroth-empty-plan-production-coverage.json and report-only LCOV
/tmp/navzeroth-empty-plan-combined.lcov retain source hash/current-file scope.
Reused owned target, no unrelated source/report/profile deletion. No keys,
root-worktree mutation or evidence promotion.

Next independent root/desktop review, draft repair PR and normal integration; then
independent optimized full-pack recovery benchmark and genuine fresh functional/
Windows acceptance with renewed exact source/production/review/receipt bindings.
Observed V13 recovery timeout remains failed/unlocated until new actual evidence.

Independent coordinator review of frozen8730 production diff CLEAR; coordinator
actually executed six matched author binaries:35storage unit+7recovery+3storage
local+6foundation+13host+1app local, all pass. These are independent executions,
not an independent compiler build. Logs /tmp/navzeroth-root-empty-plan-{storage,
recovery,storage-local,foundation,host,app-local}.log. Tooling review remains pending.
Draft PR262 targets release/stable-1.0.0-version; no release-branch push or merge
was performed by this writer.
