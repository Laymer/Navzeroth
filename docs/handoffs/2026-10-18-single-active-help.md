# Single active help bubble

Owner/sole writer Codex `/root`; baseline `8da8301`, branch
`fix/single-active-help`. [Claim](../claims/2026-10-18-single-active-help.md).

PR #198's full Chromium check passed 183/184 tests; Settings busy-fieldset help
reproducibly exposed two visible bubbles when hovering one trigger while keyboard
focus opens another. This is an independent existing shared-component defect,
kept in its own fix PR before desktop integration.

HelpTip now announces activation within its document and dismisses competing
hovered/pinned bubbles without moving focus. Listeners are removed on unmount;
Escape, outside-pointer dismissal and disabled-fieldset keyboard help remain.
No polling, external channel, new dependency or CSP change is introduced.

Actual checks: all 395 desktop unit tests passed on this main-based branch, then
all 21 focused UI tests passed after strengthening actual-focus assertions.
TypeScript check, lint, production build and all six focused browser checks passed
(two busy-fieldset keyboard checks and four production-CSP/font/navigation checks).
Both read-only reviewers found no actionable correctness defect. Policy review's
focus-assertion suggestion was applied: real element focus and post-Escape focus
are asserted. One reviewer independently ran the 395-test suite successfully.

Applicable exact-head hosted checks/merge remain pending. No ordinary-Windows or
live-game acceptance is inferred. Next: merge this independent fix after all checks,
refresh continuation 8, and verify its formerly failing busy-help interaction.
Performance/materials continues separately in continuation 9; coverage/publication
and Pages remain subsequent work. No release was published.
