# Solo advancement

Party size 1, workflow `advancement`.

Source: navzeroth-authored-v1 (original authoring, task V2.4). This is a static
scenario specification for an operator. It is not a record of an observed run, and
following it does not establish acceptance: a pass needs recorded, hashed evidence
from the frozen release build (docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md).

Unknown, unsupported, restricted, stale, false and zero are recorded as they are.
A step whose input is unknown is recorded as unknown, never as passed.

1. Load a solo plan into the addon and advance through at least one step in game.
2. Gain a level during the run and export a new capture.
3. Re-import and record the plan's advancement state and the reminders shown.

Expected: the plan advances only on observed completion. `reminder:train-after-level-up`
fires once after the level change and is not repeated for an unchanged capture.
At level 20, `reminder:slice-level-boundary` states that the guided range ends.
