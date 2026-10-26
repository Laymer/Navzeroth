# Installed workload catalog DTO repair

Owner/sole writer: Codex `/root/desktop_review`; baseline7db6e76ae94c4e5492e5977bb02a049c187b84c0,
claim56a57c81; code01bf35a7062640675304ca21cb77656d78ae2af4.
Isolated fix/stable-installed-workload-pages, `/tmp/navzeroth-stable-installed-workload-pages`.

## Actual failure

V6 package39413135905 used ffbf/b689 production0a installer
SHA73fe854cf239b5b70fdcfc01a330bc25bf325f70a94dfdc170c171db3fdf57a4.
Raw install/resources cases reported pass; first-start failed and remaining five
hosted cases did not run. No complete acceptance pass occurred and canonical eight
hosted observations plus owner smoke remain unverified. Recording/admission of raw
case timestamps is a separate tooling-owned repair; raw pass labels are not an
admitted canonical pass here.

Raw fresh report `/tmp/navzeroth-v6-hosted-39413135905/raw/raw/fresh-016fae9c9c3f45cc98356b10eacf95d7.json`
reported ready database, startup load223632ms, activation0ms, failed elapsed642561ms,
and `Actual installed world catalog is empty`. These are failed-workload partial
measurements on hosted hardware, not a successful full workflow/performance claim.

`planning_catalog` actually publishes pages only for Nodes and projected Objectives.
The driver required pages.quests.total and then looked up absent quests/items/edges
pins, so it necessarily failed even with healthy nonempty content. Its synthetic
mock incorrectly supplied those additional descriptors. This does not prove the
world data was empty or activation failed. An initial investigative suspicion of
nested RecordPage serialization was withdrawn: Rust uses serde(flatten), and the
existing top-level next_offset shape was correct and remains unchanged.

## Repair and actual checks

The existing nodes descriptor supplies the real global immutable catalog revision
and SHA for generic quests/items/edges requests. Core/storage indexes bind those
values to the entire catalog and validate the requested collection. Projected
objectives retain their separately issued catalog-bound descriptor; the planning
request validates snapshot_id separately. The current implementation uses the same
global hash for both descriptors; the distinct-hash test is explicitly synthetic
and checks that the driver respects each issued binding. No fabricated SHA,
new API, eligibility promotion, record truncation or source change is used.
Returned pages must match collection/revision/hash/offset, safe total, <=200rows,
exact remaining row count and next offset. Actual quest/item browsing and supported
source-edge/quest-stage planning remain required; unknown eligibility still refuses.

Before the assertion, a callback preserves bounded diagnostics in failed raw
reports: fixture/revision/snapshot identity, node/objective counts and at most16page
names/totals/revisions/hashes. No records or full catalog dump is captured. Existing
failed/fresh/restart/recovery/preferences/rollback behavior remains checked.

Actual validation:

- Old driver against the corrected Nodes/Objectives-only mock fails with the same
  empty-catalog error; preserved `/tmp/navzeroth-stable-installed-workload-before.log`,
  SHA82c2c9bc57012cc0f59a841d9735b5272c47e66d7c44f645e11a7f8ec430b762.
- Node17tests passed, including exact current DTO shape, mismatched page identities/
  counts/offsets/continuations, second-page edge/objective selection with distinct
  projected hash, bounded failed diagnostics and existing unknown/persistence cases.
  `/tmp/navzeroth-stable-installed-workload-node.log`,
  SHA107d21e30be393640178138fbc3c97b7644a984c8a17dc7ac17409ca717efab8.
- Actual release-mode app tests12authored +1newtypedDTO fixture passed. Real
  SnapshotApplication returns Nodes/Objectives-only descriptors and successfully
  pages all three other collections with the issued nodes pin, rejecting wronghash.
  Direct typed serde fixture proves flattened RecordPage fields. No handwritten
  DTO is represented as an actual installed capture.
  `/tmp/navzeroth-stable-installed-catalog-app.log`,
  SHA7433677446741ef086b103b7504d08bdc7bc7c31a38c7a83335cf8d35bbd754f.
- Rustfmt and gitdiffcheck passed. The initial format check requested formatting
  only in the new Rust test; it was formatted and passed. No full suite/native
  Windows installation or hosted acceptance rerun was performed by this writer.

Runtime Rust implementations and content/datagen/Cargo/source pins are unchanged;
there are explicitly new/updated Rust tests and one typed serialization fixture.
The actual fcf106 full-pack proof retains its original source/production scope and
must not be restamped. Driver/test production bytes change and require independent
review, normal integration and renewed release bindings/checks before publication.
No canonical release files, signing keys, signatures or acceptance statuses changed.

Next: separate repair PR into release/stable-1.0.0-version, independent review,
normal integration with the separate recorder repair, renewed production closure
and fresh exact-installer hosted acceptance. Owner smoke still requires that exact
verified draft installer; no tag or publication is supplied by these tests.

Independent final review: coordinator and world specialist each ran17Node checks
and cleared the implementation. World correctly identified terminology: current
objective catalog_sha256 binds content, not a snapshot; snapshot_id is validated
separately. Only comment/handoff wording was corrected after that review. The
synthetic distinct-hash fixture is not claimed to be an actual source hash.
