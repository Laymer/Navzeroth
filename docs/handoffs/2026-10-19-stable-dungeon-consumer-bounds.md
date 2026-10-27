# Stable dungeon preparation consumer bounds

Owner: /root/desktop_review, isolated fix/stable-dungeon-consumer-bounds.
Baseline14c091826aef1a6e00da1e2bf5698fdefb015f6e; claim2ddebb9b.

The actual r4 full-pack engineering scenario failed after166.44seconds with
“Preparation section exceeds32records”; no functional Pass artifact was written.
All26 source-supported dungeon lists exceed32 reward references; source quest
maximum91. These are source-area quest and dungeon-associated acquisition
references, not boss-only drops, eligibility or current-game observations.

LocalPreparationPackage schema2 permits4096 quest/reward references. Schema1
retains32; other sections/evidence/notes retain32 and all context/reference checks
remain. The adapter bounds expanded item/acquisition pairs before appending.
Manual requests remain1MiB and complete serialized reports4MiB: oversize reports
refuse without truncation, including large-party evaluation. Desktop24-row pages
preserve totals, every final reference and visible-page provenance lookups.

Investigation also found pre-existing mapper silent slicing at1024. Four lists
lose4011 associations: Stockade517, SunkenTemple1535, ZulFarrak1123,
Scholomance836. No quests are sliced in the actual corpus. This separate source
correctness blocker was sent to root/tooling; this branch does not change mapper
or core source-list contracts or claim functional coverage. Full omitted IDs
were recorded in /tmp/navzeroth-dungeon-full-counts.json for the specialist.

Actual unsliced source counts (all canonical IDs):

| Dungeon ID | Quest refs | Reward refs | Previously omitted rewards |
|---|---:|---:|---:|
| dungeon:shadowfang-keep | 3 | 135 | 0 |
| dungeon:stormwind-stockade | 6 | 1541 | 517 |
| dungeon:the-deadmines | 5 | 577 | 0 |
| dungeon:wailing-caverns | 9 | 78 | 0 |
| dungeon:razorfen-kraul | 0 | 386 | 0 |
| dungeon:blackfathom-deeps | 11 | 648 | 0 |
| dungeon:uldaman | 0 | 125 | 0 |
| dungeon:gnomeregan | 0 | 877 | 0 |
| dungeon:sunken-temple | 0 | 2559 | 1535 |
| dungeon:razorfen-downs | 7 | 55 | 0 |
| dungeon:scarlet-monastery | 7 | 171 | 0 |
| dungeon:zul-farrak | 0 | 2147 | 1123 |
| dungeon:blackrock-spire | 37 | 297 | 0 |
| dungeon:blackrock-depths | 41 | 595 | 0 |
| dungeon:onyxia-s-lair | 1 | 226 | 0 |
| dungeon:scholomance | 12 | 1860 | 836 |
| dungeon:zul-gurub | 48 | 352 | 0 |
| dungeon:stratholme | 16 | 798 | 0 |
| dungeon:maraudon | 11 | 399 | 0 |
| dungeon:ragefire-chasm | 6 | 769 | 0 |
| dungeon:molten-core | 7 | 370 | 0 |
| dungeon:dire-maul | 38 | 380 | 0 |
| dungeon:blackwing-lair | 2 | 363 | 0 |
| dungeon:ruins-of-ahn-qiraj | 1 | 287 | 0 |
| dungeon:ahn-qiraj-temple | 79 | 239 | 0 |
| dungeon:naxxramas | 91 | 156 | 0 |

Validation:

- App unit64passed/1existingignored, authored integration10passed, preparation
  integration8passed. New boundary tests preserve4096 refs and refuse4097,
  retain schema1 refusal and32-note bound. Actual-adapter1024-reference report
  keeps every member/reference and unknown source requirement. Multiple-item
  acquisitions also exercise expansion refusal before overflow. The synthetic
  maximum4096-reference single-member report measures3496777bytes; a complete
  report that exceeds4194304bytes still refuses.
- Full app coverage suite277passed/3existingignored; authored_content450/494=91.09%,
  dungeon_preparation927/999=92.79% executable lines, unchanged90% gate.
- Desktop focused8tests passed, including first/next/last/previous pages and
  all provenance requests≤48 visible member refs. DungeonsScreen69/74=93.24%
  executable lines. Typecheck/lint, app all-target Clippy, Rust format and
  whitespace checks passed.

Test fixture mistakes encountered during development were corrected: preserve
the existing acquisition referenced by an objective, use a positive fixture
reward quantity when asserting unknown conditions, and use future package3 in
negative tests because package2 is now supported. Production guards were retained.

Next: independent root review, normal integration into combined rehearsal, reviewed
mapper overflow repair and new immutable source pins, then rerun actual full-pack
functional proof. Missing Windows/manual checks remain unverified. No push, keys,
signing or publication occurred.
