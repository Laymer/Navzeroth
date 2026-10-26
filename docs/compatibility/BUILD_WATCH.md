# WoW Forever build watch

Current policy: [ADR 0008](../decisions/0008-unsigned-stable-release.md) and
[ADR 0009](../decisions/0009-automated-draft-release-acceptance.md) supersede the
historical ADR 0006 post-launch target freeze. Stable 1.0.0 targets
**1.60.1.70450 / interface 16001**. Detection never admits a build or changes that
target. Exactly **1.60.1.70245** is admitted as a historical Forever input, with
its true origin and warning; it is not target-confirmed. The workflow
[`build-watch.yml`](../../.github/workflows/build-watch.yml)
runs daily at 06:23 UTC and on demand (Actions → Build watch → Run workflow).

## What it does

- Reads Blizzard's public, unauthenticated version list
  (`https://us.version.battle.net/v2/products/<product>/versions` and `/summary`),
  the same list the Battle.net launcher reads. No credentials, no downloads, no
  client data.
- Reports the builds of the watched products in the job summary:
  - `wow_classic_beta`: the historically inspected Forever beta (1.60.1.x).
    On 2026-10-17 it reported 1.60.1.70450, matching the installation's own
    `.build.info` build key.
  - `wowf`: lists 1.60.1.x builds (1.60.1.70245 on 2026-10-17) and is the likely
    Forever live product. **Unconfirmed.** Its originally observed 1.60.1.70245
    was later admitted as a historical input; see [the observation and policy
    history](#wowf-160170245-historical-observation-and-later-admission).
- Opens one issue labelled `build-watch` per product build not seen before, and per
  new `wow*` product code (any code not in `KNOWN_WOW_PRODUCTS` in
  [`scripts/build-watch/lib.mjs`](../../scripts/build-watch/lib.mjs)). The title is
  the identity: a later run never reopens a title that already exists, open or
  closed, so closing an issue acknowledges it.

The first run opens one issue per current build (the baseline). Close them once
read.

Dedup key: the exact title plus the `build-watch` label. Don't edit those, or the
next run opens the issue again. More than five changes in one run become a single
aggregate issue instead of one each, and a failed issue lookup stops the run
instead of risking duplicates. Every value placed in an issue is validated first
(region codes, digits-only builds, `x.y.z.build` versions, hex build configs,
`wow[a-z0-9_]*` product codes); anything else fails the run.

It never admits a build, edits the repository or affects any release gate.

## When an issue opens

1. Record the public notification as an observation, not client verification.
   Closing the issue acknowledges it; preserve its exact product/version/build
   config and regions.
2. If a compatible installation is available, `local-data inspect --installation
   <folder> --product <product>` provides read-only inspection (see
   [LOCAL_DATA.md](../data/LOCAL_DATA.md)). Missing local inspection stays unverified.
3. New development inputs or a proposed target change require an explicit decision,
   source admission, exact-build layout/extraction/projection evidence and the
   affected content/release verification. Never restamp an earlier build.
4. A new product code may be the live Forever product: confirm its identity before
   updating `KNOWN_WOW_PRODUCTS` or `WATCHED`. Neither a product name nor a version
   list proves playability or compatibility.

The old “first live build after launch” freeze is superseded by ADR 0007/0008.
A later build notification does not block releasing the explicitly scoped 70450
content and does not establish support for the later build.

## `wowf` 1.60.1.70245: historical observation and later admission

Issue #183 (baseline run, 2026-10-17) reported `wowf` at 1.60.1.70245: build 70245,
build config `e8c4ef4e67bd1d94d820254984258fa8`, regions eu, kr, tw, us. Recorded
on 2026-10-18; closed by this record.

- **Product identity unconfirmed.** That `wowf` is the Forever live product is
  inferred only from its 1.60.1 versions.
- **Likely a pre-launch placeholder or staging build (inference).** 70245 is lower
  than 70374 (the former target) and the development inputs 70425 and 70450
  ([ADR 0006](../decisions/0006-v1-program.md)). No source we have
  says what the build is.
- **No local inspection.** No client running `wowf` was available, so `local-data
  inspect` was not run against it; the build is unverified.
- **Later admission, not a new observation.** ADR 0008 records the owner's explicit
  acceptance of exact 1.60.1.70245 as historical, unverified source input, preserving
  its true build and UI warning. This does not verify the target, the local client,
  live gameplay or the product identity. The earlier issue's unverified results
  remain unverified. Current stable content targets 70450.

## Future build compatibility: 1.60.1.70532

[Issue #270](https://github.com/Laymer/Navzeroth/issues/270), opened 2026-10-22,
reported public `wowf` version **1.60.1.70532**, build config
`2582289d81e380163c7a13472df4cb81`, regions eu, kr, tw and us. This records the
notification's contents, not a fresh client inspection or latest-service claim.
The owner authorized acknowledging and closing this informational issue. A closed
notification is not completed compatibility work.

Backlog: independently establish product/build identity; inspect an available
installation through the permitted read-only route; decide whether to admit the
build as an input or deliberately retarget; review exact-build source permissions,
layouts, extraction/projection, generated content and tests; regenerate affected
identity/receipt/acceptance evidence before claiming support. No admission or
70532 compatibility is established here. Keep 1.0.0 on 70450 unless a subsequent
explicit decision changes that scope. The historical 70245 exception does not grant
permission to admit unknown or future Forever builds.

## Acknowledged builds

Builds whose issue was read and followed up. Acknowledging a build records it; it
never changes the explicit 1.0.0 target (ADR 0008/0009).

| Product | Version | Build config | Regions | Seen | Issue | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `wow_classic_beta` | 1.60.1.70450 | `84bea4233331e390fda8724c6f91ffa5` | eu, kr, tw, us | 2026-10-17 | #182 | Originally development input only; later selected as the explicit 70450 content target by ADR 0007/0008, without live verification. Matches the owner's installation build key. Admitted for local data in [V2.2b](../handoffs/2026-10-17-v2.2b.md) (extraction in [V2.2c](../handoffs/2026-10-17-v2.2c.md), [V2.2d](../handoffs/2026-10-17-v2.2d.md), [re-extraction](../handoffs/2026-10-17-70450-extraction.md)); admitted in the addon as `development_unverified` with owner-reported interface 16001 ([handoff](../handoffs/2026-10-18-issue-182.md)). Live addon behaviour on 70450 is UNVERIFIED. |
| `wowf` | 1.60.1.70245 | `e8c4ef4e67bd1d94d820254984258fa8` | eu, kr, tw, us | 2026-10-17 | #183 | Originally observed without admission; later explicitly admitted as historical/unverified input by ADR 0008. Product identity remains unconfirmed. See [above](#wowf-160170245-historical-observation-and-later-admission). |

## Limits

- The list says what Blizzard's patch service serves, not what the servers accept
  or what's playable.
- Region `us` is the endpoint host; the rows cover every region the product lists
  (eu, kr, tw, us on 2026-10-17). Disagreeing regions are reported, not resolved.
- `wowf` being the Forever live product is an inference from its 1.60.1 versions.
- GitHub pauses scheduled workflows after 60 days without repository activity;
  re-enable it under Actions if that happens.
- An empty version table for a watched product fails the run (a retired product
  should be noticed, not silently reported as unchanged).
