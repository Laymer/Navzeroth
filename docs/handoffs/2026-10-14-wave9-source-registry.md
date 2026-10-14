# Wave9 — typed existing Forever source registry

Owner: Codex `/root/release_review`, sole task writer. Claim `fa9b830` preceded
implementation. Published baseline `db8530f8d5b8633292a038632e76494168310ccd`
(v0.8.0); coordination `1034f271ebab929e33fd58b7b9b8be6089e82973` normally merged
before claim. Branch `fix/forever-source-registry`, isolated worktree
`/tmp/navzeroth-wave9-source-registry`. Read STATUS/latest release and coordination
handoffs and plan8.3/19.3. Source/test/docs frozen at
`0befc3501934d9e0dcef0dbfefc066b8bef1401e` for independent Atlas review.

## Repair and policy

`content/forever/sources.json` supplies the existing schema-version-1 typed source
sidecar needed by local pack preparation. It registers exactly the four IDs already
documented in `docs/data/SOURCES.md`. The dataset cites three of them; the underlying
Blizzard artwork declaration is also retained. No provider capability is fabricated.

Only existing declarations and the retained footer/owner statement are copied.
Third-party redistribution stays Unknown; the owner's composite-only assertion is
Conditional and does not establish permission for the underlying works. All four
licenses and verified revisions are Unknown. Date-only retrieval/addition text does
not become an invented exact UTC value. The table's `n/a` direct Blizzard retrieval,
allowed fields and transformations stay NotApplicable. Offline embedded-source
access has no authentication/conditional HTTP request; access methods not expressible
from `via the composite only` remain Unknown. The owner's Git LFS file access is
recorded as FileDownload. All records remain SecondaryInference, including the
publisher's artwork and the manual composite, without promotion to actual game
observations or publisher documentation.

The sidecar preserves Unknown/Conditional/NotApplicable distinctions. It supplies
no permission to distribute a pack. Dataset bytes, fixture=false, namespace
reference, historical edition/build/interface70205/16001, forever-atlas-r1 revision,
historical producer and PNG bytes are unchanged. No pack parser, existing pack test,
schema, addon or native code was edited. Source documentation now links and explains
the typed projection.

## Actual checks

At frozen source `0befc35`, private target `/tmp/navzeroth-wave9-source-target`:

- `cargo test --locked -p azeroth-cli --test forever_sources`: 3 passed. The actual
  sidecar parses into SourceRecord/SourceRegistry, all actual dataset citations
  register without changing dataset identity, and unknown/conditional/not-applicable
  policy states remain distinct. Each of the four policies separately participates
  in a valid synthetic local pack build/verify with distribution_eligible=false.
- `cargo test --locked -p azeroth-cli`: all62 tests passed, including those three.
- Strict all-target CLI Clippy, workspace fmt and git diff-check passed.
- Actual CLI `validate-content --input content/forever --json` passed, retaining
  the existing explicit coverage/provenance gaps.

Logs: `/tmp/navzeroth-wave9-source-{tests,cli,clippy,content}.log`.
No browser/native/live checks were needed or claimed for this data-only repair.
No personal exports or new source/network retrievals were used.

## Integration boundary and next task

The independent policy pack cases use the existing explicit synthetic imagery
fixture; they do not edit or sidestep the actual Forever dataset. The parallel
strict-default parser repair remains required before the combined actual-directory
Forever pack build/verify. Root owns that integrated check, protected-main merges,
fresh exact-head hosted gates and the prospective0.8.1 assets/release. This task
does not claim that actual Forever pack preparation already passed independently.

Next: Atlas source-locked review, final main reconciliation and draft PR gates,
then combined local build/verify with distribution_eligible=false. Missing real
licenses/redistribution permission, playable records and native/live evidence
remain external prerequisites rather than newly inferred facts.
