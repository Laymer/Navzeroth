# Conservative automatic objective clustering

Owner: Codex /root/backlog_audit, sole core-module writer. Released baseline
v0.5.0 f525911ba13244cd0dbb26a78ada59fe91dd5184. Branch
feat/objective-clustering; final source/test commit 469599c. Root confirmed v0.5.0
publication before task assignment; baseline STATUS still describes preparation.
Read STATUS, release/M4.4 handoffs and plan 9.4–9.6/M4 before claim/code.

## Behavior and frozen interface

New core objective_clustering module exposes PreparedContribution,
PreparedObjective, ClusterPlanningInput and plan_clustered / controlled variant.
Contributions carry utility, actual independently evaluated truth and original
EligibilityEvidence in the same participant order for all objectives. Explicit
participant_count also covers empty catalogs. Callers own snapshot/build/content
validation and eligibility evaluation; joint callers supply the graph already
usable by every participant. This is a pure assessed-input API, not an import
contract. Prepared types are not deserializable.

Readiness, exclusions and positive utility are checked per original/participant.
Same-declared-node candidates are packed in deterministic original-ID order with
disjoint reward identities. Every singleton remains a variant. Remaining originals
regroup after each choice; both chosen-original occupancy and selected rewards
prevent reuse, including objectives whose reward sets are empty. Nearby coordinates
never establish compatibility. Completion and service durations are checked sums;
inbound travel is charged once. No shared-kill or service-duration savings inferred.

Ranking uses the existing controlled greedy planner one step at a time. Its empty
All gate is only a neutral routing/scoring adapter, never replacement readiness
evidence. Joint cluster utility is the minimum of each participant's summed
contributions. Each flattened original Choice retains its individual minimum
contribution; their utility sum can differ from the cluster fairness score.

ClusteredResult metadata schema_version=1 includes original assessments, selected
clusters, ranked whole-cluster alternatives, membership decision traces and actual
variant counters. Its plan is an unchanged PlannerResult with original IDs, travel
and duration composition. A cluster's first original carries inbound travel;
subsequent originals carry a zero-hop route at the same declared node. Flat legacy
alternatives are singletons outside the selected pack and occur on its first step;
other pack members have empty flat alternative lists. Explicit cluster metadata
retains whole-pack replacement choices. No internal scorer slot IDs escape.

Limits: 128 originals/exclusions, 1–40 participants, 1–64 original selected steps,
at most 192 variants per iteration and 1024 reward identities per variant. Packs
respect remaining original-step capacity and reward capacity. At most 64 selection
iterations produce 12,288 variant-trace and 8,192 original-trace rows. Preparation,
packing, participant sums, ranking routes and selection have cancellation checks;
errors propagate without partial success. Sorting, collection primitives and
individual evidence clones are bounded cooperative spans, not instruction-level
cancellation. The pure module supplies no separate serialized byte limit; existing
application publication/export bounds must still be applied by integration.
Stopping travel/service remain reserved. Stateful inbound travel stays refused;
a verified stateful final return remains a single final reservation. Termination
records original step limit or no feasible candidate, not a search-optimum claim.

## Actual checks and corrections

At 469599c: cargo test --locked -p azeroth-core passed 285 tests, zero failures or
ignored tests. The focused objective_clustering suite passed 14 tests. Strict
locked all-target core Clippy, workspace fmt check and git diff check passed.
Logs: /tmp/navzeroth-clustering-{focused,core,clippy}.log.

Synthetic clustering-basic.json replays declared travel amortization: the unchanged
baseline chooses competitor utility 9; clustering chooses one/two, utility 10 and
225ms including stopping service. This is a constructed comparison, not gameplay
or a universal planner improvement. Other tests cover permutation/no-op equality,
joint [9,1]+[1,9] fairness, zero/unknown member refusal, excluded/false/unknown
original evidence, overlapping/empty rewards, over-budget pack singleton fallback,
original step/node bounds, unsupported/stateful travel, overflow/shape rejection,
cancellation including the terminal checkpoint, 128-original/40-participant
workload, the exact 192-variant ceiling and large original reward sets.

The first focused run passed 9 and failed 2 harness expectations: the actual mode
is Return, and a cheaper competitor correctly precedes the pack. Corrected tests,
not the algorithm. Inspection found flat alternatives could repeat a selected
original; the updated replay assertion independently failed source 223e6c0 in an
isolated baseline worktree, then passed the fix. Reward-union inspection also found
valid originals could form a derived pack above the existing planner cap; pack
capacity now preserves valid singleton fallbacks and has a focused regression.

## Integration and limitations

Root separately owns existing planner/joint/application/native/SDK/UI integration,
actual prepared readiness construction and v1 export checks. This module alone
changes no current application workflow. Root reviewed the initial API/source and
will independently review final source and hosted jobs before normal integration.
No beam search, shared-kill inference, proximity model, speculative prerequisite
state, dataset/schema changes, real catalogs or live/native acceptance supplied.
First-fit finite packs plus singletons do not enumerate every subset or guarantee
a global optimum. Native, ordinary Windows performance and live-client checks
remain unverified. Next: integrate the pure API with guarded solo/joint services in
its separate PR, preserve explicit individual-vs-cluster utility display, and run
the actual flattened export and cancellation workflows before release.
