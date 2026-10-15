# Nine-domain inventory automation handoff

Owner/sole writer: Codex /root/inventory. Baseline `d59aa91`.
Claim `12ee753` precedes implementation. Branch `feat/release-inventory`.
Coordinator delegated own module and CLI registration; shared release contracts
remain unchanged. Frozen next-contract requirements were sent to coordinator and
source-admission/content-contract worker and recorded in
[the inventory guide](../data/RELEASE_INVENTORY.md).

Implemented pure strict versioned investigation comparison and bounded CLI
`inventory-report`. It preserves all nine domains, unknown vs evidenced empty,
source/revision/origin provenance, missing/unexpected/blocked identities,
per-source comparisons and cross-domain scenario dependencies. The deterministic
observation queue identifies unresolved enumeration, imports and scenarios.
CLI verifies local evidence pins and binds reports to the exact input SHA-256.
It never authorizes publication, changes release inventory or admits source rights.

Actual checks:

- `cargo check --locked -p azeroth-cli`: passed.
- `cargo test --locked -p azeroth-core --lib release_inventory`: 10 passed.
- `cargo test --locked -p azeroth-cli --lib release_inventory`: 4 passed.
- `cargo clippy --locked -p azeroth-core -p azeroth-cli --all-targets -- -D warnings`: passed.
- `cargo fmt --all` and `git diff --check`: passed.
- CLI command against committed input: nine unknown domain counts, 18 queued
  investigation tasks, zero evidence files and publication authorization false.

The first CLI evidence test failed because its test fixture hardcoded an incorrect
byte count. Replaced the constant with the actual byte length; all four CLI tests
then passed. No failed run is counted as acceptance.

No target game denominator, complete source corpus or source rights were invented.
Evidence independence is a reviewed declaration; differing file hashes cannot
prove the truth of an observation. Live client, native Windows and full-corpus
performance remain unverified. Hosted checks/independent review are pending at
handoff creation; the draft PR is not authorization to merge before those pass.

Next: independently review PR2 and the guided acceptance kit; integrate only after
all nine checks. Implement reviewed content v2 contracts/consumers using the
frozen requirements; accept sources and enumerate their actual target scope before
populating inventory. Final v2 binding names and party support matrix remain
coordinator-owned decisions. The preview release remains unchanged.
