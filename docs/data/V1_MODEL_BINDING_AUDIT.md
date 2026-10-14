# V1 class/role, dungeon and party binding audit

Audit baseline: `9c02f2be42f1a2110e2851e91f81031443afdafd`.
Owner: Codex `/root/public_sources`. This is a read-only implementation audit;
proposed contract work below is not an accepted schema or verified game record.

Class/role model envelopes and reusable dungeon declaration validation are
concrete engineering work that can proceed with synthetic fixtures. Neither can
populate verified Forever content without additional inputs. The party requirement
also includes workflow support, which should not be satisfied by packaging local
party state, test cases or arbitrary report IDs as game content. Its binding needs
a coordinator decision separating content dependencies from acceptance evidence.
All three stable blockers remain valid at this baseline.

## Scope and current evidence

The [implementation plan](../../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md), sections
9–11 and 20, requires independent participant eligibility, verified sharing
semantics, documented role models, dungeon preparation and representative
class/role/level scenarios. The authorized full-content scope extends the initial
paladin/mage and Deadmines examples; those examples are not a complete denominator.

The [inventory](../../release/v1/coverage-inventory.json) contains nine mandatory
domains, all with `completeness: unknown`, and zero accepted sources. Its three
relevant units have `required_record_ids: null`. The actual production reference
dataset is still build `1.60.1.70205`: six evidence records, one map space and one
image, but zero quests, items, acquisitions, nodes, edges, objectives and rules.
It is not a model catalog for `1.60.1.70374`.

[ReferenceDataset](../../crates/core/src/contract/dataset.rs) has no role-model,
dungeon-package or party-support collection. [Pack verification](../../crates/cli/src/pack.rs)
accepts a strict manifest, source registry, reference datasets and declared image
assets; unknown fields or arbitrary extra files cannot supply missing records.
[Release binding](../../crates/cli/src/release.rs), `catalog_dataset` and
`bind_records`, derives canonical IDs from the six supported typed domains and
compares the complete set to required IDs. The regression
`absent_role_party_dungeon_contracts_never_become_automatic_passes` keeps these
three absent domains blocked. Removing that check or accepting opaque IDs would
weaken the gate without implementing content support.

## Class and role models

Reusable existing implementation:

- [RoleProfileData](../../crates/core/src/gear/profile.rs) contains a `RoleId`,
  `gear-model-v1`, explicit `WeightSource`, nonempty bounded integer weights and
  distinct known-unmodeled effects. It validates the arithmetic model version,
  collection bounds and duplicate unmodeled labels. The source labels distinguish
  user preference, documented model, permitted guide and synthetic test.
- [SlotLayoutData](../../crates/core/src/gear/inputs.rs) validates explicit slot
  membership and the optional main/off-hand relationship. No built-in game slot
  table is supplied. [ProficiencyTableData](../../crates/core/src/gear/eligibility.rs)
  keeps allowed/disallowed categories disjoint; categories absent from both stay
  unknown. Class/category identifiers are caller-supplied opaque `SourceId`s.
- [GearRequest and calculate](../../crates/app/src/gear.rs) bind actual candidates
  to an exact snapshot and content revision. The model, class, categories,
  proficiency, layout, durations and trial assumptions are explicit manual input.
  The report says so. [GearScreen](../../apps/desktop/src/screens/GearScreen.tsx)
  labels entered weights as user preference or synthetic test; it does not select
  a shipped verified model catalog.

These are useful validators and evaluators, not a persisted model-record contract.
`RoleProfileData` has no full `DataContext`, fixture flag, immutable record/revision
identity, faction/race/class/level applicability, evidence IDs, model document or
representative scenario binding. A `WeightSource::DocumentedModel` source ID is a
declaration; the role validator does not resolve it against a registry or establish
its rights. Proficiency, item category mappings and layouts need their own exact
context/evidence associations. A gear recommendation report tied to one character
is an output, not a reusable source model.

Dependency-ready contract work, after coordinator approval:

1. Define a versioned immutable model record around the existing arithmetic data,
   with exact context/fixture/revision, enumerated applicability and evidence/source
   references. Bind companion proficiency/layout/category facts rather than
   inferring them from role names. Preserve missing observations separately from
   empty sets and zero weights; reject contradictory applicability or references.
2. Validate source membership, pinned revisions, permissions and each model's
   declared source association. A synthetic model must never qualify as production
   coverage. A recorded user choice can justify a preference for its stated scope;
   it does not become a verified performance prediction or automatic global default.
