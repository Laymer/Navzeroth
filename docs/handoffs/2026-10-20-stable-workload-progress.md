# Hosted workload progress after parent timeout

Owner desktop_review; isolated baseline d271bba67e8fa274e44953c9f78198b1d8fc5f6d,
claim e6f96fe3; final code1b8a7b02 (production12132a7c).
Worktree /tmp/navzeroth-stable-workload-progress. No runtime Rust/UI, source packs,
canonical release files, keys or budget/acceptance assertions changed.

Actual V13package39466044590 source1ac633145e3f1bc39cee342c4f39a404b3adcc9f,
productiona8c292b751899d17e2942dbf708cb08d9eedc629ee2debe84fcff58a71521620,
installer56c46d57337374d79087a3f8956c586861e57242116da22509c23ca6368def78:
install/resources/first-start/performance passed; persistence-recovery failed the
selected20minute workload watchdog. Offline/RC1-upgrade/downgrade never ran. Native
regression smoke32/0 and generic0.9.2upgrade7/0 had passed independently; they are
not substitutes for the later stable cases. Fresh actual958237ms and restart
840007ms passed. Restart ended19:06:37; watchdog failed19:26:47. No recover raw report
was retained, because parent Kill(true) bypasses Node finally/report save. Exact
recovery operation remains UNLOCATED; timeout does not prove corruption or OOM.
Root retained inputs /tmp/navzeroth-v13-stable-cases and full log
/tmp/navzeroth-v13-package-39466044590.log. No duplicate artifact downloads here.

New Node progress sidecar `<report>.progress.json` atomically replaces a complete
checkpoint before/after every driver-issued IPC and named workload phase. It is
bounded16KiB with last32entries, monotonic elapsed time, current phase/command;
NO args, results, imported data or error contents. It ALWAYS says passed=false,
completed=false, kind=partial_workload_progress, including after successful cleanup.
It cannot be accepted instead of a completed workload. Interrupted temporary write
leaves the previous complete checkpoint; no raw/profile evidence is deleted.

Before the unchanged parent watchdog throw/kill, PowerShell records a separate
16KiB-bounded `<report>.watchdog.json`: selected limit, elapsed, sample/error counts,
up to16actual sampled native PID peaks and truncation flag. It always remains
nonpassing/incomplete and refers to the partial progress basename. Sampling logic,
20minute per-workload,60minute suite and all actual workload assertions remain.
Cleanup-phase diagnostic write errors are caught and mark failure while preserving
existing error and ALWAYS proceeding to browser/process/policy cleanup. No claim
is made about surviving an OS crash or cancelling already-issued IPC.

Actual checks:
-46Node tests passed across existing workload/nativehelper/boundary files. Includes
 real child SIGKILL retaining pending inspect_database_backup checkpoint, bounded
 history/no error-data leakage and interrupted-replacement failure preserving old
 complete checkpoint. The helper uses monotonic performance.now by default.
-Linux PowerShell7.6.6:26helper checks passed, including real Node child/parent2s
 timeout. Child has an actual finally marker; watchdog kills it before finally,
 yet complete progress+watchdog remain, and no final report/pass exists.
-Native Windows PowerShell5.1:25existing helper checks passed. New real parent test
 requires PS7 argument-list API and explicitly did NOT execute under5.1.
-JS driver/helper syntax and gitdiffcheck passed. Existing workflow invokes both
 modified test files, so no workflow/build/permissions changes were needed.

Logs /tmp/navzeroth-workload-progress-{node,ps7,ps51}.log. First local5.1 invocation
stopped at execution policy before tests; per-process ExecutionPolicy Bypass rerun
passed25. No persistent machine policy was changed. These are synthetic engineering
helper executions, not actual installed recovery completion or Windows acceptance.

Next independent review, routine feature draft PR, parent integration after actual
failure archival. Separate real full-pack Linux recovery benchmark must locate
runtime cost before choosing optimization; this diagnostic repair alone does NOT
resolve the observed20minute recovery failure. Missing/failed cases remain so.
