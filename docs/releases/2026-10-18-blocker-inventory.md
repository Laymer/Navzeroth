# Readiness blocker inventory — continuation baseline

Owner: Codex `/root`. Baseline: `7aecba8f170bd35fe2ece0dbce2c98a4337dfa4a`.
Run: 2026-10-18, Linux, Rust 1.99.0, clean detached baseline worktree.
Command: `azeroth-cli release-readiness --root <clean-baseline> --version 0.9.2`.
Exit 0, publication allowed for the current development version; stable and
candidate readiness false; 69 blockers, zero verified evidence files and packs.
This report does not authorize publication of a renamed or changed build.
Requesting `1.0.0-rc.1` before the version PR correctly refuses with
"Requested release version differs from application, addon or lockfile" (exit 1).

The numbered messages below are the exact ordered gate output. Engineering owns
contracts and validation; the owner supplies private inputs and live/interactive
evidence. Imported counts are never independent completeness review.

| # | Exact blocker | Owner | Clearing condition |
| --- | --- | --- | --- |
| 1 | Release target is pending the first stable live build after launch (ADR 0006) | Owner → Eng | Record post-launch live tuple, freeze manifests and rebuild; V4.1–V4.2. |
| 2 | Release slice classes are unknown | Owner → Eng | Verify launch Human class list; populate and pass the slice matrix; V1.4/V4.2/V5.2. |
| 3 | Content inventory is incomplete: quests | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 4 | Required records are unknown: quests-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 5 | Content inventory is incomplete: equipment | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 6 | Required records are unknown: equipment-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 7 | Content inventory is incomplete: acquisition | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 8 | Required records are unknown: acquisition-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 9 | Content inventory is incomplete: geography | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 10 | Required records are unknown: geography-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 11 | Content inventory is incomplete: travel | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 12 | Required records are unknown: travel-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 13 | Content inventory is incomplete: dungeons | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 14 | Required records are unknown: dungeons-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 15 | Content inventory is incomplete: class_roles | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 16 | Required records are unknown: class_roles-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 17 | Content inventory is incomplete: party | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 18 | Required records are unknown: party-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 19 | Content inventory is incomplete: reminders | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 20 | Required records are unknown: reminders-inventory-pending | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 21 | Required record inventory is empty or excessive | Both | Enumerate exact slice IDs from verified facts; independently review completeness; V2.5. |
| 22 | Accepted source revisions are missing or invalid | Eng | Admit retained sources with verified revisions, rights and notices; V2.5–V2.6. |
| 23 | Initial tested source revision and production fingerprint are pending | Eng | Freeze and bind tested source commit and production fingerprint; V5.1. |
| 24 | Required acceptance observation is pending: LIVE-BUILD | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 25 | Required acceptance observation is pending: LIVE-CAPTURE | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 26 | Required acceptance observation is pending: LIVE-TRANSFER | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 27 | Required acceptance observation is pending: LIVE-RELOAD | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 28 | Required acceptance observation is pending: LIVE-RESTRICTIONS | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 29 | Required acceptance observation is pending: LIVE-HUD | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 30 | Required acceptance observation is pending: LIVE-PLAN | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 31 | Required acceptance observation is pending: LIVE-ADVANCE | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 32 | Required acceptance observation is pending: LIVE-DEVIATION | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 33 | Required acceptance observation is pending: LIVE-DUO | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 34 | Required acceptance observation is pending: LIVE-SESSION | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 35 | Required acceptance observation is pending: LIVE-NEXT-PLAN | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 36 | Required acceptance observation is pending: LIVE-REMINDERS | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 37 | Required acceptance observation is pending: LIVE-GEAR | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 38 | Required acceptance observation is pending: LIVE-DUNGEON | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 39 | Required acceptance observation is pending: WIN-INSTALL | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 40 | Required acceptance observation is pending: WIN-UPGRADE | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 41 | Required acceptance observation is pending: WIN-UNINSTALL | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 42 | Required acceptance observation is pending: WIN-RECOVERY | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 43 | Required acceptance observation is pending: WIN-OFFLINE | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 44 | Required acceptance observation is pending: WIN-FOCUS | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 45 | Required acceptance observation is pending: WIN-REFERENCE | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 46 | Required acceptance observation is pending: WIN-DISPLAY | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 47 | Required acceptance observation is pending: WIN-ACCESSIBILITY | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 48 | Required acceptance observation is pending: WIN-PERFORMANCE | Owner → Eng | Run the named acceptance check on the frozen candidate; retain hashed evidence and validate binding; V5.2–V5.4. |
| 49 | Slice acceptance matrix cannot be checked until the slice classes are known | Owner → Eng | Verify launch Human class list; populate and pass the slice matrix; V1.4/V4.2/V5.2. |
| 50 | Unit coverage is missing, duplicated or excessive | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 51 | Mandatory unit coverage has not passed: quests-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 52 | Mandatory unit coverage has not passed: equipment-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 53 | Mandatory unit coverage has not passed: acquisition-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 54 | Mandatory unit coverage has not passed: geography-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 55 | Mandatory unit coverage has not passed: travel-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 56 | Mandatory unit coverage has not passed: dungeons-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 57 | Mandatory unit coverage has not passed: class_roles-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 58 | Mandatory unit coverage has not passed: party-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 59 | Mandatory unit coverage has not passed: reminders-inventory-pending | Both | Review exact slice units and prove mandatory coverage with bound evidence; V2.5. |
| 60 | Validated offline content packs are missing or invalid | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 61 | No validated pack record binding for mandatory domain: quests | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 62 | No validated pack record binding for mandatory domain: equipment | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 63 | No validated pack record binding for mandatory domain: acquisition | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 64 | No validated pack record binding for mandatory domain: geography | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 65 | No validated pack record binding for mandatory domain: travel | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 66 | No validated pack record binding for mandatory domain: dungeons | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 67 | No validated pack record binding for mandatory domain: class_roles | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 68 | No validated pack record binding for mandatory domain: party | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |
| 69 | No validated pack record binding for mandatory domain: reminders | Eng | Build and validate target-bound packs and bind exact mandatory record IDs; V2.2–V2.6. |

## Additional publication prerequisites

The readiness report is not the entire continuation release checklist. Outstanding
engineering includes durable lifecycle, selected mutable inputs, expanded
projections, media/tooltips, bound delivery and consumers, guide facts/inventory,
desktop/installer completion and measured workloads/notices. The new per-file 90%
coverage gate, independent reviews, stability workflow and exact-main release
checks are still required before development RC publication. SignPath setup and
ordinary Windows/live observations remain operator work; unsigned development
publication does not relax stable signing. Private guide inputs remain absent
until explicitly supplied. No private input is committed here.
