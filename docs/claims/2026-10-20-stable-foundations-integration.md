# Stable foundations integration

Owner and sole integration writer: Codex `/root`.
Baseline: `3cb74e6a5bf7e0697c63b538599fcee87035fe8a` (main after PR212).
Branch/worktree: `release/stable-1.0.0-integration`, `/tmp/navzeroth-stable-integration`.

The requested initial five PRs are merged. Assemble the separately reviewed
follow-up PRs219–225 and228 on this isolated branch, preserving their individual
reviews/claims and published history. Refresh each normally, review conflict and
interaction changes, and run affected checks. This branch does not bypass main:
a separate integration PR must pass all ten protected-main checks and actual
Linux/Windows source parity on its exact final candidate before entering main.
The final version/release PR remains separate.

This foundation batch implements verifier/acceptance/defaults/objectives/roles/
exclusion/receipt primitives and the weekly update repair. It does not yet ship
the final content, issue receipts, establish hosted/owner acceptance, tag or
publish. Run one bounded real weekly workflow only after its main integration;
keep generated updates as reviewed draft PRs, deferred from this release.
