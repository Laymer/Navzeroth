# Expanded local research projections

Continuation 4 adds projection format 2 and profile
`navzeroth-local-projection-v2`. Format 1 retains its historical reader and
canonical encoding; a historical projector profile receives integrity checks
but is not claimed reproducible by a different current profile. Unknown versions
are rejected. Every projection remains research-only and unavailable for planning.

The exact retained layouts are admitted for development tuples 1.60.1.70374,
1.60.1.70425 and 1.60.1.70450, `wow_classic_beta`, explicitly selected supported
locales. Tests compare every projected column and array shape with each retained
definition. No nearest-layout or edition fallback exists.

Alongside existing item, slot, map, flight and class/race projections, the expanded
contract maps item budgets/classes/subclasses; skill lines, proficiency and class/
race masks; spell names, descriptions, levels, effects, equipment requirements,
learning links, timing, ranges and power inputs; talent ranks/tabs/prerequisites;
areas, map difficulties, UI map links, quest POI blobs/points, quest identities and
line ordering. The authoritative column list and exact hashes live in the pure
core `local_data_gameplay::TABLES` and `ITEM_STAT_INPUT` contracts.

Arrays preserve each cell's unknown/unsupported/missing state independently.
Known zero remains zero, including map ID zero. Signed masks retain the declared
width; race masks use the declared playable-race bit indices. Reference summaries
retain unresolved, unavailable, ambiguous and sentinel outcomes separately.
Known talent cycles and their dependent talent IDs are retained and blocked.
Domain source completeness includes expanded sources and is distinct from
independently reviewed content completeness or gameplay availability.

`research_budget_model_v1` calculates only a base budget amount from observed
allocation and budget inputs. Its applicability is unverified: socket cost,
bonus/scaling/server behavior are not established. Nonzero socket fractions stay
unsupported. Missing inputs never become zero. Model outputs always carry
`verified_for_gameplay: false`; they cannot justify gear advice. POI coordinates
remain uncalibrated. Quest IDs do not prove server availability and line order
is not a prerequisite.

Mapping is bounded by the existing projected-record quota plus four million
expanded cells, 32 MiB aggregate text and 64 KiB per text. Failed projection or
facts building leaves existing active state intact. Artifacts remain private,
content-addressed and bound to the catalog, tuple and projector source profile.
Guide facts and observation trust are documented in [GUIDE_VERIFIER](GUIDE_VERIFIER.md).

Desktop presentation, delivery dependencies, route design verification and exact
nine-domain inventory are later continuation tasks. No operator proof of actual
item amounts, calibrated travel/POI coordinates or universal quest eligibility
was obtained in this task.
