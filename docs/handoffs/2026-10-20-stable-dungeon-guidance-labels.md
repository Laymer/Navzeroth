# Dungeon guidance labels — schema3 consumer follow-up

Owner/sole writer /root/desktop_review; isolated fix/stable-dungeon-guidance-labels
from exact regenerated5f65c43f3715ee488704d1cf483deece603de127; claim3122771c.
The original final-content worktree and immutable packs remain untouched/clean.

Actual release-mode full scenario failed93.670seconds, peak6403580KiB on shared
Linux/WSL2 host, after staging/activation/planner and quest/role/reminder checks.
It refused the first dungeon with “Preparation check label must contain at most
160bytes”. No functional artifact was written. Source audit:46of72 checklist
labels exceed160bytes:26role heuristics167bytes and20entrance derivation/uncertainty
labels270–274bytes; maximum274bytes. All26 supported dungeons are affected.

Explicit NEW LocalPreparationPackage schema3 accepts4096 references and512UTF8-byte
check labels. Schema1 remains32refs/160bytes and schema2 remains4096refs/160bytes.
The verified-catalog adapter emits3, TypeScript declares1|2|3, and the actual full
scenario expects3. No source label is truncated or rewritten. Existing32checks,
individual evidence/notes, title, context/reference,1MiBrequest and4MiBcomplete
report bounds remain. Unknown readiness and source observations are unchanged.

Actual checks:
- Full app LLVM suite279passed/3existingignored; authored_content455/494=92.11%
  executable lines and dungeon_preparation957/1029=93.00%, unchanged90% gate.
- App unit65passed/1ignored; authored-content11 and preparation8 pass.
  New tests check160/161/512/513bytes in each schema, including multibyte UTF8,
  exact retained label text and unknown readiness through source selection.
- Desktop focused13tests pass, including schema3 complete512byte UTF8 label
  rendering and unknown readiness; typecheck/lint pass. Prior actual fullUI553
  tests/per-file90% evidence remains separately recorded at final CODEa9e.
- All-target app Clippy, Rust format and whitespace checks pass.

Separate successful source/material observations from failed runtime:
- Actual source parity helper at5f65 passed23.312s/1113792KiB, checking exact
  committed selected inputs and two mapper4 products. LFSfsck passes.
- Current-version1.0.0-rc.1 material staging passed23.701s/5641040KiB:82files,
  395722450bytes. Separate streaming verification checked the exact stagedtree,
  all lengths/SHA256 and22public corresponding-source mappings. Materialmanifest
  SHA2566cb1fb206bbb75a5b530484cc5abfa09b1e325344f4b6c50471835ed3e989389
  binds5f65; these historical measurements do not bind the repaired application.
- An initial requested1.0.0 staging correctly refused version mismatch before
  creating materials; version declarations remainrc1 until the separate releasePR.

Next: independent root review of this immutable repair, normal merge into final
rehearsal, fresh release-mode compile and complete scenario over SAME immutable
packs/oracle.22retained Python/code/terms files and a9e sourcepin are byte-identical;
no source rebind/regeneration required for this Rust consumer repair. New actual
proof must bind the repaired committed application/test, not the failed5f65 run.
No functional Pass, Windows/manual observation, signing, pushes or publication.
