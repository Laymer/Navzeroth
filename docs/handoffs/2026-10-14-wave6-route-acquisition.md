# M5.4 guarded route-derived acquisition

Owner: Codex `/root/release_review`, sole writer. Published baseline0.5.0
f525911ba13244cd0dbb26a78ada59fe91dd5184. Branch `feat/route-acquisition`.
Claim3130dd5 preceded code. Normal merged current mainb5e6e11 guard foundation
before final checks. Source frozen52010b6dee09ef1e724aa400077eadf25fc4567b;
source ownership released to root for combined integration. This task owns only
its new service/panel/DTO/tests/Gear integration and approved feature-specific
additive lib/native/client/contract/demo entries. Clustering and recipient export
remain distinct task writers/PRs. No main merge or release by this worker.

## Interfaces and behavior

`SnapshotApplication::preview_acquisition_route(text,now)` accepts the existing
strict bounded PlanningRequest. It calculates the authoritative baseline through
`with_planning` and returns schema1, route_fingerprint, planning_report and ordered
legs (selected travel then final stopping). Fingerprint includes exact captured
snapshot/content, request and chosen routes; it excludes hardware elapsed time.
The final existing PublicationGuard rejects changed identities, observation
freshness classes and expiry after compute/store waits. No cached frontend report
is accepted as input. Calculation stays outside the shared store lock while the
native host lease protects lifecycle replacement.

`assess_route_acquisition(text,now)` accepts strict schema1 with planning request,
expected_route_fingerprint,1..128 independent opportunities and optional GearRequest.
An opportunity supplies source AcquisitionId, complete ItemKey, optional exact
NodeRef manual binding, insertion leg and explicit formation/attempt/spending/
trial/cost-schedule assumptions. Absent binding remains unknown. A supplied node
must match a registered node/map/context exactly; source and target must belong
to the exact active content. No source location is inferred from item names,
quest labels, positions, atlas imagery or graph proximity.

Assessment recomputes and checks the baseline, replaces one directed route leg
B with A(start→source)+C(source→B.end), and uses checked nonnegative
`A.total+C.total-B.total` incremental time. Remaining acquisition budget is the
session budget minus the entire baseline, including its ending/service costs.
Core AcquisitionEffort receives this delta and known0 additional stopping time
because rejoin/return is already included. Formation and bounded finite trials
consume the residual budget; reserved total never double-charges baseline travel
or ending. Unknown/restricted requirements, route costs and caller observations
remain explicit. Incomplete searches preserve reachable segments/exclusions;
there is no fabricated gap connection. Estimated graph edges remain estimates
and do not establish walkable road geometry.

Stateful outbound/intermediate actions are refused. A single verified reserved
final action may remain at the complete route ending; detours cannot make it
reusable across legs. Source conditions/cost/probability and the declared player
trial model feed the existing pure assess_acquisition. Zero is distinct from
missing, probabilistic outcomes remain conditional, and infinite-horizon expected
attempts never replace tonight's finite trial count. Opportunities are independent
alternatives, never summed into a combined feasible schedule or joint probability.

Known selected quest rewards suppress duplicate credit by full ItemKey, retaining
variant/context identity. Unknown quest reward coverage is explicitly flagged and
blocks additional credited reward/practical ranking. Optional gear must reference
the same profile/snapshot/revision and supply no acquisition effort. The backend
injects only its authoritative derived effort into gear::calculate under the same
capture and final guard. Theory/eligibility stay separate from acquisition, and
unqualified probabilities or incomplete baseline/model still block practical advice.
Inputs cap at1MiB; preview/report serialization caps at4MiB via a counting writer.
These byte/count caps are not universal runtime/native-memory/latency guarantees.

## Desktop flow

Gear exposes a separate route-assumption editor, explicit preview, per-source
manual node/insertion selection and independent assessment. It never reads hidden
PlannerScreen results. Source bindings start unknown; goal/nodes/time/utility must
be explicitly supplied. Existing manual-effort comparison remains available and
its report basis is labelled separately from backend-derived route effort.
Preview/assessment commands are typed; plain browser demo reports unavailable.
The two native handlers run on blocking workers; no native serde dependency or
new exchange schema is required.

Profile/client changes reset assumptions. Same-profile refresh clears advice while
preserving compatible edits and pruning removed objective/node references.
Route/model/source/trial edits invalidate derived advice; deferred old previews or
assessments cannot repopulate it. Busy state is shared with manual Gear comparison
so overlapping modes cannot publish competing replies; editing/navigation remain
available. Route gaps, full source assessment, reward-accounting gaps and declared
assumptions remain inspectable. No default role weights, real source locations,
shared-kill savings, useful lifespan or loot eligibility is invented.

## Actual checks and corrections

Final11 route integration cases passed, plus the streamed output-cap unit. The
23 app library and6 existing Gear workflow cases passed on the branch after main
integration; subsequent test-only stateful additions passed the11 route suite.
Strict all-target app Clippy, Rust fmt and git diff --check passed.
Desktop135 unit tests across23 files, typecheck/lint/build passed. Eight affected
Gear/panel unit cases cover explicit DTOs/fingerprint, late preview edits, deferred
profile/refresh/model/client changes and preserved/pruned refresh edits.
Two affected mocked Chromium flows passed: existing manual Gear and new explicit
route Gear flow. The latter verifies actual Rust fixture baseline61000ms, remaining
39000ms, delta5000ms,3 finite trials and reserved total98000ms; frontend sends only
manual bindings/assumptions and no editable derived travel. Editing budget removes
both reserved advice and the Gear result. Port1420 was actually released afterward.

Rust-generated synthetic baseline/report/catalog/input/Gear/snapshot fixtures are
committed, with normal report equality regression and explicit
UPDATE_ROUTE_ACQUISITION_GOLDENS=1 regeneration. No personal exports were read.
Initial synthetic fixture construction retained the old uncertainty range and
used manual_entry observations inside fixture content; contract validation refused
it. Corrected uncertainty fields and synthetic metadata. A zero-probability test
initially compared JSON floating0.0 with integer0; corrected its numeric assertion.
Initial frontend wiring inserted two methods into the wrong return block; TypeScript
caught this before commit, and methods now live only in createClient. New mocks
needed honest planning-catalog responses; initial lint/type fixture/import mistakes
were corrected before final checks. A browser select label included nested options;
changed only harness locators to the existing role/name convention. One obsolete
locator run was interrupted; no failed/interrupted run is counted as passing.

Local native cargo check was actually attempted and failed at missing dbus-1.pc
before shell compilation. Native interaction, Windows actual handler/file/display
behavior, live game acceptance, real acquisition catalogs/node associations,
probability/loot evidence and role weights remain UNVERIFIED or unavailable.
Hosted exact-head Windows compilation/nine jobs and source-locked independent
review remain required before normal protected-main integration. Next: root review,
resolve additive task wiring/cluster result conflicts normally, rerun exact heads,
and audit the next release without promoting engineering fixtures to real advice.
