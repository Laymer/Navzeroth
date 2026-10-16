# Claude Code continuation prompt: Navzeroth 0.9.0 → 1.0.0

Copy this entire file into Claude Code in the Navzeroth repository. This is an
execution prompt, not a promise that complete licensed game data exists. Its
success condition is a verified, published 1.0.0 release; if a prerequisite cannot
be established, leave the preview published and report the exact unresolved gate.
Do not manufacture evidence to make the plan appear complete.

## Current local-data continuation (2026-10-16)

Read [localdata.md](localdata.md) alongside this prompt. The user now requires
independent installed-client extraction and private local catalogs, with a
separately reviewed `local_catalog` delivery contract. ForeverDB is a capability
reference, not an extraction dependency. The local-data prompt supersedes older
source/delivery choices where they differ; the existing release gates remain
enforced until the versioned replacement is implemented and reviewed. Preserve
the complete nine-domain independently evidenced inventory and actual acceptance
requirements. 70425 development extraction cannot satisfy the 70374 release target.
See [actual implementation scope](docs/data/LOCAL_DATA.md) and the latest handoff.
0.9.0 preparation is still not publication; verify actual release state.

---

You are taking over Navzeroth (Azeroth Navigator), a Rust domain/services,
SQLite, Tauri/React/TypeScript desktop and permitted Lua addon project.
Carry out the work below, including implementation, verification, protected-main
integration and gated publication. Do not stop after proposing another plan.

## 1. Establish your actual baseline before editing

1. Read `AGENTS.md`, `docs/STATUS.md`, this file, and
   `docs/handoffs/2026-10-15-v0.9.0-release.md`.
2. Read `docs/handoffs/2026-10-15-development-evidence-closure.md`,
   `docs/handoffs/2026-10-15-v1-foundation-integration.md`,
   `AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md` (relevant architecture, data,
   planning, session, test and release sections), and applicable ADRs.
3. Read the current controlling contracts and tooling:
   - `release/v1/README.md`, `coverage-inventory.json`,
     `inventory-investigation.json`, `acceptance-evidence.json`;
   - `docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md`;
   - `docs/compatibility/DEVELOPMENT_CAPTURE.md` and
     `FOREVER_70374_EVIDENCE.md` in the same directory;
   - `docs/data/SOURCE_PIPELINE.md`, `RELEASE_INVENTORY.md`,
     `CONTENT_PACK.md`, and the v2 content documentation;
   - `docs/sources/source-admission-v1.md`, `source-admission-v1.json`,
     `independent-source-strategy.md`, `foreverdb.md` and `v1-public-source-audit.md`;
   - `.github/workflows/ci.yml`, `package-windows.yml`, `source-knowledge.yml`;
   - `scripts/windows/Build-Windows.ps1`, `Release-Materials.ps1`,
     `Verify-ReleaseArtifacts.ps1`, `Acceptance-Kit.ps1`,
     `Start-AcceptanceSession.ps1` and `scripts/Build-SourceKnowledge.ps1`.
4. Fetch without discarding local work:

   ```sh
   git status --short
   git fetch origin --tags
   git log -8 --oneline origin/main
   git rev-parse 'v0.9.0^{commit}'
   gh release view v0.9.0 --json tagName,isDraft,isPrerelease,publishedAt,assets
   ```

   Resolve the actual published tag; do not guess its commit from this document.
   Compare release `SOURCE_COMMIT.txt`, all checksum entries and the exact addon
   payload with that commit. Establish whether newer main changes supersede the
   release. If 0.9.0 publication is interrupted, finish its verified draft first
   using the release handoff; do not create a second competing tag.
5. Record task owner, current baseline and one-writer ownership in a claim before
   implementing. Use isolated branches/worktrees. Preserve unrelated work.
   Do not spawn agents without an explicit current authorization. Existing task
   handoffs are historical evidence, not permission to start parallel writers.
6. Prefer current contracts, actual code and recent evidence over superseded
   historical prose. Some older release-process paragraphs describe gaps that
   have since been implemented. Inspect the code/tests before reopening them.
   In particular, the old STATUS review table has a later reconciliation; its
   historical “Open” entries are not a new bug backlog.

## 2. Non-negotiable target and operating rules

- Target **Forever 1.60.1 / build 70374 / interface 16001**, Windows desktop,
  with complete coverage of all nine domains below. Do not silently turn this
  into a focused-region release, Classic/Retail release or synthetic showcase.
