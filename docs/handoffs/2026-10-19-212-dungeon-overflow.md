# Original PR212 dungeon association overflow repair

Owner/sole writer `/root/tooling_review`; baseline
3fc850c33dfd720b6cb54b4c19ea8082f474b932; claim32ff3945. Code
2e89b1ff9761bbb3a320e20a68c350e71fc63608 was independently reviewed by root
with27 mapper tests passing before immutable source rebinding. Generated commit
cef5e1688276c0dda0e72567d72e405749468e4b preserves original authored r1.

Mapper v3 corrects silent `[:1024]` truncation without enlarging the content-v2
contract. An association of0 through1024 remains Known and complete. A larger
association becomes Unsupported without any partial value; normalized conflicts
and summary expose actual, contract limit, exposed0 and unresolved counts. This
is a field availability correction, not a claim that missing records passed.

The original corpus has four overflowing reward associations: Stockade1541,
Sunken Temple2559, Zul Farrak2147 and Scholomance1860. All8107 associations are
explicitly unresolved; the old partial fields omitted4011 references. The four
additional pack coverage gaps are intentional. All canonical quests, acquisitions,
loot and gameplay record sections are unchanged; evidence notes change only the
mapper v2→v3 identity. No future dungeon estimates, authored role content or
objective projection was copied. Future mapping must use v4 or later.

Exact source closure uses the existing read-only cache,19 retained originals and
22 corresponding-source files. Every corresponding file was independently
compared with `git show` at the pinned code commit. Upstream bytes, rights,
proofs, scopes and origins are unchanged. Per-file transformation v3 updates
require the deterministic file-list source revision to change; derived mapper
bytes/license source URLs bind the new reviewed code. Only the current updates
registry SHA was rebound. Historical source objects/evidence remain intact.

Actual checks:160 offline Python tests pass, including0/1024 boundaries and a
full generated1025-quest/1025-acquisition fixture that preserves every canonical
ID while both fields are Unsupported. Own exact CLI build, offline source-sync,
pack build and independent verify-pack pass. Pack:3files367540211bytes,
393601 coverage gaps, distribution_eligible=true. Build107.20s/6695980KiB peak
child RSS; verify49.89s/5362404KiB. Two bound mapper regenerations match committed
bytes exactly in25.26s. Git LFS objects/pointers fsck and diff check pass.

[Generation/comparison/resource measurements](../data/212-dungeon-overflow-generation-v1.json)
and [Linux reproducibility receipt](../data/212-dungeon-overflow-parity-linux-v1.json)
record exact commands, identities, products and limitations. Initial external
comparison assertions were refined to account for the intentional mapper notes,
revision and source file-list identity changes; all final exact assertions pass.

No network acquisition, signing, push or release occurred. Public raw source URLs
remain unverified until the original pinned commit history is pushed. Root must
review final outputs, integrate the separate consumer repair, refresh all protected
checks and run source-platform parity on the resulting PR212 head. Windows,
installer inclusion/performance and stable publication remain separate gates.
