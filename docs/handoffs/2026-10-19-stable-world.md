# STABLE-WORLD provenance repair

Owner and sole writer: Codex `/root/world_review`.
Baseline: `5d5fef9b9e2c2fcb7b124966b1c1afed4844127f`, PR #212.
Branch: `fix/stable-world-provenance`; claim
[2026-10-19-stable-world](../claims/2026-10-19-stable-world.md).
Independent baseline review: [review record](../reviews/2026-10-19-stable-world-pr212.md).

## Decisions and implementation

The owner explicitly answered on 2026-10-19: admit the older Forever 1.60.1.70245
datasets with true build attribution and a historical warning. This is an exact
historical-origin grant, not target verification or an unknown/future-build grant.
The stable integration ADR must retain that answer alongside unsigned/Windows
release-policy decisions.

- Added optional `EvidenceRecord.origin` with true edition and observed build.
  Dataset `context` remains the target. Historical catalogs omit the new field on
  serialization; their prior decoding/serialization semantics are preserved.
- Converter v2 retains true origin metadata, downgrades historical/unknown-build
  strong confidence, and refuses multiple builds collapsing into one evidence ID.
- Origin policy schema 2 explicitly lists only Forever 1.60.1.70245 in
  `historical_builds`. Schema 1 remains readable with its previous grants. Unknown,
  unlisted and malformed historical builds are refused. Even admitted historical
  sources claiming strong target evidence are relabel findings.
- Release source validation recognizes the same exact build grant, retaining
  source/admission/material integrity requirements. The evidence policy hash and
  byte count were refreshed; no acceptance result was promoted.
- Revision identity now hashes canonical mapped catalog, source registry,
  conflicts, shard digest, target, policy, authored input and generator version.
  Policy/context/authored/conversion changes cannot silently reuse old revisions.
- Regenerated v1 exchange JSON Schema/TypeScript and v2 reference-content schema.

## Checks and limitations

Python offline suite: 129 passed. Targeted origin/release tests: 15 + 10 passed.
Portable workspace all-target Clippy passed; format and diff whitespace checks
passed. Schema/TypeScript contract validation passed for 73 fixtures (run directly
after pnpm's automatic installer warned about a temporary dependency symlink).

Real-pack conversion ran twice with final policy and reproduced catalog, sources,
summary and conflicts byte-for-byte. Catalog SHA-256:
`77cd262a902a47e4879cd76c3ca5d911cb438a92920f617dfc4eaf3fd13846c8`;
revision `navzeroth-world-1.60.1.70450-7bf08391ae9ab558a42c2a4b`.
Actual catalog classification under the exact historical grant: 119,668 accepted
origins, 273,929 accepted unknowns, zero remaining gaps and zero relabelled evidence.
The standalone Rust harness was compiled against this branch's rebuilt library;
an earlier attempt mistakenly linked the older baseline library and could not
decode the additive field. That harness failure was not a product pass.

Before the final policy grant, the converter-v2 real pack built and verified:
367,733,628 bytes, 393,597 strict coverage gaps, distribution eligible false.
Those commands verify schema/integrity; they do not establish release admission,
candidate binding or redistribution. The final policy changes revision identity;
integrated release-pack regeneration/verification is still required.

An intermediate workspace run failed schema equality while schema artifacts were
being regenerated (only the EvidenceOrigin description differed). Two intermediate
targeted runs also needed synthetic alternate-edition policy fixtures to remove
the new target-edition historical grant. These failures were corrected, not
reclassified as passes. Final portable workspace run passed: 1466 tests, zero failed, 6 ignored.

No Windows, native desktop, live-client, ordinary-PC performance or per-file
coverage result is claimed here. Existing 90% coverage gates remain required.

## Required next work

Integrate STABLE-TOOLING's bounded reader into the standalone world converter,
then independently review that change. This branch deliberately leaves that
hookup to a separate integration commit. Regenerate committed world outputs after
integration. Add visible historical build markers in the desktop.

Source admission, exact source/material selection, retained notices, signed
content receipts and installer inclusion remain mandatory separate work. Current
world source revision/license/admitted-file mismatches remain unresolved. Do not
weaken the gates or count the empty material selection as shipped world coverage.
Independent review and exact-head checks precede integration/publication.
