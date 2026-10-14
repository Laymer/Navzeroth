# Local opt-in submissions

Owner: Codex /root/atlas_dense (sole writer). Baseline: published v0.6.0,
739a82229a5b7369cab2a6d83c4b2a935bd0ba2e. Branch feat/local-submissions.
User authorizes isolated parallel feature PRs and granular pushes.

Scope: bounded local review queue, explicit opt-in, closed minimal quest-state,
price and acquisition-outcome declarations, exact immutable dataset/source/evidence
links, CAS decisions, payload-erasing withdrawal and terminal revocation, explicit
permission-gated local JSON export. Local acceptance does not activate content,
upgrade evidence confidence, authenticate consent or upload anything.

Owned: new core/storage/app submissions modules and tests; feature-specific
additive lib/native/SDK/demo registration; ContentScreen child panel and tests.
Root owns migration9/recovery registration; content governance worker owns
content.rs and disposition enforcement. No competing edits to those modules.

Frozen API: schema1 closed Reported values preserve unknown, unsupported,
restricted, stale, false and zero. Canonical document SHA256 is the opaque row ID
and duplicate fingerprint. Safe-integer CAS revisions, 16KiB documents,
1024 total rows including erased tombstones, 32-row pages, 32KiB local export.
Export requires the accepted exact guard and active exact base; recorded source
permission must be Permitted with license/attribution, edition, exact revision,
allowed payload token and local_observation_export.v1 transformation token.
Initial/final consistent reads surround compilation; final handoff preserves
source policy/content hash/CAS. No new exchange format or transport.

Checks: pending implementation. Native/live environment acceptance remains
unverified. Next: normal merge root foundation, implement/test bounded storage,
app service and explicit local UI; record actual checks and limitations here.
