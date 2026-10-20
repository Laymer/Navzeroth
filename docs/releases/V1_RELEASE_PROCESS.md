# Full-content v1.0.0 release process

## Governing stable 1.0.0 contract

[ADR 0008](../decisions/0008-unsigned-stable-release.md) governs the authorized
**unsigned stable 1.0.0** release. Its explicit exception does not change historical
RC classification or authorize unsigned future releases. The versioned
[schema 4 acceptance contract](SCHEMA4_ACCEPTANCE.md) replaces historical Windows
acceptance requirements; schemas 1/2/3 and their recorded evidence retain their
meaning. This documentation neither migrates manifests nor records a passing gate.

Stable publication requires all **ten protected-main checks**, independent review,
source-platform parity, release stability and the applicable coverage, inventory,
source-attribution, content-integrity and material gates. Signed content receipts
remain mandatory despite the unsigned installer. Historical origins, accepted
unknowns, inferred travel, estimates and declared exclusions must remain explicit.
Live-client acceptance and the launch-build freeze were removed; no live-game
correctness or ordinary-PC performance is claimed.

Schema 4 requires eight exact-installer `hosted_windows` observations: installation,
first start, upgrade from 1.0.0-rc.1, downgrade refusal, persistence/recovery, installed
resources, offline operation and full-pack performance. Separately,
`OWNER-INSTALLER-SMOKE` requires `interactive_windows` evidence from the owner for
installation/upgrade, first start, bundled activation, representative planning and
restart/persistence. Every passing row binds the exact installer SHA256 and tested
source/production identity. Failed, missing or stale observations remain blocking;
historical WIN rows are not automatically promoted. Disclose measured resource use,
hardware and limitations, including any excluded child-process memory.

After the separate release PR merges, run fresh required checks on its exact
resulting commit; tag only that verified commit. `release-build-readiness` permits
bounded artifact preparation with pristine pending Windows rows, never publication.
`release-draft-readiness` requires all eight hosted rows for the exact installer;
only pristine pending owner smoke may remain. Independently download and verify
the draft assets, checksums, declared unsigned status, source binding, content,
notices and installed resources. Then pause for the owner's exact-draft-installer
smoke. A relevant production change or a different installer invalidates that evidence.
Only full `release-readiness`, including that smoke and every applicable gate,
authorizes publication. Once they pass, publish 1.0.0 without another approval
request; independently verify public downloads before updating/deploying the
website and download instructions. Preparation or draft readiness never supplies
publication permission.

### Current source, receipt and installer order

1. Prepare a separate version PR after reviewed content/desktop delivery. Align
   the six declarations listed in the historical version checklist below and the
   eight internal Cargo.lock entries. Regenerate addon, Reference, Today and
   synthetic-atlas outputs with their writers; regenerate and check dependency
   notices. Commit every production change before selecting a tested ancestor.
2. Run actual final-production checks and the full-pack functional scenarios.
   Refresh genuine inventory and proof FileRefs before recording final bindings.
   `scripts/generate-stable-inventory.py` resets bindings and Windows observations;
   never rerun it over signed or observed release metadata. Keep earlier scoped
   functional proofs and failures as historical evidence.
3. Independently review the immutable source/content and record checked revisions,
   production fingerprint, findings and corrections in a public review artifact.
   Record both signing identities' operator control explicitly. Commit the public
   artifact without future receipt/manifest hashes or installer claims.
4. Bind the actual tested ancestor, production fingerprint and verified inventory
   in schema4 evidence. Require `release-materials --root . --version 1.0.0`
   to return `unsigned_preparation.ready=true` for those exact evidence bytes;
   only the nine pristine pending hosted/owner rows may remain. Prepare the
   manifest/build identity using [the explicit preparation command](BUNDLE_PREPARATION.md).
5. Independently review those prepared bytes and actual material closure. Sign,
   assemble and verify the receipt using [the external-key CLI](../data/BUNDLED_CONTENT_RECEIPTS.md)
   against compiled trusted public identities and independently selected expected
   pins. Then commit the exact manifest/receipt/review FileRefs. These release and
   review metadata are excluded from the production fingerprint; product changes
   require renewed testing, review and bindings.
6. Merge the separately reviewed release PR only after its exact-head checks.
   Recheck the resulting main commit, including stability, source parity, coverage,
   packaging and revised material/build gates. Tag that verified source and retain
   the eight actual hosted results bound to the exact draft installer. Draft
   readiness permits only pristine pending owner smoke.
7. Independently download and verify the exact draft, then obtain the owner's
   smoke evidence. Require full release readiness before publication. Verify public
   downloads after publication, and only then update/deploy download instructions.

## Historical release process and superseded requirements

The following ADR0006/0007 process is retained as historical context. Where it
conflicts with the stable1.0.0 contract above (including signing, WIN rows, frozen
scope/build, live acceptance or pre-draft owner evidence), ADR0008 and schema4
govern. Historical evidence and unverified results keep their original status.


> **Superseded in part by [ADR 0007](../decisions/0007-world-data-automation.md)
> (2026-10-19, task V2.8e):** 1.0.0 readiness no longer requires the 15 `LIVE-*`
> checks, the live slice matrix or a post-launch frozen target. The target is the
> owner's detected build in `release/v1/world-data-target.json`; 1.12 and Classic Era
> data is accepted through `release/v1/accepted-origins.json`, and unknowns are
> reported as accepted unknowns. The `WIN-*` checks, hashed evidence and receipts are
> unchanged. Release notes must state that 1.0.0 ships without live-client evidence
> and without a frozen launch build. See [release/v1/README.md](../../release/v1/README.md).

