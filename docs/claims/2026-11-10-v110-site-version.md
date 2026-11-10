# V110.9a — website release metadata version validation

Owner and sole writer: Codex `/root`; baseline `acb19140`.
Branch/worktree: `feat/v1.1-guided-progression`, `/tmp/navzeroth-v1.1-guided-progression`.
Scope: scripts/site/release-metadata.mjs and its tests. Explicitly permit 1.1.0
while requiring the full stable asset set and independently supplied artifact
policy. Do not create a publication receipt or change website release data.
Checks: Node metadata tests, negative malformed/unlisted-version and missing-policy
cases, whitespace, independent read-only review.
Other V110.9 tooling and actual publication remain separate pending work.
