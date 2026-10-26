# Independent review: PR #286

Reviewer: Codex `/root`, independent of Claude Code's implementation.
Base: `b81e55c576df22d73f5d6ece3396a3b556376198`.
Head: `2c4435808a8baa2a7f8434b9aa4b2029e7bbb122`.

Result: clear for integration and fresh V22 candidate verification; no blocking
findings. Reviewed all five changed files and the surrounding installer, workload,
resource-verification and cleanup sequence. No application or transport changes.

The resolver uses the Roaming known folder rather than the ineffective APPDATA
environment override. Acceptance refuses pre-existing data before installation.
Workflow cleanup is confined to the disposable hosted runner's known application
directory. Before RC1, uninstall waits for completion and stops native processes;
the candidate's complete data directory moves to a new retained destination.
Existing destinations and non-directory sources refuse without removing data.
RC1 and its upgraded candidate use the same real known folder. Failure propagates
through the existing case recorder and cleanup; no observation is promoted.

Actual review checks: Node workflow tests 6/6; synthetic workflow gate 12/12;
actionlint exit 0; Linux PowerShell 7.5.4 helper checks 57/57; native Windows
PowerShell 5.1 helper checks 57/57 (process-only execution-policy bypass).
Native PS7 is running separately and will be recorded in the continuation handoff.

Limits: these are helper/workflow checks, not installed acceptance. The V21
attempt-1 FIRST-START failure has no established cause. Require a fresh hosted
run to verify clean first start, RC1 migration and all eight installed cases.