3. Add strict bounded pack/storage loading and exact record identity binding, then
   make application selection consume the same immutable model. Unknown nested
   fields and duplicate JSON members must fail at the import boundary; merely
   deriving Deserialize on an existing arithmetic type is insufficient.

Required real inputs: the available class/race/faction/role/level inventory; explicit
applicability; a documented model, permitted guide or recorded user choice for
each declared scenario; actual weights and attribute definitions; equipment and
acquisition facts; supported slot/proficiency/category mappings; unmodeled effects
and exclusions; reproducible representative scenarios, source terms and exact-build
validation. The four planned initial profiles remain unpopulated. Existing weighted
sum arithmetic has no conditional terms; new model semantics require their own
version, Rust/Lua parity and evidence, rather than invented coefficients.

## Dungeon preparation

[LocalPreparationPackage](../../crates/app/src/dungeon_preparation.rs) is a strong
starting point. It already carries schema version, stable package ID/title, full
context, fixture flag and dataset revision. Required sections use `Declared<T>`:
an explicit observation plus evidence IDs. Sections cover entrance, access,
quests, rewards, checks, formation/run ranges, departure/entry lists and notes.
`RewardReference` uses a full item key and acquisition ID; its role label expressly
does not infer a benefit or recommendation.

Existing checks that can be reused:

- `validate_package` checks schema, duplicate identities, bounded section/evidence
  counts, text lengths and context-sensitive predicates. Known sections without
  evidence produce a gap; unknown/unsupported/restricted/not-applicable sections
  remain unavailable rather than silently becoming empty lists.
- The calculation resolves entrance node/map, quests, full item keys, the actual
  item-to-acquisition link, evidence records and registered sources against exact
  active content. It preserves confidence and permission declarations separately.
- [The request reader](../../crates/app/src/dungeon_preparation/json.rs) refuses
  duplicate JSON keys and unsupported versions. Runtime capture and revalidation
  preserve membership, snapshot/content identity and freshness. Shared travel
  requires usable supported edges for every fresh participant.

Important limits: this is an app-local request, not an immutable content-pack
record. `validate_package` is private and reference resolution is inside runtime
calculation. Every supplied local package receives `LocalPackageUnverified`, and
`ConsumableInventoryUnavailable` remains explicit. Evidence-ID resolution does not
prove that the cited evidence supports that particular assertion. A fully parsed
package can still have many mandatory gaps. The calculated report includes local
member identity and observations; it must not be redistributed as reference data.

Dependency-ready work is to extract the declaration and pure reference-validation
parts into a shared layer, retain existing runtime behavior, and define an immutable
package identity/binding and release-completeness policy. The policy must specify
which sections are mandatory for each enumerated dungeon and how evidenced empty
or inapplicable sections are represented. Do not make arbitrary all-unknown
packages complete or invent facts merely to fill every field. References to useful
role rewards need validated model associations, not only free role labels.

Required real inputs: authoritative dungeon enumeration and identity/mode scope;
build-specific entrance/map and directed travel; access and quest-stage chains;
reward/acquisition/probability/cost facts; justified role/consumable/equipment rules;
declared run/formation estimates distinguished from measurements; checklists and
supported notes; per-assertion evidence, rights and validation. Current 32-record
section limits are implementation bounds, not proof that every available dungeon
fits them. Partitioning or larger limits needs the actual inventory and workload.

## Party support and game facts

[PartyProfile](../../crates/storage/src/party.rs) stores ordered local profile
membership with matching context/fixture identity and distinct characters; its
range is 1–40. [PartySnapshot](../../crates/core/src/entities/character.rs) is
captured participant state, not a reference record. The
[joint planner](../../crates/core/src/joint_planner.rs) accepts 2–40 distinct fresh
participants and up to 128 candidates. Its implemented fairness rule requires
positive declared utility and true readiness for every member and uses the
minimum contribution as group utility. It checks each travel edge per member.
These facts describe software behavior, not verified Forever party sizes or rules.

[Application joint planning](../../crates/app/src/joint_planning.rs) uses exact
content/snapshot state. [Eligibility](../../crates/core/src/eligibility.rs) preserves
acceptance, progress, completion, turn-in and catch-up independently; its group
eligibility documentation explicitly disclaims shareability/shared-progress proof.
[Local proposal agreement](../../crates/app/src/party_proposals.rs) is an explicit
local declaration with freshness/revision guards, not authenticated remote consent.
None of these IDs should enter a distributed party-content catalog.

