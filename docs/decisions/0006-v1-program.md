# ADR 0006: The 1.0.0 program

Accepted 2026-10-17. Baseline: `3b19812` (`origin/main`, the v0.9.2 source).
Recorded by task V0.2 as plan section 21.8 requires for material deviations. The
owner gave these decisions on 2026-10-17 in answer to the open questions in the
G8 `TODO.md`. They replace the unbounded "every 70374 record" target with a
declared, finite 1.0.0 slice. Within that slice no gate is weakened: unknown,
unsupported, restricted, stale, false and zero stay distinct, and every pass still
needs recorded, hashed evidence.

Nothing in this ADR changes code or manifests by itself. Contract and gate changes
land in their own V tasks (see [TODO.md](../../TODO.md)). Until V1.1 lands,
`release-readiness` keeps reporting the original v1 blockers.

## Decisions

### 1. Target: the first stable live build after launch

WoW Forever launches on 4 Nov 2026. The installed client today is the beta
(`wow_classic_beta 1.60.1.70425`). The 1.0.0 target moves from `1.60.1 / 70374 /
16001` to the first stable live build after launch, recorded by the owner from
`.build.info` and in game (task V4.1).

- Engineering continues against 70425 as development input (ADR 0005 §3 still
  governs it: development admission only, never acceptance).
- After launch the target is frozen in `release/v1/` and the delivery policy, the
  client is re-extracted, and packs are rebuilt and re-validated. No 70374 or
  70425 artifact is relabelled to the live build.
- Development evidence on 70374 stays on record as historical development evidence.

### 2. Scope: a declared 1.0.0 slice

1.0.0 covers this slice, with all nine content domains complete within it:

| Dimension | In scope |
| --- | --- |
| Race / faction | Human (Alliance) |
| Levels | 1 to 20 |
| Zones | Northshire Valley, Elwynn Forest, Westfall, Redridge Mountains, plus the capital/travel points the routes use |
| Dungeon | The Deadmines |
| Classes | Every class open to Human at launch (Warrior, Paladin, Rogue, Priest, Mage, Warlock, plus any Forever addition) |
| Party sizes | Solo and duo |

Everything outside the slice is a recorded **out-of-scope boundary**. It is a
separate state from `unknown` (not assessed) and from `unsupported`; release notes
list it. Later 1.x releases widen the slice. The mandatory domain and acceptance
lists stay fixed in code; the slice bounds their units, not their existence.

### 3. Sources

All of the following may supply 1.0.0 content, each with its origin edition/build
preserved and each fact validated against the frozen target:

- Local extraction from the user's own client (`azeroth-local-data`): items, maps,
  taxi data, classes and spells. The client has no quest objective or prerequisite
  tables, so quests can't come from it.
- Published-licence sources already reviewed: AtlasLootClassic, WoWSims, ATT,
  GuideMate, Mapzeroth, within their admitted scopes.
- Original authoring and operator observation through the permitted addon.
- The owner's own RXP-style leveling guides and guide generator (kept outside the
  repo at the owner's machine). The owner holds the rights and grants full use.
- ~~Owner-reported written grants from ForeverDB, Questie/QuestieDB,
  foreverchanges.pro and Olympus News Network.~~ **Withdrawn 2026-10-17** (see
  §11): no third-party website data or grants are used.

Retrieval from websites remains a separate, bounded, rate-limited, opt-in fetch step
with retained snapshots and inert parsing. Builds and tests never use the network.
RestedXP's own published guides (noncommercial licence) remain excluded.

### 4. Grant evidence

The owner exports each written grant (email or message). The export is stored as
private admission evidence, hashed, outside Git and public assets, and referenced
from the pinned `admission_registry`. For each source the admission entry records
the granting party, date, covered material, and whether redistribution, modification
and commercial use are allowed, plus required attribution/notice text and revocation
terms. A grant that doesn't cover redistribution admits that source for the
local-built path only (§5), never for bundled packs.

### 5. Hybrid delivery, locally built content in the gate

Redistributable content ships as bundled packs under the existing binding. Content
that can't be redistributed (client data, and any source whose grant excludes
redistribution) is built locally by each user. The stable gate accepts locally
built content when it is bound by a **tested-content digest**: pinned extractor
profile hash, metadata revision and expected semantic-catalog digest per
build/locale, plus signed receipts (task V1.2). A user whose local build doesn't
reproduce the digest sees that state explicitly; it never silently passes.