- Keep Rust core logic independent of I/O. Desktop state is imported state.
  Live observations remain in the normal permitted addon environment.
- No client memory reads, injection, packet decoding, socket bridge, hidden
  telemetry, input automation, or workarounds for restricted/secret combat data.
  The human performs game actions, reloads and visual comparisons.
- Preserve unknown, unsupported, restricted, stale, not-applicable, false, zero
  and justified empty sets distinctly. Never infer completion from absence,
  invalidation, readiness, a changed list or an inaccessible value.
- Parse imported Lua/SQL as bounded inert data. Implement reviewed transforms
  explicitly; do not execute imported corrections/providers.
- Keep personal SavedVariables, account paths, character names and raw exports
  private. Commit only reviewed sanitized evidence. Back up addon/data before
  changes; install the addon only after the operator closes the game and you
  verify it has exited. Do not launch, close or control the game for the user.
- User intent authorizes completing the project and its reviewed gated release.
  Honor applicable environment approval rules. Routine reversible work must not
  wait for redundant confirmation. Do not contact publishers, send messages,
  create external accounts or spend money without applicable explicit authority.
- Use compatible published permissive/share-alike grants, including applicable
  reciprocal obligations; exclude noncommercial restrictions. The repository's
  BSD licence does not relicense upstream works. Do not treat a code-only licence
  or another maintainer's permission as a general dataset redistribution grant.

## 3. What 0.9.0 actually provides

Verify these claims against the tag; do not rebuild completed foundations merely
because the original high-level plan listed them as future work.

- Reference-content v2/content-pack v2, typed eligibility/acquisition/class-role/
  dungeon facts and consumers, bounded catalog/index/cache/page infrastructure,
  migrations/rollback, party support contracts, inventory automation, candidate
  versus publication gates, material manifests and the guided Windows kit exist.
- The independent source pipeline retains four admitted narrow scopes and nine
  original/licence objects. Its **1,685 reference facts are not a playable 70374
  pack**. At handoff there are **zero verified target-usable facts**, all nine
  independent target inventories are unknown, and no complete gameplay pack is
  approved for release. Recompute these values; do not preserve stale counts.
- Sources: AtlasLootClassic selected declarations (GPLv2, Classic/unknown build),
  Mapzeroth node literals (MIT, Retail/unknown build), WoWSims implementation
  pointer (MIT, Classic/unknown build), and ATT Riverglades declarations
  (MIT, Forever/unknown build). Preserve each origin independently of target.
- ForeverDB raw staging exists, but admission and exact rights/build applicability
  remain unresolved. Its inspected metadata names origin build 70338; staged
  Forever/Classic counts do not establish 70374 completeness.
- Normal quest completion getter capture and explicitly opted-in turn-in session
  recording work in narrow source-identified development observations. Before
  the version bump, a level-13 capture had 14 readable booleans (five complete,
  nine incomplete), matching the operator's log. One actual turn-in produced a
  valid five-event session; isolated Linux service import recorded 41,604 ms
  unclassified, zero observed activity, no gaps. Status was idle after reload.
  These observations are not acceptance of the changed 0.9.0 or frozen 1.0.0.
- Current `PlanControls.lua` still declares objective mapping unavailable.
  `Diagnostics.ensurePlanControls` supplies the last snapshot; `Hud.lua` requires
  positive `objective_state` evidence. The recorder is **not** automatic plan
  advancement or deviation detection. Do not conflate these features.
- 0.9.0 is a developer prerelease. No final live/Windows acceptance row is passed
  merely because a build, helper test, browser mock or development capture passed.

## 4. Establish a reproducible blocker ledger

Run from the repository root with the pinned toolchain:

```sh
cargo run --locked -p azeroth-cli -- inventory-report --root . --input release/v1/inventory-investigation.json
cargo run --locked -p azeroth-cli -- release-readiness --root . --version 0.9.0
```

Use the current actual version if main has moved. A 0.x preview publication flag
is not stable/candidate readiness. Before final versions are set, requesting
1.0.0 correctly refuses a version mismatch; do not “fix” that by weakening gates.
Record each real blocker, required evidence, owner, dependency, validation command
and next action under `docs/`. Preserve the distinction between code absent,
content absent, rights unestablished and environment observations missing.

Use the following dependency order. Each work package gets a coherent PR; review
it independently, resolve important findings, wait for all nine checks, merge via
protected main and verify the resulting main. Keep incomplete work in draft.
Do not force-push main, bypass protection or rewrite published history.

## 5. Work package A — admitted sources and independent target inventory

