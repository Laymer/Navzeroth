# Offline release admission binding

Owner: source_admission worker. Baseline 0a68234 after node provenance, branch `feat/release-admission-gate`, worktree `/tmp/navzeroth-release-admission-gate`. Root transfers core release evidence and CLI release/material proof binding plus tests. No other writer on these files.

Require a typed pinned registry for accepted sources, matching reviewed rights, origin, pack metadata and source material file/field allowlists. Verify local proof hashes; never access network. Preserve preview/no-accepted-source paths. Registry remains private evidence, excluded from public materials. Do not change production fingerprint exclusions or compare transformed catalog bytes with original source files.
