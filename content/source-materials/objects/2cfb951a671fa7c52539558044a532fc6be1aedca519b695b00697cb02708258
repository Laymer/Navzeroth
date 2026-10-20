# Authoring statement: Navzeroth 1.0.0 slice, revision navzeroth-authored-slice-r2

Source: `navzeroth-authored-v1` (original Navzeroth authoring, BSD-3-Clause, task V2.4).

Everything in this directory was written for this project. It contains no quest
text, NPC dialogue, item names or stats, coordinates, drop rates, artwork or prose
copied from a game client, a website or a third-party guide.

What the files contain:

- `catalog.json`: 54 original role models across nine historical input class identities in levels 1 to 60, a
  Deadmines preparation checklist, and reminder rules. All of it is authored advice
  or an authored assumption.
- `party-support.json` and `procedures/`: the solo and duo party workflows and the
  steps an operator follows to exercise them.
- `evidence/role-model-assumptions.md`: the reasoning behind every weight.

What the files do not contain, on purpose:

- Game facts. Where a record needs one (the Deadmines map, entrance, access rule,
  preparation quests, travel edges, rewards, recommended level; class proficiency
  and slot layout), the field is `unknown`. Those values come later from the local
  client projection (task V2.2) or from the operator's own in-game observations
  through the permitted addon. Actual source origin/build and uncertainty remain
  explicit; acceptance of historical sources is not live verification.
- A verified game build. The catalog is labelled with the development tuple
  `wow_forever 1.60.1.70450 / 16001` because the format requires a context; the
  content does not establish verification on it. ADRs 0007 and 0008 remove the
  launch-build freeze and retain true origins and uncertainty.
- Activation. Building or verifying this pack does not install it for planning.

The Deadmines map key `authored:the-deadmines` / floor `authored:pending` is a
local placeholder identity, not a client map ID. It exists only because a dungeon
record must name a map. It must be joined to the projection's map identity before
any geography coverage counts it.

Class identities use the authored vocabulary `class:<name>` (warrior, paladin,
hunter, rogue, priest, shaman, mage, warlock, druid). They are joined to the client class records of the
local projection later. The historical class inventory is documented in `docs/data/role-coverage-v1.json`.
It does not establish a current Forever roster or a verified level cap. New
identities or ranges require review; coefficients remain original heuristics.