1. Review candidate sources directly at immutable revisions. Record publisher,
   origin edition/build, exact permitted files/fields, grant/access evidence,
   retrieval bounds, licence, attribution and required corresponding source.
   Admit only the specific scope actually supported. Published compatible grants
   can remove a need for email; email replies are not a default dependency.
2. Wowhead remains research-only without an applicable grant. RestedXP's
   noncommercial material is excluded under this policy. Warcraft Wiki needs
   exact eligible revision/attribution and media scope. QuestieDB/CMaNGOS and
   ForeverDB are not admitted just because software/repositories are open source.
   The combined `TylerAkins/wow-database` pin in the latest handoff explicitly
   distinguishes original code from unrelicensed third-party data; its combined
   outputs are not an additional grant. Investigate original numeric-fact scopes
   on their own evidence rather than making blanket copyright assumptions.
3. Use original permitted observations and explicitly licensed contributions for
   gaps. Sanitize and review contributions; do not publish personal progress as
   generic gameplay content or infer the whole world from one character's log.
4. Reuse `crates/{core,ingest,cli}/src/source_*`, the registry and retained objects.
   Discovery must not automatically admit new content or execute upstream code.
   Approve immutable changes, then acquire only permitted bytes within bounds.
5. Independently enumerate exact required identities for the target, separate
   from imported counts. Keep unknown inventories `null`; known-empty `[]`
   requires evidence. Track conflicts, unsupported corrections and missing IDs.

| Domain | Exhaustive facts to establish |
| --- | --- |
| Quests | Available quests/stages/objectives, prerequisites/branches, eligibility, repeat/reset/sharing conditions and rewards |
| Equipment | Equippable variants, slots/handedness, attributes/effects, requirements and configuration restrictions |
| Acquisition | Reward choices, vendors/crafting/drops, quantities/costs, availability/prerequisites and loot entitlement |
| Geography | Available regions/instances/floors, coordinate frames, calibrated locations and permitted imagery |
| Travel | Directed links/endpoints, access/unlocks/costs/waits, geometry and stopping routes |
| Dungeons | Instances/entrances/access, preparation chains, travel/rewards and evidence-backed checklists |
| Class/roles | Available applicability combinations, documented models, proficiency/layout facts, exclusions and representative cases |
| Party | Supported sizes/workflows, unequal progress/sharing/staleness/fairness/agreement/advancement/review, tied to concrete content |
| Reminders | Every shipped rule, supporting facts/conditions/priority/suppression and unknown/restricted/stale behavior |

6. Synchronize `inventory-investigation.json` with the reviewed authoritative
   inventory. Run `inventory-report`, compare sources with the independent
   denominator, and produce missing-record and observation queues plus concrete
   representative scenarios. An imported count, synthetic ID or local profile
   cannot substitute for an available gameplay record.
7. Exercise two offline builds into fresh directories with PowerShell 7:

   ```powershell
   ./scripts/Build-SourceKnowledge.ps1 -OutputDirectory dist/source-check-a
   ./scripts/Build-SourceKnowledge.ps1 -OutputDirectory dist/source-check-b
   ```

   Provision the pinned Rust dependencies first (`cargo fetch --locked`). Compare
   complete output hashes and retained closure on Linux/Windows. Ordinary builds
   must remain offline. Optional approved discovery is documented in
   `docs/data/SOURCE_PIPELINE.md`; use the policy's actual byte/rate limits.
8. Converge the same fact only within compatible known origins. Rank reliability
   first, then actual trusted observation freshness, with stable tie-breaking.
   Download/commit dates are not gameplay observation dates. Retain competing
   candidates, provenance, unknowns and conflicts; a preferred conflicting value
   cannot become target-verified merely by winning a rank.

Exit: exhaustive evidenced target identities and admitted inputs, or a precise
blocked-record ledger. If complete inputs cannot be established, continue
independent engineering, retain the preview and state what remains unavailable.
Never invent a denominator, use remembered game mechanics as verified data, or
promise that autonomous tooling can establish rights/completeness by itself.

## 6. Work package B — complete validated gameplay content

1. Inspect `crates/core/src/content_v2.rs`, its schema/record bindings,
   `crates/core/src/entities/`, pack validators, storage activation and desktop
   consumers. Reuse implemented contracts; add versioned/migrated fields only
   where actual acquired requirements demonstrate a gap.
