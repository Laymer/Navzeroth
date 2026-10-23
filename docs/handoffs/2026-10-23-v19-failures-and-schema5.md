# V19 actual failures and schema5 release continuation

Owner root, claim2026-10-23-v19-evidence-and-release-integration. Baseline9cc774e4.
The existing untracked 2026-10-22 claim is preserved; its offline-hang diagnosis
and assertion that script edits do not change production identity are superseded.

Run39641767585 attempt1/job119366846246 was cancelled during responsive offline
startup. Five earlier hosted cases passed; a terminal offline hang is not proven.
Attempt2/job119662619835 failed at1200725.9954ms recovery bound. Four hosted cases
passed; offline/RC1 upgrade/downgrade never ran. Full database check completed
before the watchdog killed subsequent content/planning work. No partial recovery
result is accepted. CI39641767650, coverage39641767719 and full source/parity
39641767689 passed at9cc; stability skipped and cannot qualify for future main.

Both attempt-specific raw sets, jobs, logs and three exact signed V19 files are
archivedcceeac9e; root verified all35FileRefs and Git blobs. Canonical identities
and nine observation rows remain unchanged. PR239 remains draft/main unchanged.

User approved additive schema5 with eight hosted rows plus an independently
reviewed automated exact-downloaded-draft UI/native Windows smoke. Historical
schema4 OWNER semantics remain unchanged; no human test is fabricated. Existing
20min workload/60min suite/per-file90percent gates remain. Target70450, unsigned
installer, historical70245 warning and exclusions remain. Windows11/0.8.1 upgrade
is supplemental; back up existing installation/data and obtain user-provided
elevation for the concrete launcher when ready, never disable local networking.

Separate writers: desktop recovery/progress, tooling core/CLI/schema/preparation,
world ADR/build-watch docs then new UI driver, root PS/transports/workflow. The
initial populated-plan optimization was rejected BEFORE any runtime edit: actual
hosted workflow never pins a current plan and retained fullpack DBs have NULLguard.
Now measure the actual empty-plan validation/reconstruction path; do not claim an
unexecuted branch fixes the timeout or waive post-restore planning assertions.

PR243 closed as superseded with semantic explanation; issue270 acknowledgement
and closure follows committed backlog documentation. No source data regeneration,
new build admission, tag, publication or website deployment has occurred. Next:
measured independently reviewed repairs, schema5 transport/UI tests, renew
functional/source/review/receipts, one fresh frozen candidate, all required gates,
exactmain stability, verified draft automation, publication and public/site checks.
