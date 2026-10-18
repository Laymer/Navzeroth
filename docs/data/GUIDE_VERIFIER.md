# RXP guide importer and replay verifier

Task V2.3 (ADR 0006 §2, §3 and §11). Code: `crates/ingest/src/rxp.rs` (importer),
`crates/ingest/src/rxp_verify.rs` (verifier and fact sources),
`crates/cli/src/guide.rs` (CLI). Ported from the owner's guide generator
(`gen.py`, `verify2.py`, kept outside the repository), which the owner authored
and gave permission to port. That Python and the generated Lua were read as source
and never executed.

## What the guide is, and what it is not

Under ADR 0006 §11 (owner decision of 2026-10-17), 1.0.0 sources are local only:
the user's own client data, the user's own in-game observations, admitted
published-licence material and original authoring. The quest IDs, NPCs and
coordinates in the owner's generated guides came from QuestieDB and a website
scrape, so they are **not admitted content**. A guide is route design: an order of
quest and travel steps whose facts must each be confirmed later by local
observation. The verifier shows which facts are confirmed, contradicted or still
unknown.

## Importer

`rxp::parse(bytes)` scans the file line by line for the exact call form
`RXPGuides.RegisterGuide([[ ... ]])` and takes each long-string body as data. No
Lua is run and the crate has no Lua engine. Inside each body it parses:

- header: `#name`, `#displayname`, `#next a;b`, `#group`, `#subgroup`,
  `#defaultfor`, `#version`, edition tags (`#forever`, `#classic`, `#era`, `#som`,
  `#tbc`, `#wotlk`, `#cata`) and the guide `<< ...` filter;
- `step` and `step << filter`, step tags `#optional`, `#sticky`, `#label`,
  `#completewith`, `#requires`, `#level`;
- directives `.accept [-]id[,n[,nid]]`, `.turnin [-]id[,reward]`, `.complete id,n`,
  `.abandon`, `.goto`/`.waypoint Zone,x,y[,...]`, `.fly`, `.fp`, `.zone`, `.train`,
  `.trainer`, `.xp` (all documented forms), `.maxlevel`, `.home`, `.hs`, `.vendor`,
  `.mob`, `.use`, `.collect item,n[,quest[,mask[,-1]]]`, `.gossipoption`,
  `.deathskip`, `.isOnQuest`, `.isQuestComplete`, `.isQuestTurnedIn`,
  `.isQuestAvailable`;
- notes `>>text`, `*text`, `+text`, `--` comments and line filters
  (`.train 1 >>text << Warrior`).

Filters follow the RXP documentation: `/` is OR, space is AND, `!` is NOT, with
precedence `!` > space > `/`. Races, classes and factions are evaluated. Any other
token (`tbc`, misspellings) is unsupported and never guessed.

Nothing is dropped. An unknown directive, tag, header line or text line is kept
as an explicit unsupported item with its line number. A known directive whose
required arguments don't parse is malformed. A known directive with undocumented
extra arguments (`.accept 1,junk`, `.turnin 1,2,3`) is unsupported, never
silently trimmed. Structural problems (unclosed block, `]]` without `)`, stray
`]]`, code outside a block) are recorded with their lines. Every body line,
header or step, is also scanned for the owner's placeholder fragments
(`Doodad`, `<TXT>`, `[PH]`, ...).

Bounds: 4 MiB file, 2048-byte lines, 512 guides, 4096 steps per guide, 32768
steps, 256 lines per step, 64 header lines, 160-byte labels. Oversize input is
refused as a whole, never truncated. Non-UTF-8 or NUL bytes are refused.

## Verifier

`rxp_verify::verify(file, facts, character, origin)` replays every guide in file
order for one race and class, as `verify2.py` does. A step is skipped as
`filtered` when its `<<` filter excludes the character, and as `conditional` when
a condition fails: `.isOnQuest`, `.isQuestComplete`, `.isQuestTurnedIn`,
`.isQuestAvailable` (lines excluded by their own `<< filter` are not evaluated),
or `#requires label` before a step with that label has replayed.

Checks (each finding has a check name, status, line and optional quest ID):

