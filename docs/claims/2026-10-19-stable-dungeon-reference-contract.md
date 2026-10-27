# Versioned dungeon association bounds

Owner/sole writer: Codex `/root/tooling_review`.
Baseline:14c09182; isolated branch feat/stable-dungeon-reference-contract.
Normally merge original overflow repair a906eb0c, preserving future estimates.
Implement optional explicit non-null reference_contract_version: absent/1=1024,
2=4096, only preparation quests and rewards. Preserve legacy canonical bytes,
strict references and all other limits; propagate maximum validated version on
composition without changing list/evidence precedence. Mapper4 and policy4.
Code/tests/schema only; no corpus regeneration, network, signing or publication.
