# Source admission handoff

Owner / sole writer: Codex `/root/source_admission`.
Baseline `d59aa91`; claim `55fca22`; branch `feat/release-source-admission`.

Implemented an additive pure `azeroth_core::source_admission` v1 contract and
[admission documentation](../sources/source-admission-v1.md). Root explicitly
approved the single core module registration. No other shared contract changed.
The committed registry admits no sources. It keeps source origin independent of
rights and target verification, rejects NC/unknown grants, and requires reviewed
scope-bearing rights/access evidence, exact file/field lists, immutable pins,
attribution and obligations. Hashes identify reviewed bytes, not legal authority.

Actual checks on implementation:

- `cargo test --locked -p azeroth-core --test source_admission`: 5 passed.
- `cargo clippy --locked -p azeroth-core --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.

Tests cover denied rights, unknown origins, strict versions/fields, byte limits,
duplicate IDs, immutable revisions, exact file/field/method authorization,
traversal/wildcards and retrieval overflow. They use synthetic declarations only.
No live-client, Windows, hosted nine-check or independent review result is claimed.

Research read Fanbyte's current terms and the pinned RestedXP license; Warcraft
Wiki policy retrieval failed and remains unverified. Previous QuestieDB/CMaNGOS
rights/build blockers persist. No imported gameplay data, raw exports, permission
messages or downloads were created. No complete redistributable target inventory
is available.

Next: independent review and all nine hosted checks; obtain grants and target
inventory, then implement acquisition adapters that enforce returned limits and
hashes before feeding inert staging. This module is not wired to an acquisition
adapter or the stable release evaluator; it cannot relax existing release gates.
Existing audit-only staging intentionally remains available without pretending
its data is admitted. Future release integration must evaluate actual admitted
source and target evidence together. Preserve preview until prerequisites exist.
