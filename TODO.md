# 1.0.0 program tracker

Governing decision: [ADR 0006](docs/decisions/0006-v1-program.md) (2026-10-17).
Target: the first stable live WoW Forever build after the 4 Nov 2026 launch
(70374 is no longer the target; 70425 is development input only). Slice: Human,
levels 1-20, Northshire/Elwynn/Westfall/Redridge, the Deadmines, every Human class,
solo and duo, with all nine domains complete within it.

Owner types: **Owner** = the project owner or operator; **Eng** = repository work;
**Both** = owner input, then engineering. Nothing here is a date promise, and
nothing is verified unless it says so. The previous G8 inventory of what 1.0.0
needed is in Git history (`TODO.md` at `3b19812`).

## Gate snapshot

`release-readiness --root . --version 0.9.2` after V1.1 (schema 2 manifests): 68
blockers, `stable_ready: false`, `candidate_ready: false`, 0 verified evidence files,
0 verified content packs. That's the 66 from `3b19812` plus two from V1.1: "Release
target is pending" (cleared by V4.2) and "Release slice classes are unknown"
(cleared after launch). Re-run it before trusting any count here.

| Category | Count | Cleared by |
| --- | ---: | --- |
| A, B Inventory incomplete / required records unknown (per domain) | 18 | V1.1, V2.5 |
| C Required record inventory empty or excessive | 1 | V2.5 |
| D Accepted source revisions missing | 1 | V0.6, V2.1 |
| E Tested source revision and production fingerprint pending | 1 | V5.1 |
| F, G 15 `LIVE-*` and 10 `WIN-*` observations pending | 25 | V5.2 to V5.4 |
| H, I Unit coverage missing / not passed | 10 | V2.5 |
| J, K Validated packs missing / no pack record binding | 10 | V2.1 to V2.6 |

## Phase 0: housekeeping

- [x] **V0.1 Publish 0.9.2** (Both). Published 2026-10-17 as a prerelease after
  verifying the assets; see the [receipt](docs/handoffs/2026-10-17-v0.9.2-publication.md).
- [x] **V0.2 ADR 0006 and re-baseline** (Eng). This file, ADR 0006, STATUS, and
  supersession notes in the release contract.
- [ ] **V0.3 glib alert** (Both). Eng check done: `glib` 0.18.5 is absent from the
  `x86_64-pc-windows-msvc` tree and present on Linux only through tauri → muda/tao →
  gtk ([V0 handoff](docs/handoffs/2026-10-17-v0.md)). Remaining: the owner dismisses
  GHSA-wrw7-89jp-8q8g with that reasoning.
- [ ] **V0.4 Signing** (Owner → Eng). Owner provisions cloud Authenticode signing
  and the `NAVZEROTH_SIGN_COMMAND` secret. Eng runs a signed manual build and adds
  `Get-AuthenticodeSignature` verification to the artifact verifier.
- [ ] **V0.5 CI recheck** (Eng). The hosted "Native WebView2 smoke over CDP" result
  on a head at or after `81aef88`; owner confirms the nine required checks are the
  permanent set.
- [ ] **V0.6 Grant intake** (Owner → Eng). Owner exports the written grants from
  ForeverDB, Questie/QuestieDB, foreverchanges.pro and Olympus News Network. Eng
  stores them as private hashed admission evidence and records scope per source in
  [source-admission-v1.md](docs/sources/source-admission-v1.md) (ADR 0006 §4).

## Phase 1: contract and gate (one writer on `release.rs`, in order)

- [x] **V1.1 Release manifest schema v2** (Eng). Done
  ([handoff](docs/handoffs/2026-10-17-v1.1.md)): declared slice, per-domain
  out-of-scope boundaries distinct from `unknown`, and a `null` (pending) target
  that blocks every gate until V4.2 freezes it in code.
- [ ] **V1.2 Hybrid delivery** (Eng). New delivery-policy format with a `hybrid`
  mode: tested-content digest (extractor profile, metadata revision, expected
  catalog digest per build/locale), receipt CLI (prepare/sign/verify) on
  `crates/local-data/src/receipts.rs`, and a desktop match/mismatch state.
- [ ] **V1.3 Signer rule** (Eng). Two keys may share the owner identity when the
  reviewer receipt binds a hashed independent review record (ADR 0006 §6).
- [ ] **V1.4 Slice acceptance matrix** (Eng). Class × level band × zone × party size
  sampling rows plus the typed party support matrix in
  `release/v1/acceptance-evidence.json`.

## Phase 2: content (up to three parallel worktree streams)

- [ ] **V2.1 Granted-source adapters** (Eng, after V0.6). Promote the QuestieDB
  adapter and `stage-foreverdb` staging; inert adapters for retained
  foreverchanges.pro and Olympus News Network snapshots (fetch stays a separate
  opt-in step); admission entries in `content/forever/sources.json`.
