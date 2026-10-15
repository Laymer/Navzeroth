# Independent pack, release material and node provenance review

Reviewer /root/acceptance, read-only code review. Exact revisions e6816f1 (PR132)
and 0a68234 (NodeFacts follow-up), reviewed in detached isolated worktrees before
this documentation-only branch. No implementation files changed by reviewer.

No new important findings in shared verifier extraction, canonical v2 headers,
exact bounded asset/supporting closure, target/fixture checks, actual per-record
provenance mapping, release material selection or candidate/publication separation.
The known empty node provenance gap is closed by 0a68234: each base node binds its
own NodeFacts evidence; production requires complete node declarations, every
required empty source set blocks, and coverage source unions always compare.
Offline source-admission registry enforcement is a separately assigned next review;
this review does not establish permission for a real corpus.

Actual isolated-target checks:
- e6816f1: 11 v2 pack, 14 release readiness and 9 default material tests passed.
- Both normally ignored material tests passed explicitly with PowerShell7 and
  Git LFS available: actual CLI material selection/ZIP transport and hydrated-byte
  LFS validation. These Linux runs are not Windows-native acceptance.
- All14 synthetic candidate transport checks passed, including default verifier
  refusal, marker removal/tampering and both tag environment refusal paths.
- 0a68234: 10 core v2, 4 record provenance and 10 CLI library tests passed.
  The fixture-regeneration test remains intentionally ignored.

An initial attempt to reuse another worktree's target directory returned stale
artifacts from another revision and a missing-node_facts compiler error. Those
results were discarded. Every Rust result above was rerun with the exact detached
checkout's own target directory; the stale artifact is not a product finding.

No full licensed 70374 corpus, native Windows, actual game observations or gated
1.0.0 candidate was claimed. Await exact source-admission gate commit for focused
follow-up review; coordinator retains protected-main integration ownership.
