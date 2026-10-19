# Bounded weekly world pipeline verification

The weekly workflow uses the reviewed normalization policy's exact Wago origin
build (currently1.60.1.70245), while its target remains the separately recorded
world-data target. It does not select a new admitted origin/build automatically.
An absent required table holds the run instead of acquiring an unadmitted fallback.
Current source grants, field sets, original bytes and corresponding-code pins remain
mandatory. A changed grant or origin requires independent admission review.

`--weekly-verified` is opt-in; generic generation and the full fact registration,
source-sync and source-build contracts remain intact. Retention and admitted pack
validation precede offline generic source verification, so a clean LFS-less checkout
has genuine regenerated originals before source-sync reads their closure. One exact
12069-byte prior run-manifest metadata object is selected for LFS hydration (current
size; hard bound8MiB), verified against canonical Git pointer oid/size. No raw,
media or wildcard LFS pull occurs. This is an existing quota use, not an expansion.

Only prior selected-generator shards matching declared exact bytes/hash or their
canonical LFS pointers can be removed. Unlisted files remain, collisions refuse,
and generator exclusions are logged. Existing excluded-generator shards hold the
run instead of disappearing. Newly acquired content-addressed originals/facts may
replace only exact canonical LFS pointers with independently hashed matching bytes;
wrong/linked/changed existing objects refuse replacement. New upstream originals
receive exact LFS attribute paths outside the generated fact block. No ancestor-race
sandbox is claimed for this owned checkout.

After one network-backed pass, a cache-only pass repeats generation, verification,
retention, full registration/build and readiness. Exact published material bytes
are compared, including metadata, notices, source closures, originals and facts.
The full unpublished source-knowledge catalog and verified world pack also compare.
Published closure is bounded4GiB/16384files, unpublished products10GiB (including
the existing8GiB generic source catalog contract); finite scans and streaming hashes
avoid loading these catalogs for comparison. Readiness reports and nonmaterial
logs alone are excluded. No output normalization hides differences. Raw stage and
separate offline logs retain actual timings. Only exact equality permits one draft
publish attempt; readiness failures remain disclosed blockers, never publication.

## Exactly one verification run after merge

Do this only after #212 and this repaired workflow are merged and registered on
`main`, with repository Actions contents/PR permissions already enabled and recorded.
Do not run a second local acquisition or issue signing material. If a scheduled or
manual world run is already queued/running at that exact source, use that run's
results; do not dispatch a duplicate or race the Tuesday schedule.

```sh
gh workflow view world-data.yml --ref main
gh run list --workflow world-data.yml --branch main --limit 10 \
  --json databaseId,event,headSha,status,conclusion,url
# Record the independently checked merged main SHA and UTC dispatch start first.
gh workflow run world-data.yml --ref main
gh run list --workflow world-data.yml --event workflow_dispatch --branch main \
  --limit 5 --json databaseId,headSha,createdAt,status,conclusion,url
# Select the one new run matching the recorded SHA/time; no blanket retries.
timeout 130m gh run watch "$WORLD_RUN_ID" --exit-status
gh run download "$WORLD_RUN_ID" --name "world-data-logs-$WORLD_RUN_ID" \
  --dir "/tmp/navzeroth-world-run-$WORLD_RUN_ID"
```

Record run URL/source SHA/conclusion, elapsed stage timings, pipeline and offline
logs, weekly-reproducibility.json digest and complete file identities. Verify its
one acquisition/one offline pass and exact material/product equality. Network403,
rate limit, missing table, timeout, hash/origin/grant mismatch, missing logs or failed
replay are failed/unverified results and hold completion. No purchase/quota change
or repeated acquisition is authorized by this procedure.

If output changed, inspect the single `data/world-weekly` draft: exact head/source
parents, generated file/LFS pointer closure, notices, registry/plan/code pins and
honest readiness report. A repeated output must skip a new commit; branch updates
preserve the previous tip as a merge parent and never force-push. Generated updates
remain deferred; never merge them as part of the stable release batch.

GITHUB_TOKEN-created commits/PRs normally do not trigger other workflows. Explicitly
dispatch the existing checks on the exact draft branch after recording its head:

```sh
for workflow in ci.yml source-knowledge.yml coverage.yml package-windows.yml; do
  gh workflow run "$workflow" --ref data/world-weekly
done
```

Record each run's exact head/checks and any actual failures. CI dispatch includes
release stability; source-knowledge supplies both platforms and parity. Native
coverage/packaging may correctly hold on missing final signed bundle/source evidence
for a deferred generated update; do not bypass those gates or sign automatically.
Do not dispatch downloads verification or publication for a generated data PR.
