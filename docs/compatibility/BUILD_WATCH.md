# WoW Forever build watch

Task V4.1 (ADR 0006 §1: freeze the target on the first stable live build after the
4 Nov 2026 launch). The workflow [`build-watch.yml`](../../.github/workflows/build-watch.yml)
runs daily at 06:23 UTC and on demand (Actions → Build watch → Run workflow).

## What it does

- Reads Blizzard's public, unauthenticated version list
  (`https://us.version.battle.net/v2/products/<product>/versions` and `/summary`),
  the same list the Battle.net launcher reads. No credentials, no downloads, no
  client data.
- Reports the builds of the watched products in the job summary:
  - `wow_classic_beta`: the WoW Forever beta (1.60.1.x) the owner's client runs.
    On 2026-10-17 it reported 1.60.1.70450, matching the installation's own
    `.build.info` build key.
  - `wowf`: lists 1.60.1.x builds (1.60.1.70245 on 2026-10-17) and is the likely
    Forever live product. **Unconfirmed.** Its 1.60.1.70245 is observed, not
    admitted; see [`wowf` 1.60.1.70245](#wowf-160170245-observed-not-admitted).
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

1. Check the build in your client: `local-data inspect --installation <folder>
   --product <product>` (read-only; see [LOCAL_DATA.md](../data/LOCAL_DATA.md)).
2. Before launch: a new beta build needs admission as development input, like
   70425 and 70450 (task V2.2b), then a fresh extraction and projection.
3. After launch: the first stable live Forever build is the 1.0.0 target. Record it
   (V4.1) and freeze it in code and manifests (V4.2); never relabel earlier builds.
4. A new product code may be the live Forever product: confirm, then add it to
   `KNOWN_WOW_PRODUCTS`, and to `WATCHED` if it should be tracked.

## `wowf` 1.60.1.70245: observed, not admitted

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
- **Nothing changes.** No admission, extraction, projection, code, manifest or
  release-gate change follows. 70450 stays the development input; the 1.0.0 target
  is still the first stable live build after the 4 Nov 2026 launch.

What would change this: after launch, the owner's live client `.build.info` (or
`local-data inspect --installation <folder> --product wowf`) showing product `wowf`
confirms the identity. The first stable live build it serves then goes through V4.1
(record) and V4.2 (freeze), as in [When an issue opens](#when-an-issue-opens). A new
`wowf` build before launch opens its own issue and is assessed the same way.

## Acknowledged builds

Builds whose issue was read and followed up. Acknowledging a build records it; it
never makes it the 1.0.0 target (ADR 0006 §1).

| Product | Version | Build config | Regions | Seen | Issue | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `wow_classic_beta` | 1.60.1.70450 | `84bea4233331e390fda8724c6f91ffa5` | eu, kr, tw, us | 2026-10-17 | #182 | Development input only, not the target. Matches the owner's installation build key. Admitted for local data in [V2.2b](../handoffs/2026-10-17-v2.2b.md) (extraction in [V2.2c](../handoffs/2026-10-17-v2.2c.md), [V2.2d](../handoffs/2026-10-17-v2.2d.md), [re-extraction](../handoffs/2026-10-17-70450-extraction.md)); admitted in the addon as `development_unverified` with owner-reported interface 16001 ([handoff](../handoffs/2026-10-18-issue-182.md)). Live addon behaviour on 70450 is UNVERIFIED. |
| `wowf` | 1.60.1.70245 | `e8c4ef4e67bd1d94d820254984258fa8` | eu, kr, tw, us | 2026-10-17 | #183 | Observed, not admitted; product identity unconfirmed. See [above](#wowf-160170245-observed-not-admitted). |

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
