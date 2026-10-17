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
    Forever live product. **Unconfirmed.**
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
