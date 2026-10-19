# ADR 0008: Unsigned stable 1.0.0 and revised acceptance

Accepted 2026-10-19 by the owner's supplied release implementation plan.
Baseline: `826aa21862f7ef15d290c453d5bab3caa7806474`.
Task: STABLE-1.0.0; integration owner Codex `/root`.

## Context and options

The published 1.0.0-rc.1 is an unsigned development prerelease. Its historical
readiness report has 69 blockers; those results remain unchanged. ADR 0007 on
PR #212 accepts historical origins and inferred travel, removes live-client
acceptance and the launch-build freeze, and retains signed content receipts.
The owner now requests stable 1.0.0 under an explicit revised contract. Alternatives
were retaining ADR 0006's signing/live/launch prerequisites, publishing another
prerelease, or adopting the owner's narrower stable acceptance below.

## Decision

Publish **unsigned stable 1.0.0** only after all applicable revised gates pass.
Supersede ADR 0006's stable Authenticode requirement and ADR 0007's statement that
Windows acceptance is unchanged. This does not waive content-receipt signatures.

Adopt ADR 0007's accepted origins, accepted unknown fields, true origin/build
retention, source precedence and marked inferred travel. No live-client evidence
or launch-build freeze is required. Missing records differ from accepted unknown
fields: an unknown field never satisfies an absent promised record. Historical
unverified/failed observations retain their actual status.

Windows acceptance has two separate evidence requirements:

1. Hosted Windows verifies exact installer installation, first start, upgrade from
   1.0.0-rc.1, downgrade refusal, persistence/recovery and installed-resource closure.
   Measure full-pack installation, loading, activation, browsing, planning and peak
   memory; retain workload, runner/hardware and limitations. Hosted evidence may
   satisfy these automated cases; it cannot claim ordinary-PC performance or
   interactive display/accessibility acceptance.
2. The owner manually smokes the exact independently verified draft installer:
   installation or upgrade, first start, bundled-content activation, representative
   planning and restart/persistence. Retain source identity, installer SHA-256,
   environment and actual results. Any relevant product change invalidates it.

Former WIN-* rows remain historical evidence, not automatic passes. Revised,
versioned contracts must distinguish hosted checks, owner smoke and unverified
interactive display/accessibility coverage. Do not reinterpret automated results
as `interactive_windows`. Version affected schemas explicitly and retain historical
schema reading and validation semantics.

Require all ten protected-main checks, independent final-head review, source
platform parity, release stability, genuine enumerated shipped inventory, source
attribution, integrity, signed content receipts and exact artifact/source binding.
Receipt private keys are generated outside Git with restricted permissions;
commit public identities only. Bind receipts to exact content, production identity
and independently recorded review evidence.

Trainer-spell gameplay, talents, professions and enhancements are deferred. Expose
these exclusions in inventory, UI, documentation and release notes. Supported
class/level heuristics, dungeon guidance and estimates need explicit coverage,
derivation and uncertainty. Unsupported claims remain unresolved and block the
corresponding promised coverage.

Bundle world/authored packs, notices, conflict information and required sources.
Default activation includes marked travel estimates on fresh installation and every
application-version upgrade. Persist choices within a version; disclose an upgrade
reset, retain the prior revision for rollback and activate transactionally.

Merge reviewed #214–#217 sequentially, then #212 with repairs. Defer unrelated new
PRs. Complete follow-up work in separate reviewed PRs; create a separate release PR.
Tag only the verified resulting release commit, independently verify draft assets,
then wait for the owner's exact-installer smoke. Once it passes and all applicable
gates pass, publish without another approval request. Verify public downloads before
updating/deploying the website and download documentation.

## Consequences and evidence

Unsigned status and Windows warnings must be disclosed alongside historical
origins, estimates, exclusions, measured resource use and unverified limitations.
Stable does not assert live Forever verification. Signature-status checks remain
mandatory and must require the declared unsigned status for this release.

This ADR records authorization and requirements, not implementation or a passing
release gate. The integration claim and subsequent review/handoff records must
record actual checked revisions, decisions, tests and remaining work. Purchases,
quota expansion and external account creation are not authorized by this decision.
