# Remaining Windows packaging helper audit

Owner `/root/tooling_review`; checked exact source
225db8dbf7c0b791acfc249f030deddd68129b92 in a separate owned worktree. Production
files remain unchanged. This supplements the independently owned AcceptanceKit
repair after package run39398642678 failed on its legacy fixture consuming the
current28MiB schema4 inventory through a generic16MiB reader.

Read-only inspection of `.github/workflows/package-windows.yml` after the two
AcceptanceKit steps found no additional mutable-current-inventory coupling:
NativeSmokeHelpers uses temporary synthetic files/processes; ReleasePolicy reads
the intentionally small current artifact policy and creates malformed policy
fixtures in a temporary directory; Signing uses temporary synthetic PE files.
The later Rust release_materials fixtures already read frozen historical inventory
and evidence from release/history/2026-10-19-pre-stable-v3. The exact five Node
helper suites create synthetic fixtures or inspect workflow guards. No additional
hardcoded19-row/ten-Windows expectation was found in these subsequent helpers.

Actual native Windows PowerShell5.1 checks, all exit0:

- Test-NativeSmokeHelpers:10 checks.
- Test-ReleasePolicy:17 checks.
- Test-Signing:11 checks.
- Test-FirstStartSmoke:8 synthetic harness checks, including delayed initialization
  and retained survival/dwell requirements. This does not launch the product.

Their raw outputs were retained under
`/tmp/navzeroth-packaging-helper-audit-logs/Test-<name>.log` (full filenames match
the four scripts above). Tests were rerun solely to retain raw outputs from the
previous read-only audit. The five exact workflow Node files passed40tests:
`stable-workload-lib`, `native-smoke-lib`, `native-command-boundaries`,
`verify-installed-tree` and `package-windows-workflow`; raw output
`/tmp/navzeroth-tooling-packaging-helper-node.log`. No PowerShell7/native product
or full installer acceptance result is claimed.

Optional historical-fixture limitation: Test-ReleaseContractBounds -ActualRoot
hardcodes old acceptance evidence SHA61c3f4ce220b93042dc039d83553bc0ad5073a7807b4ee33d2dca7ab4c2c91a8.
Current signed-bundle binding changes that contract hash, so that optional
actual-root assertion needs explicit maintenance when used. The normal packaging
workflow invokes the synthetic suite without -ActualRoot; this is not another
current packaging blocker. Reported to the desktop writer and release owner;
no fixture or hash was loosened here.

Latest weekly draft check snapshot is recorded in the adjacent evidence JSON.
All four dispatches target exact draft243 head
c23dd2b5e1e8566df519ac58f921395c2f2c768e. Source39398615432 completed success;
other results were still in progress at recording, never passed by inference.
The generated draft remains deferred; no new acquisition, dispatch or merge was
performed in this documentation task. Earlier weekly outcome details remain in
873c72451cb91ca1eab92b54155daed02221b842. Next: integrate the separately reviewed
AcceptanceKit repair, retain final helper/hosted results and continue release gates.
