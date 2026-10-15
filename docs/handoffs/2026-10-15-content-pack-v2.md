# Content pack v2 and verified release bindings

Owner: Codex `/root`, baseline `d59aa91`, claim `addcd22`.
Branch `feat/content-pack-v2`, separate worktree. Core contract PR131 and typed
party PR130 are dependencies normally merged into this branch; source/capacity/
consumer work retains separate ownership and PRs.

`build-content-v2 --input <directory> --output <new-directory>` assembles a single
logical catalog from catalog.json, the source registry and exactly referenced
imagery. Optional party-support.json pins the canonical catalog SHA and lists
explicit public supporting files. `verify-pack` dispatches only version 1 or 2.
V1 bytes, hashes and bounds remain unchanged. V2 bounds encoded catalog at256MiB,
material closure at1GiB; core catalog capacity arrives in the dependent PR5.

The verifier rejects noncanonical/unknown/duplicate/version/path/file/hash errors,
unresolved graphs, conflicting source declarations, matrix target/hash/dependency
mismatches and extra/missing files. Matrices resolve to actual typed catalog records.
Synthetic fixture flags remain visible and block release. Local packs may retain
unknown facts/rights; the release boundary requires zero gaps and permitted rights.
Publication additionally requires one fact record per base quest/item/acquisition,
the nine-domain inventory, recorded source grant evidence and all acceptance.

Release binding now enumerates canonical v2 models/dungeons/equipment facts/quest
facts/acquisition facts and matrix rows. Exact source sets and verified pack files
flow into the existing public material selector; private permission proofs remain
excluded. It never executes Lua/source content. Material budget now charges each
distinct absolute path once across hash/read passes, preventing a valid1GiB closure
from being rejected solely because verification reads it multiple times.

Actual checks:8 independent v2 pack tests,13 release-readiness tests,9 release
material tests (2 environment-specific integration tests left ignored here),
9 core v2 tests, one distinct-budget regression and strict CLI all-target Clippy
pass. Formatting/whitespace pass. Root repeated the independently authored pack
tests. Source worker independently repeated8 pack+9 materials+budget regression
and reviewed target/hash/closure fixes with no remaining important finding.

Initial Clippy found an eight-argument helper, fixed by deriving its pack root.
The budget refactor changed tamper failure ordering; the test now asserts the
actual earlier pinned-size refusal. An initial borrow error was corrected before
successful tests. No failed attempt is counted as a passed check.

Missing: capacity integration, app/desktop consumers, full licensed corpus,
observed mappings, Windows/full-content performance and live acceptance. Hosted
nine checks remain required before merge. This is not a1.0 release or candidate.
Next: integrate capacity wrapper, test >1024 pack/rollback flows, complete consumers,
review each PR and verify protected-main results; retain0.8.1 until external gates pass.

Capacity wrapper integration now passes a deterministic v2 pack roundtrip with
more than1,500 items while legacy v1 still refuses that catalog. The pack engine
has moved into `azeroth-ingest::pack`, with CLI compatibility re-exports, so desktop
imports use exactly the same verifier. `v2::read_verified` returns the parsed
catalog, canonical hash, sources and exact verified asset bytes in one pass.
Canonical gameplay identity enumeration lives in pure core and is shared by
party binding and release validation. Full CLI+ingest tests and strict all-target
Clippy passed after extraction; ten v2 pack tests include importer tamper refusal.
The ingest guard now recursively inspects modules and permits only the inert
`std::process::id()` naming call; process execution remains forbidden. Explicit
manifest/lock allowlists include the bounded PNG decoder already used by the pack.
An initial test comparison required JSON values because ReferenceContent has no
PartialEq; corrected before the successful run. Real content/acceptance remain absent.
