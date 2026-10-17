# RXP guide importer and replay verifier

Task V2.3 (ADR 0006 §2, §3 and §11). Code: `crates/ingest/src/rxp.rs` (importer),
`crates/ingest/src/rxp_verify.rs` (verifier and fact sources),
`crates/cli/src/guide.rs` (CLI). Ported from the owner's guide generator
(`gen.py`, `verify2.py`, kept outside the repository), which the owner authored
and gave permission to port. That Python and the generated Lua were read as source
and never executed.

## What the guide is, and what it is not

Under ADR 0006 §11 (owner decision of 2026-10-17), 1.0.0 sources are local only: the user's
own client data, the user's own in-game observations, admitted published-licence
material and original authoring. The quest IDs, NPCs and coordinates in the
owner's generated guides came from QuestieDB and a website scrape, so they are
**not admitted content**. A guide is route design: an order of quest and travel
steps whose facts must each be confirmed later by local observation. The
verifier exists to show which facts are confirmed, contradicted or still unknown.

## Importer

`rxp::parse(bytes)` scans the file line by line for the exact call form
`RXPGuides.RegisterGuide([[ ... ]])` and takes each long-string body as data. No
Lua is run and the crate has no Lua engine. Inside each body it parses:

- header: `#name`, `#displayname`, `#next a;b`, `#group`, `#subgroup`,
  `#defaultfor`, `#version`, edition tags (`#forever`, `#classic`, `#era`, `#som`,
  `#tbc`, `#wotlk`, `#cata`) and the guide `<< ...` filter;
- `step` and `step << filter`, step tags `#optional`, `#sticky`, `#label`,
  `#completewith`, `#requires`, `#level`;
- directives `.accept [-]id`, `.turnin [-]id[,reward]`, `.complete id,n`,
  `.abandon`, `.goto`/`.waypoint Zone,x,y[,...]`, `.fly`, `.fp`, `.zone`, `.train`,
  `.trainer`, `.xp` (all documented forms), `.maxlevel`, `.home`, `.hs`, `.vendor`,
  `.mob`, `.use`, `.collect`, `.gossipoption`, `.deathskip`, `.isOnQuest`,
  `.isQuestComplete`, `.isQuestTurnedIn`, `.isQuestAvailable`;
- notes `>>text`, `*text`, `+text`, `--` comments and line filters
  (`.train 1 >>text << Warrior`).

Filters follow the RXP documentation: `/` is OR, space is AND, `!` is NOT, with
precedence `!` > space > `/`. Races, classes and factions are evaluated; any other
token (`tbc`, misspellings) is unsupported and never guessed.

Nothing is dropped. An unknown directive, tag, header line or text line is kept
as an explicit unsupported item with its line number; a known directive with bad
arguments is malformed. Structural problems (unclosed block, `]]` without `)`,
stray `]]`, code outside a block) are recorded with their lines.

Bounds: 4 MiB file, 2048-byte lines, 512 guides, 4096 steps per guide, 32768
steps, 256 lines per step, 64 header lines, 160-byte labels. Oversize input is
refused as a whole, never truncated. Non-UTF-8 or NUL bytes are refused.

## Verifier

`rxp_verify::verify(file, facts, character, origin)` replays every guide in file
order for one race and class, as `verify2.py` does. Steps excluded by a filter
are `filtered`; steps whose `.isOnQuest`/`.isQuestComplete`/`.isQuestTurnedIn`/
`.isQuestAvailable` condition fails are `conditional`. Both are skipped.

Checks (each finding has a check name, status, line and optional quest ID):

