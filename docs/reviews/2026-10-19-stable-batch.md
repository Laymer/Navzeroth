# Stable batch review record

Coordinator: Codex `/root`; baseline main `826aa21862f7ef15d290c453d5bab3caa7806474`.
Three specialist reviewers were explicitly requested by the owner. Their work was
read-only, with existing worktrees preserved. Findings below are independent
review reports, not product acceptance or completed repair claims.

## Final heads reviewed

- desktop_review: #214 `7ee59f543ae53b7282274f38e0cf07fe1caba13b`, no blocker;
  21 targeted desktop tests passed. Exact locale intersection preserves missing
  locales and enGB/enUS identities without replacement.
- tooling_review: #215 `8487485d74b356280ee42d69c9a40e63f68f8f51`, no blocker;
  all 28 coverage-tool tests passed. Descriptor bounds and FIFO refusal checked;
  concurrent ancestor replacement remains outside the documented guarantee.
- desktop_review: #216 `3345e3ac761fe6be1f9ab881ad51418179d4a614`, no blocker;
  all 40 RXP tests passed. Each declared objective index must be present, including
  zero/unknown/max and invalid/repeated index distinctions.
- tooling_review: #217 `688e716482d54afdf302c99bfcd3746b4d6b8591`, no blocker;
  one legacy-version core regression and 13 storage lifecycle tests passed.
  Historical catalog/checkpoint v1 behavior remains; v2 dispositions fail closed.

Refreshed merge candidates still need review and all applicable checks. No merge
is asserted by this review record.

## #212 at `5d5fef9b9e2c2fcb7b124966b1c1afed4844127f`

Blocking tooling findings (tooling_review):

1. `data-generators/navzeroth_datagen/http.py:191`: cached body bytes are not bound
   to recorded URL/hash/size. An offline synthetic cache with metadata for
   `original bytes` returns `altered bytes` while provenance retains the original
   digest. Verify consumed bytes before registering evidence.
2. HTTP response/cache reads, SQL gzip expansion, `register_facts.py` and
   `to_facts.py` input reads lack bounds or check them after allocation. Bound
   regular-file reads, response consumption and decompression before parsing.

World/release review (world_review) identified source build 70245 being recorded as
70450 in five Wago evidence records; target identity must never relabel origin.
Generated registry verified revisions and admission/license/source-file closure
need repair before release material selection can accept the pack. Final detailed
world findings and independent material exercise remain in progress.

Completion gaps confirmed by desktop_review: generated quests contain no planner
objectives; composed record merging uses repeated linear membership checks;
inferred travel settings remain process-local and are changed after activation.
These are required follow-up work, not objections to accepted historical origins,
unknown fields or inferred travel policy.

Actual independent #212 checks: all 125 offline Python tests passed; local LFS
pointer/object fsck passed; content-set/world-browser suites passed 11 tests with
one whole-world test ignored. world_review reports verify-world-data passed for
35 files / 475355 records, zero errors. Passing these checks does not establish
installer shipment, release readiness or bounded network behavior. No Windows or
live acceptance was performed by these reviewers.

## Policy review

Independent desktop_review reviewed integration head `146ee99` against `826aa21`:
ADR 0008 and the task claim match the owner's supplied release plan, no blocking
inconsistency. This approval covers policy documentation, not unimplemented gates.
