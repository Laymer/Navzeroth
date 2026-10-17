# Duo fairness

Party size 2, workflow `fairness`.

Source: navzeroth-authored-v1 (original authoring, task V2.4). This is a static
scenario specification for an operator. It is not a record of an observed run, and
following it does not establish acceptance: a pass needs recorded, hashed evidence
from the frozen release build (docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md).

Unknown, unsupported, restricted, stale, false and zero are recorded as they are.
A step whose input is unknown is recorded as unknown, never as passed.

1. With two fresh members, set per-member utilities and calculate the estimate.
2. Inspect how the estimate divides benefit between members.

Expected: each member's utility and its stated source are visible. The role models
contribute model scores only, labelled with their weight source; no member's
benefit is presented as measured.
