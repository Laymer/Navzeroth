# Handoff: bounded QuestieDB raw-source staging

Owner / sole writer: Codex `/root/public_sources`.
Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
Branch / worktree: `feat/questiedb-staging` / `/tmp/navzeroth-v1-ingestion`.
Claim `27d3997` precedes code `ed1a1df`. Source-audit dependency:
[PR119](https://github.com/Laymer/Navzeroth/pull/119), frozen at `7ec7429`.

## Result and interface

`azeroth_ingest::questiedb` stages the four audited raw Forever tables. It
recognizes the complete inert wrapper and exact field declarations, reads the
deferred literal table without executing Lua, and exposes immutable in-memory
rows. No rows become domain quests/items or activatable content. Reports contain
counts, exact input byte hashes and limitations rather than gameplay payloads.

- `stage_raw(input, Entity, SourceOrigin)` returns a `StagedTable` or a closed
  `Rejection`; rows are available through `records()`.
- `inspect_correction(input, path, SourceOrigin)` fingerprints an explicitly
  unsupported provider from the pinned eighteen-path inventory. It never parses
  or applies provider code, including a provider that appears literal-only.
- `summarize(tables, corrections)` produces a deterministic serializable
  `StagingReport` and rejects duplicate files/entities or differing declared
  origins. Raw records do not implement Serialize.
- `SourceOrigin` contains optional caller-declared revision, edition and build.
  Missing values remain `None`; known non-Forever editions are rejected. A
  revision-string match is explicitly a declaration comparison, not source
  authentication. Every report keeps build verification, full target inventory,
  correction coverage, derived passes, distribution eligibility and playable
  dataset flags false. All eighteen corrections remain missing or unsupported.

The shared existing literal reader has a private SOURCE policy. It preserves nil
as a present value and positional hole, false, zero, empty tables and absent
fields distinctly; integer tokens remain exact i64 and overflow is rejected
instead of rounded into f64. Existing SavedVariables entry points still reject
nil and retain their previous grammar/limits. Explicit SHA-256 uses the existing
workspace `sha2=0.10.9`; only ingest's dependency/lock entry changed.

Source ceilings: 8 MiB per file, 32 nested tables, 64 KiB per decoded string or
identifier, 1,000,000 values per parsed literal and 100,000 rows. Both metadata
and data literals have a value budget; the exact metadata declaration is checked
before parsing data. Keys/IDs, duplicate keys, unknown fields, unsupported
expressions, malformed UTF-8, nonfinite numbers and trailing wrapper content fail
closed. The source reader has no Lua engine, process execution or dynamic loader.

## Important source distinction

Actual-byte validation found that canonical `itemMeta` declares sixteen fields,
while the pinned raw Forever `itemKeys` declares fifteen. `teachesSpell` at
canonical index 16 belongs to later provider data. `Entity::fields()` describes
the **raw** declaration and therefore returns 36/15/15/7 fields for
Quest/NPC/Item/Object. The source audit's canonical 36/15/16/7 metadata map remains
correct; a raw adapter must not silently assume those declarations are identical.

## Actual checks

After loading `/home/laymer/.cargo/env`:

- `cargo test --locked -p azeroth-ingest`: **62 passed**, 0 failed, 0 ignored.
  Includes fourteen new staging tests and a source-budget unit test, plus all
  existing SavedVariables, import, session, watcher and no-execution tests.
- `cargo clippy --locked -p azeroth-ingest --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- A temporary local example ran the four already downloaded, hash-verified
  QuestieDB files through the real staging API, without source execution.
  Final results on the changed parser are recorded below. No copied upstream
  rows or local driver are committed. Driver:
  `/tmp/navzeroth-local-questiedb-audit.rs`; output:
  `/tmp/navzeroth-staging-real.log`. The temporary example was removed before
  committing. This was a Linux debug parser measurement, not Windows/game or
  ordinary-hardware release performance acceptance.

| Entity | Parsed records | Bytes | SHA-256 | Debug parse time |
| --- | ---: | ---: | --- | ---: |
| Quest | 4,244 | 1,057,548 | `d5be8516877505334f02073826ea0f22a394ddbf604bb5a33f5a39de5bb1ee87` | 81 ms |
| NPC | 10,119 | 2,033,176 | `a8c4169690726323dac3c6441f74f7a4d9cafcc08ec42231d72ff28c9471f0ee` | 170 ms |
| Item | 14,889 | 2,181,706 | `987d037b6df94d5b0b1dc326a1670581e3abb952736a4ae055fd9f161328b71c` | 203 ms |
| Object | 6,645 | 1,011,387 | `af4536c12c5b1da5fd32cb33be5c9c01d2436f97a5b9f60e2cdd238273b2cc09` | 76 ms |

These are parsed raw rows only. They are not corrected effective records,
reference/coordinate/semantic validation, permitted distribution, availability
on 70374, or complete content coverage. The final combined local report leaves
the source build null and all eighteen correction paths missing.

Initial failures were resolved rather than counted as passes: `cargo` was not
on the initial shell PATH; the old no-engine test's exact dependency allowlist
needed the approved SHA-256 dependency; and the first raw Item probe correctly
rejected the assumed sixteen-field header. Tests and the local probe passed
after those changes. No imported data was patched to make the parser pass.

## Review and limitations

Independent reviewer `/root/acceptance` found that bare named table keys could
exceed the source's 64 KiB string bound, while equivalent quoted keys could not.
Code `ed1a1df` adds a source-only identifier cap before allocation and a
bare/quoted boundary regression; the final tests and four-file probe include
that fix. Final independent review at exact code commit
`ed1a1dfe7cda0735372ce54a23c16670a71d199e` found no remaining blocker or important
finding. The reviewer independently passed all 62 ingest tests, checked all
eighteen correction paths against the source audit, verified all four file
lengths/SHA-256 values and reproduced every parsed count. The independent probe
measured 79/195/231/97 ms per entity; all four results retained used 234,544 KiB
peak RSS and 0.758 s wall time under Linux WSL2 on Ryzen 9950X3D, Rust 1.99.0.
These are local debug measurements, not Windows/live acceptance. The observed
raw body value counts were 158,056 / 419,956 / 417,446 / 193,273 and maximum table
depths 6 / 6 / 3 / 5. The reviewer confirmed explicit nil and wide integers,
false build/distribution/playable flags, and eighteen missing corrections.
Peer logs: `/tmp/navzeroth-v1-ingestion-peer-tests.log`,
`/tmp/navzeroth-v1-ingestion-peer-real.log` and
`/tmp/navzeroth-v1-ingestion-peer-memory.json`. The temporary peer driver was
removed; missing `/usr/bin/time` was replaced with Python child-resource
measurement and was not counted as a passing invocation. Coordinator owns
integration and all nine exact-candidate gates; this handoff does not claim
those hosted checks have completed on this task PR.

Unimplemented by design in this raw-only task: correction composition,
symbolic constants/operators, runtime branches, derived passes, source conflict
resolution, reference validation and playable field semantics. The fixed
correction inventory names its audited revision; other revisions remain
unaudited even if their raw syntax matches. No source permission, license,
record-level build evidence, role model, quest stage or equipment slot is
invented. No CMaNGOS SQL execution/adapter or public schema change is included.

## Next task

The CLI owner can expose the bounded staging report through an explicit local
audit command, preserving false eligibility and original source declarations.
Coordinator owns independent source-policy/coverage/release-gate validation.
Only implement restricted correction normalization after the exact dependency
and operation semantics are reviewed; until then every affected content family
remains incomplete. Permission/build/content-inventory blockers continue to hold
full-content stable publication independently of this parser's tests.
