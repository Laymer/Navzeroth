# STABLE-CATALOG-CANONICALIZATION

Owner/sole writer Codex `/root/world_review`; [claim](../claims/2026-10-19-stable-catalog-canonicalization.md).
Baseline `531aa4848c03e3600b62484093c7a054216b6178`, implementation
`9bb2096c113e88cd525653c31df41c47b202701b`; isolated branch
`fix/stable-catalog-canonicalization`, `/tmp/navzeroth-stable-catalog-performance`.
No push or release acceptance promotion.

## Proof and implementation

ReferenceContent::Validate checks format/schema, the validated envelope and all
base/v2 collection capacities, references, contexts, cycles and domain rules.
Numbers in this representation use validated finite/normalized wrappers. Typed
observations and validated records cannot carry discarded wire fields or invalid
observation states. Re-encoding and strict-decoding that already typed content
therefore adds no semantic validation. canonical now validates and clones it.
Source registry validation, source editions, conflict references, sorting,
deduplication and capacity refusals remain.

The discarded compact wrapper encoding and catalog_content_sha256 call were only
capacity checks. The latter built a full Value and pretty byte buffer and computed
a digest that canonical never used. A bounded counting serializer preserves both
bounds without those allocations or hashing: compact wrapper, and pretty content
plus one newline. Sorting object keys changes bytes/hash but not serialized byte
length. Actual public catalog hashing and mandatory stored-document hash, coverage
and provenance-table comparisons are unchanged. No storage hash is trusted as a
substitute for reading and validating its content.

At the stored-JSON boundary, a borrowed RawValue wrapper now retains original
content bytes for strict content_v2::decode exactly once. The wrapper still denies
unknown/duplicate fields, and sources/conflicts retain their typed/registry
validation. This also closes the former nested ItemDefinition unknown-field
stripping before stored-hash comparison. Valid historical bytes remain readable;
v1 code is untouched. serde_json's raw_value feature enables borrowing without a
dependency version or lockfile change.

## Actual checks

- Continuous-mode LLVM storage + app suites:406 passed, zero failed, two existing
  ignored. Three added storage cases verify exact stored bytes/hash and all
  unavailable/false/zero/empty states; invalid typed format/schema/dangling node/
  provenance capacity preserve active content; original stored nested unknown,
  duplicate and unexpected wrapper fields fail closed. A capacity boundary unit
  verifies compact and sorted canonical-pretty counts including escaped Unicode
  and the trailing newline. Initial test fixture errors (wrong NodeFacts shape,
  then indexing an empty synthetic item list) were corrected before passing runs.
- Local raw LLVM content_catalog.rs summary369/400 (92.25%). The LCOV result with
  the repository's rustTestRanges474–501 removed is336/359 (93.59%). Initial
  storage-only coverage88.75% was incomplete; adding meaningful existing app
  integration suites exercised its consumer paths. Thresholds were not changed.
- Storage all-target Clippy with warnings denied, formatting and diff checks pass.
- Actual full world-v2 + authored data: both optimized debug and release harnesses
  staged, activated and reopened the same composed identity as baseline:
  set-57916ee57afb91d6c8e4c6ba285dfe98;27 unique dungeons,4245 quests,8800 nodes,
  one typed map conflict, zero coverage errors. Actual origin classification:
  119668 accepted origins,273929 accepted unknowns,zero gaps/relabelled evidence.
- All three stored revision documents and declared hashes are byte-identical to
  the unoptimized baseline. These include202058089-byte world and202076423-byte
  composed documents, plus the19503-byte authored document.

[Machine-readable measurements and input pins](../data/stable-catalog-storage-experiment-v1.json)
record elapsed times excluding compilation. Debug full-path time fell from
582.23s to421.95s in these single runs. Debug maximum child RSS fell from6699344
KiB to6067368KiB. Optimized release: world staged at24.35s, activation at119.46s,
reopen finished at139.48s; maximum child RSS6060940KiB (about5.78GiB). These are
shared Linux development-host observations with concurrent builds, not Windows,
ordinary-PC, installation or product performance acceptance. High memory use
remains a material limitation; strict core decoding still builds comparison trees.
The temporary sample profile is synthetic and deliberately retargeted to the
actual nonfixture catalog scope; it supplies no live-client acceptance.
Mapper-v3 dungeon entrance estimates were explicitly not tested here.

Logs: /tmp/navzeroth-catalog-performance-{coverage-full,clippy-final,debug-world,
release-world,byte-compatibility}.log. External harness sources and SQLite outputs:
/tmp/navzeroth-integrated-world-harness (baseline),
/tmp/navzeroth-optimized-world-harness (debug),
/tmp/navzeroth-optimized-world-release-harness (release).

## Next

Root independently reviews and integrates this separate repair, then refreshes
protected exact-head checks and source-platform coverage. Hosted Windows must
measure installer installation, full first-start/loading/activation/browsing/
planning and peak application memory. Source/material/receipt gates and the exact
installer owner's manual smoke remain independent requirements. Any relevant
product change invalidates previous installer smoke evidence.
