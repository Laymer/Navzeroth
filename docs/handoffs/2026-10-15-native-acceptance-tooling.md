# Native acceptance tooling and first guided 70374 capture

Owner/sole writer: Codex `/root`; baseline main
`e45eed1b2bde95b9a565f144ec81d49758c05c90`, claim `ebc267d`, branch
`fix/native-acceptance-tooling`. Existing release worker `/root/acceptance`
reviewed read-only; no new agent was spawned. All findings clear; reviewer
independently passed six focused capture tests.

The current machine exposes actual Windows PowerShell 5.1.26100.9549 on Windows
NT 10.0.26300.0 through WSL. Ran all six helper suites natively against the checkout:
18 acceptance-kit, 8 first-start observer, 30 addon installer, 19 release material,
29 packaging gate and 14 candidate transport checks: **118 passed**. Git was
available in an existing Windows installation, outside the default PATH. Test
fixtures were temporary synthetic folders/processes. These results do not satisfy
interactive Windows acceptance or imply real full-content candidate readiness.

Hosted packaging had not run Test-AcceptanceKit.ps1. Added it for both Windows
PowerShell 5.1 and PowerShell 7 within the existing packaging check. No stable or
candidate gate semantics changed.

Added `inspect-capture --input <selected SavedVariables file>`: bounded, read-only
literal parsing, duplicate JSON refusal, a closed diagnostic version/status
allowlist and existing typed character snapshot validation. Output excludes raw
exports, player/account/item/quest identities, locations, paths and arbitrary
errors/details. Reports only build strings/counts and optional fixture/level/
clock/count snapshot summaries. Diagnostic and snapshot target-tuple matches are
separate; unknown/mismatched builds stay explicit. Session contents now pass the existing bounded session decoder (see follow-up). Installed-revision/live-acceptance proof fields are
always false. This command supports development evidence gathering before a full
candidate exists; final candidate acceptance cannot be bypassed. Guide:
[DEVELOPMENT_CAPTURE.md](../compatibility/DEVELOPMENT_CAPTURE.md).

The operator authorized guided checks, exited the running client and confirmed it
closed. Backed up the old addon and its one SavedVariables file privately; installed
exact main e45eed1 via its existing installer. Compared all **21 installed addon
files** with exact Git blobs, checked no extras and verified SavedVariables hashes
unchanged. This is a native addon update and preservation check, not the complete
WIN-ADDON/recovery procedure. Addon version remains 0.8.1. Raw backups, paths and
exports remain private outside Git.

After requested probe/snapshot/reload, the selected persisted file parses as
70374: version 1.60.1/interface16001; level Known30, 30 quest records, 19 equipment
slots, no objective records, uncertain clock, no session export. Diagnostic lists
16 untested/0 failed capabilities. Presence is not field verification. Operator
visual level comparison, displayed build and error report are still pending at
this record; a follow-up explained `/dump GetBuildInfo()`. Neither aggregate LIVE
rows nor source inventories have been promoted.

Actual implementation checks: all **136 CLI tests passed**, 3 existing ignored;
strict all-target CLI Clippy, Rust formatting and whitespace passed. Seven capture
regressions cover privacy/nonmutation, malformed/duplicate/executable/oversized
inputs, unsupported formats/statuses, unknown/mismatched tuples, link/parent-link
refusal and an actual committed typed snapshot fixture. An initial test assumed
that fixture level12; reading it showed22 and the assertion was corrected before
the final passing run. Initial development presence check used legacy generic
`export`; corrected to the actual `snapshot_export` producer key before review.
Linux link regressions do not constitute Windows junction runtime coverage.

The 1.0.0 release-readiness command still refuses the production version mismatch,
as expected. Complete independently evidenced target inventories/corpus, applicable
rights, content population, actual live advancement/deviation/session bindings and
full-corpus native measurements remain outstanding. No tag/release is authorized
by these partial development results.

Next: obtain the pending operator comparisons, exercise guarded clock/session and
quest/equipment comparisons without automating game inputs, record sanitized
results precisely, and address observed binding failures. Verify this PR's nine
hosted checks and independently review before protected integration. Final
acceptance still requires the complete frozen 1.0.0 candidate.


Operator follow-up: reported `/dump GetBuildInfo()` as 1.60.1/70374/Oct12 2026/
16001, visible level30, no bugs and matching values. This establishes the narrow
source-identified development probe/capture/persistence/level comparison, not
complete field or final candidate acceptance. Guided manual recorder commands
(start/two samples/stop/export/reload) are now in progress. Extended inspection to
validate present session exports via the existing decoder plus strict duplicate
JSON refusal; summary exposes only fixture/target markers, count and maximum
relative elapsed time, never active-play inference. Additional regression preserves
elapsed zero and refuses unsupported session versions. A negative draft test
incorrectly assumed sequence zero was invalid; the existing type allows zero,
so the test now rejects an actually unsupported version instead.

Manual recorder follow-up succeeded by operator report; actual persisted session
passes its contract with seven events and max relative time46,102ms. Actual
snapshot inspection distinguishes16known equipment slots from19total and zero
readable quest completion flags from30quest records. This identifies a real
quest binding gap (the current GetInfo-only adapter expects a boolean isComplete)
for the next task. Requested direct function-type observations for the separate
completion APIs; no implementation assumption or capability promotion yet.
Requested a fresh trusted snapshot before checking reload trust reset. Existing
worker independently reviewed the final session/counter delta and passed all
seven capture tests, with no findings.


Integration follow-up: see the [development evidence closure](2026-10-15-development-evidence-closure.md)
for protected merge commits, resulting-main checks, downloaded artifacts and later
operator observations. Earlier pending statements above describe their record time.
