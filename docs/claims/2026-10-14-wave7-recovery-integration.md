# Claim: typed local-state recovery and registered-source compatibility

Owner: Codex /root, sole shared integration writer. Baseline protected main
1da934a (schema9 foundation), published739a822/v0.6.0.
Branch feat/local-state-recovery; /tmp/navzeroth-wave7-recovery-integration.
Read STATUS, wave7 coordination/foundation handoffs and plan8.1/8.6/19.3.
Scope: new migration10 widening submission source IDs to existing256-byte core
contract without rewriting schema9; bounded complete migration-history checks;
registration of frozen typed feature validators; populated backup round trips;
recovered plans explicitly historical rather than authenticated guarded successes.
Preserve source/immutable data, payloads/counters/tombstones/full contexts; unknown
or corrupt data fails closed. Feature modules remain their workers' sole ownership.
Implement schema prerequisite first, then normally merge frozen dependencies and
wire validators/derived provenance before release. User authorizes granular commits,
separate PRs/protected integration and verified releases. No automatic optimizer.
