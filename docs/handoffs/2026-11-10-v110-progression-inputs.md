# V110.4e — desktop progression inputs

Owner/sole writer Codex `/root`; baseline `b656db00`; isolated PR #297 branch.
Claim: `docs/claims/2026-11-10-v110-progression-inputs.md`.

## Implemented

Imports now opens a progression panel explicitly for its selected stored snapshot.
Users can paste a companion export or enter money in copper and available talent
points. Blank fields and all other manual collections remain unknown; zero is
preserved. Copper stays decimal text, including values above JavaScript's safe
integer range. The application assigns provenance and capture metadata.

Every mutation sends the loaded exact predecessor binding. Client, profile or
snapshot changes remount input state; mismatched reads/receipts refuse. Parent
snapshot transitions disable controls. Read generations discard superseded
responses; closing ignores outstanding completions. Refreshes every minute use
backend capture freshness and retain drafts. Source, fixture, unknown/restricted/
unsupported, partial/complete enumeration and capture age are visible. Captures
remain separate rather than being implicitly merged or selected as newest.

## Actual checks

- 571 desktop unit tests in 68 files passed; full desktop per-file 90% line gate
  passed (aggregate 96.01%; all three new source files 100%). Report retained at
  `/tmp/navzeroth-v110-inputs-ui-coverage`, log
  `/tmp/navzeroth-v110-inputs-ui-coverage.log`.
- Typecheck, lint and whitespace passed. Initial typecheck caught a wrong source
  field/token in new code/tests and lint caught a test non-null assertion; repaired.
- One actual Chromium command-mock journey passed: invalid/valid companion import,
  manual zero/unknown capture, origin display and retained mock records after reload.
  Build was exercised by Playwright; existing large-chunk warning remains.
- Fourteen focused input tests cover limits, errors, identity changes, late reads/
  writes, refresh, completeness and provenance. Existing six import tests passed.
- Independent read-only app_progression_review found no blocker; suggested explicit
  capture-age wording because fields can be older. Wording updated. Reviewer ran
  whitespace check but did not rerun tests.

## Limits and next task

The browser test uses command mocks; native SQLite persistence is covered by the
earlier application/storage checks, not inferred from browser reload. No native
Windows, addon capture or live-game validation is claimed. Recommendations still
require field-level freshness. Manual spell/talent/profession/inventory/enhancement
editors require the typed gameplay catalog and are not supplied by this panel.

Next: versioned gameplay facts and deterministic guidance services, saved plan
dependencies and the required Progression/Gear workflows. Exact70862 interface
evidence, content refresh, release tooling and all1.1.0 acceptance remain pending.
No main merge, tag or publication.
