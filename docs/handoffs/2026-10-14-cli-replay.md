# Bounded synthetic CLI application-service replay

Owner: Codex /root/backlog_audit, sole writer.
Baseline main8183c2f03bde836c02c69814bfd0701622e16e7a.
Branch feature/wave5-cli-replay; /tmp/navzeroth-wave5-cli-replay;
draft PR[73](https://github.com/Laymer/Navzeroth/pull/73).
Read STATUS, wave5 coordination and plan9.5/18/19.3; format approved by root
before implementation. Main5f3b6c7 (recovery) merged normally asc561151.
Published history and unrelated worktrees preserved. No shared app/native edits.

## Implemented

`replay --fixture <file>` supports strict version1 synthetic fixtures documented
in [PLANNER_REPLAY.md](../data/PLANNER_REPLAY.md). One solo or2..40 distinct joint
characters must comprise the entire exact context/fixture cohort. Participant
indexes explicitly map utility-vector order to derived imported profile IDs;
no character/build/context/source-revision relabelling. Content identity and
expected snapshot IDs are supplied to the existing application's planning guards.

Uses SnapshotApplication::in_memory, ANAV1 typed JSON import, content staging/
activation, local party save and normal solo/joint planning. This reuses actual
eligibility/filtering/travel/fairness logic rather than duplicating it in CLI.
Adds portable app/storage dependencies; lockfile changes only the CLI dependency
entry, not versions or external package resolution.

Declared UTC is mandatory. Public current-snapshot freshness must be trusted/fresh
and at most10seconds old; positive fixtures capture exactly at declared UTC.
The narrow replay window leaves ample margin under current30minute application
policy. No private freshness function was exported or copied into CLI.

Duplicate/unknown JSON fields, future versions, mismatched/duplicate cohorts,
invalid indexes, stale/uncertain captures and malformed options fail with static
safe errors. Contract bounds apply alongside16MiB complete input,128 candidates/
exclusions and4MiB output. Serialization uses a capped writer before stdout and
avoids an intermediate whole-plan JSON Value. Exact selected files/ancestors are
checked for symlink/reparse/traversal/nonregular inputs. No source scraping, Lua
execution, native window, filesystem database or external service was added.

Reports carry explicit synthetic labels and declared context/revision/indexes,
selected/excluded/readiness/alternatives/travel/termination plus application
assumptions/filtering. Source-policy enum/availability/offline fields retain
unknown and false; these are fixture metadata, not production permission claims.
No generated profile IDs, measured hardware timings or optimizer improvement
claims appear. Input unknown/restricted/false/zero do not become recommendations.

## Actual checks

Atc97b8c4, full locked CLI suite40 passed,0 failed,1 existing ignored atlas fixture
generator. Eight replay integration cases pass: deterministic solo/duo services;
explicit reversed actor/utility mapping; whole cohort/context/schema/freshness;
unknown/restricted/false/zero and exclusions; nested unknown/duplicate/byte bounds;
CLI success/complete stdout/static failures; symlink/nonregular/traversal;
40participant success and41 refusal. The bounded writer unit regression confirms
no byte beyond the cap is appended and oversize serialization fails.

Locked CLI all-target Clippy -D warnings, Rust fmt --all --check and diff checks
pass. Both documented cargo replay commands actually ran on the authored fixture
files, selecting one service objective with61000ms estimated time; solo utility10,
duo minimum-participant utility8. Repeated output bytes are identical in tests.
Root-compatible elapsed publication guards are not encoded as measured outcomes.

Initial compile findings were an incorrect envelope-bound constant name and an
attempt to reference private app context; resolved using the existing4MiB
MAX_ENVELOPE_BYTES and public current-snapshot query. No app visibility change.
No test result is inferred from unrelated or obsolete hosted checks.

## Limits and next task

The format is intentionally synthetic-only; ordinary/real personal exports and
near-expiry captures are outside scope. Logical actor/snapshot identity remains
qualified through the application. Static JSON byte caps do not establish general
runtime/memory performance budgets. Existing bounded extensions are contract
metadata and are not interpreted. Hostile concurrent filesystem replacement is
outside portable path-check confinement guarantees. No live WoW/native Windows
acceptance, source permission, beam-search improvement or M4.7 actual session
comparison is established by these fixtures.

Next: exact-head source review/hosted checks, reconcile additive CLI pack/main
interfaces through normal main merge if needed, integrate only through protected
main, then include this command's exact checks in the batch release audit. The
larger program's real content/live acceptance and remaining engineering stay in
the task ledger; this task does not complete those milestones.
