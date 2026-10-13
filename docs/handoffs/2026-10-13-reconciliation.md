# Reconciliation, review fixes and planner delivery

Owner: Codex root, sole writer; claimed 2026-10-13.
Baseline: origin/main 6bb2649; integration/reconciliation-planner in
/tmp/navzeroth-reconciliation. User authorizes two green reviewed PR merges.
Inventory: 2026-10-13-reconciliation-inventory.json records every local/remote
branch tip, worktree and stash before implementation. All other worktrees clean.
Existing five-file planner patch preserved as f6b26d1 on its original branch.
Outstanding stack: 29c3188, 2b96b03, ccb2751, 6e562f3; outstanding documentation:
origin/feature/M2.2-ingest bd78d29 and origin/feature/M6.1-atlas-screen fde5da4.
Read STATUS at both baselines, planner/content handoffs and plan 9.5–9.8/M4.
Scope: preserve histories; revalidate/fix important findings; finish M4.4, then
M4.5 separately. Native interaction/live client evidence remains unavailable.


## Revalidation and changes

At the combined e7fe53c code, the review's domain findings remained present:
gear classify checked only main+off pairs; the recommendation adapter omitted
certainty; eligibility collapsed unknown siblings/partial history/reset state;
facts omitted identity; reminder default used snapshot IDs and UTC-only cooldown;
SessionKey omitted context; normalized records used EvidenceId alone; cycle edges
came only from prerequisites; SQLite lacked promotion. The desktop command key
finding was already fixed by snake_case Tauri commands. The refuted travel finding
remains refuted; no travel-readiness relaxation was made.

Focused fixes now cover all relevant hand placements and complete-only public
scores; unknown catch-up, partial history, reset status and exact participant facts;
outcome dedup with optional session-relative cooldown and legacy history decoding;
context-qualified summaries; shared evidence keyed by record/evidence with content
consistency checks; eligibility and objective-stage cycle edges with ambiguous
owners/conditional edges reported as gaps. Original review records retain reviewed
revisions and conclusions, including conflicts between their historical summaries.

SQLite v4 adds a global newest-256 promotion audit. Profile-scoped history reads
revalidate documents. Promotion takes expected current ID and reason, validates
under an immediate transaction, rejects provably older or foreign captures and
commits selection plus audit atomically. Tests cover watcher races, audit failure
rollback, same-position/incomparable selection, pruning and v2/v3 populated upgrades.
Automatic imports, hashes, preferences, parties and content stay intact.

M4.4 supplies blocking Tauri catalog/plan commands, explicit manual candidate
estimates, utility/version label, origin/stopping nodes, budget and exclusions.
1 MiB request/128 candidate and exclusion/32 selected-step limits are retained.
Exact profile/current snapshot/active revision and fresh trusted capture are required.
Quest stages require declared membership and known accepted, incomplete, not-turned-in
state and known incomplete objective state. Planning never simulates acceptance or
unlocks. Unsupported acquisitions and joint participants are filtered with reasons.
The UI invalidates results on edits/refresh and ignores superseded responses.
Synthetic planning content is available explicitly from Content; it is not real data.

## Checks before first PR

Locked workspace tests excluding native desktop: 487 passed, zero failures, one
existing ignored generator (before the additional populated-v3 upgrade regression).
Locked workspace Clippy and fmt passed. Desktop typecheck/lint/build and 78 unit
tests passed; seven Chromium command-mock workflows passed. Schema drift test and
TypeScript regeneration are unchanged; 73 contract and 33 transport fixtures passed;
reference content validation passed; four committed imagery pointer blobs checked.
Lua parity/functional suites passed in workspace tests. Local luacheck is unavailable;
its hosted job is required for this delivery. Native Linux command compilation remains
limited by development libraries; hosted Windows compilation will be checked.
No native interaction or live WoW claims. Next: first PR CI/merge, then M4.5.


## First protected merge and inventory audit

PR #40 merged b816477 as main 301dc606c8323fa4f5115e7f408fcd647fecf193.
All eight jobs passed on final PR revision in run 38364972389, including native
Windows compilation and Lua static checks. Reviewed transactions, command key
bindings, exact profile/revision guards, unknown-state paths and UI response
invalidation before merging. A full-SHA match guarded the merge.

Every initial ref below with outstanding commits is an ancestor of 301dc60;
original histories remain reachable. The preserved planner checkpoint f6b26d1
is also an ancestor. Other initially inventoried tips already belonged to main.

| Initial outstanding ref | Tip | Reachable from main 301dc60 |
| --- | --- | --- |
| feature/M3.6-content-controls | 6e562f3907ad | yes |
| feature/M4.4-planner-workflow | 6e562f3907ad | yes |
| feature/M7.3-party-eligibility | ccb275192e7a | yes |
| feature/M7.3-party-persistence | 29c31885633d | yes |
| feature/M7.3-party-screen | 2b96b03bc442 | yes |
| origin/feature/M2.2-ingest | bd78d29e1b2e | yes |
| origin/feature/M3.6-content-controls | 6e562f3907ad | yes |
| origin/feature/M6.1-atlas-screen | fde5da41c8ba | yes |
| origin/feature/M7.3-party-eligibility | ccb275192e7a | yes |
| origin/feature/M7.3-party-persistence | 29c31885633d | yes |
| origin/feature/M7.3-party-screen | 2b96b03bc442 | yes |

Refresh discovered origin/feature/M9.4-windows-packaging, created after the initial
inventory. This concurrent work is outside the reconciliation scope and remains
untouched. All original branches/worktrees are retained; no stash or history was
rewritten. M4.5 continues on a separate feature branch from b816477.
