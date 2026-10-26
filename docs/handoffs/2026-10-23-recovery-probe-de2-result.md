# Completed de2 hosted Windows engineering probe: helper failure

Owner tooling_review; baseline dee9998a17186708dc1d4711ddc43e5c389b6447,
claim37560d94. This task only archives exact evidence. No production edits,
signing, workflow retry or acceptance observation is part of this record.

Run39916898751 / job120292303530, opt-in branchverify/recovery-probe-v20-integrated,
actual source de2fd3e69bcce8db55c73d98fed492e2ee485261, completed FAILURE.
[Exact raw archive and hash inventory](../../release/history/2026-10-23-recovery-probe-de2/history.json).
The original downloaded artifact has eight files (not six); all eight are
retained without normalization, plus original ZIP, run/job/artifact API responses
and full completed job log. ZIP6112B SHA256
7e807c1c3de7cdc2ff307684da30dbb04563687d8be06898541eeae2254d8a92
matches the actual GitHub artifact API digest for artifact12486660622. Nested
Git attributes preserve the original bytes across checkout line endings.

Actual hosted Windows Python3.12.10 regressions:8 passed; inert ownership checks:6
passed; reviewed-evidence native Git transport tests:18 passed, no skipped tests.
Artifact/API/source-tree/ZIP/historical-installer/metadata/rustc phases passed.
The new optimized Rust app recovery harness COMPILED successfully in1128.766s,
exit0, binary SHA256
37c0ca08adf4728b3f05f295fc4cae16f4570171fe98adc07325078ae46ab525.
The historical fixture installer helper then FAILED in0.516s, exit1.

The actual error is Native-Smoke-Helpers.ps1:58: Get-InstalledDisplayVersion calls
Get-ChildItem on HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall,
which does not exist on this fresh runner. ErrorActionPreference Stop propagates
that absent-root error. The helper fails before issuing ownership. No valid marker
exists, so cleanup correctly was NOT attempted. No install completion, Rust
recovery workload execution, backup, inspect, restore, runtime timing or memory
measurement is established by this run. The engineering-result passed:false and
acceptance_observations_promoted:false are preserved. No current-installer/native
UI acceptance or ordinary-PC claim follows from successful compilation/tests.

The separately authorized repair must distinguish a known absent registry root
(no record) from access/provider errors (unknown/failure), preserve ambiguity and
malformed-version refusals and add inert fresh-root tests. Root and desktop own
that repair. Any subsequent probe needs an independently reviewed source and
explicit coordinator decision; this failed run is never relabelled as passed.
