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
Alternatives are displayed in declared order; automatic alternative selection,
revalidation event dispatch, stateful advancement and client rendering belong
to later milestones.

Checks: `cargo test --locked -p azeroth-lua-harness` consumes all 192 cases in
`fixtures/predicates/v1` unchanged, including the three Lua-only decoder traps,
and all 11 route-plan contract fixtures. Standalone Lua 5.1 and synthetic
callbacks do not verify Forever API availability, client events, frame behavior,
script budgets, TOC compatibility or a real quest transition (M2.6).
