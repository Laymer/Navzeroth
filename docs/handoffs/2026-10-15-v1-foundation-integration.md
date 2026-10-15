# 1.0 foundation integration and external prerequisites

Owner: Codex `/root`; implementation baseline `d59aa91`; this documentation task
claimed at `5146a32`. The user authorized one coordinator and three workers,
isolated worktrees, protected-main integration after nine hosted checks, and the
later gated release. Version 0.8.1 remains the published preview.

## Source decision

The user identified wowhead.com after clarifying that permission was only verbal
and that no dataset location or inventory was known. No grant or source content
has been admitted. The source registry remains empty. The permission request in
[the unsent draft](../sources/permission-requests/wowhead-unsent.md) has not been
sent. The registry excludes noncommercial licences; an accessible website is not
itself redistribution evidence. Unknown nine-domain target inventories remain
unknown, rather than being inferred from imported record counts.

## Integrated foundations

- PR126 source admission: merge `91c8468`. All nine PR checks passed; resulting
  main CI `38712962391` and packaging `38712962341` passed.
- PR129 inventory automation: merge `5d6fe44`. All nine PR checks passed; resulting
  main CI `38714849141` and packaging `38714849136` passed.
- PR128 candidate/publication separation: merge `0626c34`. All nine checks passed
  at `fb2ab32`; resulting main CI `38715934293` and packaging `38715934077` passed.
- PR127 guided acceptance kit: merge `5146a32`. All nine checks passed at
  `e98aebc`; resulting main CI `38717111614` and packaging `38717111956` passed.
  Root repeated all 18 synthetic PowerShell 7 acceptance-kit checks. These are
  harness tests, not native interactive or game observations.

- PR131 core v2: merge `6b406bc`. All nine checks passed at `4fc045e`;
  resulting main CI `38719592781` and packaging `38719592769` passed.

- PR130 typed party support: merge `ca55644`. All nine checks passed at
  `33ce7d3`; resulting main CI `38721214252` and packaging `38721214275` passed.

- PR133 catalog capacity: merge `c3d2927`. All nine checks passed at
  `339b353`; resulting main CI `38722453315` and packaging `38722453392` passed.

- PR132 shared pack verification and source-admission release binding: merge
  `444d66a`. All nine checks passed at `671e438`; resulting main CI `38723888671`
  and packaging `38723888607` passed.

- PR134 authored consumers: merge `4cf1263`. All nine checks passed at `0510863`;
  resulting main CI `38725723364` and packaging `38725723328` passed.

- PR135 catalog pagination: merge `e90b0d4`. All nine checks passed at `83cff65`;
  resulting main CI `38727295537` and packaging `38727295575` passed.

- PR137 verified image storage: merge `62d81d9`. All nine checks passed at
  `04dfc4e`; resulting main CI `38728676492` and packaging `38728676494` passed.

- PR136 stored-catalog Atlas: merge `23c4f31`. All nine checks passed at
  `a6d40e1`; resulting main CI `38730740786` and packaging `38730740835` passed.

- PR140 verified desktop import and bounded inspection: merge `be6b7b9`.
  All nine checks passed at `e7e9bfe`. Resulting main workflows:
  [CI 38732137048](https://github.com/Laymer/Navzeroth/actions/runs/38732137048)
  and [packaging 38732137042](https://github.com/Laymer/Navzeroth/actions/runs/38732137042).
  This documentation PR must wait for both workflows before merging.

All independent engineering in this batch is integrated. Original release-plan
content population, live bindings, candidate freeze, acceptance, publication and
post-release identity artwork remain blocked by the prerequisites below.
Review corrections include exact per-record source provenance, node evidence,
authored dungeon travel facts, recovery closure bounds, narrow-window picker
layout and transaction-consistent Atlas reads. No finding was waived to merge.

## Integrated local verification

The combined engineering tree at `a36e3c2` passed 983 Rust tests (four ignored),
strict workspace Clippy excluding the native desktop shell, 207 desktop unit tests
across 35 files, desktop typechecking/lint, all 73 contract fixtures and v2 schema
validation including nine rejection cases. Generated TypeScript contracts match
committed files. The final importer head `e7e9bfe` differs from that tested tree
only in two handoff documents; production code is identical. The separately invoked PowerShell/Git LFS tests passed as recorded
in the admission-gate and independent-review handoffs. These checks do not replace
hosted merge gates or live/native interactive acceptance. Independent final review
of the combined tree found no new important blocker. The inventory report retains
all nine domains as unknown, zero verified evidence files and publication disabled.

## Release prerequisites still unavailable

A complete independently evidenced 70374 inventory, redistributable corpus,
reviewed source grants, actual permitted live advancement/deviation/session
observations and real Windows/full-corpus measurements are absent. Live runbook
checks and interactive Windows checks remain unverified. The available session
is a Linux development environment; hosted Windows packaging is not evidence of
an operator performing all acceptance procedures.

Do not populate synthetic gameplay data, freeze 1.0.0 versions, claim acceptance,
tag or publish to work around those gaps. The required release-readiness command
currently refuses 1.0.0 because application/addon/lockfile versions still identify
0.8.1. Retain the preview until the full content and acceptance gates pass.
The gallery link is prepared; image generation remains conditional on the later
published, verified 1.0.0 release.

Next: obtain identifiable source material and reviewable redistribution rights,
independently enumerate the complete target corpus, then populate and validate it.
Resolve live bindings from permitted observations before freezing the candidate,
running operator acceptance and passing the final publication gate. Current main
and this documentation merge must retain all nine hosted checks.
