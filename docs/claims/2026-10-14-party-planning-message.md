# Party planning availability correction

Owner/sole writer: Codex /root. Baseline `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
Worktree `/tmp/navzeroth-v1-party-copy`, branch `fix/party-planning-availability`.
The acceptance audit found an unconditional "Session planning is not available yet"
message immediately below the implemented joint planner. Correct that stale copy
in `PartyScreen.tsx`; preserve actual planner eligibility/readiness gates. Read
STATUS, agreed recipient-export handoff and plan sections 9.4–9.5 before editing.
Validation: existing Party screen/planning tests and whitespace. No new test is
needed for this reversible text change. Record actual result in a handoff.