| Check | Fails when |
| --- | --- |
| `bracket_balance` | a structural issue was found, or no guide block exists |
| `header_required`, `header_duplicate`, `header_tag`, `edition`, `guide_name`, `guide_filter` | `#name`/`#group`/`#version` missing, a tag repeated, a malformed `#version`, not exactly one edition per guide or several editions in one file, a repeated guide name, two guide filters |
| `facts_edition` | the guide's edition is not the facts' edition (unverified when the facts' edition is unknown) |
| `facts_build` | unverified when the facts don't name a client build |
| `next_guide` (warning) | a `#next` name is not a guide in this file |
| `quest_exists` | the fact source says the quest is absent |
| `double_accept`, `re_accept_done` | the quest is already in the log, or was turned in |
| `prerequisite_any`, `prerequisite_all` | none of / not all of the prerequisites were turned in before accept |
| `required_level` | the required level is above the `.xp` gate level and the XP estimate. A warning when only the estimate reaches it; `asserted` when only the `N-M` section name claims it |
| `race_allowed`, `class_allowed`, `repeatable`, `seasonal` | the quest is not available to the character, or is repeatable or seasonal |
| `quest_log_capacity` | the log exceeds the fact source's capacity |
| `complete_in_log`, `objective_index` | `.complete` for a quest not in the log, or an objective index out of range |
| `turn_in_in_log`, `double_turn_in` | turn-in before accept, or a second turn-in |
| `objectives_marked` (warning) | turn-in with fewer objectives marked than the quest has |
| `reward_index` | `.turnin id,n` beyond the reward count |
| `zone`, `coordinate`, `flight_node`, `train_spell` | unknown zone; coordinate outside 0-100 (both negative is accepted as RXP's dashed-line form); unknown flight node (a zone name is also accepted, as in `verify2.py`); spell not trainable by the class |
| `placeholder_text` | any line contains a placeholder fragment |
| `step_location` (warning) | a step has directives but no `.goto` or travel |
| `abandon`, `left_in_log` (warnings) | abandoning a quest not in the log; quests left at the end |
| `directive` | malformed (error) or unsupported directive |
| `step_tag` | an unknown tag; `#level n` when no `.xp` gate guarantees n; `#completewith` on a step with accept/turn-in/complete/abandon lines (they may not run); all unsupported. An error for `#requires` naming a label no step defines |
| `step_filter`, `line_filter`, `header_line`, `text` | unsupported item |

Statuses are `ok`, `error`, `warning`, `unverified`, `unsupported` and
`asserted`. **An unknown fact is `unverified`, never `ok`.** `asserted` means the
guide claims something with no fact behind it; it is not ok either.

- `replay_clean`: zero error, unverified, unsupported and asserted findings.
  Warnings don't affect it.
- `passes`: `replay_clean` and facts whose origin claims evidence. Synthetic
  facts never pass.

`#optional` steps and `-id` lines follow `verify2.py`. A pickup there never
enters the log; `.accept -id` while already on the quest is a no-op, not a double
accept. A turn-in of a quest not in the log is skipped silently. A turn-in of a
quest in the log removes it and gains its XP, like every other turn-in.

The XP estimate ports the owner's model: quest XP times a 2.2 kill factor, with
the Classic grey-out penalty. It needs quest XP, quest level and the per-level XP
table as facts. If any is unknown, the estimate is unavailable and
`estimated_level` is null. Only `.xp` gates raise the guaranteed level; `N-M`
section names never do.

Reports carry check names, statuses, line numbers and quest IDs only. Guide text
(names, notes, zones) is never echoed.

### Deliberate differences from `verify2.py`

- Editions: `verify2.py` required `#forever`. Here each guide needs exactly one
  edition tag, and it must equal the facts' `edition`.
- Quest log: `verify2.py` warned past 20 and errored past 40. Here the capacity is
  a fact, and exceeding it is an error.
- No allow-list: `verify2.py` exempted quest 1714 from the repeatable check. Here
  repeatable quests are always errors, and a route that needs one must say so in
  review.
- `step_location`: `verify2.py` also exempted `.zone`, `.isQuest*` and the travel
  directives. Here `.zone`, `.fly`, `.train`, `.trainer`, `.hs` and `.deathskip`
  count as a location, while `.isQuest*` conditions and `.fp` (a flight path
  learned in place) don't.
- `.fp` names are checked like `.fly` destinations.
- Brackets: `verify2.py` counted `RegisterGuide([[` against `]])` and searched for
  stray `]]`. Here each block is scanned line by line, with line numbers. A `[[`
  inside a body is plain text, because Lua level-0 long strings end only at `]]`.
- Section-name levels: `verify2.py` treated an `N-M` name as a level floor. Here
  it is `asserted`, not a gate.

## Fact sources

`FactSource` is a trait. Every answer is a typed `Fact<T>` (`Known` or
`Unknown`), and quests are `Known`, `Absent` or `Unknown`. `JsonFacts` reads a
`navzeroth-guide-facts` version 1 document (strict: unique keys, no unknown
fields, 8 MiB, 50000 quests, 4096 names):

```json
{
  "format": "navzeroth-guide-facts",
  "version": 1,
  "origin": "local_observation",
  "edition": "forever",
  "client_build": "1.60.1.70425",
  "quest_log_capacity": 20,
  "level_xp": [400, 900],
  "zones": { "known": ["Zone Name"], "complete": false },
  "flight_nodes": { "known": ["Node Name"], "complete": false },
  "quests": {
    "complete": false,
    "absent": [],
    "records": { "123": { "required_level": 1, "level": 1, "prerequisites_any": [],
      "prerequisites_all": [], "races": "any", "classes": ["Warrior"], "repeatable": false,
      "seasonal": false, "objectives": 1, "reward_choices": 0, "xp": 100 } }
  },
  "spells": { "456": { "classes": ["Warrior"] } }
}
```

- `edition` is required.
- `client_build` is a label of at most 64 characters (letters, digits, `.`, `_`,
  `-`). When null or absent, the build is unknown.
- A missing field, or a `"key": null`, is unknown, never zero or false.
- `complete: true` makes unlisted names or quests known-absent.
- `origin` is one of `synthetic`, `local_observation`, `local_client_data`,
  `admitted_source` or `original_authoring`. **The facts file declares its own
  origin.** `facts_are_evidence` (and therefore `passes`) only records that claim.
  An evidence claim still has to be bound to the real observation or extraction
  records before it counts toward release evidence.

No QuestieDB or scraped facts are committed. The only committed facts are the
invented ones in `fixtures/guides/synthetic-facts.json`.

## CLI

```sh
azeroth-cli guide verify --guide <file> --facts <file> [--race <Race>] [--class <Class>]
```

Both files go through the bounded, component-checked local read boundary
(symlinks, reparse points and `..` refused). The character defaults to the first
guide's `#defaultfor` (or its `<<` filter) when that names one race and one class.
The JSON report goes to stdout.

Exit codes:

- 0: `passes`.
- 3: `replay_clean` against synthetic or unverified research facts.
- 1: not clean, or an input was refused.
- 2: usage error (input is never echoed).

## Next step

Per-class Human 1-20 routes (Warrior, Paladin, Rogue, Priest, Mage, Warlock) with
a Deadmines block, solo and duo, are not generated yet. They need local quest
facts first. The facts file should be built from the user's own observations
through the permitted addon, plus local client data for zones, flight nodes and
spells, and bound to those records. Then the routes can be verified to
`passes: true`.

## Bound research facts (continuation 4)

Historical facts/report version 1 retains its original origin rules. Facts version
2 accepts only `origin: "unverified_research"`; it produces report version 2 with
`facts_are_evidence: false` and `passes: false`, including a clean replay. Unknown
versions are refused. A research digest cannot establish release acceptance.

`azeroth-cli local-data guide-facts-build --destination <private-store>
--projection <digest> [--inputs <comma-separated-digests>]` builds deterministic
private facts and observation objects. `guide-facts-verify` takes the destination
and `--guide-facts <digest>` to reproduce the full binding. Inputs must belong to
the exact projection catalog and tuple; cache payloads are not observations.
The manifest binds projection, catalog, inputs, facts and observations by hash.

True observed turn-ins/completions identify candidate quest records, preserving
source metadata and conflicting observations. Required level, prerequisites,
eligibility, rewards and XP remain unknown. False observations do not prove
universal absence. Client zone/flight labels remain an incomplete vocabulary;
client quest IDs and line order establish neither server availability nor quest
prerequisites. Synthetic inputs remain explicitly marked in manifest gaps.