2. Populate reviewable partitions: quests/equipment/acquisition; geography/travel/
   dungeons; class/role models/reminders/party support. One writer per partition;
   share-file ownership explicitly. If parallel work is not authorized, do these
   sequentially. Retain one assembled immutable logical catalog.
3. Resolve every reference across partitions/regions, eligibility predicate,
   reward/choice, model applicability, dungeon dependency and provenance link.
   Unresolved conflicts/corrections/permissions block affected records.
4. Keep compatible v1 documents, hashes and rollback history. Verify v2 bounds:
   250,000 reference identities, 250,000 evidence records, 256 MiB encoded catalog,
   1 GiB verified material closure; retain smaller record/planning limits. Overflow
   must refuse without truncation. Check immutable indexes/cache and revision-bound
   desktop pages of at most 200 rows, atomic activation and rollback.
5. For each input directory, use supported current commands, with fresh outputs:

   ```sh
   cargo run --locked -p azeroth-cli -- build-content-v2 --input <reviewed-input-directory> --output <new-pack-directory>
   cargo run --locked -p azeroth-cli -- verify-pack --input <new-pack-directory>
   ```

   Angle-bracket values are placeholders to resolve, not literal shell arguments.
   Validate exact target, zero mandatory gaps, non-fixture identity and permitted
   distribution. Source-knowledge output is not a gameplay pack.
6. Generate canonical required record bindings using the existing release
   validator; do not invent matching strings or weaken the comparison. Populate
   authoritative inventory/source/pack/material records only from verified outputs.
   Keep source registry private evidence out of public material selections; include
   all actually required distributable licences/attribution/corresponding source.
7. Verify actual Gear, Party, Dungeon, Atlas, Planner, Reminder and Session
   consumers against these records. Synthetic profiles/scenarios do not prove
   class models or party gameplay coverage. Test >1,024 records, cross-region
   dependencies, migration/rollback, paging stability and refusal at capacity.

Exit: complete licensed target packs match the independent inventory and every
mandatory content dependency has a real validated record and scenario.

## 7. Work package C — supported fresh observations and plan behavior

1. Read `addons/AzerothNavigator/Core/{Snapshot,Predicate,Plan,Hud,PlanControls,Session}.lua`,
   `Diagnostics.lua`, Rust planning/export/session services and the binding tests.
   The observed normal APIs are `C_QuestLog.IsComplete`,
   `C_QuestLog.IsQuestFlaggedCompleted` (presence only for the latter),
   `C_EventUtils.IsEventValid`, and actual `QUEST_TURNED_IN` dispatch. The operator
   also reported `QUEST_LOG_UPDATE` valid; dispatch semantics still need evidence.
2. Define provenance-bearing, target/version-bound completion mappings from real
   quest/stage/objective records to permitted observable facts. Current-log ready
   state, actual turn-in, quest history and arbitrary objective stages are distinct.
   Do not derive one from another without an explicit verified semantic mapping.
3. Carry reviewed mappings through the existing bounded export boundary without
   silently changing snapshot/plan exchange formats. Version any extension/new
   contract explicitly; reject malformed/duplicate/unresolved/wrong-context data.
4. Supply fresh runtime observations to guidance. Do not mutate imported snapshot
   history, reuse an old capture ID to pretend it is fresh, or bypass the existing
   snapshot-supersession/dependency checks. Design an explicit bounded observation
   projection/overlay with base lineage, participant, context, source, timing and
   freshness checks. Missing mapping or inaccessible/secret values stay unknown.
5. Bind only supported normal events/getters, bounded to loaded requirements, with
   explicit lifecycle/consent and no hidden polling or background collection.
   Guard every observation and keep fixed redacted diagnostics. No manual “next”
   command may substitute for observed completion. Do not invent an addon bridge.
6. Advance only the intended participant and step on positive fresh evidence.
   Check two successive steps, stale/unknown/missing cases, changed snapshots,
   invalidation versus completion, alternative selection and data/rules/model
   revision changes. Detect supported deviations honestly; request revalidation
   or fresh desktop replanning when coverage is unavailable. Never infer arrival.
7. Preserve the explicit `start-quests` recorder behavior: manual mode unchanged,
   paused/stopped events ignored, build matching, bounded IDs, no reward reads,
   unclassified time, atomic refusal, manual export and reload-to-idle. Complete
   any additional session observations required by the runbook on actual APIs.
8. Run meaningful Rust/Lua parity/contract/desktop checks and then operator-guided
   development checks on the exact installed source. Never mark final acceptance
   from a synthetic plan or these pre-freeze observations.

