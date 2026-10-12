# Core domain reference (M1.2)

The core contains all 19 entities from plan section 6.1. It performs no I/O and
reads no clocks. These are domain records; exchange documents wrap them in the
versioned envelopes described in [the contract reference](../schemas/v1/README.md)
(M1.3), with generated JSON Schema and TypeScript types.
No live-client fact is established by deserializing a record.

## Construction and JSON

Each entity is a `Validated<EntityData>` alias. Edit the public draft `*Data`, then
call `Entity::try_new(data)`. Validated records provide shared access, not mutable
access. `into_inner` consumes the wrapper for editing followed by revalidation.
Serde deserialization performs the same semantic checks. Fields are snake_case;
closed enums reject unknown variants. Additional metadata fields are ignored.

Use `contract::decode_envelope`/`decode_any` for exchange documents and
`json::decode_json::<Entity>(input)` at a bare domain JSON boundary. It rejects
input over 1 MiB before parsing, retains serde_json's recursion limit, checks any
declared schema version, and returns safe application errors. M1.3/M2 must also
bound transport/file reads before allocating this string. Raw `*Data` are drafts
and must not be exposed as accepted import results.

Schema version 1 is supported. Dataset revisions, ruleset versions, score-model
versions and application versions are separate. Build identity includes edition,
build and interface; locale belongs to GameBuild, not entity identity. Demo and
reference namespaces never mix. Identifiers are distinct Rust types, serialize
as strings, preserve spelling, and reject controls/edge whitespace or >256 bytes.
No ID generation or external lookup occurs in core.

UTC timestamps, durations, counters and integer comparisons use named units and
JSON-safe integer ranges. Currency is unsigned integer copper with canonical
base-10 string encoding, including values above JavaScript's integer precision.
Other numeric values are finite and within the safe numeric magnitude; normalized
coordinates are [0,1]. Text is bounded to 4,096 bytes. Record maps reject duplicate
JSON keys, preserve deterministic key order, and hold at most 1,024 entries.
Declared collection limits are engineering limits, not claims about game limits.

## Observation semantics

`Observation<T>` has private status/value/metadata and checked constructors.
Known requires a non-null value; all other statuses omit value entirely. Explicit
null on an unavailable observation is rejected. False, zero and empty collections
remain known values. An empty equipment slot is `EquipmentSlot::Empty`, not null.

Omitted observation metadata means unrecorded information: source defaults to
Unrecorded, clock trust to Uncertain, and optional times/session/sequence to absent.
Elapsed time or sequence requires a producer session ID. Receipt time is kept
separate from capture time. A snapshot's capture metadata never silently fills in
missing per-field metadata. Import adapters must deliberately provide justified
per-field provenance; a producer's claim alone does not verify clock trust.

`freshness(metadata, context)` uses matching session-relative times first,
otherwise trusted producer/current UTC times. Missing/untrusted time, reversal,
or an unrepresentable age returns Uncertain. Age equal to the caller's limit is
Fresh; larger age is Stale. Session changes prevent comparing unrelated monotonic
clocks. Receipt time never proves observation age. Freshness never rewrites status.

For other optional fields, absent `instance` means no recorded instance context;
absent area bounds selects the whole named map/floor; absent plan/rule expiry
means no time expiry was declared. Unknown gameplay facts use Observation rather
than optional values or fabricated defaults. Missing quest/history entries never
prove incompletion. Party snapshots retain separate participant identities,
observations and timestamps.

## Records and constraints

- GameBuild, EvidenceRecord and ProviderCapability retain capability/evidence
  categories. Verified capabilities require evidence references; demo evidence
  cannot claim a live build was verified. References do not prove source truth.
- ItemDefinition, EquippedItem and AcquisitionSource preserve build/variant,
  unknown modifiers, declared requirements, costs and acquisition uncertainty.
- QuestDefinition and QuestProgress separate acceptance, completion and turn-in;
  reset/repeatability semantics remain explicit rather than inferred.
- CharacterProfile, CharacterSnapshot and PartySnapshot validate nested contexts,
  quest-key identity and unique participants. Preference values are boolean in v1.
- MapSpace, TravelNode and TravelEdge use named map/floor/instance spaces and
  directed edges; normalized rectangles include their boundaries. Transform
  scale must be nonzero; durations and ranges cannot be negative or inverted.
  A Position's origin is the map's top-left corner, x east and y south in [0,1]
  (C_Map convention, unverified on the client until M0.4); `map_id` is the
  addon-exported uiMapID as a decimal string. A CoordinateTransform applies
  `target = scale * normalized + offset` per axis.
