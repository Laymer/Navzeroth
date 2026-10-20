# STABLE-WORLD provenance repair

Owner and sole writer: Codex `/root/world_review`.
Baseline: `5d5fef9b9e2c2fcb7b124966b1c1afed4844127f` (PR #212).
Branch: `fix/stable-world-provenance`; worktree `/tmp/navzeroth-stable-world`.

Authorized scope: immutable generated revision identity, explicit true origin
build metadata and release classification, affected tests and contracts, review
and handoff. Coordinate bounded JSON loading with STABLE-TOOLING; no concurrent
writer to `world_to_content.py`. Source admission, material selection and signed
receipts remain separate prerequisites; integrity gates must not be weakened.
