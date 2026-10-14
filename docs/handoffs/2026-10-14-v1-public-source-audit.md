# Handoff: v1 public sources and content inventory

Owner / sole writer: Codex `/root/public_sources`.
Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
Branch / worktree: `docs/v1-public-source-audit` / `/tmp/navzeroth-v1-sources`.
Claim commit: `560ab61`, before audit edits. The coordinator retains ownership of
shared STATUS, task ledger and the versioned coverage/release-evidence contract.

## Delivered

- [Readable audit](../sources/v1-public-source-audit.md): QuestieDB-first audit,
  then CMaNGOS historical comparison; source policy/build limitations, source
  field mappings, twelve mandatory coverage families and concrete adapter tasks.
- [Machine-readable audit](../sources/v1-source-audit.json): two immutable source
  commits/trees, 109 selected file identities, 26 complete file SHA-256 values,
  all 74 entity field indices, raw ID-line counts and explicit unknown target
  denominators. This audit JSON is not a production dataset/registry or the
  coordinator's release-evidence schema.
- No gameplay data, third-party runtime code, public exchange-schema change or
  content-source permission promotion. Existing historical records untouched.

## Actual validation

Read the GitHub primary repositories and specified provenance/license pages;
obtained both pinned commits and untruncated recursive trees through `gh api`.
QuestieDB: 694 tree entries at `9bc909a464ce0af80af83ce8a4f959b5f911fe88`.
CMaNGOS: 432 tree entries at `ec4f596146be6467ea93c57397858e329e2db852`.
Complete raw downloads remain only in `/tmp/navzeroth-source-audit-inputs`.

Local Python verified all 26 complete files' byte counts and Git blob SHA-1
against those trees before calculating SHA-256. Four raw Questie tables have
4,244 quest / 10,119 NPC / 14,889 item / 6,645 object ID lines, with no duplicate
IDs under the documented lexical method. No claim of parsed row validity or
effective corrected count follows. The 36/15/16/7 metadata field maps were
extracted and checked. Existing real-content payload collection counts were
read directly at the baseline.

`git diff --check` passed. JSON identity, SHA formats, unique file paths,
field-map sizes, mandatory-unit unknown denominators and false distribution/
build-acceptance assertions passed. No production code changed, so application
tests were not rerun for this documentation task. No upstream Lua/SQL or upstream
test commands were executed. No live-client/Windows acceptance performed here.

Initial sandboxed API pipelines failed on network access; approved read-only
retries succeeded. A broad clone was interrupted before completion; no checkout
or clone result is counted as evidence. Narrow source-byte retrieval replaced
that acquisition. A local text-copy newline mismatch in CMaNGOS AUTHORS was
identified by the Git blob check, corrected only to the exact matching original
bytes and reverified. Truncated inspection output was never assigned a full-file
SHA-256. These are retrieval/measurement corrections, not upstream modifications.

## Decisions and limitations

QuestieDB's inspected tree has no general data license; its interpreter notices
do not authorize data redistribution. Its documented prior builds, held rows,
partial models and missing public generated provenance cannot prove full 70374
coverage. CMaNGOS includes GPLv3 and separate Blizzard-material restrictions and
targets patch 1.12. Neither source is accepted for release assets.

All eighteen QuestieDB correction providers and dependent passes need explicit
coverage dispositions. Complete raw-table parsing alone is insufficient. Keep
partial facts in staging: current domain quests require real stages, and items
require slots; do not invent these to pass validation. Preserve nil/empty/false/
zero and masks wider than u32. NPC waypoints are not player roads. Unknown
target inventories block stable release rather than becoming zero missing rows.

## Review focus and next task

Review source-license/build claims against immutable links, distinction between
full-byte versus tree-only hashes, field-index accuracy and correction order.
Peer review will be recorded by the assigned independent reviewer/coordinator;
this author does not claim self-review as independent acceptance.

Next dependency-ready task is a bounded, deterministic Questie raw-table staging
parser with synthetic malformed/limit/duplicate/nil/zero/wide-mask tests and
explicit unsupported correction coverage. It must not execute source code,
activate partial rows or ship upstream data. Coordinator assigned isolated
`/tmp/navzeroth-v1-ingestion`, branch `feat/questiedb-staging`, baseline `bb3782c`.
Claim that separate task before code changes; the CLI remains another owner's
surface. Source permission, complete target inventory, role models and real
client/Windows acceptance remain distinct external prerequisites.
