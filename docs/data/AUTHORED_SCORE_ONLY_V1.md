# Individual authored heuristic scores, application response v1

The direct desktop `compare_gear` command retains the existing full gear response
when the selected model provides usable equipment facts. When an applicable
installed model has unavailable slot layout or class proficiency, it instead
returns `mode: "authored_score_only_v1"`. This distinct response has no baseline,
configurations, configuration rankings or practical recommendations. Exchange and
content schemas are unchanged; this is a versioned application DTO.

Inputs use the existing bounded GearRequest: at most 1 MiB, 128 distinct candidate
identities, current snapshot and active revision. The model must match the user's
explicit class assumption, cover the fresh observed level, and pass its role,
applicability and exclusion predicates. Unknown, unsupported, restricted, stale or
mismatched requirements are not interpreted as applicability. Source facts and
snapshot observations are never rewritten.

The response includes the request with the source model's weights, fixture/context
identity, original `source_layout` and `source_proficiency` observations, and
individual `{item, score}` records. Both `equipment_eligibility` and
`acquisition_availability` remain explicitly unknown. Items appear in stable
identity order, rather than score or recommendation order. Core score arithmetic
preserves contributions, missing attributes, unmodelled effects and certainty.
Zero can be only a subtotal when certainty is incomplete.

Manual layout, proficiency, category and acquisition inputs cannot replace missing
source facts. The response cannot establish equippability, availability, legal
configurations, baseline gains or practical recommendations. Scores are original
heuristics, not measured damage, healing or survivability. The desktop renders a
separate section and clears it when relevant inputs change.

`gear::calculate`, route/acquisition assessment and the legacy full-report
application method remain strict. They cannot consume a score-only response as a
GearReport. Ordinary full-report serialization is unchanged. Native command wiring
and real installed content require their own verification.


## Explicit declared-role context

The addon currently exports role as unavailable; selecting a model does not fill
that fact. The user may independently check **Use a declared role for individual
heuristic scores** and choose a role. Only then does GearRequest contain optional
`score_role_assumption`. Its absence retains the captured-role checks and unknown
role refuses current bundled models. An empty declaration is refused, and the
chosen role must match the selected model's declared role. Class still requires
an explicit matching assumption and level must be freshly observed.

Only `RoleIs` leaves are replaced by boolean literals in a separate arithmetic
predicate tree. Nested all/any/not and other predicates still evaluate normally;
general captured-role field checks are conservatively refused under this option.
No observation, snapshot, timestamp or stored model is manufactured or modified.
Applicability/exclusions must still resolve affirmatively in this declared context.

Responses retain `imported_role` unchanged and name `role_assumption` separately.
Unknown, unsupported, restricted, stale or mismatched captured roles stay as
recorded. The role declaration makes hypothetical arithmetic possible; it never
establishes the actual role or equipment eligibility. Strict gear, route and
practical calculations explicitly reject requests containing this score-only flag.
The checkbox is unchecked by default and resets on profile/model changes. It does
not supply acquisition inputs or turn an omitted budget into an acquisition claim.
