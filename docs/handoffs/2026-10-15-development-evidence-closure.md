# 70374 development evidence integration

Owner and sole writer: Codex `/root`; documentation baseline main
`83f38c808cf6581e77d5beb5dd329131074111c5`, claim `f3f5801`, branch
`docs/70374-development-evidence`. No production changes. Existing acceptance,
inventory and source workers performed read-only reviews; no new agents.

## Protected integration and artifacts

PR148 (capture/native tooling) merged as `8f24d89`; PR149 (separate completion
getter) merged as `840939f`; PR150 (scoped MIT Forever declarations) merged as
`c7e0b7b`; PR151 (opt-in session milestones) merged as `83f38c8`. Each passed all
nine hosted checks and three source jobs before merge, after independent review.
All four resulting main commits also passed all nine checks and three source jobs:

| Commit | CI | Windows package | Independent sources |
| --- | --- | --- | --- |
| `8f24d89` | 38780761014 | 38780761042 | 38780761277 |
| `840939f` | 38782754429 | 38782754554 | 38782754632 |
| `c7e0b7b` | 38785820604 | 38785820923 | 38785820632 |
| `83f38c8` | 38788666532 | 38788666326 | 38788666141 |

Downloaded both source artifacts from merged `c7e0b7b`. Verified all 1,079 Git
blobs, nine original/licence objects and input/generator/catalog/closure hashes
against the committed receipt. Both source archive SHA256:
`4cba99db2f1cdf6f347024bb4db79c8bf1681333c05db3a4a4eae53a25626716`.
The 1,685 facts remain reference knowledge, with zero verified target-usable facts
and nine unknown independent inventories. No complete coverage is inferred.

PR151 final head `9cf62df` passed CI 38786821303, package 38786821251 and
sources 38786821032. Its actual package source is
`809ba1f8215e30e7b25b3706514d0ea8292c0650`; that entire tree equals merged main
`83f38c8`. Downloaded the five-file preview package and checked four SHA256
entries, exact source metadata/notes, ZIP integrity, all 21 addon Git blobs,
licence inclusion and installer PE framing. Addon ZIP SHA256:
`1e581afba695eebee3fe27a64d6b3184088d762ac0ec0511486c18640d0116b5`.
The resulting-main package (run 38788666326) downloaded and passed the same
source/hash/21-addon-file checks, with metadata naming actual main `83f38c8`.
Its addon ZIP SHA256:
`64af8bc37a29ebcc0b3ae21fb8b3e971c3d4e02e5f0cc1f0941ddf982575b01a`.
Uncompressed addon files match the tested package exactly. Both main `83f38c8`
source artifacts passed all 1,081 Git-blob/nine-material checks. Source archive
SHA256: `e6697e5909c2330682ec36b1b5083f5b92a02a437601b8f90c757318577c4747`.
No release was published. Versions remain 0.8.1.

Local scope: 262 CLI/Lua tests passed, zero failed, three existing ignored; final
18 affected tests passed after counter/formatting changes. Strict all-target
CLI/Lua Clippy, formatting and whitespace passed. Initial event-overflow test
assumed the event cap preceded the JSON limit; corrected its test-only cap without
weakening production bounds. Hosted lint found overlong lines, corrected before
final passing CI. Final independent delta review found no important issues.

## Actual development observations

These are source-identified development results, not frozen 1.0.0 acceptance.
No aggregate release row is promoted. Raw SavedVariables and backups stay private.

The existing evidence log records the operator-confirmed build 1.60.1 / 70374 /
interface 16001, level-30 capture, seven-event manual recording, trusted clock
capture and uncertainty after reload. Current-log completion flags were initially
all unknown, which identified the separate getter binding failure.

With the operator-confirmed game closed and no running WowB process, installed
all 21 files from checked `840939f`, backed up prior addon/project SavedVariables
privately and verified SavedVariables unchanged. Operator captured/reloaded without
errors on a different, visibly level-13 character. Strict parsing reports 14 quest
records, all 14 completion flags readable, five true and nine false. The operator
confirmed those exact visible counts. Clock remains uncertain; equipment has 19
slots, ten known; objectives remain absent. Complete current-log quests are not
inferred to be turned in. Keep distinct anonymous profiles for distinct characters.

The operator reported EventUtils functions present and IsEventValid true for both
QUEST_TURNED_IN and QUEST_LOG_UPDATE. Only turn-in is used. After PR151 checks and
artifact verification, the operator closed WoW again. Installed all 21 files from
the downloaded, source-verified `809ba1f` package with a new private backup and
unchanged SavedVariables. Main follow-up checks were still running. The requested
manual test uses `guided-char2`: snapshot, explicit start-quests, exactly one
operator-performed turn-in, stop/export/reload. The operator confirmed exactly one
quest turned in, export/reload complete and
no errors. Actual strict inspection validates fixture=false, target 70374 tuple,
five events, exactly one quest-turn-in and max relative elapsed 41,604 ms. This
verifies the narrow normal-event callback/export flow on the identified package.

A temporary local verifier (outside Git) used the existing inert parser/typed
contracts and SnapshotApplication service on an isolated in-memory database to
import this actual snapshot and session together. It succeeded, with 0 ms observed
activity, 41,604 ms unclassified and 0 ms gaps. No personal database changed, no raw
identifiers were printed, and no native UI/Windows acceptance is inferred. The
operator subsequently ran status after reload and confirmed `session idle`;
the opt-in recording did not automatically resume. Actual paused-event exclusion,
restrictions, full session analysis and final candidate scenarios remain unverified.

## Source follow-up

The read-only source reviewer recovered the exact mixed-source repository:
`TylerAkins/wow-database` at `a8e22674a7148cd0e58049b577e387093edbb852`.
The coordinator reopened its pinned primary documents:
[attribution](https://raw.githubusercontent.com/TylerAkins/wow-database/a8e22674a7148cd0e58049b577e387093edbb852/ATTRIBUTION.md)
and [source register](https://raw.githubusercontent.com/TylerAkins/wow-database/a8e22674a7148cd0e58049b577e387093edbb852/docs/SOURCES.md).
They distinguish GPL-3.0 repository code from third-party game/website data,
which the repository does not relicense. The documented QuestieDB permission is
a maintainer-specific account, explicitly not a new upstream data licence. No
additional grant for numeric quest/item/coordinate subsets was established by
this review, so no combined outputs were admitted. Existing directly reviewed MIT
ATT declarations remain admitted. This is a source-admission disposition, not a
claim that every individual underlying fact requires permission. No publisher
contact, imported Lua execution or client-file extraction was performed.

## Remaining release requirements

The exact 1.0.0 readiness command refuses the intentionally unchanged 0.8.1 version.
A separate 0.8.1 development audit reports candidate_ready=false/stable_ready=false:
all nine independently evidenced inventories, accepted complete gameplay packs and
25 final acceptance observations remain missing. The preview publication flag is
not stable readiness. The actual documentation/main comparison preserves both production and inventory
fingerprints. Production SHA256:
`51bdce28c68867edf66f8deda827198b529712159ece926b43e9be365a50d6ad`.
No fingerprint, evidence or packaging gate was bypassed.

Next: finish this documentation PR through normal checks. Resulting main
`83f38c8` and the development turn-in/import/reload checks are verified. Continue complete
licensed target content and supported advancement/deviation work only with actual
inputs. Freeze 1.0.0 only after production/content/static readiness, then complete
all required candidate-bound live/Windows scenarios and full-corpus performance.
