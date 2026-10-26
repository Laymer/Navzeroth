# V13 hosted recovery timeout

Owner `/root`; archive claim832786d4, baseline d271bba67e8fa274e44953c9f78198b1d8fc5f6d.
Actual PR checkout1ac633145e3f1bc39cee342c4f39a404b3adcc9f has the same tree.

CI39466044577 passed all nine applicable jobs (stability skipped). Both actual
whole-corpus source platforms and parity39466044598 passed. Coverage39466044635
passed all276 measured files at exact90/100, with clean native receipts and positive
CLI/Windows-desktop execution. Pages39466044520 validated; deployment was skipped.
These successes remain distinct from the failed installer acceptance gate.

Package39466044590 failed its final gate at19:31UTC. Its native gate passed32 smoke
checks and seven upgrade checks, including exact synthetic content preservation.
Installer26129327bytes SHA256
56c46d57337374d79087a3f8956c586861e57242116da22509c23ca6368def78
was verified unsigned and bound to1ac633. The actual hosted producer failed even
though GitHub's continue-on-error step display says success; the final guard failed.

Actual hosted results:
- Installation, installed resources/notices, first start and performance passed.
- Persistence/recovery failed: the recover workload exceeded its20-minute limit.
- Offline, RC1 upgrade and downgrade-refusal cases never ran.
- Owner smoke remains unverified.

Standalone restart passed in840007ms, preserving settings, the selected content
revision and false inferred-travel choice. The subsequent recover process started
after restart finished at19:06:37 and hit its limit at19:26:47. No recovery JSON
survived the parent process-tree kill. The exact operation is unlocated; there is
no evidence establishing a specific hang, out-of-memory condition or successful
backup/restore. The parent kill bypasses the Node finally/report write.

Actual performance, tied only to this historical installer:
- Installation7829.4323ms; content load382553ms; default activation6ms.
- Browse16737.0761ms; planning1797.27ms.
- Observed native PID5620 PeakWorkingSet64:6369615872bytes;4694 samples at a
  target200ms interval, zero sampling errors.
- Microsoft hosted VM, AMD EPYC7763, RAM17174360064bytes. WebView2 descendants and
  installer are excluded; OS cache was uncontrolled and possibly warm. These are
  engineering measurements, with no ordinary-PC or live-game claim.

Raw5cases, standalone fresh/restart/state, exact acceptance overlay and selected
native reports are byte-preserved under release/history/2026-10-20-v13-recovery-timeout.
Signed V13 identities and successful coverage evidence are separately archived.
Archive7472db34 preceded repair integration; all24 FileRefs and committed Git blobs
were checked byte-for-byte after commit. Tooling alone streamed selected compressed
LCOV; root independently checked retained raw verdict/receipts and derived summaries.
World independently audited all five case identities/hashes and overlay preservation.

Desktop is the sole writer of a narrow bounded workload-progress repair at isolated
baseline d271, claim e6f96fe3. It must retain diagnostic phases/issued IPC and parent
watchdog measurements before process termination, without arguments or imported data.
Tooling independently owns a full-pack Linux recovery benchmark at baseline d271,
claim fdc0c520. It will time actual startup, backup, inspection, restore, checks and
reopen, preserving strict state assertions and a finite overall execution bound.
No runtime optimization is selected until there is measured evidence. Existing
20-minute workload,60-minute suite and170-minute job bounds remain unchanged.

Current canonical nine observations remain pristine/unverified; the actual overlay
is historical evidence. Main remains807ef674, PR239 is draft, generated PR243 is
deferred. No stable tag, owner pass, publication or website deployment exists.
Next: diagnostic repair review and measured recovery analysis, then exact renewed
source/review/receipts and fresh applicable hosted gates. Failures hold publication.

## Diagnostic review continuation

Draft repair PR261 is frozen at1b89bd212604defabf94ef3b48b9fe0f5998521b,
code1b8a7b0275f0d3d52f5e6b0bbef2c1025b293377, production12132a7c.
World independently cleared the bounded checkpoint/watchdog and cleanup behavior,
with46Node and26PS7 checks. Root independently ran the complete five-file workflow
Node command (54passed, zero skipped/failed) and26PS7 helper checks, including an
actual parent timeout retaining nonpassing progress while bypassing child finally.
Logs: /tmp/navzeroth-root-progress-node.log and
/tmp/navzeroth-root-progress-ps7.log. No root nativePS5 execution is claimed.
This repair preserves diagnostics only; recovery remains failed/unlocated on Windows.
It has not yet been integrated or promoted into a newly signed release candidate.

The separate Linux full-pack diagnostic is still running. Actual backup copying
completed409ms and inspection123727ms. Restore and subsequent checks/reopen remain
pending at this entry. These are diagnostic timings, not a successful complete run
or Windows acceptance. Read-only review is evaluating duplicate full validation of
an unchanged no-plan reconstruction; destructive-boundary revalidation must remain.
