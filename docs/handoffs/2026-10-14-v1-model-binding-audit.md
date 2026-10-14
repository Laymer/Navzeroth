# V1 model, dungeon and party binding audit handoff

Owner / sole writer: Codex `/root/public_sources`.
Baseline: `9c02f2be42f1a2110e2851e91f81031443afdafd`.
Worktree / branch: `/tmp/navzeroth-v1-model-audit` /
`docs/v1-model-binding-audit`. Claim `6d8291b` precedes the audit.

## Findings and proposed next work

The [audit](../data/V1_MODEL_BINDING_AUDIT.md) traces actual production types,
validators, app consumers, pack/release bindings and representative synthetic
fixtures. It preserves all three missing-binding blockers.

- Class/role arithmetic, slot layout and proficiency validators are reusable, but
  current role profiles lack immutable context/scope/revision/evidence bindings.
  The app currently consumes explicit manual assumptions. A versioned model
  envelope and its strict loading/selection/binding are dependency-ready after
  coordinator contract freeze; real scoped models still need documented or
  permitted inputs or explicit recorded user preferences and validation.
- Dungeon preparation already has context/revision/fixture declarations, bounded
  evidence-bearing sections and exact content reference checks. It is local app
  request data, with private validation mixed into runtime calculation and explicit
  unverified/consumable gaps. Pure declaration/reference-validator extraction and
  durable record/completeness binding can proceed without invented dungeon facts.
- Parties are runtime state and workflow support; storage/planner capacity is not
  proof of Forever party mechanics. Recommended coordinator decision: separate
  typed support/acceptance binding from packed game records, with actual content
  dependencies and real scenario evidence. If retaining a gameplay-record domain,
  first define the concrete sourced party facts; opaque scenario/report/profile
  IDs must not become records. This decision requires versioned release contract
  compatibility and must not waive mandatory local party workflows.

No schema is chosen or changed. New sidecars require explicit pack, storage and
consumer support; existing strict pack validation cannot be bypassed. Source
population, exact-build completeness, permissions and real acceptance remain
blocked independently of reusable validation code.

## Verification scope

Read shared instructions, STATUS, current integration/enforcement handoffs and
implementation-plan sections 9–11 and 20. Inspected the linked production types
and their call paths; checked exact release binding behavior and regression names.
Parsed the current inventory: nine unknown mandatory domains, zero accepted
sources. Parsed the real reference dataset: build 1.60.1.70205, six evidence
records, one map space/image and zero gameplay records in the quest/item/
acquisition/node/edge/objective/rule collections.

Parsed the three gear case fixtures, all explicitly synthetic with
`synthetic_test` weights. The preparation example is fixture/demo-r1/
UNVERIFIED_DEMO; both solo and duo replay examples identify a synthetic format and
have fixture snapshots/content. These checks did not execute Lua, fetch sources,
validate gameplay or grant distribution rights. An initial search used a nonexistent
planner directory glob; it was corrected to the actual `joint_planner.rs` module
before citing its bounds and fairness behavior.

Only new audit/claim/handoff documentation is changed. Actual local checks pass:
23 relative file links resolve and 24 symbol assertions resolve across eleven
production files; results are in `/tmp/navzeroth-v1-model-binding-audit-checks.json`.
Whitespace checks pass. No unrelated test suite is represented as verification
of this read-only audit. Independent review is requested before
coordinator integration. No README, code, schema, inventory, source registration
or acceptance status is edited.

The previous CLI task is independently complete at PR122 head
`f69f8b6d69e1d9be0b245149bde10a83e0b2e125`: all nine required checks pass on
CI `38636360082` and package `38636360007`; the tag-only draft job is skipped.
That result verifies the CLI task, not the missing model bindings discussed here.

Next: coordinator freezes a concrete model/dungeon contract task and decides how
party support relates to content and acceptance. Implement only that bounded
engineering scope with synthetic compatibility/reference tests. Keep source
population and full-content release blocked until real prerequisites are met.
