# v0.3.0 consolidated live checklist

Run only on the previously recorded Forever client 1.60.1.70338 / interface 16001.
Record actual GetBuildInfo tuple, installed candidate source SHA, OS, locale,
combat state and transition for each result. Keep personal exports private.
A missing environment check stays UNVERIFIED; synthetic replay and hosted Windows
packaging establish separate evidence, never actual game API behavior.

- [ ] Install candidate addon; load without Lua errors; check diagnostics build gate.
- [ ] Compare UTC clock probe to trusted UTC; explicitly confirm; capture/import;
      inspect trusted age, then reload and confirm age becomes uncertain until
      a new explicit comparison/confirmation and capture. Prior user-reported
      trusted/reload results are limited evidence, not a blanket capability grant.
- [ ] Explicitly capture quests with /anavdiag snapshot <anonymous-profile-id>.
      Inspect readable accepted/completion flags (including false); verify headers
      are excluded and unavailable history stays unknown. Missing log entries must
      not establish turn-in. Confirm partial/failing reads preserve prior export.
- [ ] Capture readable equipment item IDs; inspect absent links and unknown
      variant/instance/modifier observations. Confirm missing links are not empty
      slots. Confirm position diagnostics do not invent map/floor/instance identity.
- [ ] Export a compatible desktop plan; /anavplan paste and explicit Import;
      inspect next action, expiry, age, dependency-check availability and revalidation.
      Try invalid/oversized replacement; previous valid plan must remain loaded.
      Show/hide/clear work. A fresh snapshot replacement requests revalidation.
      Missing objective completion mappings never advance a step. Reload clears plan.
- [ ] Explicitly start/sample/pause/resume/stop recorder; inspect visible state and
      reason. No background sampling occurs. Export to session_export, then use
      desktop Sessions text/selected-file import under matching profile/context.
      Inspect unclassified coverage, paused gaps, level observations and clock
      uncertainty separately. Duplicate import is idempotent; conflicting reuse
      refuses without changing history. Explicit deletion removes selected history.
- [ ] Reload never resumes the previous recording timeline; prior successful export
      survives. Start creates a new runtime identity. Monotonic reset and bound failures stop
      recording visibly. UTC drift warns and marks UTC uncertain while monotonic
      recording continues. Failed export leaves previous export intact.
- [ ] Check experimental UI for focus/drag behavior and combat restrictions/taint;
      record unsupported/restricted observations without bypassing restrictions.
- [ ] Native Windows Gear/Sessions/Planner interaction and accessibility: record
      actual installer version/source, successful import and navigation separately
      from command-mock browser flows and hosted packaging smoke tests.

No real content authoring, validated stage mapping, joint planning, party messaging,
calibration or full live release acceptance is supplied by this candidate.
