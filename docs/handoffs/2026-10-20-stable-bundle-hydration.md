# Exact bundle inputs for stable native coverage

Owner/sole writer `/root/tooling_review`; baseline `580361d9`; claim `fa2145f0`;
immutable production repair `53bb559181c487fdf51de5842e499eed9c5278ae`.

Coverage's default LFS-pointer checkout previously restored only two role inputs.
Stable native coverage subsequently invokes the real assembler and needs the
entire declared signed resource closure. The RC path did not exercise that path;
no stable coverage failure run is invented here.

The stable-only native adapter now runs `scripts/hydrate-bundle-inputs.py` before
production CLI assembly. The helper verifies ordinary committed manifest/identity
bytes, the identity's manifest digest, all FileRefs against exact HEAD canonical
LFS OID/size or streamed ordinary Git blob bytes, then fetches only pending exact
paths in bounded batches. It refuses malformed pointers, missing/changed inputs,
links, duplicate references, wrong pins and excessive counts/bytes; postfetch
checks every resource and rechecks source/metadata. Limits are4MiB manifest,
64KiB identity,8192 files,576MiB/file and2GiB aggregate. Subprocesses are bounded;
the adapter allows20minutes for the whole hydration command. This helper proves
HEAD byte closure, not signature trust or publication readiness. The existing
compiled-policy assembler and staged verifier remain mandatory final authorities.
No imported content executes. Retained22 generator code paths/pins are unchanged.

Windows autocrlf also changed the pretty public review JSON. Exact LF attributes
now cover only the original and forthcoming v2 stable public review paths. Reads
remain hash-strict; historical v1 review bytes are untouched.

Actual checks:

-11 temporary Git/LFS offline-cache tests PASS: pointer bytes fail resource digest
  before hydration, exact cache hydration/idempotence, wrong working/HEAD bytes,
  duplicate/missing/unsafe refs, bounds, failed fetch, metadata/source mutation,
  malformed pointers, ordinary large Git blob, symlink refusal and autocrlf review
  refusal followed by exact LF success, plus replacement-v2 LF preservation.
-12 existing role hydration tests PASS; native bundle adapter ordering test1 PASS;
  coverage workflow Actionlint and `git diff --check` PASS.
-Actual current84-resource manifest checked with no network. Then restored only
  the22 declared LFS resources in this owned isolated worktree to exact HEAD
  pointers and ran offline hydration:84 files/395740838 bytes verified,22 restored
  from existing local objects. Artifact
  `/tmp/navzeroth-tooling-bundle-hydration-actual-stubs.json` retains that result.
-Initial new-suite run had a test-fixture no-op Git commit error (one error among
  eight tests); allowing empty synthetic fixture commits corrected it. Final
  eleven-test suite passes. No production behavior was loosened for that error.

No Windows native coverage, network fetch, signature generation or acceptance
observation was performed. Existing signed production must be archived/reviewed
and rebound by the release owner after independent review of this production
repair. Next: independent review, then normal integration and renewed frozen
production/public review/receipt; run actual required hosted checks. No push or
merge has been performed by this task.
