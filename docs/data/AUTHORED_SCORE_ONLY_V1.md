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
