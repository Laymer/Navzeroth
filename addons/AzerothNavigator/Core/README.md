# Pure plan/HUD foundation (R4)

These modules execute only repository-owned logic. JSON imports are data;
there is no evaluation of imported Lua, network bridge or client API binding.
The existing diagnostic entry point does not yet offer a plan-import UI.

`Decode.decode(text)` returns a value or `nil, code`. JSON null is
`Decode.null`, distinct from absent members, false and zero. Object/array kind
and integer versus floating number lexemes survive decoding in metatables.
Defaults: 32 KiB text, 32 container levels, 8192 visited values, 1024 entries per
container, 4096 decoded UTF-8 bytes per string. Duplicate keys, invalid JSON,
invalid Unicode and non-finite numbers fail. Fixture runners may explicitly
raise decoder budgets; `Plan.importText` always uses the production defaults.

`Predicate.validateInputs(tree, snapshot, party)` validates the complete rule
and every observation before evaluation. `Predicate.evaluate(tree, context)`
returns Rust-compatible truth, steps, ordered reasons and warnings or an error
code. Context supplies snapshot, optional party, freshness, stale policy and
optional rule limits. Default limits are depth 32, nodes 1024, steps 1024;
callers can only lower them. Freshness takes an injected UTC/session clock;
no clock or player observation is read by this module. Unknown is never true.
Identifiers and sorted modifier maps use UTF-8 byte semantics. Party copy
comparison applies Rust defaults and ignores unknown struct members.

`Plan.new({context=..., fixture=...})` freezes the acceptance boundary.
`state:importText(json)` validates a route-plan v1 envelope, context,
dependencies, IDs, timing, predicates, participants, evidence, source claims,
and addon limits (100 steps, 8 alternatives, rule depth 12). Failure retains
the previous plan. Success returns `{id, fixture, steps}`. `state:get()` returns
an independent decoded envelope; `state:clear()` removes the loaded plan.
`state:revision()` changes on successful changed-content replacement or clear;
identical reimports and failed replacements retain progression.
Fixture and real documents cannot be interchanged at this boundary.

`Hud.new(state, observe, render)` accepts callbacks. `hud:refresh()` calls
`observe()` for an evaluation context with an explicit `fixture` flag, derives a
new model, passes it to
`render(model)` and returns it. Models distinguish `no_plan`, invalid
observations/context, uncertain clock, expiry, evaluation errors, unknown,
blocked, ready, invalidated and no applicable steps. Fixture labels, stable step IDs, explanations,
alternatives and predicate diagnostics are retained. True invalidation retains the step as invalidated and requests revalidation; it
does not prove completion. Unknown prerequisite/invalidation suppresses ready.
For PlanStep gates, not_applicable declares no prerequisite/no invalidation, so
it yields true/false respectively; all other unavailable statuses stay unknown.
M4.6 adds participant-scoped, in-memory advancement. A fresh, known true
objective observation records completion for that participant and step; false,
missing, unknown, restricted, unsupported and stale observations never advance.
Observed completion is retained within that loaded revision to avoid oscillation;
a changed plan or clear resets it. This is not persisted across HUD instances/reloads.
Completion uses the exact declared step objective ID. Quest-stage readiness and
invalidation are not completion evidence; adapters must explicitly supply the
aggregate objective observation when supported.

Alternatives are considered in declared order only among unambiguous loaded steps
for that objective and participant. They need fresh known-false completion,
known-true prerequisites and known-false invalidation. IDs without a loaded readiness
definition produce a gap rather than an inferred unlock/route. Keep a ready selected
alternative even when the primary becomes ready; reconsider only when selected
readiness/completion changes. Completing an alternative does not complete the
primary, and its own ordered step is not recommended again. Models include
anchor_step_id, selected_alternative, completion, rejected_alternatives and
requires_revalidation. All advice and advancement reject stale predicate inputs,
even when the injected caller requests allow_stale.

Optional context.dependencies carries dataset_revision, ruleset_version,
score_model_version and optionally snapshots (the exact ordered plan input refs).
A mismatch suspends evaluation as dependencies_changed. The model explicitly reports
dependency_inputs_supplied; omitted live version inputs do not claim verification.
Expiry/context/fixture checks suspend advancement. Live event dispatch, rendering,
objective observation adapters and the plan-import UI remain future environment work.

Checks: `cargo test --locked -p azeroth-lua-harness` consumes all 192 cases in
`fixtures/predicates/v1` unchanged, including the three Lua-only decoder traps,
and all 11 route-plan contract fixtures. Standalone Lua 5.1 and synthetic
callbacks do not verify Forever API availability, client events, frame behavior,
script budgets, TOC compatibility or a real quest transition (M2.6).

## Synthetic gear comparison (R11)

`gear::lua::compile_synthetic` compiles library-only, fixture-labelled scoring
inputs: attribute values become integer thousandths under the existing Rust
rule. Bounds are 64 candidates, 64 profile weights, 128 attributes/item,
64 effects/item and 32 KiB serialized JSON. Keys and availability metadata are
retained; mixed contexts, duplicate keys, live source claims and non-synthetic
weight sources fail. This foundation accepts only SYNTHETIC build/interface in
demo context; no exchange schema changes are involved.

`Gear.new(json)` uses the fixed production decoder and returns a private scorer
or nil/code. Catalog entries are sorted by full item key. `scorer:score(indices)`
scores a caller-selected set of 1-based sorted-catalog indices; repeated or
out-of-range indices fail. `scorer:rank(sets)` orders at most 128 distinct sets
by total, certainty and sorted keys. Every result is labelled model score with
its version/source and retains unmodeled effects and all unavailable statuses.
Returned results are copies. `Gear.definitivelyExceeds(a, b)` requires complete
certainty, equal role/version and a strictly greater total.

`Gear.fixedMul(a, b)` accepts safe integers and uses base-1000 limbs for exact
multiply/divide/half-away rounding; `Gear.toFixed(value)` reproduces Rust's first
IEEE-754 conversion. Arithmetic outside the safe range fails without wrapping.
These are scoring-only comparisons. They do not establish equip eligibility,
configuration legality, acquisition availability or a practical recommendation.
