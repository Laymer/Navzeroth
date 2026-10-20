# ADR 0007: World-data automation and the generated-content gate

Accepted 2026-10-19. Baseline: `origin/main` after PR #208 (V2.7). Recorded by task
V2.8 W0. The owner gave these decisions on 2026-10-19 while planning how the
generated world data (ADR 0006 §14, [WORLD_DATA.md](../data/WORLD_DATA.md)) becomes
content the application uses. They supersede parts of ADR 0006, listed under
Consequences. Plan: tasks V2.8a to V2.8f in [TODO.md](../../TODO.md).

## Decisions

### 1. One pipeline, run locally and weekly

`python3 data-generators/pipeline.py` runs every stage: detect the client build,
generate, verify, register facts, build content, evaluate the gate, publish. It ends
with a feature branch and a draft pull request; the owner merges. A weekly GitHub
workflow runs the stages that work without Wowhead (CMaNGOS and wago.tools) and opens
or updates a draft pull request only when the generated output changed.

### 2. Whole world

Content covers every race, faction, zone, quest, NPC, travel point, dungeon and loot
table the data provides, not only the 1.0.0 slice.

### 3. Accepted origins

1.12 (CMaNGOS) and Classic Era (Wowhead, TALOD hand lists) data is used as Forever
content. Records keep their true origin edition and build in their evidence and are
never relabelled. `release/v1/accepted-origins.json` names the accepted origins
(`vanilla_1_12`, `wow_classic_era`) for every domain; the release gate accepts
evidence from those origins instead of refusing it. The application marks such
facts ("from 1.12 data", "from Classic Era data"). AGENTS.md's rule against mixing
editions silently still holds: the mixing is explicit, recorded and visible.

### 4. Precedence

When sources disagree: the owner's addon observations, then Forever client tables,
then authored content and the owner's guides, then 1.12 and Classic Era data.
Every conflict is listed in a report beside the generated content.

### 5. Gate rules

Kept: every counted fact traces to hashed evidence and signed receipts; unknown,
unsupported, restricted, stale, false and zero stay distinct and an unknown is
never written as zero.

Dropped: the 15 `LIVE-*` acceptance checks and the post-launch target-build freeze.
The `WIN-*` rows are unchanged.

Unknowns are accepted: unknown, unsupported and restricted fields in generated
content are reported as accepted unknowns, not coverage gaps.

### 6. Target build

Generated packs carry the owner's installed client build, detected from its
`.build.info` (known install paths, then the git-ignored `data-generators/local.json`)
and recorded in `release/v1/world-data-target.json`, which the weekly workflow
reuses. A client patch produces new packs for the new build; old packs are not
relabelled.

### 7. Limits

The research catalog registers every generated fact; its fact, byte and closure
limits are raised to fit the measured data with headroom. Retained source objects
are stored through Git LFS and the source-knowledge workflow checks LFS out. The
content-pack identity and byte limits are raised so the whole world fits in one
pack. Exact values are recorded in [WORLD_DATA.md](../data/WORLD_DATA.md) by the
tasks that change them.

### 8. Inferred travel

Walking edges join service and quest nodes within one zone by straight-line
distance at run speed. Flight durations are path length divided by flight speed.
Both are marked `inferred`.

### 9. Application

An origin marker shows where a fact came from, and a world browser lists NPCs,
vendors, trainers, quests and loot from the generated pack.

### 10. Execution

PR #208 is merged once its checks pass. The work then proceeds on `main` with
worktree-isolated parallel agents and an independent review, as in V2.7.

## Consequences

- **1.0.0 can ship without live-client evidence and without a frozen launch build.**
  Release notes must say so.
- Content may contain facts that are wrong for Forever. They stay visible through
  the origin marker and the conflict report; Forever-confirmed facts replace them
  under the precedence of §4.
- Larger limits raise desktop memory use and load time; the tasks measure and record
  them.
- Git LFS storage and bandwidth grow with retained facts and weekly runs.
- Superseded: ADR 0006 §1 (target freeze) and the `LIVE-*` part of §8, and the
  statements in ADR 0006 §14 and WORLD_DATA.md that 1.12 and Classic Era data stays
  unverified until validated against a frozen target.
