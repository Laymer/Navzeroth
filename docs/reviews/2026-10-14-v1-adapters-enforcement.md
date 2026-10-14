# Independent reviews: v1 adapters and enforcement

Integration owner: Codex `/root`; foundation baseline `3cf4f0a`.
Every task has its own isolated claim, branch, published commits and PR. The
coordinator combines those histories normally for fresh protected-main checks.

| Task / reviewed revision | Independent reviewer | Result |
| --- | --- | --- |
| [PR116](https://github.com/Laymer/Navzeroth/pull/116), `0ae8806` | `/root/onboarding` | No blocker/important findings; 21 affected desktop tests pass |
| [PR117](https://github.com/Laymer/Navzeroth/pull/117), production `f1a2af5` | `/root` | No blocker/important findings; bounded copy UI preserves existing exports and explicit focus; 115 Lua harness tests pass |
| [PR118](https://github.com/Laymer/Navzeroth/pull/118), production `1767eeb` | `/root/acceptance` | No blocker/important findings; 42 focused tests independently pass; author complete harness has 119 passes |
| [PR120](https://github.com/Laymer/Navzeroth/pull/120), production `1152b3b`, harness fix `03ef364` | `/root/acceptance`, with `/root` CLI inspection | Upload-directory collision and ignored-addon input findings corrected; 13 release integration plus 9 library checks and 27 PowerShell checks independently pass; success/failure wrapper exit behavior also independently verified |
| [PR121](https://github.com/Laymer/Navzeroth/pull/121), production `ed1a1df` | `/root/acceptance`, with `/root` source inspection | Source identifier-length gap corrected; all 62 ingest tests independently pass; no remaining blocker/important findings |
| [PR122](https://github.com/Laymer/Navzeroth/pull/122), production `e637a902`, unchanged through `4f725fc` | `/root/acceptance`, with `/root` file-boundary inspection | No blocker/important findings; 32 focused CLI/library tests independently pass; all four real pinned files match audited hashes/counts and report only staging metadata |
| Root candidate docs `3ddc2a2` | `/root/acceptance` | Independently verified five-file download, four hashes, source/tree identities and all 21 addon ZIP files; 19 relative links and 25 mandatory row IDs match; no blocker/important findings |

The root docs review's optional Escape clarification is applied: Escape closes
the export window when its edit box has focus. Opening the view deliberately
does not focus it; explicit selection remains user controlled.

The source adapter and CLI never execute imported Lua, apply executable or
symbolic corrections, fabricate gameplay records, or assert source permission.
Raw Item fields are the audited 15, distinct from canonical metadata's 16. Nil
holes, missing fields, false, zero and wide integer masks retain their meaning.
The independent four-table measurements match 4,244 quests, 10,119 NPCs,
14,889 items and 6,645 objects. Eighteen correction providers remain unapplied;
all distribution, target-completeness and playable-dataset flags remain false.
Linux measurements are scoped staging measurements, not Windows acceptance.

The release boundary verifies paths, bytes, source revisions, tracked production
ancestry/identity, actual pack contents and record IDs. It keeps stable output
blocked by missing inventory, real acceptance, rights, three unsupported model
bindings and required asset delivery. Existing 0.x/prerelease allowance retains
honest blockers. The separate materials-delivery task must implement and verify
actual content/source/license output before removing its explicit blocker.

All nine hosted checks passed for PRs116/117/118/120/121. PR122's first package
run inherited the old test harness and failed despite its assertions passing;
normal merge `4f725fc` includes the independently reviewed exit fix. Fresh PR122
and combined integration checks remain required. No live or interactive Windows
acceptance is inferred from any portable, mocked or hosted result.

Final coordinator documentation review at `20b4aa7` by `/root/acceptance` found
no blocker/important findings. It independently checked all 79 plan IDs, 60 local
links in the seven changed documents, the 25 unverified acceptance rows and the
recorded combined result (272 passes, zero failures, one ignored). Its actual
CLI smoke against that clean revision reports preview allowance with
`stable_ready=false`, 68 blockers and zero verified packs. The optional stale
"will provide" wording in the release contract is corrected to the implemented
present tense. The separate materials task remains pending.
