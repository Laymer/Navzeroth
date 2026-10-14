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

## Frozen implementation

Production and tests: `1c9e8777f6da3757f18a5e409b31ae563bf9f851`.
Granular history: claim `ce2c46b`, normal frozen coordination merge `3a082b8`,
endpoint overlay/fit `863b44d`, accessible route ownership/pages `1c9e877`.

The view renders at most50 hop rows/links and one map frame. All returned hops,
including stopping/tail entries, remain reachable with page buttons and global
Up/Down/Home/End/PageUp/PageDown navigation; selection/focus use the global index.
The selected native button has aria-pressed; schematic selection additionally uses
thickness/dash/endpoint size, inheriting currentColor for forced-color compatibility.
Frame controls never combine floors/instances. Fit all endpoints uses the entire
returned same-frame route, while the overlay explicitly identifies links from the
current page. Fit selected uses index0 correctly, including coincident/zero endpoints,
and existing bounded zoom/pan rules; later manual navigation cancels resize refit.

Only explicit schematic_connection=true, both placed on the exact frame image,
matching full map/floor/instance/context and finite in-frame pixels can form a
link/fit box. No raster or artificial calibration/evidence metadata is generated.
MapView accepts only the actual image fields it uses, so route frames do not pretend
to contain Atlas layers/transforms. Per-hop details preserve placement/evidence,
unknown/restricted/unsupported observations, known false and zero. Gaps are never
joined; original trace/alternatives/stopping data and termination remain inspectable.
Hop/frame rendering is bounded; the original backend-bounded decision JSON remains
in its details block and is not a new independently paginated trace representation.

Each new guarded inspection response remounts selection, page, frame and viewport.
PlannerScreen retains its original client/profile/refresh/generation/job-cancel/pin
owner gates and binds the route to the exact original report object. A reply must
match all serialized original report fields (ordered arrays, context/fixture/request,
choices/stopping, clustering and trace), plus its top-level context/fixture; JSON
property order is ignored. Mismatch clears the old plan/pin/export and asks for a
new calculation. The existing native command performs authoritative recomputation;
no absent hash/token is fabricated. These are imported estimates, not live position
or continually renewed freshness claims; actionable exports retain backend guards.

## Actual checks

All checks ran in this isolated branch with Node/pnpm and synthetic fixtures:

- `pnpm exec vitest run src/screens/RouteInspection.test.tsx src/screens/PlannerScreen.test.tsx src/atlas/viewport.test.ts` in apps/desktop:24 tests/3files PASS.
- `pnpm exec vitest run` in apps/desktop:189 tests/31files PASS.
- `pnpm --filter azeroth-desktop typecheck`:PASS.
- `pnpm --filter azeroth-desktop lint`:PASS.
- `pnpm --filter azeroth-desktop build`:PASS,77 transformed modules.
- `pnpm exec playwright test e2e/workflow.spec.ts --grep 'content activation enables|route segment keyboard'`:2 mocked Chromium cases PASS. New103-hop synthetic flow proves initial index0, fit expansion, End/tail focus, absent cross-floor tail link, previous-page/Home focus, empty alternate frame and input invalidation. Existing content/planner/export flow passes with its actual report supplied in the mocked route DTO.
- `git diff --check`:PASS. Browser server stopped; `ss -ltnp 'sport = :1420'` confirmed no listener and lease returned to root.

Initial checks found/corrected MapKey.map_id typing, test fixture namespace/confidence
literals and forbidden non-null assertions. First test queries expected a region
where MapView uses a group, an absent zoom label and non-unique text; corrected
assertions inspect real line coordinates/roles. A script `--` forwarded unit filters
incorrectly and ran the full suite; final focused runs used pnpm exec directly.
The first new browser attempt used getByLabel for native selects and timed out;
corrected to the existing role/accessible-name selector. Final source checks above
pass; assertions/cases were retained. Native Windows focus/DPI/performance, actual
screen-reader operation and live recorded-build/game geometry remain UNVERIFIED.
No Rust/native files or schema/dataset/version identities changed.

Next: independent source review, all nine exact-head hosted jobs, root reconciliation
with the other isolated UI tasks, then the authorized0.8 release/asset checks and a
fresh released-source audit. Do not claim native/live acceptance or complete M6
merely from these portable UI checks.
