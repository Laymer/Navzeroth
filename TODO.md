# 1.0.0 program tracker

Governing decision: [ADR 0006](docs/decisions/0006-v1-program.md) (2026-10-17).
Target: the first stable live WoW Forever build after the 4 Nov 2026 launch
(70374 is no longer the target; 70425 and 70450 are development inputs only). Slice: Human,
levels 1-20, Northshire/Elwynn/Westfall/Redridge, the Deadmines, every Human class,
solo and duo, with all nine domains complete within it.

Owner types: **Owner** = the project owner or operator; **Eng** = repository work;
**Both** = owner input, then engineering. Nothing here is a date promise, and
nothing is verified unless it says so. The previous G8 inventory of what 1.0.0
needed is in Git history (`TODO.md` at `3b19812`).

## Continuation (2026-10-18)

Reconciliation merged as PR #191 at `167a454`, independently reviewed with all nine required checks.
Owner and sole writer for reconciliation: Codex `/root`, baseline `7aecba8`;
[claim](docs/claims/2026-10-18-continuation-policy.md).
PR #188 (authored slice, including Windows path fixes), #189 (observed `wowf`
70245, not admitted), and #190 (70450 addon development admission) are merged.
V2.4 is implemented with explicit fact gaps; it is not accepted slice content.

- [ ] Operator-deferred real-cache verification: explicitly select cache files and
  repeat exact-build/locale checks only after WoW closes. The owner chose synthetic
  fixtures on 2026-10-18 so they can continue playing; do not read mutable caches.

The continuation order is: policy reconciliation; durable extraction lifecycle;
cache/hotfix/observations; gameplay projections and guide facts; media/tooltips;
versioned delivery and guarded consumers; guides and exact inventory; desktop and
installer; performance/materials; substantial review/test/CI; development version
and publication; Pages redesign. Each is a separate independently reviewed PR.
See ADR 0006 §13 for the early development RC policy. None is a stable gate pass.

Continuation 2 merged as PR #192 at `1cb6bb9`, with independent review, all nine
exact-head checks, source workflow and a successful private real-client lifecycle
comparison. [Handoff/receipt](docs/handoffs/2026-10-18-extraction-lifecycle.md).
Active: continuation 3, selected cache/hotfix/observation inputs; Codex `/root`,
[claim](docs/claims/2026-10-18-local-inputs.md),
[handoff](docs/handoffs/2026-10-18-local-inputs.md).
Later input adapters, expanded projections, media, guarded gameplay consumers,
reviewed inventory, performance, coverage, publication and Pages remain pending.

## Gate snapshot

Clean baseline `7aecba8`, 2026-10-18: 69 blockers, `stable_ready: false`,
`candidate_ready: false`, zero verified evidence files and content packs.
[Exact ordered blocker inventory, owners and clearing conditions](docs/releases/2026-10-18-blocker-inventory.md).
Publication allowance for 0.9.2 is not readiness for a changed RC.

## Phase 0: housekeeping

- [x] **V0.1 Publish 0.9.2** (Both). Published 2026-10-17 as a prerelease after
  verifying the assets; see the [receipt](docs/handoffs/2026-10-17-v0.9.2-publication.md).
- [x] **V0.2 ADR 0006 and re-baseline** (Eng). This file, ADR 0006, STATUS, and
  supersession notes in the release contract.
- [x] **V0.3 glib alert** (Both). Done 2026-10-17: alert 1 (GHSA-wrw7-89jp-8q8g)
  dismissed as `not_used` with the `cargo tree` reasoning from the
  [V0 handoff](docs/handoffs/2026-10-17-v0.md).