### 6. Receipt signers

The local-catalog receipts require two signer identities. The owner holds both the
operator and reviewer keys. Instead of two distinct people, the reviewer receipt
must bind the SHA-256 of an independent review record (by an AI reviewer in an
isolated worktree, or a human). This independence is weaker than two people, and
release notes state it. Unknown, duplicate, revoked and noncanonical receipts still
refuse (task V1.3).

### 7. Locales

enUS is required and tested. Other locales installed in the user's client may be
extracted locally. Bundled packs stay enUS.

### 8. Acceptance

The 15 `LIVE-*` and 10 `WIN-*` rows stay. Slice rows (task V1.4) cover every class
in every level band (1-10, 11-20), and every zone and party size of the slice in at
least one row; with six classes that's at least 12 live rows. The Deadmines is
covered by the existing `LIVE-DUNGEON` row, not by a matrix dimension. The owner is the operator, with two accounts
for duo rows, a second machine or clean VM for bare-machine Windows rows, and
NVDA/Narrator for accessibility.

### 9. Releases, signing and alerts

- Prereleases are `1.0.0-rc.N` only, published as GitHub prereleases. No further 0.x.
- 1.0.0 ships **Authenticode-signed** through a cloud signing service the owner
  provisions, wired through `NAVZEROTH_SIGN_COMMAND`.
- Dependabot alert GHSA-wrw7-89jp-8q8g (`glib` 0.18.5): `cargo tree` shows it only
  on the Linux GTK stack, not for `x86_64-pc-windows-msvc` (task V0.3 record). The
  owner dismisses it with that reasoning.
- All 15 optional review items from G4 to G6 are fixed before the RC freeze.

### 10. Authority and execution

For this program the owner authorizes routine feature-branch pushes, merging the
program's PRs to protected main after every required check passes on the exact
head and independent review, pushing `v1.0.0-rc.N` tags and publishing them as
prereleases, and, **once `release-readiness --version 1.0.0` reports
`stable_ready: true`**, tagging `v1.0.0` and publishing it as stable. This does not
authorize skipping or retrying away a failing check, force-pushing main, rewriting
published history, purchases or account creation. Parallel worktree-isolated
agents are allowed under ADR 0005 §1 rules (one writer per task).

### 11. Amendment 2026-10-17: local-only sources

The owner withdrew the grant path (§3 fourth bullet, §4). 1.0.0 content comes only
from data the project can gather itself:

- the user's own installed client, through local extraction (hybrid delivery, §5);
- the user's own in-game observations through the permitted addon (quests, NPCs,
  objectives and turn-ins, which the client tables don't contain);
- published-licence material already admitted and retained in the repository;
- original authoring, including the owner's guide designs (their quest and
  coordinate references are route design inputs, confirmed by observation before
  they count as content).

No website retrieval, scraping or provider correspondence is part of the program.
Task V0.6 (grant intake) and V2.1 (granted-source adapters) are dropped. §4 no
longer applies; no grant evidence was admitted or stored.

### 12. Amendment 2026-10-17: build 70450 as development input

The owner's installed beta client updated from 70425 to `1.60.1.70450` (installation
metadata read on 2026-10-17; Blizzard's public version list reports the same for
`wow_classic_beta`). On the owner's request, task V2.2b admits 70450 for local
extraction and projection on the same terms as 70425 (ADR 0005 §3): development
input only, never a release target, never relabelled, `target_tuple_matches` false,
no nearest-build fallback. The admitted local-data builds are listed once in
`ADMITTED_BUILDS` (`crates/local-data/src/metadata.rs`). The in-game addon does not
admit 70450 yet: its admission waits for the interface number read from the client.

## Consequences

- The fingerprint invalidates all LIVE/WIN evidence on any production change, so
  polish and gate work come before the RC freeze. Post-freeze product fixes produce
  a new rc and re-run evidence (the re-run rule is decided in task V5.5).
- Packs built on 70425 are research inputs until rebuilt and validated on the live build.
- Release notes for 1.0.0 state the slice, every out-of-scope boundary, the signer
  arrangement (§6) and the source attributions.
- Superseded in part: the 70374 target and whole-world inventory scope of
  [release/v1/README.md](../../release/v1/README.md) and
  [V1_RELEASE_PROCESS.md](../releases/V1_RELEASE_PROCESS.md), and ADR 0005 §3's
  first bullet (the release target). ADR 0005's other decisions stand.
