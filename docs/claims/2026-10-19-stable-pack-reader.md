# STABLE-PACK-READER

Owner/sole writer: Codex `/root/tooling_review`.
Baseline `5d5fef9`; branch `fix/stable-pack-descriptor-reads`, isolated worktree
`/tmp/navzeroth-stable-pack-reader`. Harden shared v1/v2 pack file reads against
links, special files and ordinary changes around descriptor-bound bounded reads.
No ancestor-race sandbox promise. Preserve contracts and importer semantics.
