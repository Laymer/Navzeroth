# V110.9a — explicit 1.1.0 website asset validation

Owner/sole writer Codex `/root`; baseline `acb19140`; branch
`feat/v1.1-guided-progression`. Claim: `docs/claims/2026-11-10-v110-site-version.md`.

Validator explicitly permits 1.1.0 and requires all eight stable assets plus a
matching artifact policy. Malformed/unlisted versions remain refused even with a
matching policy. Website data, artifact policy and publication receipts are unchanged.

Actual checks on Node v24.19.0: focused test file passed; experimental Node coverage
reported 100% lines/functions and 90.91% branches for release-metadata.mjs.
`git diff --check` passed. Read-only independent reviewer
`/root/review_v110_contract` reported no blockers and reran tests/whitespace checks.
These are synthetic validator checks, not actual asset or release verification.

Remaining V110.9 work: Windows acceptance, evidence import, inventory generation,
bundle tooling and all four feature journeys. No 1.1.0 readiness claim.
Next independent task: V110.4a progression observation completeness and bindings.
