# Synthetic planner replay format1

Run the existing portable application planning services from one explicitly
selected bounded JSON fixture:

```sh
cargo run --locked -p azeroth-cli -- replay --fixture fixtures/planner/solo-basic.json
cargo run --locked -p azeroth-cli -- replay --fixture fixtures/planner/duo-basic.json
```

These commands were run on the committed synthetic examples. They produce JSON
reports for declared manual assumptions, not verified real-world routes, measured
session times or proof of a beam-search improvement.

## Fixture interface

Top-level fields: `format: "azeroth-synthetic-planner-replay"`, `schema_version:1`,
`now_utc_ms`, `snapshots`, `content` and `request`. Snapshots use existing typed
CharacterSnapshot envelopes; content uses existing ContentRevision including
source/conflict records. Every envelope must explicitly have fixture=true and
namespace=demo. All selected characters/context/fixture identities are preserved;
no build or dataset is relabelled. Sources with unknown policy remain unknown.

Request fields: `mode: "solo" | "joint"`, `participants`, `goal_profile`,
`origin`, `stopping_node`, `budget_ms`, `stopping_service_ms`, `candidates`,
`excluded`. Participants are indexes into the fixture snapshot array. The request
must select the entire cohort exactly once: one solo or2..40 distinct joint
characters. Reordering indexes is explicit; each candidate's utility vector uses
that request order. Candidate fields: `objective`, `node`, `utilities`,
`completion_ms`, `service_ms`. Goal/utility/time values are declared assumptions.
The application derives profile IDs after imports and verifies current snapshot
IDs/content revision when planning; profile IDs are never supplied by the fixture
or echoed in the report. Snapshot source_revision is preserved as captured;
active planning content identity comes from the explicit ContentRevision.

Complete input is limited to16MiB, each character envelope to the existing4MiB
contract bound, and content/dataset to existing contract bounds. The complete
input limit includes the entire cohort. At most128 candidates/exclusions; utilities
must cover every participant. Duplicate indexes/characters/exclusions and missing
or foreign identities are refused. Duplicate JSON members, unknown fields,
future fixture/contract versions and malformed options fail with static nonzero
errors. Explicit optional null/empty members omitted by typed serialization are
outside this strict subset; omit those fields instead. Existing bounded exchange
extensions remain typed contract metadata; the replay does not interpret them.

The runner imports ANAV1 JSON into an in-memory application store, stages and
activates content, creates a local party when requested, then calls the normal
solo/joint service. No filesystem database, native window, socket, imported Lua
execution, live observations or external services are involved.

## Time and output

Explicit UTC is required. The public application's current-snapshot query must
report fresh trusted capture age at most10seconds. This intentionally narrow
synthetic window leaves ample margin under the application's current30minute
capture policy; real/near-expiry replay inputs are outside this feature. Positive
fixtures capture exactly at declared now. The application may advance supplied
UTC by actual compute elapsed for its final freshness publication gate; no hardware
elapsed measurement is included in output or used as a scenario expectation.

Reports identify schema1, fixture=true, mode, declared UTC, logical participant
indexes and exact content context/revision. They include selected steps,
exclusions/readiness evidence, alternatives, travel and termination from the
application result, plus its filtering and explicit assumptions. Fixed
source-policy fields preserve unknown versus known enum/false values; they are
fixture metadata, not production source permission evidence. Unknown/restricted
eligibility blocks advice; false and zero remain explicit. JSON is serialized
through a4MiB bounded writer completely before any stdout publication. Byte caps
do not establish universal runtime/memory budgets or native performance.

Selected files and all ancestors must be regular/no-symlink paths; parent traversal
and Windows reparse paths are refused. There is no recursive discovery or raw
input echo on errors. As with local pack workflows, hostile concurrent filesystem
replacement is outside path-check confinement guarantees.
