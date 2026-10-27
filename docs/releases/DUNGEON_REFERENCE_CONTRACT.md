# Versioned dungeon association lists

Content catalog schema2 retains its historical dungeon reference bound when
`reference_contract_version` is absent or explicitly1: each preparation_quests
and rewards Known value contains at most1024 unique, resolving identities.
Explicit2 permits4096 in those two fields only. Every other existing collection,
evidence, travel, checklist and boss bound is unchanged. Unknown and Unsupported
remain distinct from a Known empty list.

The optional field is non-null. Null, strings, floats, booleans, negative values,
zero and unrecognized versions fail validation even when lists are unavailable.
Absence remains absent on serialization, preserving legacy canonical catalog
bytes. Explicit1 stays explicit on serialization; pack canonicalization therefore
preserves both accepted forms without a general unknown-field exception.

Dungeon composition validates both version declarations and carries the maximum
explicit version. This is structural capacity, not a source fact or evidence
promotion. Existing authored/historical list precedence, observations and typed
conflicts are unchanged. A legacy authored record inheriting a complete long
historical list consequently remains valid after serialization and storage.

Mapper4 declares profile2 and preserves complete source-linked lists through4096.
Larger associations are wholly Unsupported with exact bounded-association counts,
never partial Known lists. Canonical underlying quest/acquisition records remain
shipped. Mapper3 remains the original-scope Unsupported-overflow repair; its
version and evidence are not relabelled. Full corpus regeneration, exact source
retention, admission bindings and functional proof follow independent code review.
The local preparation package/report retains its separate versioned bounds and
4MiB total report limit;4096 acquisition IDs may expand to more item/source pairs
and must be refused when the separate consumer bound is exceeded.
