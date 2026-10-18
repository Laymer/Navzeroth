# Continuation 3 — selected caches, hotfixes and observations

Owner and sole writer: Codex `/root`; 2026-10-18.
Baseline: `7f5587a7335eebce3a49dd8ff0b37fea88c08d84`, reviewed continuation-2
branch, PR #192 then pending exact-head checks. PR #192 merged as `1cb6bb9`
after all nine checks/source workflow and real-client lifecycle checks passed.
This dependent branch incorporates resulting main before its own PR. Original checkout and `.claude/` preserved.
Branch/worktree: `feat/continuation-local-inputs`, `Navzeroth-inputs`.

Scope: versioned input lineage; bounded reviewed cache envelopes with exact
build/locale matching; explicit unsupported dispositions; permitted snapshot and
session observations as separate research evidence; selected-file persistence
and CLI/application interfaces. Mutable cache reads require a reliable native
closed-game precondition and stable before/after identity. No implicit discovery,
Lua execution, decryption or gameplay availability promotion.

Checks: malformed/truncated/oversized and mixed-tuple inputs; operation ordering
and conflicts; unsupported versions; source mutation/process refusal; private
artifact verification; historical snapshot/session compatibility; relevant Rust,
Lua and desktop checks. Missing real-cache/operator checks remain unverified.

Next: implement pure contracts/parser first, native input boundary and inert
observation adapter second; request read-only independent review before merge.

Operator decision: use synthetic cache fixtures only for this continuation while
playing. TODO: explicitly select real cache files and repeat verification after
the operator closes the game. Do not read mutable caches during this session.