- [ ] **V0.4 Signing** (Owner → Eng). SignPath Foundation, chosen by the owner. Eng
  part done ([handoff](docs/handoffs/2026-10-17-v0.4.md)): tag signing in the package
  workflow, a byte-level and Authenticode verifier, and the code signing policy.
  The home page now has the required "Code signing policy" section, and prepared
  application answers are in [CODE_SIGNING.md](docs/compatibility/CODE_SIGNING.md#application-prepared-answers),
  including an open question about non-OSI content packs. Remaining: the owner
  applies and configures SignPath
  ([setup](docs/compatibility/CODE_SIGNING.md#owner-setup-one-time)); then a
  `test_signing` run, and SmartScreen on the first signed installer.
- [x] **V0.5 CI recheck** (Eng). Done: the native smoke failed on PR #170 from a real
  startup race, fixed in PR #172 ([handoff](docs/handoffs/2026-10-17-v0.5.md)); it
  passes on later heads. Branch protection still requires the nine checks (owner to
  confirm they are the permanent set).
- [~] **V0.6 Grant intake**: dropped 2026-10-17 (ADR 0006 §11, local-only sources).

## Phase 1: contract and gate (one writer on `release.rs`, in order)

- [x] **V1.1 Release manifest schema v2** (Eng). Done
  ([handoff](docs/handoffs/2026-10-17-v1.1.md)): declared slice, per-domain
  out-of-scope boundaries distinct from `unknown`, and a `null` (pending) target
  that blocks every gate until V4.2 freezes it in code.
- [x] **V1.2 Hybrid delivery** (Eng). Done ([handoff](docs/handoffs/2026-10-17-v1.2.md)):
  `hybrid` policy mode with a tested-content digest binding, a readiness gate that
  needs declared digests and a valid receipt bound to the tested commit, and
  explicit `local-data receipt-*` commands. Remaining for later tasks: receipt
  preparation (V5.4), declaring the digests (V4.2), switching the committed policy,
  and the desktop match/mismatch indicator (needs V2.2 projections).
- [x] **V1.3 Signer rule** (Eng). Done ([handoff](docs/handoffs/2026-10-17-v1.3.md)):
  format 2 policies and receipts let the owner hold both keys when the payload binds
  `independent_review_sha256`; format 1 keeps distinct identities.
- [x] **V1.4 Slice acceptance matrix** (Eng). Done ([handoff](docs/handoffs/2026-10-17-v1.4.md)):
  evidence `matrix` rows; every class in every level band, and every zone and
  party size, must pass live. Blocks once until the slice classes are known. The
  typed party support matrix (`release_party.rs`) already exists; aligning its
  supported sizes with the slice is left to V2.4.

## Phase 2: content (up to three parallel worktree streams)

- [~] **V2.1 Granted-source adapters**: dropped 2026-10-17 (ADR 0006 §11). Quest,
  NPC and objective facts come from the owner's own in-game observations instead.
- [~] **V2.2 Local extraction projections** (Eng). First part done (PR #175,
  [handoff](docs/handoffs/2026-10-17-v2.2.md)): typed, sealed, reproducible
  projections of items, maps/UI maps, flight nodes and paths, classes and races from
  a verified research catalog (`local-data project` / `projection-verify`).
  Remaining: run it on the private 70425 catalog (development evidence only), stat
  amounts and slot rules, spells/talents, cache/hotfix import, activation/rollback,
  binding projection digests into hybrid delivery.
- [~] **V2.3 Guide port** (Eng). Importer and verifier done (PR #176,
  [handoff](docs/handoffs/2026-10-17-v2.3.md), [GUIDE_VERIFIER.md](docs/data/GUIDE_VERIFIER.md)):
  inert RXP importer, `verify2.py` replay checks with typed unknowns, `guide verify`.
  Remaining: a facts builder from the owner's addon observations plus V2.2 client
  projections, then per-class Human 1-20 routes with a Deadmines block (solo, duo).
- [x] **V2.4 Authored domains** (Eng). PR #188 merged at `7aecba8`:
  authored role heuristics (no WoWSims numbers), Deadmines preparation, solo/duo
  scenarios and reminders. [Handoff](docs/handoffs/2026-10-17-v2.4.md).
  Observable gameplay facts, activation and release admission remain pending.
- [ ] **V2.5 Inventory and review** (Both). Replace each `<domain>-inventory-pending`
  with slice units and exact required IDs; independent enumeration review; owner
  confirms disagreements in client; passing coverage rows.
- [ ] **V2.6 Release materials** (Eng). Notices, licences and corresponding source
  for every bundled source; no Blizzard art or client fonts; no private catalogs in
  public assets.

## Phase 3: polish (before the RC freeze)

- [x] **V3.1 Optional review items** (Eng). Done (PR #177,
  [handoff](docs/handoffs/2026-10-17-v3.1.md)): 14 fixed, 1 decided-not-to-fix with
  a guard test. Native WebView2, Windows contrast themes and screen readers are left
  for owner acceptance (V5.3). Open: whether Sessions, Content and Imports should
  also follow "one primary button per region".
- [ ] **V3.2 Locales in the desktop** (Eng) for extracted non-enUS catalogs.
- [ ] **V3.3 Performance workload** (Eng) for the full slice (WIN-PERFORMANCE).

## Phase 4: after launch (4 Nov 2026)

- [ ] **V4.1 Live build tuple** (Owner) from `.build.info` and in game. A daily
  [build watch](docs/compatibility/BUILD_WATCH.md) opens an issue when Blizzard's public
  version list shows a new Forever build or product code.
- [ ] **V4.2 Freeze the target** (Eng). Manifests, delivery policy, addon/edition
  data; re-extract; re-fetch snapshots; rebuild and verify packs; review deltas.
  Never relabel 70425 artifacts.
- [ ] **V4.3 Live development pass** (Owner). Install the addon and run diagnostics.

## Phase 5: candidate and acceptance

- [ ] **V5.1 Acceptance-candidate freeze** (Eng). A later `1.0.0-rc.N`
  after `candidate-readiness` exits 0; `Build-Windows.ps1 -AcceptanceCandidate`;
  verify assets. The early development `1.0.0-rc.1` is a separate continuation
  milestone (ADR 0006 §13), packaged without that switch; it cannot clear V5.1.
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

Order: V0 → V1.1 → V1.2-V1.4 in parallel with V2.2-V2.4 → V2.5 → V2.6 and V3 →
launch → V4 → V5 → V6.
