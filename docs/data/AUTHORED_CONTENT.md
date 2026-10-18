# Authored content for the 1.0.0 slice

Task V2.4 ([ADR 0006](../decisions/0006-v1-program.md) §2, §8, §11). Input
directory: [`content/authored/slice-v1/`](../../content/authored/slice-v1/).

Authored content is the project's own writing: role model assumptions, a dungeon
preparation checklist, reminder rules, and party workflow procedures. It states no
game facts. Fields that need one stay `unknown`, so they are counted as pack
coverage gaps, until the local client projection (V2.2) or the operator's addon
observations supply and confirm the value against the frozen release target.

## Files

| File | Contents |
| --- | --- |
| `catalog.json` | v2 reference content, revision `navzeroth-authored-slice-r1`: 9 role models, the Deadmines dungeon record with a 12-item checklist, 6 reminder rules, one placeholder map space |
| `sources.json` | the one source `navzeroth-authored-v1` (original authoring, BSD-3-Clause, redistribution permitted) |
| `party-support.json` | party support matrix schema 2: sizes 1 and 2, 12 rows |
| `procedures/*.md` | one operator procedure per party row |
| `evidence/authoring-statement.md` | what the content is and is not |
| `evidence/role-model-assumptions.md` | the reasoning behind every weight |

The committed JSON is in canonical form. A test checks that a build produces the
same bytes, and that `party-support.json` names the SHA-256 of `catalog.json`.
After editing the catalog, build it, copy the pack's `catalog.json` back, and
update `catalog_sha256` and the changed procedure pins in `party-support.json`.
A content change is a new revision, not an edit under the same revision.

```sh
cargo run --locked -p azeroth-cli -- build-content-v2 --input content/authored/slice-v1 --output <new-dir>
cargo run --locked -p azeroth-cli -- verify-pack --input <new-dir>
```

Building or verifying the pack does not activate it for planning.

## Authored-content rules

`azeroth_core::content_v2::authored::validate_authored` runs after the normal v2
decode. `build-content-v2` and `verify-pack` (and every reader of a verified v2
pack) apply it whenever every source in `sources.json` has access `authored` and
the catalog is not a fixture; such a pack must declare exactly one source. It
refuses, on top of the graph validation:

- evidence from any source other than the declared authored source, or evidence
  marked `verified_on_build` or `cross_edition_unverified`;
- any game-fact record: quests, objectives, items, acquisitions, travel nodes or
  edges, map imagery or embeddings, and equipment/quest/acquisition/node facts;
- a dungeon entrance, access rule, preparation quest list, travel edge list or
  reward list, or a role model's class proficiency table or slot layout, whose
  status is anything but `unknown`. A known value (even an empty list) states the
  fact, and `not_applicable`, `unsupported` or `restricted` is also a claim about
  the game;
- a model weight source other than `documented_model` citing the authored source;
- a map space or dungeon map that is not an `authored:` placeholder, or whose
  transform is not `unknown`; a model class outside the `class:` vocabulary;
- predicates that read quest, objective, equipment, location or participant facts.
  Authored predicates may read only declared state: level, role and preferences.

Passing these rules means the content claims nothing it cannot know. It does not
mean the advice is correct.

## Identities that wait for a join

- Classes: `class:warrior`, `class:paladin`, `class:rogue`, `class:priest`,
  `class:mage`, `class:warlock`. The launch class list is pending (ADR 0006 §2);
  a Forever addition is one more `class:<name>` model.
- Attributes: `stat:strength`, `stat:agility`, `stat:stamina`, `stat:intellect`,
  `stat:spirit`, `stat:armor`. They map to client item stat types once V2.2
  projects stat amounts.
- The Deadmines map: `authored:the-deadmines`, floor `authored:pending`. It is not
  a client map ID and must not count as geography coverage.
- Preference keys read by rules and checks: `plan.deadmines`,
  `remind.flight-points`, `party.duo`. Until a capture reports them they are
  unknown, and the reminder engine omits the rule.
- Context: the catalog is labelled `wow_forever 1.60.1.70425 / 16001`
  (development input) only because the format requires one. It is rebuilt for
  the frozen live target and never relabelled.

## Party support matrix schema 2

`release_party::SupportMatrix` schema 1 is unchanged (target 70374, sizes 2 to 40).
Schema 2 follows the slice:

- `target` is required and must equal the code-side frozen target of release
  schema 2, so it is `null` while that target is pending. The v2 pack verifier
  compares a non-null target with the catalog; the release gate requires it to
  equal the inventory's frozen target and refuses a pending one.
- Sizes 1 to 40. Which workflows a size has is fixed in code
  (`release_party::applies`): a party of one has `joint_planning` (served by the
  single-character planner), `stale_members`, `advancement` and `review`;
  `unequal_progress`, `sharing`, `fairness` and `agreement` are not applicable to
  one person. Sizes of two or more have all eight. Rows must cover exactly the
  applicable pairs.
