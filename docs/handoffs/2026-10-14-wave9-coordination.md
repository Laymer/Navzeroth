# Wave9: published0.8 audit and two local pack repairs

Owner/sole coordinator writer: Codex /root. Claim7703805. Published baseline
v0.8.0, db8530f8d5b8633292a038632e76494168310ccd,
published2026-10-14T16:48:12Z, draft=false, prerelease=true.
User authorizes isolated parallel PRs, granular pushes, protected-main merges and
recurring verified releases. Routine feature/release work requires no new approval.

## Publication and exact assets

Release PR107 checked8a20754d11a05c508b30ccae39267e80d8ba58ab passed all nine
latest jobs (CI38597137485/package38597137502). PR synthetic package source
fb012c8adb3e8d07e88cdd051b3fa2cf9b5a5ef6 has the same tree as that head.
Main db8530f8 passed all nine jobs (CI38598770149/package38598770145),
and its full tree equals the checked release head. Tag package38599893558 passed
both package and Draft GitHub release jobs at the exact annotated-tag source.
Tag and main source are identical. PR/main/tag assets each pass exact five-file,
strict four-payload checksum, executable MZ, ZIP integrity, all20 addon/license
source bytes, release notes/version and raw40-ASCII-plus-LF source checks.
Authenticated draft metadata confirmed sizes and GitHub digests for allfive assets.
The direct draft-tag REST lookup returned404; authenticated release listing found
release412954624 and all asset digests matched. That failed lookup is not a pass.
Publication was verified through gh release view; original clean main then
fast-forwarded035f28e→db8530f without deleting worktrees or rewriting history.

| Published file | SHA256 |
| --- | --- |
|AzerothNavigator-0.8.0-x64-setup.exe|1deeab755f441df682c25590d97bc3470778f733f4dcc198e64dca204c7b9fca|
|AzerothNavigator-addon-0.8.0.zip|6b5c70e32bf4d6b193ce1d0e1ba4a493b7089791c4ee41416000b614ee6221b2|
|CANDIDATE_NOTES.md|d10012af22f45d49a971763c48466a93a41ede5ab0f62399b7256c53edf96f25|
|SOURCE_COMMIT.txt|6d5ce89ba3efa60386b4c5641003cab94a337557642c0db2085f4c209a0eda59|
|SHA256SUMS.txt|7879f4440ddc34caa8388d928a47f1fafef2fe01b3450082891060545ddfabab|

[Published release](https://github.com/Laymer/Navzeroth/releases/tag/v0.8.0).
Setup5,250,558 bytes, addon49,615 bytes. Actual verification JSONs and run logs
are /tmp/navzeroth-v080-{pr,main,tag}-assets-verified.json,
-v080-publication.json and -v080-{pr,main}-asset-check.log.
Versioned code5b7 matches final release/main except docs. Actual816 portable Rust,
195 desktop/40 mocked Chromium, strict Clippy/fmt/type/lint/build and content/schema
checks are scoped in the release handoff. Pointer checks and materialized PNG-byte
checks remain distinct. Native/live/source/model acceptance remains unverified.

## Released-source audit and sole writers

Full-plan audit against the now published source confirms all four wave8 UI scopes
implemented. It also reproduces two finite local pipeline failures in existing data:
strict canonical comparison first rejects typed payload.map_embeddings=[] because
serialization omits its valid empty default; a temporary comparison copy omitting
only that default then reaches absent content/forever/sources.json. No source was
changed by diagnosis. Logs /tmp/navzeroth-wave8-{source-gap,missing-registry}-repro.log.
Unknown/incomplete/conditional rights do not prevent local pack preparation under
the existing policy; a passing policy regression requires distribution_eligible=false.
Three materialized referenced PNGs match declared hashes. These are engineering
repairs, independent of missing real playable records and source permissions.

| Task | Sole writer | Isolated worktree / branch | Owned scope |
| --- | --- | --- | --- |
|19.3 strict typed empty default|/root/backlog_audit|/tmp/navzeroth-wave9-pack-default; fix/content-pack-empty-default|Dataset comparison and focused parser/pack regressions only|
|8.3 existing source registry|/root/release_review|/tmp/navzeroth-wave9-source-registry; fix/forever-source-registry|Typed sidecar, focused source-policy/citation tests and relevant source docs|

/root/atlas_dense performs independent source-locked peer reviews after freezes;
root owns coordination/release integration and final combined verification. Each
writer reads STATUS/latest handoff/relevant plan, claims owner+baseline BEFORE code,
uses granular commits/pushes and a separate draft PR. No concurrent task writers;
private targets/worktrees, no browser lease needed by these repairs.

Parser scope recognizes only exact schema1 payload.map_embeddings typed [] in a
comparison-only serialized shape after validated decoding. Preserve original raw
input bytes, full context/ID/revision/hash and strict duplicate/unknown refusal.
Do not accept arbitrary empty unknown fields/null, loosen generic source/manifest
parsing, or alter immutable Forever data to erase the field. Tests cover actual
reference parsing, omission/explicit[] equivalence, malformed/duplicate/future/unknown
fields and prior-output preservation. Canonical pack output follows existing rules.

Registry scope uses existing SourceRecord/SourceRegistry schema1 and four IDs from
SOURCES.md (three cited by the dataset). Copy only recorded metadata. No exact UTC
retrieval time is invented from date-only text; verification revisions and underlying
licenses remain Unknown. The owner's composite assertion never grants independent
third-party permission. Preserve Unknown/Conditional/NotApplicable distinctly and
keep distribution_eligible=false. No edition/build/fixture/revision or PNG change.
Focused tests parse all rows, ensure citation registration and preserve policy states.

## Next release and external ledger

These repair existing intended local pack behavior, so reconcile them into a0.8.1
patch preview after source peer reviews/allnine latest exact-head gates. Verify the
combined actual real-directory local pack build/verify without distributing the
permission-incomplete pack. Cut the next release only with actual checks and exact
PR/main/tag assets, then re-audit the published source. No ready exhaustion yet.

Remaining external prerequisites: consolidated recorded70338 client field/event/
restriction/export/plan/session/UI/taint evidence; actual native focus/hit-testing/
DPI/monitors/display/installation/performance acceptance; permitted playable records,
four real gear models, verified Deadmines content and complete comparable sessions;
verified party messaging/combat-log formats and official Forever endpoint/auth/terms/
authorized response. M12 expansion stays focused-release/evidence gated. Authenticated
hosting remains deferred/local-first. No workaround, fabricated data/license or
unchanged blocker-only release is authorized by this scheduling decision.