- [ ] **V2.2 Local extraction projections** (Eng). Equipment, geography, travel and
  class/role projections from decoded tables; cache/hotfix import; activation and
  rollback; other installed locales; deterministic digest.
- [ ] **V2.3 Guide port** (Eng). Inert RXP-format importer and route replay
  verifier in Rust, ported from the owner's guide generator; per-class Human 1-20
  routes with a Deadmines block; owner reviews them in play.
- [ ] **V2.4 Authored domains** (Eng). Deadmines preparation, Human class role
  models (WoWSims-derived, assumptions documented), solo/duo scenarios, reminder rules.
- [ ] **V2.5 Inventory and review** (Both). Replace each `<domain>-inventory-pending`
  with slice units and exact required IDs; independent enumeration review; owner
  confirms disagreements in client; passing coverage rows.
- [ ] **V2.6 Release materials** (Eng). Notices, licences and corresponding source
  for every bundled source; no Blizzard art or client fonts; no private catalogs in
  public assets.

## Phase 3: polish (before the RC freeze)

- [ ] **V3.1 Optional review items** (Eng), all of them:
  G4 #10 (handoff wording); G5a #5-7 (planner Clear help omits exclusions, profile
  help understates discards, two Gear help texts), #12 (dropped `.saved-plan` /
  `.marker-panel` hooks), #13 (several `primary` buttons on Gear and Calibration),
  follow-ups (3:1 `--tone-warn` border, Help on per-item labels); G5b O5 (Help
  triggers in busy disabled fieldsets), O7 (Compatibility baseline at 320 px/200%);
  G6 O1 (nested-panel assertion per screen), O2 (`InsidePanel.Provider` across
  portals), O3 (58em threshold and unsupported-engine tests), O4 (pressed-button
  forced-colours specificity and test, redundant prop type), O5 (`.ui-button`
  margin on Atlas pin and route hop buttons), O6/O8 (resize-listener assertions,
  `toHaveCount(0)` vacuity). Sources: [G4](docs/reviews/2026-10-17-g4.md),
  [G5a](docs/reviews/2026-10-17-g5a.md), [G5b](docs/reviews/2026-10-17-g5b.md),
  [G6](docs/reviews/2026-10-17-g6.md).
- [ ] **V3.2 Locales in the desktop** (Eng) for extracted non-enUS catalogs.
- [ ] **V3.3 Performance workload** (Eng) for the full slice (WIN-PERFORMANCE).

## Phase 4: after launch (4 Nov 2026)

- [ ] **V4.1 Live build tuple** (Owner) from `.build.info` and in game.
- [ ] **V4.2 Freeze the target** (Eng). Manifests, delivery policy, addon/edition
  data; re-extract; re-fetch snapshots; rebuild and verify packs; review deltas.
  Never relabel 70425 artifacts.
- [ ] **V4.3 Live development pass** (Owner). Install the addon and run diagnostics.

## Phase 5: candidate and acceptance

- [ ] **V5.1 Freeze rc.1** (Eng). `1.0.0-rc.1` in the six declarations and eight
  lock entries; `candidate-readiness` exits 0; `Build-Windows.ps1
  -AcceptanceCandidate`; tag and publish the prerelease; verify the assets.
- [ ] **V5.2 Live acceptance** (Owner). `LIVE-BUILD` first, then the other 14
  `LIVE-*` rows, the slice matrix rows and the `LIVE-GUI` addon checks
  ([runbook](docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md),
  [addon GUI](docs/compatibility/ADDON_GUI.md)).
- [ ] **V5.3 Windows acceptance** (Owner). Ten `WIN-*` rows, bare-machine install,
  upgrade from 0.9.2, DPI/multi-monitor, WebView2 visuals, NVDA/Narrator,
  SmartScreen on the signed build ([install](docs/compatibility/WINDOWS_INSTALL.md)).
- [ ] **V5.4 Evidence recording** (Eng). Hashed evidence in the manifest, signed
  receipts, independent evidence review.
- [ ] **V5.5 Fix loop** (Both). A product fix means a new rc and re-run rows; the
  owner signs off on the per-change re-run rule.

## Phase 6: stable

- [ ] **V6.1** Version 1.0.0, `docs/releases/1.0.0.md` (slice, boundaries, signing,
  signer arrangement, attribution), independent review, nine fresh checks.
- [ ] **V6.2** `release-readiness --version 1.0.0` reports `stable_ready: true`;
  tag `v1.0.0`; verify every downloaded asset; publish stable; re-verify; receipt
  ([process](docs/releases/V1_RELEASE_PROCESS.md)).

Order: V0 → V1.1 → V1.2-V1.4 in parallel with V2.1-V2.4 → V2.5 → V2.6 and V3 →
launch → V4 → V5 → V6.
