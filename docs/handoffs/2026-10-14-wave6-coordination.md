# Wave6 route and agreement batch

Owner: Codex /root coordinator. Released source baseline:
f525911ba13244cd0dbb26a78ada59fe91dd5184, v0.5.0.
Published 2026-10-14T10:42:55Z. User authorizes parallel task PRs, protected-main
merges and recurring verified releases; no further publication permission needed.

## Publication evidence

Release PR79 checked head5211591637bc236edf17a1b38f4028d17540ea0c passed all nine
applicable jobs. Final PR package38546596998 used synthetic merge
 eb17c80b2946fa244e710679180fad4b73a5bf53 with an identical source tree.
Main CI38548441102 and package38548441114 passed all nine applicable jobs at
f525911. Tag v0.5.0 peels to this exact main; tag run38549824996 passed both
Windows package and Draft GitHub release jobs.

Downloaded PR, main and tag packages each passed exact five-file asset-set,
four checksum, MZ executable header, ZIP integrity, exact19 addon source files and
root license, version and release-note byte verification. The final tag package:

| Asset | SHA256 |
| --- | --- |
| AzerothNavigator-0.5.0-x64-setup.exe | 5c09a3474f76e27e1f541e45f19fcd2b911afbeb4e9aef38e0f9bce08dfe8e8b |
| AzerothNavigator-addon-0.5.0.zip | 4c6898063c2a83924ee2c0463dcce439020e4c4244478edfcfb2706073cc80c4 |
| CANDIDATE_NOTES.md | fc2cdd594b728602147b6cc44967fccac89e10841dcfb401be4038eb0fed8f70 |
| SOURCE_COMMIT.txt | cb9042565549d6fabbfbee693592de67673dce24aa97874391c8b2207ca75729 |

https://github.com/Laymer/Navzeroth/releases/tag/v0.5.0 is published,
draft=false, prerelease=true, with exactly those four files plus SHA256SUMS.txt.
The original clean main worktree fast-forwarded from2d5de64 to f525911 after
publication. Published history and other worktrees remain preserved.
Combined source checks:677 portable Rust tests,131 desktop tests,25 mocked browser
workflows; actual checks and limits are in the v0.5.0 release handoff.

## Frozen ownership

| Task | Sole writer | Worktree / branch | Scope |
| --- | --- | --- | --- |
| M4.3 conservative clustering | /root/backlog_audit | /tmp/navzeroth-wave6-clustering; feat/objective-clustering | New pure core clustering module, module export, fixtures/tests |
| M5.4 route-derived acquisition | /root/release_review | /tmp/navzeroth-wave6-route-acquisition; feat/route-acquisition | New app feature module/tests, feature-local DTO/Panel, GearScreen |
| M4.5/M7.5 agreed recipient export | /root/atlas_dense | /tmp/navzeroth-wave6-recipient-export; feat/agreed-recipient-export | New app feature module/tests, PartyAgreement UI, Lua harness regressions |
| Coordination | /root | /tmp/navzeroth-wave6-coordination; docs/wave6-coordination | STATUS/ledger, publication evidence and ownership |
| Shared integration, after API freeze | /root | Separate claimed worktree/PR | Existing planners, app module exports, publication-guard helpers, native/client/contract/demo SDK wiring |

Each worker claims exact baseline before code, owns its task exclusively and pushes
small coherent commits in a separate draft PR. Shared root wiring follows frozen
APIs and is a separate PR. Focused independent review and all nine fresh exact-head
hosted checks precede every protected-main merge. Combined checks and exact-source
PR/main/tag asset verification precede the next minor preview release.

## Released-source audit and subsequent batches

Clustering preserves original objectives/readiness/positive utility, groups only
explicit same-node locations, retains singleton fallbacks and counts travel once.
No shared-kill savings are inferred. Joint utility is minimum participant summed
utility. Flattened original choices, disjoint memberships/rewards, bounded search
and cancellation must hold.

Route acquisition recomputes an authoritative guarded plan, binds explicitly
selected manual exact-context source nodes to directed route legs and evaluates
checked incremental detour cost within residual budget. Unknown graph costs stay
unknown; source location and independent-trial/group assumptions are declared.
Agreed export recomputes original joint choices, requires all recorded local
acceptances, exports recipient-only applicability with full cohort provenance and
rechecks snapshot/content/membership/typed freshness and bounded expiry.

Still independently ready afterward: M7.6 evidence-gap preparation, section12.1
Today/Settings, M9.1 compatibility/coverage consolidation and M10 ordinary opt-in
reference-window engineering. A fresh audit additionally found missing M3 durable
content quarantine/revocation and local submission/moderation/withdrawal foundations.
Rollback alone does not prevent bad content reactivation. These local foundations
can proceed without world expansion; acceptance cannot upgrade evidence confidence.
Actual M12.4 acceptance remains blocked by focused-release/source prerequisites;
this preview tag does not satisfy focused-gameplay acceptance. Hosting is deferred.

Real permitted quest/acquisition/role catalogs, recorded-client capabilities and
consolidated live behavior, native Windows interaction/DPI/monitor/restore checks,
party messaging, combat-log format and official Forever API evidence remain
blocked or unverified. Do not invent game facts, API absence or successful checks.
Next task: freeze feature interfaces and implement shared guarded wiring while
three independent feature workers proceed.
