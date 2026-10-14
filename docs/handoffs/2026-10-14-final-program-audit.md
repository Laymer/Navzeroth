# Final recurring PR/release program audit

Owner/sole writer Codex /root. Claimebe474b precedes records. Actual latest release:
[v0.8.1](https://github.com/Laymer/Navzeroth/releases/tag/v0.8.1),
published2026-10-14T18:02:38Z, draft=false, prerelease=true, release413023625.
Annotated tag peels d11853e179120fa3570c6283bf999263561c1b01.
This records exhaustion of dependency-ready engineering under the approved program,
with explicit external/deferred work. It does not claim full product, focused-release,
real gameplay, source-license or interactive native acceptance.

## Exact release and asset evidence

Release PR111 head0f4189a8618a2e8bc002c106357e2023feb29286 passed all nine
latest jobs (CI38607643318/package38607643395). PR artifact synthetic source
fcb8f38f3fd5e7a2d2443bfd5e5a397e65ffb090 has the same full tree as that head.
Protected main d11853e has the identical full tree and passed all nine jobs
(CI38609020673/package38609020595). Tag38610614205 passed both package and
Draft GitHub release jobs at the exact tagged source. The tag-only Draft job is
excluded from PR/main nine; superseded/cancelled jobs are never passes.

Downloaded PR/main/tag assets each pass exact five-file set, strict four-payload
checksums, executable MZ, ZIP integrity, all20 addon/source-license bytes, notes/
version and raw40ASCII-plus-LF source metadata. Authenticated draft listing verifies
all five sizes/digests; actual public-tag metadata verifies the same five digests
after publication. Tag assets are the published assets:

| File | SHA256 |
| --- | --- |
|AzerothNavigator-0.8.1-x64-setup.exe|7e0350622f99c7babf8046b8842a6de98a813ed0d1a73fb379c2a5dcdee61587|
|AzerothNavigator-addon-0.8.1.zip|cec390088947f9d1d1f8cca89adb9547e5710227f18002c92ad6592ea7139154|
|CANDIDATE_NOTES.md|d53743179da7a6fb3b47d5c5ec5e821586061637a10aaa45a5db086076d46200|
|SOURCE_COMMIT.txt|25497a122916128347078b12e6730e0a147fb2f392f46446c2372d855a64976b|
|SHA256SUMS.txt|5b134472f57974487cde316ce4817e2684781c060d2c2efa7f33e0e3bddf5e85|

Setup5,258,043 bytes/addon49,615 bytes. Evidence JSON/logs:
/tmp/navzeroth-v081-{pr,main,tag}-assets-verified.json, -publication.json,
-final-publication-proof.json, -draft-metadata.json and -tag-run.log.
Original clean main fast-forwarded db8530f8→d11853e only after actual publication.
No worktrees, branches, user changes or published history were removed. The earlier
parser-workdir local-main mistake was corrected with preservation-safe reset--keep
while its coordination branch remained intact; no remote main push/rewrite occurred.

Root reopened the [official addon policy](https://eu.forums.blizzard.com/en/wow/t/wow-user-interface-add-on-development-policy/1642)
before0.8.1 publication. Source stays public/unobfuscated/free; addon changes from0.8
are version-only, without advertisements/donation prompts, automation or bridge.
This is a scoped source inspection, not legal certification or actual-client acceptance.

## Actual published-source completion audit

Every non-doc byte of actual tagv0.8.1 equals versioned tested9192552. Actual822
portable Rust/195desktop/40 mocked Chromium, strict Clippy/fmt/type/lint/build,
73 schema/TypeScript and73CLI contract/33transport fixtures pass. Exported exchange
schema and generated TS have no drift. Four exact unfiltered LFS pointers pass;
all8 existing PNGs preserve bytes/hashes/dimensions through actual isolated0.8.1
regeneration. All8 historical Forever Git blobs match0.8; typed sources.json is the
sole content addition. Its unknown/conditional policies remain conservative.

Actual real-directory local build+verify pass:1 dataset/0fixtures/6files,
20,278,245 bytes/3gaps/distribution_eligible=false. Manifest remains local_reference,
generator0.8.1 and exact wow_forever/1.60.1.70205/16001/reference/fixturefalse/
forever-atlas-r1. Raw dataset SHA531860e5189146b17bfee5b2654dfbef58c4acb922d692f0f16c73a9726789db
is unchanged. The permission-incomplete pack stays private and is not a release asset.
Scope/failure/source evidence is in the0.8.1 release handoff and
[candidate engineering requirement map](2026-10-14-candidate-engineering-audit.md).
Interrupted local checks and the corrected atlas-helper field error are recorded
there; incomplete/failed invocations were not counted as passes.

After publication, root inspected the actual tag, production module inventory,
source TODO/todo!/unimplemented! markers, every M0–M12 task row and their prerequisite
conditions against the plan. The79 unique ledger IDs exactly match the79 numbered
section17 tasks:55 IMPLEMENTED_UNVERIFIED,16 BLOCKED,8 DEFERRED_WITH_REASON.
No current READY/TODO/IN_PROGRESS task row remains. IMPLEMENTED_UNVERIFIED is not
an acceptance promotion: partial real-model/catalog/live/native scope is explicit.
The two actual pipeline omissions found after0.8 are now resolved; broader optional
optimizers do not replace missing source/client prerequisites. No further finite
independent engineering omission was found. The final docs add only records;
product/configuration/fixtures/content remain byte-identical to the published tag.

## Explicit unblock requirements

| Remaining scope | Required new evidence/input |
| --- | --- |
|M0.4–0.6/M2.6 and game-facing M4/M5/M7/M8/M9|Sanitized recorded70338 plus installed addon revision/conditions for actual equipment/item/quest/position/events, export/SavedVariables/plan progression/session/restriction/HUD/taint/focus/reload behavior; complete real planned-versus-actual/duo sessions|
|M3.3/M3.4/M5 real support/M6/M7.6|Permitted build-specific playable quest/service/travel/acquisition records, four documented real role models/weights, verified Deadmines records and actual geometry/calibration/usage rights; current catalog remains0quests/items/acquisitions/nodes/edges/objectives/rules|
|M9/M10.2 and desktop acceptance|Actual supported Windows interactive focus/hit-testing/chord/DPI/multiple-monitor/borderless/fallback/installation/replacement/uninstall acceptance and ordinary-hardware workload measurements; hosted smoke is separate evidence|
|M7.4|Verified permitted party-message API, limits/format/expiry/throttling and opt-in conditions; no socket bridge substitutes|
|M8.5|Real permitted combat-log format and intended-use evidence|
|M11.1–11.5|Official Forever endpoint/namespace/auth/retention/terms and an authorized real response; unverified support is not proof of absence, and no namespace/credential/account workaround is invented|
|M12.1–12.4/M12.6/M12.7|Focused-release prerequisites plus permitted regional/class/role/level/model/coverage units and actual expanded-size acceptance|
|M12.5|Hosting remains deferred/local-first; authenticated distribution requires an actual justification and permitted source policies|

The complete task-by-task ledger is docs/IMPLEMENTATION_TASKS.md. Re-enter the
parallel PR/release loop when new prerequisite evidence or a concrete ready bug
arrives. This final publication audit does not justify another release containing
only unchanged blockers. Root completes the program after this records PR passes
fresh exact-head checks and is reconciled; no required engineering is omitted to
make a smaller success claim.
