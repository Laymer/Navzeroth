# Party planner availability message

Owner/sole writer Codex /root; baseline `bb3782c`, claim `43932bb` before code.
The Party screen rendered an unconditional "Session planning is not available
yet" directly below the implemented `PartyPlanning` controls. It now explains
that a second character is needed only when the selected party has one member.
Existing planner eligibility, content, freshness and participant gates are intact.

Actual validation: existing PartyScreen, PartyPlanning and PartyAgreement suites
pass (3 files, 21 tests). Offline frozen-lockfile install reuses 216 cached
packages; no lock changes. Whitespace passes. This is a display-copy correction,
not a new planner capability or live/native acceptance result.

Next: independent source review, all nine current hosted gates, protected-main
integration, then continue source/live acceptance work under the v1 plan.
