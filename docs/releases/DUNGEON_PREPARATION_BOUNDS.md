# Dungeon preparation package bounds

LocalPreparationPackage schema1 retains its historical 32-entry section limits.
Schema2 supports up to 4096 unique quest IDs and expanded item/acquisition reward
references in each list. Schema3 retains those reference bounds and permits
check labels through512UTF8bytes, preserving source derivation and uncertainty.
Schema1/2 check labels remain bounded to160bytes; labels are never truncated.
The installed v2 catalog adapter emits schema3. Other
sections, individual evidence lists and notes retain 32 entries; total evidence
remains limited to 1024. Titles, note lengths and predicate limits are unchanged.
The outer preparation request and explanatory report remain schema1.

Manual request JSON remains limited to 1 MiB and every returned report to 4 MiB.
A package with too many references, or a party evaluation whose complete report
exceeds the byte limit, is refused with an explicit error. Reports are never
truncated. The adapter checks expanded reward counts before appending; multiple
items from one acquisition count separately. A larger package schema grants no
source admission, eligibility or target-validation status.

The desktop shows 24 quest references and 24 reward references per member per
page, with totals and previous/next controls. Every report reference is reachable;
provenance lookups follow the visible pages. Reward references describe source
associations, not guaranteed drops or established role benefits. Source-area quest
associations do not establish eligibility, prerequisites or current-game validity.

This consumer contract does not enlarge core v2 per-record source-list bounds or
repair mapper truncation. The separately recorded source-overflow repair must
refuse or explicitly classify unsupported associations before coverage can pass.
