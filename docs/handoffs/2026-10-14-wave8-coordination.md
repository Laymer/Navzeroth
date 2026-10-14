# Wave8: published0.7 audit and final ready UI batch

Owner: Codex /root coordinator. Published baseline v0.7.0,
035f28ece68b0b81680ec42bbe2a792b8a95140d. Published2026-10-14T15:14:38Z,
draft=false, prerelease=true. User authorizes recurring parallel task PRs, protected
main merges, incremented tags and verified assets without another permission step.

## Publication evidence

Release PR101 head95b888de0b3fc62238e606eb5c54bee2e8209dcb passed all nine
latest applicable jobs: CI38584017151, package38584017173. Downloaded PR assets
use synthetic33db06cec05ac01dcaf44f6ea7412e05e56ad7fb; its tree equals the checked
head. Main035f28e passed all nine jobs: CI38585319491, package38585319651.
The commit check-runs endpoint twice returned HTTP500; the exact-source workflow
run/job endpoints then verified each latest job on the main push. Failed lookups
were never counted as test passes. Annotated tagv0.7.0 peels to exact main035f28e;
tag38586577252 passed package and Draft GitHub release jobs.

PR, main and tag downloads passed strict four-checksum verification, the exact
five-file set, MZ header, ZIP integrity, exact19 addon/root-license bytes, notes,
versions and40-character source metadata with LF/no BOM. Tag assets:

| Asset | SHA256 |
| --- | --- |
| AzerothNavigator-0.7.0-x64-setup.exe |03dd963af90e7e7e67cb7b437d9d7df4bd5a477e71e289762807eda90f546b5a|
| AzerothNavigator-addon-0.7.0.zip |50984065092fa0919ac7103ffbbc400dcf21a4d9beb40af1c5921aa377daea86|
| CANDIDATE_NOTES.md |9f5beb4c4c528717756f04bc74e7139ba57a3ec49141bf894ffe701d7c24078a|
| SOURCE_COMMIT.txt |356b2ed8d7ced0295cef92f04c1c381af54e35ed425c04da97817c9e87a03230|

https://github.com/Laymer/Navzeroth/releases/tag/v0.7.0 publishes those four files
plus SHA256SUMS.txt. Original clean main fast-forwarded739a822→035f28e only after
publication. No other worktree/history/user changes removed. Verification JSONs
/tmp/navzeroth-v070-{pr,main,tag}-assets-verified.json retain actual checksums.
Candidate801 Rust/180 desktop/36 mocked browser checks and73/33 contract/transport
fixtures, no drift,0 content errors/3 gaps and unchanged PNG/exact pointer evidence
are recorded with precise source/failure limits in the candidate handoff.

## Released-source audit and sole writers

Actual published0.7 contains all wave7 engineering: Today/Settings and guarded
plan references, durable restrictions/fallback, opted-in local review/tombstones/
permission-gated export, typed exact explanations and populated historical recovery.
No native/live acceptance is promoted. Read-only full-plan audit identified three
additional finite UI requirements; current released code still lacks them.

| Task | Sole writer | Isolated worktree / branch | Frozen scope |
| --- | --- | --- | --- |
|12.2 route selection/fit|/root/atlas_dense|/tmp/navzeroth-wave8-route-fit; feat/route-segment-fit| Accessible leg/hop selection, bounded per-frame schematic fit/pagination from existing guarded exact DTOs |
|12.4 HUD appearance|/root/backlog_audit|/tmp/navzeroth-wave8-hud; feat/hud-appearance| Pure bounded preferences, resize/font/contrast/movement persistence and conservative actual UI-operation gates; permitted Lua bindings only |
|12.5 reference controls|/root/release_review|/tmp/navzeroth-wave8-reference; feat/reference-interaction| Explicit opt-in focused-main shortcut and requested native interaction/pass-through control, preserving actual IPC caller gate |

Each worker reads latest records/plan, claims owner+baseline before code, owns one
task and makes granular coherent pushed commits in a separate draft PR. Root owns
coordination/release and normal reconciliation. No concurrent edits to task files;
additive shared entries stay with the relevant feature and root reconciles them.
Use distinct target directories. Browser1420 requires an explicit single-worker lease.
Final task heads and release/main source require all nine latest applicable jobs.

Route views consume the existing authoritatively recomputed full-context/snapshot/
content/fixture request. Select known leg/hop endpoints only inside the exact same
map/floor/image frame; preserve gaps and incomplete traces. No invented road geometry,
map-position observation or floor connection. Bounded pages retain tail entries;
selection0, zero-cost hops, false and unknown states remain distinct. Invalidate
selection and late replies with original full provenance, including changed content.

HUD code currently has no combat/taint UI gate. Add a conservative injected gate
requiring existing recorded-build restriction, necessary methods and a successful
actual boolean combat query. Known combat or unknown/unavailable/failed query defers
UI mutations; keep one latest request and refresh the plan model before replay.
Width280–960/default420, height160–720/default220, font100/125/150/175/200 and
contrast default/high are explicit bounded choices. Additive settings preserve
existing false/unknown/future-layout data. Capture a local font baseline for
idempotent scaling; do not mutate global fonts. Persist only validated UIParent-
relative anchors/finite bounded offsets actually returned by GetPoint; otherwise
report movement as session-only. No protected-action frames or gameplay automation.

Reference shortcut is explicitly opt-in and focused-main-only Ctrl+Shift+F8;
ignore repeats/composition/editable targets and pending commands. Native verifies
actual main caller/focus; no global OS/game-input interception or set_focus.
Interaction is a separate requested interactive/pass-through choice using supported
native setters. Report dispatch receipts and actual queried visibility/topmost;
no observed interaction mode is invented when native APIs offer no getter. Apply
before show, refuse unsafe focused-reference passive transition, preserve default
absence/hidden/topmostfalse, and keep read-only get_reference_view as the only
reference IPC command. No recreation/provider/watch/import actions on refresh.

## External prerequisites and next step

Real playable sources/models/Deadmines content, consolidated recorded70338 checks,
complete real sessions, actual party message/combat-log capability, official Forever
API support and native Windows focus/taint/DPI/monitor/display/performance evidence
remain unavailable/unverified. Do not turn API presence into verified support or
relabel70205 as70338. World expansion stays gated; hosting remains deferred.

Next: implement these three independent tasks in parallel, source-lock peer reviews,
resolve actual findings, reconcile each exact-head checked PR into protected main
and cut0.8.0 with verified PR/main/tag assets. Re-audit actual published source again;
complete the program only when no dependency-ready engineering remains, retaining
an explicit blocker/deferred ledger without an unchanged-ledger-only release.