| Check | Fails when |
| --- | --- |
| `bracket_balance` | a structural issue was found, or no guide block exists |
| `header_required`, `header_duplicate`, `header_tag`, `edition`, `guide_name`, `guide_filter` | `#name`/`#group`/`#version` missing, a tag repeated, a malformed `#version`, not exactly one edition per guide or several editions in one file, a repeated guide name, two guide filters |
| `next_guide` (warning) | a `#next` name is not a guide in this file |
| `quest_exists` | the fact source says the quest is absent |
| `double_accept`, `re_accept_done` | the quest is already in the log, or was turned in |
| `prerequisite_any`, `prerequisite_all` | none of / not all of the prerequisites were turned in before accept |
| `required_level` | the required level is above the level gate (`.xp` gates and the `N-M` section-name convention); a warning when only the XP estimate reaches it |
| `race_allowed`, `class_allowed`, `repeatable`, `seasonal` | the quest is not available to the character, or is repeatable or seasonal |
| `quest_log_capacity` | the log exceeds the fact source's capacity |
| `complete_in_log`, `objective_index` | `.complete` for a quest not in the log, or an objective index out of range |
| `turn_in_in_log`, `double_turn_in` | turn-in before accept, or a second turn-in |
| `objectives_marked` (warning) | turn-in with fewer objectives marked than the quest has |
| `reward_index` | `.turnin id,n` beyond the reward count |
| `zone`, `coordinate`, `flight_node`, `train_spell` | unknown zone, coordinate outside 0-100, unknown flight node (a zone name is also accepted, as in `verify2.py`), spell not trainable by the class |
| `placeholder_text` | a note contains a placeholder fragment the owner's verifier rejects |
| `step_location` (warning) | a step has directives but no `.goto` or travel |
| `abandon`, `left_in_log` (warnings) | abandoning a quest not in the log; quests left at the end |
| `directive` | malformed (error) or unsupported directive |
| `step_filter`, `line_filter`, `step_tag`, `header_line`, `text` | unsupported item |

Statuses are `ok`, `error`, `warning`, `unverified` and `unsupported`. **An unknown
fact is `unverified`, never `ok`.** A report `passes` only with zero errors,
unverified and unsupported findings; warnings do not fail it. Optional steps and
`-id` lines follow `verify2.py`: their pickups stay out of the default log, and a
turn-in of a quest not in the log is skipped silently.

The XP estimate ports the owner's model (quest XP times a 2.2 kill factor, the
Classic grey-out penalty). It needs quest XP, quest level and the per-level XP
table as facts. Once any is unknown the estimate is unavailable, and level checks
the gates alone cannot settle are `unverified`.

Reports carry check names, statuses, line numbers and quest IDs only. Guide text
(names, notes, zones) is never echoed.

## Fact sources

`FactSource` is a trait; every answer is a typed `Fact<T>` (`Known` or `Unknown`)
and quests are `Known`, `Absent` or `Unknown`. `JsonFacts` reads a
`navzeroth-guide-facts` version 1 document (strict: unique keys, no unknown
fields, 8 MiB, 50000 quests, 4096 names):

```json
{
  "format": "navzeroth-guide-facts",
  "version": 1,
  "origin": "local_observation",
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

A missing field is unknown, never zero or false. `complete: true` makes unlisted
names or quests known-absent. `origin` is one of `synthetic`, `local_observation`,
`local_client_data`, `admitted_source` or `original_authoring`; the report's
`facts_are_evidence` is false for `synthetic`. No QuestieDB or scraped facts are
committed; the only committed facts are the invented ones in
`fixtures/guides/synthetic-facts.json`.

## CLI

```sh
azeroth-cli guide verify --guide <file> --facts <file> [--race <Race>] [--class <Class>]
```

Both files go through the bounded, component-checked local read boundary
(symlinks, reparse points and `..` refused). The character defaults to the first
guide's `#defaultfor` (or its `<<` filter) when that names one race and one class.
The JSON report goes to stdout. Exit codes: 0 passes, 1 does not pass or an input
was refused, 2 usage error (input is never echoed).

## Next step

Per-class Human 1-20 routes (Warrior, Paladin, Rogue, Priest, Mage, Warlock) with
a Deadmines block, solo and duo, are not generated yet. They need local quest
facts first: a facts file built from the user's own observations through the
permitted addon (and local client data for zones, flight nodes and spells), so
the routes can be verified to `passes: true` with `facts_are_evidence: true`.
