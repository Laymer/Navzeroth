# Role profiles and the gear model score (M5.1)

Status: structure and arithmetic only. This file defines what a role profile is
and how a model score is computed. It contains **no real weights** and makes no
claim about how Forever equipment should be valued. The plan (sections 10.3 and
20.3) forbids inventing weights or asserting that Classic weights hold for
Forever, and this document follows that.

Implementation: `crates/core/src/gear/` (profile in `profile.rs`, arithmetic and
breakdowns in `score.rs`). Score model version: `gear-model-v1`.

## What a profile is

A profile is data, validated like every other record:

| Field | Meaning |
| --- | --- |
| `role` | `RoleId` of the scenario the profile serves. |
| `version` | `ScoreModelVersion`. Only `gear-model-v1` is implemented; any other value is rejected, because the arithmetic below is defined by the version. |
| `weight_source` | Where the weights come from (see below). Required, never inferred. |
| `weights` | Integer thousandths per `AttributeId`: `1500` means 1.500. Non-empty, at most 1024 entries, fractional or out-of-range weights are rejected. |
| `known_unmodeled` | Free-text list of effects the model knowingly does not cover. Copied into every breakdown as `profile_unmodeled`. |

### Weight source labels

| Label | Use |
| --- | --- |
| `user_preference` | The owner or player chose the weights. |
| `documented_model` (+ `SourceId`) | A documented model registered in `docs/data/SOURCES.md`. |
| `permitted_guide` (+ `SourceId`) | Guide data whose terms permit this use, registered in `docs/data/SOURCES.md`. |
| `synthetic_test` | **SYNTHETIC engineering assumption for tests, not game truth.** Never a default for a real character. |

A real weight table needs an owner decision or a registered permitted source.
Neither exists yet, so none is shipped. Registering a source is a change to
`docs/data/SOURCES.md` and is out of scope for this task.

## The four planned profiles

The plan names four scenarios. For each, this task delivers the structure only.

| Scenario | Planned role id | Weight source | Modelled attributes | Conditional terms | Known unmodeled |
| --- | --- | --- | --- | --- | --- |
| Paladin duo questing | not yet assigned | undecided; owner decision or registered source required | undecided | none implemented | see below |
| Paladin dungeon tanking | not yet assigned | undecided | undecided | none implemented | see below |
| Mage duo questing | not yet assigned | undecided | undecided | none implemented | see below |
| Mage dungeon damage | not yet assigned | undecided | undecided | none implemented | see below |

"Undecided" is deliberate. Attribute identifiers are opaque strings supplied by
the dataset; choosing which of them matter, and how much, is the owner's
decision (or comes from a registered source). Conditional terms (a weight that
applies only under a stated condition) are not implemented in `gear-model-v1`.

Categories of effect that a weighted attribute sum cannot represent, and which
therefore appear as unmodeled rather than as zero: proc and on-use effects,
set bonuses, enchants and gems not present in the item's attribute map,
talent interactions, and any effect whose text the dataset supplies without a
numeric model. An item whose `effects` observation is not known, or is known
and non-empty, gets an `unmodeled_effects` entry and a lowered certainty.

## Score arithmetic (`gear-model-v1`)

Everything is integer arithmetic on thousandths, so a Lua evaluator can
reproduce it exactly (M5.6/M5.7). There is no floating-point accumulation.

1. **Value to fixed point.** `value_milli = round_half_away_from_zero(value * 1000)`,
   computed as one IEEE-754 double multiplication. A result beyond the safe
   integer range (9,007,199,254,740,991) is an error.
2. **Contribution.** `contribution_milli = round_half_away_from_zero(value_milli * weight_milli / 1000)`,
   computed exactly in 128-bit integers, per item and per attribute.
   Half cases round away from zero: `+0.5 -> 1`, `-0.5 -> -1`, `+1.5 -> 2`, `-1.5 -> -2`.
3. **Configuration.** For each attribute, the configuration's `value` is the sum
   of the items' values and its `contribution` is the sum of the items' already
   rounded contributions. This can differ by rounding from rounding the summed
   value once; the per-item rule is the specified one.
4. **Total.** `total` is the checked sum of all contributions, so contributions
   add up to the total exactly. Any intermediate or final value outside the safe
   integer range is an error, not a wrap.
5. **Ordering.** Candidates are processed in `ItemKey` order and attributes in
   `AttributeId` order. Configurations rank by total (highest first), then by
   certainty (most certain first), then by the sorted item keys. Nothing
   depends on input order.

## What a breakdown says

A `ScoreBreakdown` is labelled `model score` and carries the `ScoreModelVersion`,
the role, the weight source, one contribution row per modelled attribute
(attribute, value, weight, contribution), the exact total, the unmodeled
effects, the profile's known-unmodeled list, a certainty level and the reasons
for it.

The total covers modelled attributes only. It is **not** a prediction of
damage, healing or survivability and must never be rendered as a percentage of
any of them: a higher model score only says that, under these weights and these
modelled attributes, one configuration scores higher.

### Certainty

| Level | When |
| --- | --- |
| `complete` | No item-level gap relative to the model. It does not mean the model is complete; see `profile_unmodeled`. |
| `reduced` | Any of: effects not known, a non-empty effect list, an attribute with no profile weight, an unknown unique group in a configuration of two or more items, an unresolved baseline slot, an unknown `two_handed` in a main-hand/off-hand pairing. |
| `low` | An item's attributes are not known (unknown, unsupported or restricted). The modelled total then omits that item's attributes entirely. |

Unknown values are never counted as zero in a `complete` result. A score is
`definitively_exceeds` another only when both are `complete`, share the model
version and role, and this total is strictly higher; a reduced or low score is
never definitively better or worse.

## Rankings

Only two of the three plan rankings exist here.

- **Theoretical:** every candidate in the scenario context, ignoring eligibility
  but enforcing configuration legality.
- **Obtainable:** only candidates whose eligibility is `Eligible`. Future
  progression (a level not yet reached), ineligible and unknown candidates are
  listed in `excluded` with their reasons. Acquisition availability, cost and
  probability are **not** evaluated; that is M5.4.
- **Practical:** not implemented (M5.4); nothing is labelled practical.

Results are labelled "Best among verified candidates" because the candidate set
is whatever the caller supplied, never an absolute best.

## Synthetic fixtures

Everything under `fixtures/gear/` is marked `"synthetic": true`: edition
`wow_forever`, build and interface `SYNTHETIC`, namespace `demo`, fictional
identifiers, and a single weight of 1000 (`1.000`) for `synthetic-power` that is
an engineering assumption for tests, not game truth.
