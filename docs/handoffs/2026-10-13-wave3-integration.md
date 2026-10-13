# Wave 3 task 10: integration and 0.3.0-rc.1 candidate

Owner: Codex /root, sole integration writer. Claimed baseline main90026ed,
branch integration/0.3.0-rc.1; /tmp/navzeroth-wave3-candidate.
Read STATUS/latest wave-3 handoffs, original plan release gates and packaging scripts.
The user authorizes ten reviewed merges and CI candidate artifacts. No tag or
GitHub release publication is included. Preserve published history and releases.

Implementation begins with independent packaging preparation; all task tips must
reach integrated main before final candidate verification. Actual checks, reviews,
merge ancestry, artifact hashes/download links and limits will be recorded here.
Live/native checks remain evidence-scoped, never inferred from mocks or CI.

## Implemented and locally verified

All eight implementation task tips are combined in this isolated worktree without
rewriting published history. Shared native command registrations/DTOs, Rust module
exports and desktop navigation were reconciled by coordinator. No exchange schema
kind/version changed. Seven workspace lock entries and six declarations agree at
0.3.0-rc.1. New feature commands stay on the existing blocking boundary.

Actual integrated checks: locked Rust fmt/Clippy,549 Rust tests passed (zero failures,
one existing ignored generator); desktop typecheck/lint/build,87 unit tests and nine
mocked Chromium flows passed.73 schema/type fixtures and33 transport fixtures passed;
contract regeneration/export produced no schema/type diff. Content validation passed
with existing explicit evidence/transform gaps; four committed LFS pointers checked.
Lua static lint and actual Rust plan/session transport through Lua and desktop loaders
passed in the integrated harness. Initial pointer check found materialized local LFS
files; only this new candidate worktree restored committed pointer bytes, preserving
original worktrees/images. PowerShell is unavailable locally; actual Windows packaging
and prerelease filename handling are verified by hosted workflow, never inferred.

Read-only cross-reviews: B coordination/routes/session import/recorder/candidate
wiring; C quest/plan/equipment; A gear services/screen and integrated addon union.
Integrated addon reviewer independently reran31 focused tests (all passed).
Important reproduced findings fixed: retained baseline equipment eligibility;
UTC trust loss after first event; false-labelled synthetic session observations;
minute refresh clearing manual gear/planner edits, including partial node associations;
and reload plan import needing conservatively labelled cached snapshot observations.
Each focused owning handoff records its regression/checks; no unresolved blocker or
important finding remains in these reviewed slices.

## Ordered hosted integration audit

| Task | PR | Dependency / review |
| --- | --- | --- |
|1 Coordination|[47](https://github.com/Laymer/Navzeroth/pull/47)|baseline449a726; B review|
|2 Quest observations|[48](https://github.com/Laymer/Navzeroth/pull/48)|47; C review|
|3 Gear services|[50](https://github.com/Laymer/Navzeroth/pull/50)|47; A review, baseline fix confirmed|
|4 Session imports|[51](https://github.com/Laymer/Navzeroth/pull/51)|47; B review including identity/clock followups|
|5 Route inspection|[49](https://github.com/Laymer/Navzeroth/pull/49)|47; B review including partial-edit followup|
|6 Plan controls/HUD|[52](https://github.com/Laymer/Navzeroth/pull/52)|48; C final cached-reload review|
|7 Gear screen|[54](https://github.com/Laymer/Navzeroth/pull/54)|50; A review|
|8 Recorder/Sessions|[55](https://github.com/Laymer/Navzeroth/pull/55)|48+51; B review, A binder contribution|
|9 Equipment/position|[53](https://github.com/Laymer/Navzeroth/pull/53)|48+52; C review|
|10 Integration/candidate|[56](https://github.com/Laymer/Navzeroth/pull/56)|all above; A addon/B packaging reviews|

PRs are marked ready and merged in this task order only after every current-head
job passes (eight CI plus Windows packaging), review findings resolved, baseline/head
SHA and prerequisite ancestry checked. Strict protected-main checks additionally
require merging current main into each next head, preserving published history;
shared conflicts are reconciled and affected checks rerun before pushing.
GitHub PRs carry commit-specific cross-review comments and actual check links.
The final commit-locked post-merge audit, exact task-tip ancestry, final main checks,
verified artifact hashes/source-tree comparison and candidate download links are
posted on PR56 after completion. This keeps the final artifact source immutable;
no extra documentation-only source mutation/tag/release is needed after packaging.

Candidate artifacts are produced by the
[Windows package workflow](https://github.com/Laymer/Navzeroth/actions/workflows/package-windows.yml):
installer, addon ZIP, checksum file, candidate notes and SOURCE_COMMIT.txt. PR packaging
may build a synthetic merge commit; compare against that recorded source or use the
post-merge main workflow for an exact main SHA. Validate downloaded hashes, ZIP contents
against git source and addon/root license. Six version gates use actual prerelease
source version on Windows. Workflow creates/releases nothing unless an explicit
version tag exists; none is created here. Main now also receives packaging checks.

## Limits and next task

Experimental70338 reads/UI have no new live evidence. Earlier reported level12 and
clock trusted/reload-age results remain narrowly scoped. Native window interaction
is unverified; hosted install smoke and command mocks remain separate evidence.
No real weights/catalogs/quests, verified quest-stage completion mapping, geometry
fabrication, joint upgrade probability, lifespan assumptions, background sampling,
party messages or calibration added. Equipment uses explicitly numbered inventory
slots and unresolved variants/instances/modifiers; no verified semantic slot map.
Plan state clears on reload; cached observations never become live observations.

Session UI may suppress an action receipt/onChanged if a periodic refresh supersedes
an in-flight mutation; storage stays atomic and the next explicit/periodic refresh
shows history. This bounded usability limit is recorded in task8 handoff. Retention
is explicit128 entries/profile without silent pruning. See the
[consolidated live checklist](../compatibility/RC1_LIVE_CHECKLIST.md).
Next task: collect sanitized client-specific evidence for this checklist and author
permitted real content; stage mappings, joint planning and calibration require their
missing data. Published v0.2.0 and earlier releases remain unchanged.

## Candidate packaging checkpoint and review follow-ups

[Pre-merge candidate downloads](https://github.com/Laymer/Navzeroth/actions/runs/38415134097/artifacts/11823887545)
were verified from source8e83a9f608518274803455b773ce6ac883f8fefd (synthetic PR merge;
head0cdffcc). All four SHA256 entries, ZIP integrity,19exact addon source files,
root/addon license and committed notes matched. Hosted Windows additionally passed
installed0.3.0-rc.1 version and bundled-license assertions. Artifact expires
2026-10-27T14:45:35Z. Final main downloads/checks/ancestry are in the commit-locked
PR56 audit after its final source is merged; this checkpoint is distinct.

An earlier interim download38413573852 passed hashes/ZIP integrity but differed
from source bytes solely by Windows CRLF conversion. The workflow now disables
conversion before checkout; the exact-byte verification above reproduced the fix.

Final recorder review reproduced a semantic error: manual stop emitted a game
logout event. Owning task8 now records recording-disabled evidence with runtime
state still stopped; running and paused stop regressions preserve measured coverage,
unclassified spans and pause gaps without asserting game logout. The pure
SessionBatch/exchange schemas remain unchanged. A second reproduced recorder issue
converted absent UTC into an array during Lua reencoding; optional null now stays
absent while false and zero survive. Worker B reviewed both fixes without remaining
important findings. The final integrated portable Rust run passed550 tests with
zero failures and one existing ignored test; Clippy, formatting and Lua lint also
passed. Earlier check counts remain historical.
