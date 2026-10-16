# 0.9.1 release evidence (copied from /tmp)

Small files the release agent kept only under `/tmp`, preserved here by task G1 on
2026-10-16 and not edited. They are evidence for the
[publication record](../../2026-10-16-v0.9.1-publication.md).

- `readiness.json`: `release-readiness --version 0.9.1` output taken at the
  implementation commit `9896674` (66 blockers, 26 pending, publication allowed).
  Its `candidate_source_commit` is therefore `9896674`, not the released commit.
- `draft-receipt.json`, `tag-receipt.json`, `public-receipt.json`,
  `main-package-receipt.json`: the release agent's checksum receipts for the draft,
  the tag package, the public download and the resulting-main package (source
  commit `92f3f51`, 21 addon files, SHA-256 of four of the five assets).
- `verify.py`: the script that produced the receipts.
- The 1,059-test log (113 KB) and the Clippy log are summarized in the release
  handoff: 1,059 passed, 0 failed, 4 ignored (re-summed from the log by G1).
