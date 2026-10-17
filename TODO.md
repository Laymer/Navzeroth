# What 1.0.0 still needs

Target: Forever `1.60.1 / 70374 / interface 16001`, all nine content domains.
Written 2026-10-17 by task G8 against `origin/main` at `1806028`
(source version 0.9.1; 0.9.2 is a separate, still-prerelease program). Nothing in
this file is a promise of a date, and nothing here has been verified unless it says so.

Owner types: **Owner** = the project owner or a Windows/WoW operator who has the
client, the machine or the authority; **Eng** = engineering work an agent or
developer can do in the repository; **Both** = needs an owner decision and then
engineering.

## Where this comes from

The numbers below come from running the repository's own gate on `1806028`
(the `main` source of v0.9.1, since the requested version must equal the declared
version; `--version 1.0.0` is refused today with "Requested release version differs
from application, addon or lockfile"):

```sh
cargo run --locked -p azeroth-cli -- release-readiness --root . --version 0.9.1
```

Reported: exit 0, `publication_allowed: true` (preview only), `stable_ready: false`,
`candidate_ready: false`, **66 blockers**, 26 pending entries (1 initial tested
binding plus 25 observations), `verified_evidence_files: 0`,
`verified_content_packs: 0`, inventory SHA-256
`1b7b7f1fd30a4a76e8bd363beaf31e505a61d6d648de030df9980eeee5d1ea21`. Re-run it
before trusting any count here; this file is a snapshot. Contract:
[release/v1/README.md](release/v1/README.md). Process:
[docs/releases/V1_RELEASE_PROCESS.md](docs/releases/V1_RELEASE_PROCESS.md).

Rule that applies to every item: unknown, unsupported, restricted, stale, false and
zero stay distinct. A pass needs a recorded method, operator, time, environment and
hashed evidence; synthetic, mocked, hosted-CI and Linux results never stand in for
live or interactive Windows rows ([AGENTS.md](AGENTS.md)).

## 1. The 66 readiness blockers, grouped

| # | Category | Count | Blocker numbers in the report |
| --- | --- | ---: | --- |
| A | Content inventory incomplete (one per domain) | 9 | 1, 3, 5, 7, 9, 11, 13, 15, 17 |
| B | Required records unknown (`<domain>-inventory-pending`) | 9 | 2, 4, 6, 8, 10, 12, 14, 16, 18 |
| C | Required record inventory empty or excessive | 1 | 19 |
| D | Accepted source revisions missing or invalid | 1 | 20 |
| E | Initial tested source revision and production fingerprint pending | 1 | 21 |
| F | Acceptance observation pending: 15 `LIVE-*` | 15 | 22 to 36 |
| G | Acceptance observation pending: 10 `WIN-*` | 10 | 37 to 46 |
| H | Unit coverage missing, duplicated or excessive | 1 | 47 |
| I | Mandatory unit coverage has not passed (one per domain) | 9 | 48 to 56 |
| J | Validated offline content packs missing or invalid | 1 | 57 |
| K | No validated pack record binding (one per domain) | 9 | 58 to 66 |
| | **Total** | **66** | (9+9+1+1+1+15+10+1+9+1+9) |

Every category except E, F and G clears only when real content, rights and
acceptance exist; E, F and G clear only when evidence is recorded against a frozen
candidate. No category can be cleared by editing a manifest alone.

## 2. The nine 70374 content-domain inventories (all unknown today)

Each is currently a single placeholder unit `<domain>-inventory-pending` with
required records `null`, and accounts for blockers A, B, I and K above (36 of 66).
The `null` means unknown; it is not "empty" and not "unsupported". A justified known
empty unit would be `[]`. The nine rows are enumeration tasks, not a claim to list
every game record: replace them with audited region/model/party coverage units.

- [ ] **quests**
- [ ] **equipment**
- [ ] **acquisition**
- [ ] **geography**
- [ ] **travel** (directed)
- [ ] **dungeons** (dungeon/preparation)
- [ ] **class_roles**
- [ ] **party** (party support/scenarios)
- [ ] **reminders**

For each domain:

- What: an independently reviewed, exhaustive enumeration of the 70374 target
  (units with ID, scope and exact required record IDs), then validated pack records
  that match those IDs exactly, then a passing coverage row per unit.
- Why it blocks: unknown denominator means completeness cannot be claimed
  (blockers A, B, C, H, I, K).
- Verified by: `release-readiness` (the clearing signal), `inventory-report`
  ([docs/data/RELEASE_INVENTORY.md](docs/data/RELEASE_INVENTORY.md)), pack verifier,
  and an independent target review. A count copied from staged source records is not
  a completeness claim.
- Owner type: **Both**. Eng builds adapters, packs and validators; the exhaustive
  target review and any in-client confirmation need an operator on 70374.
- Governing docs: [release/v1/README.md](release/v1/README.md),
  [docs/data/COVERAGE.md](docs/data/COVERAGE.md),
  [docs/data/V1_MODEL_BINDING_AUDIT.md](docs/data/V1_MODEL_BINDING_AUDIT.md).

Known extra gap (blocker K, per release README): no validated pack yet supplies
class/role, party-scenario or dungeon preparation records. The v2 pack path can bind
them, and the CLI keeps blocking their absence. Real, validated packs are a stated
prerequisite, in addition to rights and acceptance.

Current evidence state, unchanged by this task: 1,685 reference facts with zero
verified target-usable facts; ForeverDB staging separates 557 Forever-labelled from
3,491 Classic-derived quest records (373 overlapping IDs) and is raw staging only,
with no playable activation ([docs/STATUS.md](docs/STATUS.md)). The local-data
extraction subsystem decoded 505 of 1,346 table candidates from build 70425/enUS
(research only; not 70374 content, not release evidence;
[docs/data/LOCAL_EXTRACTION.md](docs/data/LOCAL_EXTRACTION.md)).

## 3. Rights and licensing for content (blocker D, plus delivery)

- [ ] **Accepted source revisions with admitted rights.** What: for each source that
  will ship, an immutable 40-hex revision, retrieved-file hashes, licence,
  attribution, permitted transformations and redistribution evidence, bound by a
  pinned `admission_registry`. Why: `unknown`, `conditional` and `prohibited`
  permissions block publication; public availability is not permission. Verified by:
  `release-readiness` (blocker D), source admission review. Owner: **Both** (owner
  authority for permissions and any publisher contact; Eng for the evidence
  packaging). Docs: [docs/sources/source-admission-v1.md](docs/sources/source-admission-v1.md),
  [docs/data/SOURCES.md](docs/data/SOURCES.md),
  [docs/sources/independent-source-strategy.md](docs/sources/independent-source-strategy.md).
  Today: four narrow published-licence scopes are admitted (AtlasLootClassic
  declarations, Mapzeroth Retail node literals, a WoWSims pointer, ATT Forever
  Riverglades point declarations). No ForeverDB material is admitted; the 0.34.0
  addon advertises GPLv3 with corresponding-source scope and 70374 validation still
  pending. The historical atlas has `distribution_eligible=false`.
- [ ] **Corresponding source, notices and licence files shipped** with the approved
  content assets, matching the release asset manifest exactly. Owner: **Eng**,
  after rights are settled. Docs: [docs/releases/RELEASE_MATERIALS.md](docs/releases/RELEASE_MATERIALS.md).
- [ ] **No Blizzard art, textures or client fonts in the app or addon.** ADR 0005
  decided original art and bundled OFL fonts (Inter, Cinzel). Keep it true at
  freeze; `site/assets/dragon-citadel.jpg` stays unbundled (no recorded rights).
  Owner: **Eng**. Docs: [docs/decisions/0005-v092-program.md](docs/decisions/0005-v092-program.md).
- [ ] **Private client data stays out of Git and public assets.** The local-data
  catalogs are private; any release use needs the unfinished signed-receipt and
  delivery-policy integration. Owner: **Both**. Docs: [localdata.md](localdata.md),
  [docs/data/LOCAL_DATA.md](docs/data/LOCAL_DATA.md).

## 4. Candidate-bound live acceptance (blockers E and F; 15 `LIVE-*` rows)

All rows are `unverified` and must become `pass` with method `live_game`, operator,
time, environment and hashed evidence, bound to the frozen candidate's tested source
commit, production fingerprint and inventory hash. Build 70425 is development
admission only (ADR 0005); **an observation on 70425 cannot satisfy a 70374 row**,
and `target_tuple_matches` stays false for it. A shallow checkout that lacks the
tested commit cannot pass.

- [ ] Freeze the candidate: complete content first, then freeze versions, the
  inventory hash, tested commit and production fingerprint (blocker E). Owner:
  **Both**. Verified by: `candidate-readiness`, then `Build-Windows.ps1
  -AcceptanceCandidate` (cannot be built while content is unknown).
- [ ] `LIVE-BUILD`, `LIVE-CAPTURE`, `LIVE-TRANSFER`, `LIVE-RELOAD`,
  `LIVE-RESTRICTIONS`, `LIVE-HUD`, `LIVE-PLAN`, `LIVE-ADVANCE`, `LIVE-DEVIATION`,
  `LIVE-DUO`, `LIVE-SESSION`, `LIVE-NEXT-PLAN`, `LIVE-REMINDERS`, `LIVE-GEAR`,
  `LIVE-DUNGEON`. Run `LIVE-BUILD` first. Owner: **Owner/operator** (needs the 70374
  client and characters; `LIVE-DUO` needs two characters with differing progress).
  Why it blocks: pending rows block candidate and stable readiness. Verified by:
  the runbook procedure, evidence files in the manifest, independent review.
  Docs: [docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md](docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md),
  [docs/compatibility/GUIDED_ACCEPTANCE_KIT.md](docs/compatibility/GUIDED_ACCEPTANCE_KIT.md).
- [ ] Expand rows to every mandatory inventory unit (class, role, level, region,
  party size); the aggregate duo run alone does not establish full-content
  acceptance. Owner: **Both**. Docs: runbook, last section.
- Already on record and not promoted: development-only captures on 70374 (level 13,
  five completed and nine incomplete quests; a five-event turn-in session) and an
  installed-addon diagnostic. They are development evidence, not candidate-bound
  rows ([docs/compatibility/FOREVER_70374_EVIDENCE.md](docs/compatibility/FOREVER_70374_EVIDENCE.md)).
  Restricted and paused cases are still unverified.

## 5. Native and interactive Windows checks (blocker G; 10 `WIN-*` rows)

Method must be `interactive_windows`; `automated` cannot satisfy it. Hosted
Windows smoke and PowerShell reachable from the Linux session do not count.

- [ ] `WIN-INSTALL`, `WIN-UPGRADE`, `WIN-UNINSTALL`, `WIN-RECOVERY`, `WIN-OFFLINE`,
  `WIN-FOCUS`, `WIN-REFERENCE`, `WIN-DISPLAY` (scale, monitors, supported game
  modes), `WIN-ACCESSIBILITY`, `WIN-PERFORMANCE` (recorded hardware, declared
  full-size workloads). Owner: **Owner/operator**. Verified by: runbook plus hashed
  evidence. Docs: [docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md](docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md),
  [docs/compatibility/WINDOWS_INSTALL.md](docs/compatibility/WINDOWS_INSTALL.md).
- [ ] Interactive installer pages, bare-machine prerequisites and selected-WoW-path
  dialogs, never exercised by a human on Windows
  ([0.9.1 publication record](docs/handoffs/2026-10-16-v0.9.1-publication.md)).
  Owner: **Owner/operator**.
- [ ] WebView2 (the shipped renderer) visual fidelity of the redesign: backdrop,
  90% shell sheet, nested flat panels, pressed buttons, white-flash fix, legibility
  over the backdrop at other sizes, and container-query support (needs Chromium 105
  or later; only Playwright's Chromium has run it). Owner: **Owner/operator**.
  Docs: [docs/handoffs/2026-10-17-g6.md](docs/handoffs/2026-10-17-g6.md).
- [ ] Hosted "Native WebView2 smoke over CDP" must pass on the head that fixed the
  exact-label lookups in `scripts/windows/Native-Smoke.mjs` (G6 follow-up commit
  `81aef88`). Recorded as unverified in the G6 handoff; check the CI run, do not assume.
  Owner: **Eng**.

## 6. Addon live-client GUI, taint and focus checks

`LIVE-GUI` in the runbook is supporting evidence, not a release-gate row, and every
item is unverified: the Lua mock cannot show real frames, templates, taint or focus.
Run on the recorded 70374 client; a 70425 run is development evidence only.
Owner: **Owner/operator**. Docs: [docs/compatibility/ADDON_GUI.md](docs/compatibility/ADDON_GUI.md),
runbook section "LIVE-GUI".

- [ ] Entry points: `/anav`, the minimap button and Options, AddOns, including that
  Escape closes the window and leaves the game menu usable.
- [ ] Templates fall back cleanly; no missing textures or unreadable text.
- [ ] Every control has a tooltip with title, body, `Slash command:` and `Gate:`.
- [ ] Combat: actions refuse, `/anav` and the minimap button refuse to open, no error
  or taint message.
- [ ] Taint, focus and action bars: opening does not take keyboard focus; no
  "action blocked", changed keybinding or action-bar change.
- [ ] Minimap drag and persistence across `/reload`; hide and restore.
- [ ] Plan box: real ANAV1 paste up to 32 KiB, oversize paste refused not truncated.
- [ ] Clock: Check clock and Trust clock follow compare-then-confirm; Trust clock is
  unavailable without a check and after 120 seconds.
- [ ] Settings persist across `/reload` in `AzerothNavigatorDB.settings`.

## 7. Screen-reader and WebView2 accessibility checks

Never run with assistive technology; all remain unverified. Owner: **Owner/operator**
(a Windows machine with Narrator or NVDA/JAWS). Verified by: a recorded session
feeding `WIN-ACCESSIBILITY`. Docs: runbook `WIN-ACCESSIBILITY`;
[docs/reviews/2026-10-17-g4.md](docs/reviews/2026-10-17-g4.md).

- [ ] HelpTip announcement (always-mounted `role="tooltip"` bubble with
  `aria-describedby`) and the "Help: <label>" names, for about 50 triggers.
- [ ] Escape dismissal of hover-opened help, and keyboard reach of help inside busy
  or disabled fieldsets.
- [ ] High-contrast and forced-colours rendering in WebView2, including text
  selection and the pressed-button cue.
- [ ] 320 px and 200% text layouts judged by eye (e2e asserts no horizontal
  overflow only), rail/bar switch honouring the Settings text scale.

## 8. Signing and SmartScreen decision (owner)

- [ ] **Decide** whether 1.0.0 ships Authenticode-signed. Today the installer is
  unsigned and SmartScreen warns; signing is optional through the repository secret
  `NAVZEROTH_SIGN_COMMAND`, and obtaining a certificate or cloud signing account is
  an owner decision ([AGENTS.md](AGENTS.md)). If signed: provision the secret, run a
  signed build, and have a human record SmartScreen behaviour on Windows. If not:
  state in the 1.0.0 notes that it is unsigned. SmartScreen behaviour of the
  unsigned build is unverified. Owner: **Owner**. Docs:
  [docs/compatibility/WINDOWS_INSTALL.md](docs/compatibility/WINDOWS_INSTALL.md),
  `.github/workflows/package-windows.yml`.

## 9. Known external blockers and repository settings

- [ ] **Dependabot alert GHSA-wrw7-89jp-8q8g (medium) on `glib`.** Read-only check on
  2026-10-17: alert number 1 is still `open`. `glib` 0.18.5 is a Linux-only transitive
  dependency of the Tauri GTK stack; the fix needs gtk-rs 0.20 and automatic updates
  fail because gtk-rs 0.18 is pinned upstream. Earlier notes say Windows does not use
  it (not re-verified here). Why it matters: an open advisory on a stable release.
  Needed: upgrade when Tauri allows it, or an owner-recorded dismissal with the
  reasoning. Owner: **Both** (dismissal is the owner's; the upgrade is Eng). Docs:
  [docs/reviews/2026-10-13-wave1-main.md](docs/reviews/2026-10-13-wave1-main.md).
- [ ] **Branch protection required checks.** Read-only API query on 2026-10-17 shows
  nine required contexts on `main`: Rust format, Rust Linux, Rust Windows, pnpm Linux,
  pnpm Windows, Lua static checks, Desktop shell Windows, Mocked desktop Chromium,
  Windows installer and addon package. [docs/STATUS.md](docs/STATUS.md) still says
  only five are enforced; that text is stale relative to the live setting (STATUS is
  being edited by another task and was not touched here). Remaining decision: confirm
  the nine are the intended permanent set, and re-check that the setting is still in
  place on the exact merge candidate. ADR 0005 authorizes making every CI check
  required for the 0.9.2 program only. Owner: **Owner**.
- [ ] **Independent review of the exact merge candidate** and fresh required checks
  on the exact head (release process, step 3). Owner: **Both**.
- [ ] **Release steps once the gates pass:** matching versions in the six declarations
  and eight internal `azeroth-*` lock entries, `docs/releases/1.0.0.md`, tag
  `v1.0.0` on the accepted commit, tag packaging and draft-release jobs, download and
  verify every asset (hashes, sizes, source commit, notices), then publish stable and
  re-verify. The five-file packager also needs validated content and source packaging
  to meet this scope. Main push, merge and release need explicit owner
  authorization; the 0.9.2 program authority does not extend to 1.0.0. Owner:
  **Both**. Docs: [docs/releases/V1_RELEASE_PROCESS.md](docs/releases/V1_RELEASE_PROCESS.md).
- Explicitly deferrable by the release contract, only with a stated boundary: official
  API, party messaging, combat-log analysis, hosted distribution. They cannot waive a
  core offline workflow. Identity image is deferred until after a verified 1.0.0.

## 10. Carried optional review items (G4, G5a, G5b, G6)

None is a blocker; none blocks the readiness gate. All are **Eng**. Source reviews:
[G4](docs/reviews/2026-10-17-g4.md), [G5a](docs/reviews/2026-10-17-g5a.md),
[G5b](docs/reviews/2026-10-17-g5b.md), [G6](docs/reviews/2026-10-17-g6.md).
G4 optional 4 to 9 and G5a 8 to 11 were addressed by G6 (per its handoff); the
G4/G5a/G5b/G6 important findings were fixed in their own tasks.

- [ ] G4 #10: handoff wording in `docs/handoffs/2026-10-17-g4.md` (docs only).
- [ ] G5a #5 to #7: planner "Clear objective selections" help also omits exclusions
  (Clear tip was folded in; the remainder is open); profile help understates what a
  profile change discards; two Gear help texts rely on behaviour the screen does
  not state.
- [ ] G5a #12: dropped `.saved-plan` and `.marker-panel` class hooks (no CSS targets
  them; confirm nothing external does).
- [ ] G5a #13: several `primary` buttons per screen (Gear, Calibration); decide
  emphasis.
- [ ] G5a follow-ups: a 3:1 border pair for `--tone-warn`; per-item labels (category
  status, objective node, acquisition source) still plain labels without Help.
- [ ] G5b O5: Help triggers inside busy `fieldset disabled` leave the tab order
  during an import or save (G5a made the trigger a `span role="button"`; re-check the
  Import and Settings case).
- [ ] G5b O7: no baseline measurement of the Compatibility screen on the pre-G5b
  main at 320 px / 200%.
- [ ] G6 O1: nested-panel e2e asserts nested panels exist only for Party; add the
  assertion per screen known to nest.
- [ ] G6 O2: Panel nesting context crosses portals; wrap portal or modal content in
  `InsidePanel.Provider value={false}` if a dialog is ever added.
- [ ] G6 O3: no test for the viewport at the 58em container-query threshold or the
  unsupported-engine path.
- [ ] G6 O4: pressed Button rule specificity vs forced-colours; no high-contrast or
  forced-colours render test for `aria-pressed`; redundant `aria-pressed` prop type.
- [ ] G6 O5: `.play-screen .ui-button` margin now applies to Atlas pin and route hop
  buttons.
- [ ] G6 O6: weak "stops listening for resize" assertion in HelpTip tests and
  `toHaveCount(0)` vacuity risk with `exact` selectors.
- [ ] G6 O8: same resize-listener assertion strength note.

## 11. Not part of the 1.0.0 gate but easy to confuse with it

- The 0.9.2 prerelease (separate PRs under ADR 0005). It stays a developer
  prerelease and does not change any count above.
- Build 70425 captures and the local-data subsystem: development and research
  inputs only.
- Hosted CI, mocked Chromium, Linux harness runs and synthetic PowerShell checks:
  necessary, never sufficient.
