# Role model assumptions (levels 1 to 20)

Source: `navzeroth-authored-v1`. Score model `gear-model-v1`; weights are integer
thousandths per point of an attribute (1000 = 1.0).

These weights are **authored leveling heuristics**. They are not measured, not
simulated and not verified game mechanics. They only say which attributes this
project assumes matter more for a role while levelling from 1 to 20, so that two
items can be compared with an explained, reproducible model score. A model score
is never a damage, healing or survivability figure.

No numbers, formulas or code were taken from WoWSims or any other simulator. The
admitted WoWSims entry is a Classic paladin pointer for reference only, and its
scope excludes ported coefficients, so it is not cited as a weight source.

## Shared assumptions

- One primary attribute per role carries 1000; secondary attributes are scaled
  against it by judgement, in round steps.
- Stamina is weighted in every model because early levelling is limited by
  downtime and deaths more than by speed.
- Armor (`stat:armor`) appears only in the warrior tank model, at a low weight per
  point, because armor values on items are much larger than attribute values.
- Not modelled anywhere: weapon damage, speed and procs; set bonuses; on-use and
  on-equip effects. Weapons are compared manually.
- Attribute identities `stat:strength`, `stat:agility`, `stat:stamina`,
  `stat:intellect`, `stat:spirit`, `stat:armor` are this project's vocabulary.
  Their mapping to the client's item stat types is pending the projection's stat
  amounts (task V2.2); until then no item attribute matches them.
- Class proficiencies and slot layouts are game facts and are left `unknown`.

## Models

| Model | Class | Role | Weights | Reasoning |
| --- | --- | --- | --- | --- |
| `model:warrior-melee-damage-1-20` | warrior | melee_damage | strength 1000, agility 600, stamina 400 | Strength is assumed to drive melee output; agility helps both damage and avoidance. |
| `model:warrior-tank-1-20` | warrior | tank | stamina 1000, strength 500, agility 500, armor 20 | Surviving pulls matters most in a small group. |
| `model:paladin-melee-damage-1-20` | paladin | melee_damage | strength 1000, stamina 500, intellect 300, agility 300 | Strength first; intellect keeps a mana pool for self-healing between pulls. |
| `model:paladin-healer-1-20` | paladin | healer | intellect 1000, spirit 500, stamina 400 | Mana pool first, regeneration second. |
| `model:rogue-melee-damage-1-20` | rogue | melee_damage | agility 1000, strength 500, stamina 400 | Agility is assumed to be the main melee attribute for the class. |
| `model:priest-healer-1-20` | priest | healer | intellect 1000, spirit 800, stamina 300 | Mana pool and regeneration carry a healer through a dungeon. |
| `model:priest-ranged-damage-1-20` | priest | ranged_damage | intellect 1000, spirit 700, stamina 400 | Solo levelling relies on mana and on recovering it between fights. |
| `model:mage-ranged-damage-1-20` | mage | ranged_damage | intellect 1000, spirit 600, stamina 400 | Mana pool first; low armour makes stamina useful. |
| `model:warlock-ranged-damage-1-20` | warlock | ranged_damage | intellect 1000, stamina 800, spirit 500 | Health is assumed to be a resource for the class, so stamina ranks high. |

Each model applies when the character's declared role equals the model role
(`role_is`). It does not check the class: the predicate language has no class
field yet, so the class is carried by the model's `class` identity and must be
matched by the caller.

## Review

Every weight is open to change by the owner. A change is a new catalog revision,
never an edit that keeps `navzeroth-authored-slice-r1`.
