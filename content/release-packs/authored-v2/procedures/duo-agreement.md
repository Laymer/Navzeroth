# Duo agreement

Party size 2, workflow `agreement`.

Source: navzeroth-authored-v1 (original authoring, task V2.4). This is a static
scenario specification for an operator. It is not a record of an observed run, and
following it does not establish acceptance: a pass needs recorded, hashed evidence
from the frozen release build (docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md).

Unknown, unsupported, restricted, stale, false and zero are recorded as they are.
A step whose input is unknown is recorded as unknown, never as passed.

1. Propose the duo estimate from one member.
2. Record each member's local acceptance.
3. Refresh one member's real progress and re-import.

Expected: local acceptance is recorded per member and is not presented as remote
consent. After the refresh the old proposal is invalidated and a new estimate and
agreement are required. The Deadmines checklist item `deadmines:plan-agreed` stays
unknown in the authored catalog.