Exit: supported runtime bindings actually consume verified content mappings;
missing/restricted/stale cases remain truthful. Real acceptance follows the freeze.

## 8. Work package D — validate, integrate and freeze the real candidate

Use affected checks during development and the following required release checks:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --exclude azeroth-desktop -- -D warnings
cargo test --locked --workspace --exclude azeroth-desktop
cargo run --locked -p azeroth-cli -- validate-fixtures
pnpm install --frozen-lockfile
pnpm contracts:types
pnpm contracts:check
node scripts/validate-content-v2-schema.mjs
pnpm --dir apps/desktop typecheck
pnpm --dir apps/desktop lint
pnpm --dir apps/desktop test:unit
pnpm --dir apps/desktop build
pnpm --dir apps/desktop test:e2e
```

Provision Playwright/native prerequisites through documented tooling. Browser
checks are mocks. Windows also runs native desktop Clippy/tests and the existing
PowerShell helper checks under supported shells. Run atlas pointer/schema/content
checks from CI; don't smudge historical LFS content or relabel its origin.

For every merge candidate and resulting main require these nine named checks:
Rust format; Rust Linux; Rust Windows; Lua static checks; pnpm Linux; pnpm Windows;
Desktop shell Windows; Mocked desktop Chromium; Windows installer and addon package.
Also require the source workflow's Linux/Windows/parity jobs when applicable.
Investigate failures; do not report cancelled/pending/skipped required jobs as pass.

Only after all production work, complete licensed packs and static requirements
are ready, make the final-version PR. Align root Cargo.toml/package.json, desktop
package.json/Tauri config, addon Version.lua/TOC and internal Cargo.lock entries.
Regenerate affected synthetic fixtures through generators and byte comparisons.
Do not change dataset identities to match application versions. Write 1.0.0 notes,
installation/upgrade/rollback instructions and the README branding-gallery link.
Merge reviewed production already versioned **1.0.0**, then freeze its source,
application/addon/content/workflows/test tooling and inventory. Record exact hashes.

**This root `initial.md` is part of the production fingerprint.** Do not edit it,
README, tests, site, workflows, versions or content after the freeze and pretend
old acceptance still applies. Post-freeze evidence/notes changes belong only under
`docs/` and `release/`, and must preserve both production and inventory bindings.
A production change requires a new candidate and renewed affected acceptance.

## 9. Work package E — candidate packaging and actual acceptance

1. Run `candidate-readiness --root . --version 1.0.0` through the CLI. Candidate
   readiness may defer pending live/Windows observations and initial tested-source
   binding only. Complete licensed content and static contracts are still required;
   malformed/tampered/stale evidence and current-candidate failures block.
2. On Windows, build explicitly non-publishable artifacts:

   ```powershell
   ./scripts/windows/Build-Windows.ps1 -ExpectedVersion 1.0.0 -AcceptanceCandidate -OutputDirectory dist/acceptance-candidate
   ./scripts/windows/Verify-ReleaseArtifacts.ps1 -Version 1.0.0 -Directory dist/acceptance-candidate -AcceptanceCandidate
   ```

   Use a clean checkout and normal checks. Never use AllowDirty/SkipTests, candidate
   flags or a renamed archive to bypass stable gates. Stable/tag packaging has no
   candidate fallback. Candidate artifacts must fail stable artifact verification.
3. Verify downloaded candidate source metadata, complete checksums/materials,
   inventory digest/content hashes and every installed addon file. Record app/addon
   identity, actual `GetBuildInfo`, OS/hardware/WebView2, display matrix and operator.
   Keep backups/raw exports private. Recheck installed identities after sessions.
4. Invoke `Start-AcceptanceSession.ps1` with actual resolved Root, Artifacts,
   InstalledAddon, ExpectedSourceCommit, Scenarios and a new private OutputDirectory.
   Read its mandatory parameters; do not invent a canned scenario file or bypass
   its candidate/material/tool-root verification. Scenarios come from the reviewed
   independent inventory. The operator performs every game/visual/window action.
5. Complete every subcase in the current runbook for these 25 mandatory rows:

   - LIVE-BUILD, LIVE-CAPTURE, LIVE-TRANSFER, LIVE-RELOAD, LIVE-RESTRICTIONS,
     LIVE-HUD, LIVE-PLAN, LIVE-ADVANCE, LIVE-DEVIATION, LIVE-DUO, LIVE-GEAR,
     LIVE-DUNGEON, LIVE-REMINDERS, LIVE-SESSION, LIVE-NEXT-PLAN.
   - WIN-INSTALL, WIN-UPGRADE, WIN-UNINSTALL, WIN-RECOVERY, WIN-OFFLINE,
     WIN-FOCUS, WIN-REFERENCE, WIN-DISPLAY, WIN-ACCESSIBILITY, WIN-PERFORMANCE.

   Also cover all inventory-linked class/role/level/region/party-size scenarios.
   Run the integrated real differing-progress duo: import two distinct profiles,
   establish shared eligibility/catch-up, agree a plan, include a justified gear or
   preparation opportunity, transfer/advance, handle deviation, review and replan.
6. Predeclare full-corpus Windows workloads, hardware, timing resolution and sample
   counts, including cached lookup usually <100 ms and focused planning p95 <500 ms
   as specified in the runbook. Preserve raw timings/CPU/private-memory samples,
   cancellation/search/import and repeated-replan results. Synthetic catalog timings
   cannot replace measurements of the accepted full corpus.
7. Missing hardware, operator, observations or subcases remain unverified. Do not
   automate restricted actions or waive the matrix to obtain a pass. Fix products
   in focused PRs, produce a new candidate and renew candidate-bound evidence.
8. Independently review sanitized results before updating release manifests. Each
   accepted record must identify method/operator/time/environment, tested source,
   production fingerprint, inventory digest, exact files/hashes and actual outcome.
   A human “pass” string is not proof; automated helpers cannot satisfy live_game
   or interactive_windows methods. Keep raw personal exports outside Git.

## 10. Work package F — full gate, final assets and publication

1. Merge reviewed evidence under docs/release only and verify unchanged production
   and inventory fingerprints and tested-ancestor lineage. Require zero mandatory
   blockers and a successful full gate:

   ```sh
   cargo run --locked -p azeroth-cli -- release-readiness --root . --version 1.0.0
   ```

   Require stable_ready=true and publication allowed. Do not switch the requested
   version to 0.x/rc to evade this. Retain all nine hosted gates and independent
   evidence review as separate requirements.
2. Build ordinary final Windows artifacts (no AcceptanceCandidate flag), verify
   the full expected material/artifact set with Verify-ReleaseArtifacts.ps1 and
   compare application, addon and content payloads with the tested candidate.
   Final SOURCE_COMMIT.txt must name the actual release commit, not an ancestor.
   Explain metadata-only differences; investigate every unexplained payload change
   and repeat native installation/import/plan smoke checks. Any actual product
   change invalidates the relevant candidate evidence.
3. With the accepted commit on protected main and its checks complete, tag exactly
   that SHA as v1.0.0 without moving an existing tag. Wait for both tag packaging
   and draft-release jobs. The default stable workflow must execute the full gate.
4. Download the draft's complete assets to a fresh directory. Verify exact file
   set, all checksums, source SHA, addon ZIP entries/bytes/licence, installer and
   content/material manifests, original/licence/corresponding-source closure and
   absence of private registry/personal evidence. Run the checked artifact verifier
   from the exact source checkout. Do not publish an unverified GitHub draft.
5. Publish only that verified draft as a stable release. Re-download public assets,
   compare bytes/hashes with the verified draft and record public URLs, tag/source,
   workflow runs, checksums, actual checks and limitations in the release handoff.
   Preserve all published history. If any gate fails, retain 0.9.0; do not publish
   a “complete” 1.0.0 with placeholders or mandatory blockers.
6. After 1.0.0 is published and verified, use the available imagegen skill for an
   original fantasy-cartography identity featuring compass, plotted route and
   “Navzeroth”. Save `docs/brand/navzeroth-identity.png`, inspect it and merge a
   documentation-only PR. Do not move this image generation before the release.

## 11. How to communicate and finish

Make small coherent changes; record actual commands, failures, checked commits,
decisions, limitations and the next task in handoffs. Continue independent work
when a particular operator/data prerequisite is unavailable. Ask concise necessary
questions, explain which real observation/permission is missing, and never ask for
credentials, raw personal exports in Git or repeated routine approvals.

Do not report “done” because tests pass, imported counts look large, a preview
publishes or the budget is running low. Completion means the exact full target
content/rights, runtime behavior, all required candidate-bound acceptance,
zero-blocker stable gate and verified public 1.0.0 artifacts actually exist.
If that is externally blocked, leave completed work integrated and draft work
reviewable, keep 0.9.0 published, and report the concrete unmet records/grants/
observations plus the next permitted action. No false completion or endless
replanning is an acceptable substitute.
