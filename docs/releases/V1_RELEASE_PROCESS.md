# Full-content v1.0.0 release process

This is the release contract for the authorized v1 program, starting from
`bb3782cbac13b5fb3e6ca8196278ba7fcc427474`. The published product remains the
[0.8.1 developer preview](https://github.com/Laymer/Navzeroth/releases/tag/v0.8.1).
Current packaging, its verified artifacts and its acceptance limits are recorded
in the [final preview audit](../handoffs/2026-10-14-final-program-audit.md).

The target is Windows desktop support and all enumerated content available in
**Forever 1.60.1.70374**. The user's 2026-10-14 Windows 11 report identifies that
build/interface 16001 and addon 0.8.1 diagnostics without errors while out of combat.
Exact installed source and the remaining real acceptance are unverified. Bind every
acceptance run to the actual client and installed source. Neither the older 70338 reports nor the historical
70205 atlas establishes 70374 compatibility. Keep application, exchange schema,
game dataset and upstream source revisions distinct.

Publication and protected-main integration are already authorized by the user's
program. A failed or missing mandatory gate holds the release at candidate status;
authorization does not supply evidence or source permissions. No new confirmation
is needed for work already authorized here.

## Freeze scope and evidence before release claims

The coordinator owns the canonical contract under `release/v1/`: `README.md`,
`coverage-inventory.json` and `acceptance-evidence.json`. Those files and the
stable-readiness CLI are being implemented in separate PRs; they must be integrated
and tested before stable publication. This document does not claim that enforcement
exists in the 0.8.1 baseline.

The inventory must enumerate quest, equipment, acquisition, geography, directed
travel, dungeon/preparation, class/role and party workflows across the whole frozen
baseline. Geographic atlas coverage is distinct from playable catalog coverage.
An unknown denominator, an unassessed unit or a missing mandatory record blocks
full-content completion. Required validations include references, prerequisites,
coordinate meaning, directed connections, acquisition links, role-model provenance,
attribution and distribution eligibility. Fixture counts never count as real content.

Official API access, party messaging, combat-log analysis and hosted distribution
may remain explicitly deferred when their prerequisites cannot be verified. A
deferral must identify its boundary; it cannot remove a mandatory offline workflow
or content unit from the inventory.

## Sources and distributable content

Audit QuestieDB first and CMaNGOS Classic DB as a historical comparison. For every
accepted source, retain its immutable revision and retrieved-file hashes, source
location, applicable license and notices, attribution, permitted transformations
and distribution, exact edition/build, field mappings, correction coverage,
conflicts and validation evidence. Public availability alone is not permission.
Unresolved rights exclude a source from shipped assets. Historical Classic records
must not silently become verified Forever facts.

Only bounded deterministic adapters may produce content. Never execute imported
Lua or SQL. Report rejected executable corrections and their coverage impact.
Retain unknown, unsupported, restricted, stale, false and zero separately; do not
fill a source gap with an invented route, stat weight or mechanic.

Upstream-derived records keep their applicable terms. The project's BSD-3-Clause
license does not relabel them. Include required notices and corresponding source
materials with the approved content assets. The existing historical atlas has
incomplete rights and `distribution_eligible=false`; its local-reference pack is
excluded from the published preview and is not an approved v1 content asset. See
[source registry](../data/SOURCES.md) and [local pack format](../data/CONTENT_PACK.md).

## Review and acceptance

Use three isolated implementation workers plus a coordinator, with one recorded
owner and baseline per task, pushed coherent commits, one PR per task and independent
review. Shared interfaces have one writer. After each batch, reconcile protected
main, re-audit every implementation-plan task and continue independent work while
external checks remain unavailable.

Run affected checks for each PR and all nine required checks on the exact merge
candidate: Rust format, Rust Linux, Rust Windows, Lua static checks, pnpm Linux,
pnpm Windows, Desktop shell Windows, Mocked desktop Chromium, and Windows installer
and addon package. Reconcile normally and rerun affected checks after conflicts.
Record actual checked commits and completed outcomes, including failures. Mocked
browser tests and hosted Windows smoke installation do not replace real acceptance.

Live evidence must identify the installed addon/application revision, client
build/interface, environment, operator, sanitized artifacts and outcome. Exercise
snapshots, SavedVariables, plan transfer and advancement, reloads, restrictions,
HUD behavior, stale observations and session review. Demonstrate one complete real
duo workflow: import both characters, plan eligible shared progress with differing
progress, include a justified opportunity, advance the addon plan, handle deviation,
review the session and generate the next plan.

Interactive Windows evidence must cover installation, upgrade, uninstall/data
retention, backup/restore, offline operation, focus, reference-window interaction,
DPI/multiple monitors, accessibility and performance on recorded hardware and
declared realistic workloads. Missing environment checks remain unverified.

The planned enforcement command is
`cargo run --locked -p azeroth-cli -- release-readiness --root . --version <version>`
with optional `--evidence <path>` and a JSON report. Do not treat it as available
until the gate PR lands. Its canonical contract binds evidence to a tested source
commit and a production-tree SHA-256, with ancestry/tree checks so documentation
updates can retain evidence without pretending changed production was tested.
Test missing evidence, wrong build/source revision, failed mandatory coverage,
valid records and prerelease handling. Retest affected behavior after fixes.

## Candidate, tag and publication

1. Keep ordinary engineering previews labelled as such. Prepare `1.0.0-rc.N` only
   when the mandatory inventory and acceptance rows permit entry to the release
   candidate batch. Record failures and fix them in focused PRs. Prerelease handling
   must not turn incomplete evidence into stable readiness.
2. Create the final version PR with matching versions in root `Cargo.toml`, root
   and desktop `package.json`, Tauri configuration, addon `Core/Version.lua` and
   addon TOC. Update internal Cargo.lock entries and affected generated fixtures
   through their actual generators. Do not relabel game datasets to match app versions.
3. Include `docs/releases/<version>.md`, compatibility declarations, complete coverage,
   source attribution, installation/removal instructions and real evidence. The
   source/license asset manifest must describe the exact shipped content. Obtain
   independent acceptance review and fresh required checks before merging.
4. Recheck protected main, run the stable-readiness gate for `1.0.0`, and tag the
   exact accepted commit `v1.0.0`. Preserve published history and never force-push
   main. Verify both tag packaging and draft-release jobs at that source.
5. Download and verify the installer, addon ZIP, notes, source commit, checksums,
   validated offline content and all required notices/source materials. Match the
   exact asset set, hashes, sizes and source revision to the release manifest;
   inspect addon ZIP contents and licenses. Checksum agreement establishes integrity,
   not source authenticity or permission. The current five-file packager needs
   additional validated content/source packaging before it can meet this v1 scope.
6. Publish the verified draft as stable only after every mandatory gate passes.
   Then verify the public assets' hashes, sizes, metadata and source commit. Record
   the actual release URL, tag, jobs, artifacts and remaining optional deferrals in
   the release handoff.

If mandatory coverage, permissions or real acceptance is still missing, preserve
the candidate and report the exact blocker. Do not publish stable v1.0.0, replace
real checks with synthetic passes, or claim the full target was delivered.
