# Route segment selection and per-frame fit

Owner: Codex /root/atlas_dense (sole writer). Published baseline v0.7.0,
035f28ece68b0b81680ec42bbe2a792b8a95140d, published2026-10-14T15:14:38Z.
Branch feat/route-segment-fit; isolated /tmp/navzeroth-wave8-route-fit.
Read STATUS,0.7 release handoff, plan12.2 and frozen wave8 coordination before code.

Scope: accessible keyboard leg/hop selection, bounded hop/frame pages and exact
same-frame endpoint fit using the existing guarded inspect_plan_route DTO. Unknown
placement, missing floors, incomplete traces, stopping/tail hops, zero costs and
false states remain explicit. Endpoint links remain schematic; no road/floor
geometry, new map observations or live bridge is introduced.

Owned: RouteInspection, feature-local selection/fit helpers/tests/CSS, additive
MapView overlay/fit support and necessary PlannerScreen provenance guards/tests.
No App/reference/native/core/service changes are planned. Root owns reconciliation.
User authorizes isolated task PRs and granular pushes. Browser1420 requires a lease.

Checks/decisions/limitations: pending implementation. Native/live acceptance remains
unverified. Next: implement full-data navigation with bounded rendering, verify
late-reply invalidation and exact-frame fit, source-lock checks and draft separate PR.
