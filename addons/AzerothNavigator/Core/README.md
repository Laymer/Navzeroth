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
requires_revalidation. Alternative recommendations set travel_requires_revalidation
and requires_revalidation because v1 global travel has no per-alternative route.
All advice and advancement reject stale predicate inputs,
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

## M2.1 character capture/export core

Snapshot.capture(spec, providers) uses explicit, context-qualified capability
declarations. Only verified_available real providers or synthetic_fixture test
providers are called; untested inputs remain unknown. Scalar unavailable states
and missing map records stay distinct. Failed provider calls emit a closed reason
and never client error text. Returned snapshots are independent decoded copies.

Snapshot.encode(snapshot, {fixture, locale}) produces bounded ANAV1 v1 text.
Character documents contain objects/maps rather than arrays; empty maps encode as
{}, optional nulls are preserved, decimal coordinates retain double precision,
keys are sorted by UTF-8 bytes, and false/zero remain values. It validates context,
identity, observations, metadata and source/fixture consistency before success.
Bounds: 32768 total bytes, depth 32, 8192 values, 1024 keys per object, 512-byte
UTF-8 strings (a conservative producer limit, below the consumer's 4096).
No truncation, raw Lua execution or client I/O occurs.

Snapshot.readLevel(getter, metadata) is an injected, guarded one-read adapter.
Missing getter -> Unsupported; exceptions, restricted arithmetic or invalid
return shapes -> Unknown; valid integer 1..65535 -> Known. It does not turn API
presence into a global capability declaration. Snapshot.store writes only a
complete snapshot_export after validation, retaining prior exports on failure.

Only Diagnostics.lua binds an experimental explicit level capture to the
user-reported 70338 build. Other adapters, clocks, locale codes and real capture
verification remain future work. Existing diagnostic encoder/format are unchanged.

## v0.2.0 explicit clock trust

Clock.new(utcSeconds, monotonicSeconds) injects callbacks; no client I/O in Core.
probe records an uncertain UTC candidate and clears prior trust. confirm requires a
pending probe, continuity within five seconds and a monotonic delay no longer than
120 seconds; Diagnostics exposes it only as explicit operator acknowledgment after
comparison with trusted UTC. Readable return shapes alone never set clock trust.

capture samples monotonic before/after UTC (maximum two-second span), rejects bad
values, and detects backward readings or cumulative/adjacent drift beyond five
seconds. Any failure/jump revokes trust. UTC candidates are recorded as integer
milliseconds even while uncertain; missing clocks leave the timestamp absent.
No session-relative time is exported into the persistent logical producer stream.
clear and a new Clock instance revoke all trust; nothing is persisted.

Callbacks are experimentally GetServerTime/GetTime in Diagnostics, not verified
Forever capabilities. Existing level capture remains available if clocks fail.

## Experimental bounded quest capture (wave 3 task 2)

`Snapshot.readQuests(getCount, getInfo, context, metadata)` enumerates at most
1024 log entries, with a final count consistency check. Exceptions, incomplete
entry shapes, duplicates and changed/oversized counts reject the whole read.
Confirmed headers and malformed IDs are excluded with bounded reason counts.
Quest identity is the canonical decimal API ID; no authored identifiers are guessed.
Accepted records carry Known true; readable boolean completion retains false.
Unreadable completion and turn-in history remain Unknown. Quest objectives and
snapshot objectives stay empty because no validated stage mapping is available.
Missing adapters produce an empty unknown map with an explicit diagnostic.

Only the manual `/anavdiag snapshot <anonymous-profile-id>` command binds these
experimental reads, on the previously recorded 70338 tuple. Successful captures
store diagnostic counts separately in `snapshot_quest_diagnostics`. Incomplete
reads or export size failure preserve the previous text and sequence. No background
capture, completion-history API or event subscription was added. Synthetic harness
success does not verify actual-client readability or restricted-context behavior.
