# QuestieDB local staging CLI

Owner / sole writer: Codex `/root/public_sources`.
Baseline: `d49b1b745027493d0aea0b3a070c735e2737813f`.
Branch / worktree: `feat/questiedb-staging-cli` /
`/tmp/navzeroth-v1-staging-cli`. Claim `f1c4c1f` precedes implementation.
Onboarding explicitly released main/lib registration ownership at enforcement
head `1b7c64c`; normal merge `52b7dba` preserves that history before registration.
Frozen implementation: `e637a902b873f62eb52a7df90ceef1e932d8b9e6`.
Task PR: [122](https://github.com/Laymer/Navzeroth/pull/122).
Normal merge `4f725fc` also inherits independently reviewed enforcement fix
`03ef364`, which clears the packaging test harness's stale failure exit status
after its successful assertions. CLI source and tests remain byte-identical to
the frozen implementation; no enforcement production behavior changed.

## Result

`azeroth-cli stage-questiedb` accepts one or more explicit `--quest`, `--npc`,
`--item` and `--object` local file paths. Optional `--revision`, `--edition` and
`--build` are caller declarations applied to all selected files; omitted fields
remain unknown. It returns the existing ingest `StagingReport` as JSON on stdout.
There are no network requests, directory discovery, correction execution, file
writes, record serialization, schema changes or playable conversion. General
README reconciliation belongs to the coordinator; this task adds a dedicated
[usage guide](../data/QUESTIEDB_STAGING.md).

Every option occurs at most once, at most four raw files are selected, and each
file is limited to 8 MiB. The reader checks all path components for symlink or
Windows reparse traversal, rejects nonregular files and parent traversal, checks
canonical duplicate selections, and bounds the actual read even if a file grows.
Opened-handle size/modification metadata is compared before and after reading;
Unix device/inode identity is also compared. Observed changes are rejected.
This follows the existing local pack/replay file policy; these metadata checks
do not claim an atomic snapshot against a hostile concurrent filesystem writer.
Only explicitly selected paths are inspected. Reports and errors omit local
paths and gameplay rows. Rejected inputs emit no partial JSON.

Exit status `0` means the staging report completed, `1` means an input/data or
source-declaration rejection, and `2` means an unsupported argument shape.
The success code does not mean the source is accepted for gameplay or shipment.
The raw-only interface supplies no correction files: all eighteen corrections
remain missing/unapplied and every build, inventory, distribution and playable
acceptance flag remains false. No source revision or build is inferred from a
filename. Even matching the audited revision is only a declaration comparison.

## Actual checks

At the frozen implementation:

- `cargo test --locked -p azeroth-cli`: **91 passed**, 0 failed, 1 existing ignored
  `write_synthetic_poster` generator. This includes all thirteen inherited release
  integration tests and ten new staging CLI tests. Log:
  `/tmp/navzeroth-staging-cli-tests.log`.
- `cargo clippy --locked -p azeroth-cli --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- Synthetic coverage includes deterministic complete/partial/zero-row reports,
  exact hashes, omitted and declared origins, unchanged false acceptance flags,
  absent neighboring-file discovery, no raw/path output, invalid/duplicate
  arguments, canonical duplicate paths, missing/nonregular paths, parent
  traversal, exact 8 MiB/+1 bounds, unsafe/malformed/wrong-entity Lua, symlink
  ancestors, a Unix socket, and non-UTF-8 local paths/declarations.
- The actual CLI executable read the four previously pinned local QuestieDB
  files, with the audited revision and `wow_forever` explicitly declared and no
  build declaration. Every parsed count, byte length and SHA-256 below matched
  the source audit. Final-code output
  `/tmp/navzeroth-staging-cli-real-final.json` is byte-identical to the first
  command output `/tmp/navzeroth-staging-cli-real.json`. No temporary parser
  harness or upstream gameplay fixture was needed or committed.

| Entity | Parsed raw rows | Input bytes | SHA-256 |
| --- | ---: | ---: | --- |
| Quest | 4,244 | 1,057,548 | `d5be8516877505334f02073826ea0f22a394ddbf604bb5a33f5a39de5bb1ee87` |
| NPC | 10,119 | 2,033,176 | `a8c4169690726323dac3c6441f74f7a4d9cafcc08ec42231d72ff28c9471f0ee` |
| Item | 14,889 | 2,181,706 | `987d037b6df94d5b0b1dc326a1670581e3abb952736a4ae055fd9f161328b71c` |
| Object | 6,645 | 1,011,387 | `af4536c12c5b1da5fd32cb33be5c9c01d2436f97a5b9f60e2cdd238273b2cc09` |

Source inputs remain outside the repository under
`/tmp/navzeroth-source-audit-inputs/questiedb/`; original source path separators
are represented by `__` in those local filenames. CLI usage accepts either
those explicit paths or an ordinary local source checkout; it never derives the
declared origin from their layout. The actual report has four raw summaries,
no missing raw entity, eighteen missing corrections, zero applied corrections,
null source build and false build/target/distribution/playable verification.
These are Linux CLI checks, not Windows interactive or live-game acceptance.

Initial failures are not counted as passes: the sandbox prohibited creating the
Unix socket fixture; the approved rerun then passed all ten new tests and both
complete-suite runs. Strict Clippy initially requested `as_chunks` for exact
argument pairs; the idiomatic correction passed lint and the complete suite.
Normal merge and commit writes initially hit sandbox Git-lock restrictions;
approved retries preserved the existing branch histories and unrelated files.

## Review and next task

Independent reviewer `/root/acceptance` found no blocker or important finding
at exact implementation `e637a902b873f62eb52a7df90ceef1e932d8b9e6`, reviewed usage
and handoff at `7dbac38`, and verified unchanged CLI source/tests through
`4f725fc`. In a private checkout the reviewer independently ran
`cargo test --locked -p azeroth-cli --test questiedb --test release_readiness --lib`:
**32 passed** (nine library, ten staging and thirteen release tests). The initial
socket fixture was blocked by sandbox permissions; the rerun after the
environment permission update passed all 32. Peer log:
`/tmp/navzeroth-v1-staging-cli-peer-tests.log`.

The final peer CLI independently parsed all four pinned raw files and verified
every count, byte length and SHA-256 against the source audit. Its report
`/tmp/navzeroth-v1-staging-cli-peer-real.json` is byte-identical to the author's
final report, with null build, eighteen missing corrections, zero applied
corrections and false acceptance flags. Review confirmed bounded read-only
behavior, no execution, report privacy, unverified declarations and preserved
`release-readiness` registration. These remain Linux checks, not native/live or
source-distribution acceptance. Coordinator owns current nine-job merge-candidate
checks and general README integration. The task's final handoff push requires
fresh hosted checks; no earlier failed or superseded run is counted as a pass.

Do not promote these raw counts to corrected target-content coverage. Source
permissions, exact-build completeness, correction composition, reference/field
semantics, playable models, live evidence and native Windows evidence remain
separate prerequisites. Restricted correction normalization requires its own
reviewed operation/precedence mapping and permitted source policy. The CLI is a
reproducible local inspection surface for that future work, not a source approval.
