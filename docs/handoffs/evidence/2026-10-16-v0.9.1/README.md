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
  commit `92f3f51`, 21 addon files, SHA-256 of four of the five assets;
  `SHA256SUMS.txt` is the fifth file and lists the other four). The draft, tag and
  public receipts agree with the published installer hash `874c80b5...`.
  `main-package-receipt.json` does not: its installer hash is `ecf8e0c2...`
  because it describes the installer built by the separate "Windows package" run
  on the push to `main` (run 38894373306), while the release published the one
  from the tag run (38896174846). Both runs built commit `92f3f51`; the installers
  differ (6,871,442 vs 6,868,991 bytes), so the build is not byte-reproducible.
  See the publication record.
- `verify.py`: the script that produced the receipts. It is not re-runnable from
  the repository: it hard-codes `/tmp/navzeroth-release-v091` (a release worktree
  that no longer exists) and takes that checkout's `HEAD` as the source commit, so
  from another checkout it fails or checks the wrong commit. Treat the receipts as
  recorded output, not as something to regenerate.
- The 1,059-test log (113 KB) and the Clippy log are summarized in the release
  handoff: 1,059 passed, 0 failed, 4 ignored (re-summed from the log by G1).