[QuestDefinition](../../crates/core/src/entities/reference.rs) already has
`shared_progress: Observation<bool>`, completion semantics, prerequisites and
stages. It has no complete group-context-specific sharing/credit/loot contract.
One shared-progress boolean does not establish quest shareability, assistance
semantics, per-stage credit, group-size eligibility or player acquisition probability.
Actual supporting quest/item/acquisition facts remain in their respective content
domains and must not be manufactured from a successful synthetic duo test.

Recommended decision before code: treat party workflow coverage as a typed release
support/evidence binding over exact content/model dependencies, separate from the
pack's gameplay records. Enumerate supported workflow and party-size combinations,
bind the implemented policy/source revision and actual tested artifacts/environment,
and require the appropriate live/manual acceptance. The existing
[release contracts](../../crates/core/src/release.rs) already carry exact file
hashes, tested production identity and acceptance method; they lack this typed
party coverage matrix. A label or test filename alone is not a validated entry.
Synthetic scenarios can test the binding validator but cannot satisfy real coverage.

This changes the current all-domains-as-pack-records interpretation and therefore
needs a versioned coordinator-owned inventory/evidence contract, compatibility
tests and a fail-closed transition. It does not waive content dependencies or
mandatory local duo/differing-progress/stale/advance/review workflows. Optional
party messaging remains separate.

If the coordinator instead retains a `party` gameplay-record collection, first
define the concrete sourced facts it represents and their applicability/reference
semantics. No existing local party/profile/report type is that contract. Real
sharing/credit/group/loot rules and complete content applicability are missing, so
populating or accepting such records is blocked. A generic opaque record wrapper
cannot make that work dependency-ready.

## Compatibility and a finite implementation sequence

| Work | Ready engineering after contract freeze | Still required for release acceptance |
| --- | --- | --- |
| Model declarations | Versioned model/applicability/provenance wrapper; strict validation; immutable identity; application selection | Actual permitted scoped models and representative scenarios |
| Dungeon declarations | Shared pure package/reference validator; durable package binding; explicit gap/completeness rules | All available dungeon identities and supported exact-build facts |
| Party support | Typed support/dependency/acceptance declarations if the coordinator adopts the evidence-binding approach | Enumerated supported sizes/workflows, underlying game facts and real acceptance |
| Pack/storage/release integration | Versioned manifest/record loading, hashes, source union, bindings and atomic activation | Permitted shipped content, license/source delivery and tested consumers |

Prefer retaining the current exchange payloads where possible. A separately
versioned supporting-record payload may avoid changing ReferenceDataset v1, but
it still needs explicit pack manifest, storage/activation and application-consumer
support; an unlisted sidecar cannot bypass the strict existing pack verifier.
Alternatively, adding typed dataset collections is a demonstrated schema requirement
that needs explicit version/migration/compatibility tests. This audit chooses no
wire format and does not authorize silently widening the current schema.

Acceptance tests for the chosen implementation should reject future/unknown or
duplicate fields, mixed contexts/fixtures, missing references, source mismatch,
synthetic-as-production models, forged opaque record IDs, wrong revisions/hashes,
unjustified completeness and omitted inventory units. Preserve known false/zero,
empty versus absent, restricted/unsupported observations, stale members and source
conflicts. Existing local commands and saved records must retain their behavior.
End-to-end tests must show that shipped typed records are actually consumed; a
packaging-only binding is not evidence of a usable class or dungeon workflow.

## Audit verification and limits

Inspected actual production types and call paths linked above, the strict pack
and release binding implementation, current inventory and production dataset.
Parsed all three `fixtures/gear/case-*.json` files: each is synthetic and uses
`synthetic_test` weights. The preparation example is `fixture: true`, `demo-r1`,
`UNVERIFIED_DEMO`; the solo/duo replay examples label their format synthetic and
contain fixture snapshots/content. Their outputs are engineering evidence only.

No code, schemas, source registry, acceptance status, inventory denominator or
game data was changed. No additional gameplay/build/Windows test was performed
for this documentation audit. Link/symbol checks and review are recorded in the
[handoff](../handoffs/2026-10-14-v1-model-binding-audit.md). The
[public-source audit](../sources/v1-public-source-audit.md) and unknown source
permissions/completeness remain unchanged.
