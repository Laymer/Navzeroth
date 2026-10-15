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
The resulting main commits through `c7e0b7b` also passed all those checks.
Resulting `83f38c8` checks are pending at this record:

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

PR151 final head `9cf62df` passed CI38786821303, package38786821251 and
sources38786821032. Its actual package source is
`809ba1f8215e30e7b25b3706514d0ea8292c0650`; that entire tree equals merged main
`83f38c8`. Downloaded the five-file preview package and checked four SHA256
entries, exact source metadata/notes, ZIP integrity, all 21 addon Git blobs,
licence inclusion and installer PE framing. Addon ZIP SHA256:
`1e581afba695eebee3fe27a64d6b3184088d762ac0ec0511486c18640d0116b5`.
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
operator-performed turn-in, stop/export/reload. Actual callback result is pending.

## Remaining release requirements

The exact 1.0.0 readiness command refuses the intentionally unchanged 0.8.1 version.
A separate 0.8.1 development audit reports candidate_ready=false/stable_ready=false:
all nine independently evidenced inventories, accepted complete gameplay packs and
25 final acceptance observations remain missing. The preview publication flag is
not stable readiness. The actual documentation/main comparison preserves both production and inventory
fingerprints. Production SHA256:
`51bdce28c68867edf66f8deda827198b529712159ece926b43e9be365a50d6ad`.
No fingerprint, evidence or packaging gate was bypassed.

Next: inspect the pending turn-in export and operator result, verify resulting main
`83f38c8`, and finish this documentation PR through normal checks. Continue complete
licensed target content and supported advancement/deviation work only with actual
inputs. Freeze 1.0.0 only after production/content/static readiness, then complete
all required candidate-bound live/Windows scenarios and full-corpus performance.
