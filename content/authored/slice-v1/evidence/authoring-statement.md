# Authoring statement: Navzeroth 1.0.0 slice, revision navzeroth-authored-slice-r1

Source: `navzeroth-authored-v1` (original Navzeroth authoring, BSD-3-Clause, task V2.4).

Everything in this directory was written for this project. It contains no quest
text, NPC dialogue, item names or stats, coordinates, drop rates, artwork or prose
copied from a game client, a website or a third-party guide.

What the files contain:

- `catalog.json`: role model assumptions for each Human class in levels 1 to 20, a
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
  through the addon, and are confirmed against the frozen release target.
- A verified game build. The catalog is labelled with the development tuple
  `wow_forever 1.60.1.70450 / 16001` because the format requires a context; the
  content does not depend on it and is rebuilt for the first stable live build
  after launch. Nothing here is relabelled to that build.
- Activation. Building or verifying this pack does not install it for planning.

The Deadmines map key `authored:the-deadmines` / floor `authored:pending` is a
local placeholder identity, not a client map ID. It exists only because a dungeon
record must name a map. It must be joined to the projection's map identity before
any geography coverage counts it.

Class identities use the authored vocabulary `class:<name>` (warrior, paladin,
rogue, priest, mage, warlock). They are joined to the client class records of the
local projection later. The class list for launch is still pending (ADR 0006 §2);
another class is added as another `class:<name>` model without a format change.
