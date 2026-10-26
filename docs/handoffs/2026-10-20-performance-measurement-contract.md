# Hosted performance measurement contract repair

Owner/sole writer: Codex `/root/desktop_review`. Baseline3e87ee2fff5f5f30c8fdd2fdfb0a882968c0b715,
claim76b3c2ff; immutable codeec8bc5ee703a9dd55beabd7e485eefff8fa4797c.
Isolated fix/stable-performance-measurement-contract,
`/tmp/navzeroth-stable-performance-measurement-contract`.

## Actual V8 outcome, not current acceptance

Package39424036854/job118631930831 used checkout5d9c89522241eff5285494503289c940a9849d1c,
which has the exact e5ec source tree, production52d0089987d97f0ade79ce839a06fa2a00da3370a1013ba150fab2c19f50a1f6.
Installer SHA d3b1058f07862d7f926cf5517dd993662d32a10122d0127edf8bcee07d787922.

Raw INSTALL, RESOURCES and FIRST-START report pass; PERFORMANCE reports fail.
Persistence/recovery, offline, RC1 upgrade and downgrade cases never ran. Recorder
successfully retained four case records and exact statuses. The final actual-outcome
guard correctly failed the job despite continue-on-error producer configuration.
Owner smoke remains unverified. No current canonical observation is promoted.
Only small raw/overlay artifacts were downloaded, not installer/native artifacts:
12270270379 (28893B) and12270385152 (992488B). Local copies:
`/tmp/navzeroth-v8-hosted-39424036854/{raw,overlay}`;
job log `/tmp/navzeroth-v8-package-39424036854.log`, actual failure lines5083–5088.

The raw fresh workload itself passed. Its actual issued catalog diagnostics show
8820nodes,6039projected objectives,4245quests,24359items,32365edges; browsing and
source-backed planning completed. This confirms the prior page-DTO driver repair
worked on the installed full catalog, without implying other case results.

Actual PARTIAL engineering measurements from this old exact installer:

- install8552.4756ms; content load340613ms; default activation3ms;
- browse15464.280199999921ms; planning1747.5748000000603ms;
- fresh workload elapsed994553ms;
- sampled native-app peak6366273536bytes,4884samples,0sampling errors, target200ms;
- recorded hardware Microsoft Corporation Virtual Machine, Intel Xeon Platinum8573C,
  RAM17174360064bytes; no logical-CPU count was recorded in this case.

These are retained actual values, NOT a completed HOSTED-PERFORMANCE pass. Memory
scope excludes WebView2 descendants and installer; allocations after the last
successful sample may be missed. Resource verification preceded the fresh database,
OS cache was uncontrolled/possibly warm, load includes verify/parse/stage/compose,
activation excludes database open/WebView2. No ordinary-PC or live-game claim.
No historical fail is relabelled after repairing the validator boundary.

## Cause and narrow repair

The producer supplied measurements as an OrderedDictionary with six keys. The
strict JSON-style validator inspected PSObject.Properties, which exposes seven
adapter properties for that dictionary rather than six metric properties. Existing
fixtures deserialized measurements as PSCustomObject and did not exercise live
producer construction. The real producer therefore failed at Hosted-Acceptance.ps1:54
with `Performance metric set differs` after completing the fresh workload.

The six-value construction now lives in pure New-StablePerformanceMeasurements,
used by both actual producer and real Invoke-StableCase regression. It returns a
PSCustomObject with ordered metric properties and preserves values without coercion.
No generalized dictionary admission was added. The validator additionally checks
exact case-sensitive metric names: the new LOAD_MS alias negative initially passed
the old count-only/case-insensitive lookup, then correctly refused with the narrow
name-set guard. This initial test failure is not recorded as a passing check.
Existing six-key count, numeric types, finite/range checks, date/operator/source/
production/target/installer/artifact/owner requirements and all time bounds remain.
No performance, memory or coverage threshold is relaxed.

## Actual validation

Native Windows PowerShell5.1.26100.9711 and Linux PowerShell7.6.6 each passed:
25producer helper checks plus42stable-preparation/hosted-evidence checks. The real
Invoke-StableCase regression proves original six-key OrderedDictionary failure is
retained, the shared actual constructor passes and JSON roundtrips exactly, extra/
missing/case-renamed keys refuse, all six metrics reject eight invalid type/range
values each, and upstream string values are not coerced to valid numbers. Existing
unknown/owner/preference/identity/restoration checks remain. Gitdiffcheck passed.

Logs `/tmp/navzeroth-performance-contract-{ps7,ps51}.log` are byte equal,
SHA6fa0ac4ee831d0927b31e0559c9c061f7b2300ab55657a7dfdc910be5f2a36c3.
`/tmp/navzeroth-performance-preparation-{ps7,ps51}.log` are byte equal,
SHA6f01fafcc19112b4f0eee52f19f27ea31f57734919380fa23190267b6b9a8ec8.
No installer, actual network controls, source/data/runtimeRust or canonical release
bytes, signatures, keys or acceptance statuses were changed. Mocks are explicitly
synthetic. Actual fcf full-pack proof keeps its source/production identity and scope.

Next: independent code review, separate draft repair PR into the release branch,
normal integration after immutable receipt/failure archival, renewed production
closure, and fresh exact-source hosted acceptance including all four unrun cases.
Owner smoke still requires the exact verified draft installer. No tag/publication.
