# Private guide development inspection

2026-10-18, Codex `/root`, read-only. The owner supplied a private guide-design
folder. Selected generator/model/planner/verifier files were parsed as AST/text;
none was executed. Their references to QuestieDB/scraped data confirm that generated
facts remain inadmissible under ADR 0006. No private source or game-fact list was
copied into the repository.

One owner-generated Human 1–60 guide file was read by the existing bounded inert
RXP importer/replay verifier for Human Warrior, using deliberately empty synthetic
research facts v2 (not a real-client facts set). It contained 17 guides and 547
replayed steps. The report recorded 63 errors, 39 warnings and 4,275 unverified
findings; 2,155 internal checks were OK. Exit 1, replay_clean false,
facts_are_evidence false and passes false. These are exploratory parser/replay
results, not proof of gameplay defects or successful route validation. This wider
1–60 file is not an acceptance check of the requested 1–20 slice.

No Lua or generator code ran. Raw report contents/quest IDs were not retained or
published; temporary empty facts were removed. Private local metadata receipts
record source hashes and structural inspection separately. The user's originals
were unchanged. Class-specific routes still need build-bound prerequisites,
eligibility, XP/reward and travel evidence; observations alone cannot supply them.
