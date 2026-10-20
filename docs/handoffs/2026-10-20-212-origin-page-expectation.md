# Dungeon provenance page integration fixture

Owner/sole writer Codex `/root`; baselineee36ae02, claimd09d6303.
The consumer's member paging performs separate provenance requests. Hosted
1623cb90/ee36ae02 runs failed an old OriginPlacement assertion that the first
request contains all4records, while actual badge assertions passed. These failed
jobs remain failures, not coverage/native/e2e passes.

Testae9cd77a retains all rendered dungeon/quest/reward/travel badges and verifies
every request's subject/revision, nonempty queries, and the exact deduplicated
record-key union. Batching/order and duplicate member lookups are permitted;
missing/extra/wrong-scope records still fail. No production behavior changed.
Root full66files/546UI tests passed, all per-file TS/TSX linecoverage>=90%
(95.88%aggregate), typecheck/lint/diff passed. Independent tooling reviewed exact
ae9cd77a and reran12OriginPlacement/Dungeons tests:passed, no blocker.
Logs:/tmp/navzeroth-212-origin-{full-coverage,typecheck,lint}.log.

Merge this fixture into212's feature branch, then require fresh all-ten protected
checks and source parity on exact resultinghead. No Windows install/native/full
functional/manual acceptance was inferred from this fixture. Final content proof
branch must incorporate it before its immutable code pin/regeneration.
