# Solo planning

Party size 1, workflow `joint_planning`.

Source: navzeroth-authored-v1 (original authoring, task V2.4). This is a static
scenario specification for an operator. It is not a record of an observed run, and
following it does not establish acceptance: a pass needs recorded, hashed evidence
from the frozen release build (docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md).

Unknown, unsupported, restricted, stale, false and zero are recorded as they are.
A step whose input is unknown is recorded as unknown, never as passed.

For a party of one, the planning workflow is the single-character planner; the
joint planner is not used and refuses fewer than two participants.

1. Import one fresh capture of a Human character in levels 1 to 20.
2. Declare the character's role. Select the role model whose class and role match
   (for example `model:mage-ranged-damage-1-20` for a mage). Note that the model
   applies by declared role only; check the class yourself.
3. Plan a session with an explicit origin, stopping point and budget.
4. Record which objectives were offered and why; record each unknown input.

Expected: a plan for one character whose explanations name the role model and its
weight source. Expected gaps: class proficiency and slot layout are unknown in the
authored models, so gear comparisons report those inputs as unknown.
