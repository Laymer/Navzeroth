# Browser demo application version

Owner/sole writer Codex `/root`, baseline `9d046b7`, branch
`fix/browser-demo-version`. [Claim](../claims/2026-10-18-browser-demo-version.md).
Coverage preparation found the browser status still returned hard-coded 0.6.0.
It now uses the existing package metadata import, matching the reference view
and future version bumps. Demo mode and unverified capabilities remain explicit.

Actual checks: all 414 desktop unit tests, typecheck and lint passed locally.
The regression compares application status with both package metadata and the
reference view and checks that demo/unverified capability labels are preserved.
Independent read-only review and exact-head CI/source/packaging precede merge.
Next: refresh the separate coverage branch after this fix reaches main.
No release, live evidence or acceptance promotion was performed.

First refreshed Windows CI exposed the independent lifecycle journal race; that
failed head was not merged. Separate PR #202 now merged as `3288fa0` after full
exact-head checks and review. This branch is refreshed onto it without changing
the version correction. Rerun exact-head CI/source/package and final read-only
review before merging. Prior 414-test local result is historical, not a claim
about the new head's full suite.
