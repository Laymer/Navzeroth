# Role model assumptions (levels 1 to 60, revision r2)

Source: `navzeroth-authored-v1`. Score format `gear-model-v1`; weights are integer
thousandths per point (1000 = 1.0). These are original authored heuristics, not
measured damage, healing, survivability, simulation results or verified mechanics.
No simulator coefficients or game formulas were imported.

## Input inventory and supported advice range

The admitted historical CMaNGOS quest shard identifies nine distinct single-class
restriction masks: warrior, paladin, hunter, rogue, priest, shaman, mage, warlock
and druid. Its 4,245 records have minimum-level fields from 0 to 60. The mapping
uses the same authored `class:<name>` vocabulary as the world converter. Exact
bytes, origin, class counts and derivation are recorded in
`docs/data/role-coverage-v1.json` and reproduced by
`scripts/update-authored-role-coverage.py` against the registered run manifest.

This is a historical input inventory, not confirmation of the current Forever
class roster or a playable level cap. Zero is not a player level. Item requirement
fields above 60 do not establish a supported player-level range. The supported
heuristic range is deliberately 1–60, split into 1–20, 21–40 and 41–60. There are
18 class/role combinations and 54 models. No advice is promised outside that range.

## Shared assumptions

The largest attribute weight is 1000. Secondary attributes use round judgement
ratios. Stamina appears in every model as an assumed survival/downtime concern.
The same ratios are reused across all three bands as **uncalibrated extrapolation**;
separate identities make that scope explicit and allow later independent review.
The bands do not imply known skill unlocks, talents, forms, ability scaling or
class progression. Selecting a role is a declared scenario, not proof that the
character has particular abilities or that the class can perform that role well.

Class proficiency and slot layout remain `unknown`. A matching explicit class
assumption and fresh observed level are required; role predicates and exclusions
must evaluate affirmatively. Unknown, restricted, unsupported, stale or mismatched
inputs never make a model applicable. Raw score-only output cannot establish
equipment eligibility, configuration legality, baseline gains or availability.

Unmodelled throughout: weapon damage/speed/procs, pet damage, class abilities,
talents, forms, set bonuses and on-use/on-equip effects. A subtotal with missing
attributes or effects remains incomplete; a numeric zero is not proof of no value.

## Original weights and reasoning

Attribute names below abbreviate the `stat:` vocabulary. Every row applies to
all three bands with the explicit extrapolation limitation above.

| Class / declared role | Weights | Authored reasoning |
| --- | --- | --- |
| warrior / melee_damage | strength 1000, agility 600, stamina 400 | Strength leads the assumed melee attribute mix; agility and stamina are secondary. |
| warrior / tank | stamina 1000, strength 500, agility 500, armor 20 | Survival leads; armor has a small per-point weight because its numeric scale differs. |
| paladin / melee_damage | strength 1000, stamina 500, intellect 300, agility 300 | Preserve the original melee-first heuristic with a small mixed-resource allowance. |
| paladin / healer | intellect 1000, spirit 500, stamina 400 | Assume a resource-first healing scenario with a survival allowance. |
| paladin / tank | stamina 1000, strength 500, intellect 300, armor 20 | Survival-first mixed-resource scenario; no threat or ability formula is asserted. |
| hunter / ranged_damage | agility 1000, stamina 400, intellect 300 | An agility-first declared ranged scenario; pets, weapons and ability scaling are excluded. |
| rogue / melee_damage | agility 1000, strength 500, stamina 400 | Preserve the original agility-first melee heuristic. |
| priest / healer | intellect 1000, spirit 800, stamina 300 | Resource pool and recovery dominate this assumed scenario. |
| priest / ranged_damage | intellect 1000, spirit 700, stamina 400 | Resource-first ranged scenario with a survival allowance. |
| shaman / melee_damage | strength 1000, agility 500, stamina 400, intellect 300 | Mixed-resource melee scenario; no weapon proc or spell interaction is assumed. |
| shaman / ranged_damage | intellect 1000, spirit 500, stamina 400 | Generic resource-first ranged scenario, without spell coefficients. |
| shaman / healer | intellect 1000, spirit 700, stamina 400 | Generic resource-first healing scenario. |
| mage / ranged_damage | intellect 1000, spirit 600, stamina 400 | Preserve the original resource-first ranged heuristic. |
| warlock / ranged_damage | intellect 1000, stamina 800, spirit 500 | Preserve the original higher survival/resource allowance; pets and abilities remain excluded. |
| druid / melee_damage | agility 1000, strength 500, stamina 400 | Generic agility-first scenario; forms and their conversions are not modelled. |
| druid / ranged_damage | intellect 1000, spirit 500, stamina 400 | Generic resource-first ranged scenario. |
| druid / healer | intellect 1000, spirit 700, stamina 400 | Generic resource-first healing scenario. |
| druid / tank | stamina 1000, agility 600, armor 20 | Survival-first scenario without asserting form-specific armor or ability effects. |

## Reminders and representative scenarios

Twelve role/level reminders cover tank, healer, melee_damage and ranged_damage in
each band. They ask the user to review assumptions and prepare; they provide no
class abilities, spell training, talent, profession or enhancement gameplay. Their
predicates inspect declared role and fresh level, with an inclusive lower and
upper bound. Class-specific predicates are not expressible in the current rule
language, so no reminder pretends to enforce a class restriction. Class matching
remains mandatory for model selection.

Automated scenarios exercise every model at both inclusive endpoints and refuse
the first level above its range. Unknown/mismatched class, unavailable/stale level,
unknown/mismatched role and unresolved exclusions refuse selection. Reminder
scenarios cover all 54 model/band combinations and omit unknown, restricted,
unsupported and stale role observations. These are synthetic tests, not client
acceptance or a claim that the advice is optimal.