> **Superseded in part by [ADR 0006](../decisions/0006-v1-program.md) (2026-10-17):**
> target (first stable post-launch live build), scope (declared Human 1-20 slice),
> sources (local-only under §11, including the owner's guides), hybrid delivery with
> locally built content, the signer arrangement, `1.0.0-rc.N`-only prereleases,
> signing, and authority. The 2026-10-18 continuation (§13) authorizes an early
> development rc.1 and excludes stable publication during this continuation. The
> latest published version is 0.9.2. The task tracker is [TODO.md](../../TODO.md).

This is the release contract for the authorized v1 program, starting from
`bb3782cbac13b5fb3e6ca8196278ba7fcc427474`. The latest published product is the
[0.9.2 developer preview](https://github.com/Laymer/Navzeroth/releases/tag/v0.9.2)
(receipt: [0.9.2 publication record](../handoffs/2026-10-17-v0.9.2-publication.md)).
Determine the latest published tag with `gh release list` rather than trusting this
sentence. Packaging of the 0.8.1 baseline and its acceptance limits are recorded in
the [final preview audit](../handoffs/2026-10-14-final-program-audit.md).

The target is Windows desktop support and all enumerated content available in
**Forever 1.60.1.70374**. The user's 2026-10-14 Windows 11 report identifies that
build/interface 16001 and addon 0.8.1 diagnostics (a historical report) without errors while out of combat.
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
`coverage-inventory.json` and `acceptance-evidence.json`. Current source includes
these contracts and CLI/Windows/workflow enforcement in separate reviewed tasks.
The 0.8.1 baseline did not include that enforcement; 0.9.x releases do. Incomplete real
inventory/evidence and the remaining delivery contracts still block stable output.

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

ADR 0006 §11 supersedes the historical provider audit path: use only selected
client extraction, permitted observations, admitted retained licensed material and
original authoring. No website retrieval or grant intake is in scope. For every
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

Use one implementation writer with a recorded owner and baseline per task,
pushed coherent commits, one PR per task and independent read-only review.
The continuation explicitly requests reviewer agents; additional implementation
workers require explicit authorization. Shared interfaces have one writer. After each batch, reconcile protected
main, re-audit every implementation-plan task and continue independent work while
external checks remain unavailable.

Run affected checks for each PR and every CI check on the exact merge
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

The enforcement command is
`cargo run --locked -p azeroth-cli -- release-readiness --root . --version <version>`
with optional `--evidence <path>` and a JSON report. Its canonical contract binds evidence to a tested source
commit and a production-tree SHA-256, with ancestry/tree checks so documentation
updates can retain evidence without pretending changed production was tested.
Test missing evidence, wrong build/source revision, failed mandatory coverage,
valid records and prerelease handling. Retest affected behavior after fixes.

Current enforcement deliberately blocks stable output while validated offline
content, license/attribution and corresponding-source delivery are unimplemented.
The existing five-file package cannot satisfy that requirement merely by filling
the evidence manifest. Three mandatory domains (dungeons, class/role models and
party coverage) have binding support through the v2 pack path
(`content_v2/records.rs`, `crates/cli/src/release.rs`), but no validated pack
supplies those records, and the CLI still blocks their absence (it never turns an
absent contract into a pass). Supplying real, validated packs is an explicit
prerequisite in addition to missing content rights and real acceptance. Current preview allowance is never a stable-readiness result.

## Early development RC publication (ADR 0006 §13)

This path is separate from the acceptance/stable sequence below:

1. Complete the ordered continuation engineering and review/test/CI PRs. Merge
   the separate version/notes PR declaring `1.0.0-rc.1` consistently. Notes list
   actual supported tuples, capabilities, signing status and all remaining gates,
   with installation, upgrade, removal, checksums and signing instructions.
2. Require fresh CI, Coverage gate, source and ordinary Windows packaging on the
   exact resulting main commit, plus the stability workflow. Run readiness with
   the actual version and preserve the blocker inventory. Candidate/stable
   readiness remains false until independently established.
3. Tag that exact commit `v1.0.0-rc.1`. Use ordinary prerelease packaging without
   `-AcceptanceCandidate`. Verify tag jobs and every draft asset against source,
   hashes, notices and the exact asset manifest. Unsigned installers are permitted
   only with explicit signing status.
4. Publish as a GitHub prerelease; download the public assets again and verify
   metadata, hashes, sizes and source. Record a publication receipt. This does
   not authorize stable publication. The Pages PR follows verification, links
   only to verified RC assets, and records deployed page/link checks.

## Acceptance candidate, tag and stable publication

The following sequence remains for the later acceptance/stable program. Stable
publication is outside the current continuation's authorization.

1. Label ordinary engineering previews as development prereleases. ADR 0006 §13
   permits an early `1.0.0-rc.1` with ordinary packaging, without
   `-AcceptanceCandidate`, after the continuation engineering, coverage and
   version/notes PRs and exact-source checks. Its notes must list every missing
   acceptance prerequisite and installer signing status. The RC name does not
   establish candidate readiness. Acceptance-candidate packaging still requires
   the full candidate gate; stable publication still requires the stable gate.
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