- MapImage (ADR 0002) records raster layers (relative lowercase asset paths,
  declared SHA-256 digests, sizes equal to ceil(frame / divisor)), image-space
  pins measured on a declared layer, an optional pixel calibration observation
  that requires evidence, control points whose declared residuals must agree
  with that calibration, and an explicit same-edition `applicable_builds` list
  (the image's own build plus evidence-backed others). MapEmbedding places a
  child map's rectangle inside its parent with one exact context; datasets
  reject cycles and chains deeper than 4. Core carries digests; it never hashes
  or decodes files.
- Objective and RoutePlan validate participant references, exclusions, travel
  continuity, revisions and expiry. Plans also carry capability assumptions,
  revalidation triggers, per-step evidence (a subset of plan evidence) and
  duration estimates; expected durations must lie inside declared ranges.
  An empty plan is allowed to represent no feasible steps; its inputs still
  identify what was considered.
- GearRecommendation separates theoretical/obtainable/practical results. Known
  occupied-slot or unique-group conflicts are rejected. Obtainable/practical
  require explicit known eligibility. Core does not infer missing equip rules,
  calculate scores or establish game eligibility merely from type validity.
- ContextRule carries trigger, predicate, text, priority, cooldown, dismissal,
  deduplication, evidence and optional expiry. Rendering/scheduling is later work.
- SessionSummary distinguishes observed/estimated/unclassified time and gaps;
  intervals cannot overlap or exceed the named producer session duration.

Requirements embedded in records are checked against the owning build/namespace,
including predicate branches that could later be short-circuited. Reference
datasets and evidence bundles resolve their local references inside the
envelope (see the contract reference); cross-dataset resolution, predicate-only
references and graph analysis remain later content work. Local record validity
does not establish a complete dataset.

## Pure projection

`atlas::project_position(position, context, dataset)` places a position on the
first applicable image in dataset order, walking embeddings (exact context)
with the position's build carried unchanged and checked against
`applicable_builds`. It returns a `Projection` (full-frame pixel, notices for
cross-build reuse and embedding fallback, the weakest calibration confidence)
or an explicit `ProjectionGap` (namespace mismatch, no image, instance,
different floor, build mismatch, uncalibrated, out of bounds). A gap is an
outcome to explain, never a guessed marker. `solve_axis_aligned_calibration`
fits a per-axis least-squares transform from at least three spread pairs and
reports residuals; rotation and shear are not modelled.

## Pure rules

`evaluate(predicate, context)` and `evaluate_rule(rule, context)` return Truth
(True/False/Unknown), structured reasons, warnings and executed step count.
`should_trigger()` is true only for a definite True result. A supported pure
predicate is not an implementation of its future addon event integration.

All section 6.6 operators are implemented. Generic fields are level, role,
boolean preference, quest state and objective completion. Equality requires the
matching scalar type; numeric comparison currently selects level and an integer.
Game predicates cover level, quest/objective state, exact equipment and requested
modifiers, role, same-space area containment, preferences and selected participants.
Equipment matches do not treat a missing modifier as zero. Participant lookups
use each participant's own observations and timing, not the primary character's.

Three-valued AND is dominated by False and OR by True; NOT Unknown remains
Unknown. Empty all is True; empty any is False. `known` tests usability under the
current freshness policy, so unavailable/uncertain/stale-rejected inputs yield
False for that explicit query. Other value-dependent predicates yield Unknown.

StalePolicy defaults to Reject. AllowStale permits known stale values with a
warning attached to the result; uncertain clocks remain unusable. Callers must
retain warnings when showing advice. Reasons/warnings describe visited inputs;
short-circuited inputs are not fabricated into the evaluation trace.

Rule limits default to depth 32, 1,024 nodes and 1,024 evaluation steps. Callers
may lower them, never raise them. Validation checks the entire tree and all
embedded contexts before execution. Execution charges nodes, field/observation
reads, participant scans and modifier comparisons. Limit exhaustion is an error,
not False or a partially successful evaluation. Rule expiry uses trusted current
UTC; missing trust yields Unknown and reaching expiry yields False. Cooldown,
dismissal, trigger dispatch and UI behavior remain the caller's responsibility.

## Wave-1 foundations and wave-2 boundary

Merged library modules include travel, eligibility, gear, reminders, sessions, sources and coverage. SQLite snapshot ordering and bounded ANAV1/SavedVariables parsing live in separate storage/ingest crates. These are synthetic or imported-state foundations, not verified live integrations. Wave 2 retains exchange schemas and uses explicit desktop DTOs; see ADR 0004.

## Wave-2 services and independent library foundations

`planner` applies exclusions and three-valued predicate/quest eligibility before
exact utility/time comparisons. Utilities and versioned goal profiles are caller
inputs; completion, service and stopping time count toward expected feasibility.
Routes retain their declared uncertainty. Intermediate stateful travel is excluded
rather than assuming cooldown resets. The baseline does not simulate future quest
unlocks or invent party fairness weights.

`gear::acquisition` separates guaranteed, uncertain and unavailable opportunities.
Probability estimates require an explicit independent player-acquisition model;
finite trial capacity differs from infinite-horizon expected attempts. Caller
spending budgets differ from observed balances, whose freshness must support use.
Full reward identities avoid duplicate credits. No currency/time conversion exists.
`gear::lua` compiles bounded synthetic scoring DTOs; Lua agrees with Rust fixed-point
components/rounding/ties. Neither compilation nor scoring establishes equip legality.

`analytics` consumes validated session builds, checks public timing totals and
clock fields, and preserves original identity/context/observation metadata. Observed
level change, unique turn-ins and equipment-change counts do not invent XP, loot or
combat data. Observed/estimated/unclassified/gap/uncovered time remains separate.
Covered monotonic time is not wall-clock duration.

`azeroth-app::snapshots` provides explicit import/query/profile/watch DTOs over
bounded normalization and existing SQLite storage on blocking workers. Opaque
profile IDs resolve the complete key; receipt freshness and fixture flags survive
storage outcomes. Desktop imports are character-only for this wave. Lua plan state
is transactionally replaced only on validated import; invalidation is not completion.

Storage schema v2 adds immutable, bounded reference revisions, source/conflict
records and context/fixture-qualified active pointers. Staging/activation revalidate
coverage, sources and identity; failure preserves the old active revision. Rollback
and supported online SQLite backup retain existing snapshot hash behavior. These
storage tables are separate from the unchanged v1 exchange schema.
