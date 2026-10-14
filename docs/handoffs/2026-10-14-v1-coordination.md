# V1 public-data and acceptance implementation

Owner/sole shared-interface writer: Codex /root. Baseline `bb3782c`, clean main.
[Claim](../claims/2026-10-14-v1-release-contract.md) `8dad2e7` precedes changes.
The user's supplied plan explicitly authorizes three implementation workers,
isolated task PRs, reviewed protected-main integration and eventual publication.
No stable publication is permitted before mandatory coverage and real acceptance.

## Implemented contract

`6db096b` introduces release-only schema version 1, the fixed nine content domains
and 25 mandatory live/Windows rows. `c6a5792` rejects unsupported schema versions
even for prereleases, bounds collection work before evaluation and uses keyed
lookups. Unknown inventory differs from known empty subunits; each mandatory
domain still requires an actual nonempty inventory. Source permissions, revision
pins, artifacts, complete unit records and tested-production identity are required.
Existing exchange/source-registry/content-pack contracts remain unchanged.

Current manifests are deliberately unverified: zero accepted sources/packs,
unknown full-content enumeration, no completed coverage/acceptance rows. They
must never be read as evidence of complete coverage. The CLI/packaging boundary
is a dependent task, not already delivered by the pure validator.

Actual checks: 12 focused core release tests pass, including valid **synthetic**
declarations, missing/failed rows, wrong build/revision, unknown versus zero,
permissions, conflicts/corrections, strict parsing, limits and prereleases. Strict
core all-target Clippy passes after correcting two `collapsible_if` findings;
fmt and whitespace pass. The first cargo invocation failed because cargo was
absent from shell PATH; subsequent checks use the installed absolute tool path.
No failed/partial invocation counts as a pass.

## Batch ownership and independent review

| Task | Writer / branch | State |
| --- | --- | --- |
| Public source inventory | public_sources / docs/v1-public-source-audit | PR119 at `7ec7429`; root independently verified all 26 complete-file identities and reviewed the audit; no blocker/important findings |
| Acceptance runbook and 70374 log | acceptance / docs/v1-acceptance-runbook | PR114 at `7d9d93c`; root source review has no blocker/important findings; all nine hosted gates pass |
| Current onboarding | onboarding / docs/v1-onboarding | PR113 at `1e923c5`; acceptance peer review has no blocker/important findings; all nine hosted gates pass |
| Shared release contract | root / feat/v1-release-contract | PR115; onboarding cleared corrected core `c6a5792`; all nine checks pass on original `7f94919`; combined candidate needs fresh checks |
| CLI/workflow enforcement | onboarding / feat/v1-release-enforcement | Separate dependent task, claim `17d73ba`; no core writer overlap |
| Copyable addon exports | acceptance / feat/addon-copy-export | PR117 at `047db62`; root reviewed production `f1a2af5`, no blocker/important findings; 115 Lua harness tests pass |
| Guarded 70374 capture | root / feat/forever-70374-capture | PR118 at `8ec4edc`, includes copy UI; acceptance reviewed production `1767eeb`, no blocker/important findings; 119 Lua harness tests pass; live behavior unverified |
| Party planning availability text | root / fix/party-planning-availability | PR116 at `0ae8806`; onboarding peer review cleared; 21 affected desktop tests pass |
| Bounded Questie staging | public_sources / feat/questiedb-staging | Separate implementation in progress; raw partial facts only, no executable corrections or accepted gameplay dataset |

All changes use isolated worktrees. The original main working tree and historical
branches, handoffs and published tags remain preserved. Each task must pass all
nine current hosted checks and independent review before integration. PR115 now
combines the foundation task branches through normal merges (combined source
tip `78e7499`) and will run all nine checks again. This preserves each task PR
and its published commits while checking one exact integration candidate. Main
remains `bb3782c` at this record; no new tag/release has been created. See the
[independent review record](../reviews/2026-10-14-v1-foundations.md).

## New evidence and discovered work

The user reported GetBuildInfo `1.60.1 - 70374 - Oct 12 2026 - 16001 - "" - " "`,
Windows 11, out-of-combat conditions, no errors from `/anavdiag probe` and `status`,
and addon version `0.8.1`. The acceptance worker preserves this narrowly in
`FOREVER_70374_EVIDENCE.md`; installed file hashes, exact Windows hardware and
serialized diagnostic output are not yet inspected. The 70338 evidence and 70205
reference data retain their original scope.

The baseline addon gates capture/clock/plan/session/HUD to 70338. It also
declares objective completion mapping unavailable, and has no snapshot copy-out
UI. These are concrete implementation gaps, even though historical finite preview
work was closed. PRs117/118 implement the copy UI and guarded 70374 admission,
preserving unknown/restricted states; changing a build string cannot establish
live facts. Objective completion mapping still lacks real verified inputs.
The source audit must not infer redistribution permission from public access or
Questie lineage. No third-party game records or personal exports are committed.

The pinned public-source audit accepts neither QuestieDB nor CMaNGOS for shipped
assets. QuestieDB's inspected public tree has no general data redistribution
license, and its documented build lineage does not prove complete 70374 content.
CMaNGOS targets historical 1.12 and includes both GPLv3 and separate restrictions
on Blizzard material. Raw ID-line counts (4,244 quests, 10,119 NPCs, 14,889 items,
6,645 objects) do not establish effective corrected records or a complete target
denominator. All 18 correction providers and derived passes remain unapplied.
These concrete limitations block content population and full-content release;
bounded staging and release enforcement can proceed independently.

Next: finish batch-1 reviews/checks, integrate normally, validate the CLI/workflow
gate and export UI, continue accepted bounded adapters, and run guided 70374
checks on identifiable candidate artifacts. Enumerate every supported coverage
unit before content population. Unknown mandatory source/coverage/model records
and missing live/Windows evidence block v1.0.0; optional API/messaging/combat-log/
hosted-delivery deferrals do not waive the mandatory local workflows.
