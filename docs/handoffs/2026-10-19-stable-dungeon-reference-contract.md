# Full dungeon association contract — code only

Owner/sole writer `/root/tooling_review`; baseline14c09182; claimb4410565.
Isolated branch feat/stable-dungeon-reference-contract. Normal merge2c516e2a
preserves original repaira906eb0c and future dungeon entrance/checklist/travel
features. Two conflicts were resolved by keeping original Unsupported overflow
handling plus future adjacent travel and the future policy field scope/review.
Immutable implementation6e80cc7e is frozen for independent review.

[Contract](../releases/DUNGEON_REFERENCE_CONTRACT.md) limits the optional explicit
non-null reference_contract_version to1/2. Missing and1 retain1024;2 permits4096
only for dungeon preparation quests and rewards. Missing serializes absent;
explicit1 serializes present, retaining strict known-field/canonical pack rules.
All other bounds and uniqueness/resolution remain. Existing shared unique() also
imposes1024, so the scoped reference helper applies its explicit limit before
using the existing catalog uniqueness check. Composition validates both versions
and retains the maximum structural profile while preserving list/evidence
precedence and existing conflicts. Full composed graphs re-decode in both orders.

Mapper4 explicitly declares profile2 and carries complete lists through4096;
overflow still yields whole-field Unsupported with actual/exposed0/unresolved
counts. The normalization policy transformation becomesv4 without adding any
upstream fields or grants. Mapper3 original evidence is retained unmodified.

Actual verification:

- Final targeted core:20 content tests and6 composition tests pass,1 explicit
  fixture-regeneration test ignored. Invalid/null/type/version cases, canonical
  absence/explicit1,4096/4097,1024/1025, duplicate/unresolved IDs, travel limits,
  nonnullable schema and both full-graph composition orders are covered.
-175 offline Python tests pass, including generated4097 canonical quests and
  acquisitions retained with Unsupported association fields, and4096 Known bound.
- New real pack build/canonical verification/read_verified test passes with1025
  complete references and legacy-profile refusal.
-9 storage catalog tests pass, including exact1025-reference reopen and rollback.
- Full core LLVM suite491passed/0failed/2ignored. LCOV line coverage using the
  existing rustTestRanges exclusions unchanged: content_v2.rs328/355=92.39%;
  content_v2/schema.rs79/83=95.18%; dungeon_composition.rs128/128=100%.
- All-target core/ingest/storage Clippy with -Dwarnings, fmt and diff checks pass.

The initial large core test caught that shared unique() still enforced1024;
the scoped helper now separates its explicit limit from duplicate checking.
The initial full LLVM suite failed the inherited source-admission test's exact
v2 expectation after the normally merged original v3 regeneration. This fixture
now requires actual currentv3; final v4 regeneration must update it exactlyv4.
After that correction the full suite passed. An initial external LCOV helper
lookup used a main-checkout script absent there; the final calculation used this
worktree's unchanged helper with a temporary dependency symlink, now removed.

No corpus regeneration, network, signing or push occurred. This code branch's
normally merged historical/generated inputs are not a final v4 pack or proof.
After independent code review and integration of cache/consumer prerequisites,
the separate desktop rehearsal writer must regenerate once with a reviewed
immutable code pin, then rebind exact admission/closure/inventory/functional proof
and source parity. Actual full-corpus association counts, hosted Windows, installed
resources/performance, manual smoke and publication remain separate gates.
