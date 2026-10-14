# Inspect local QuestieDB raw files

`stage-questiedb` reads explicitly selected local files and prints one JSON
staging report. It uses the bounded QuestieDB raw adapter; it does not fetch
sources, discover neighboring files, execute Lua, apply corrections, write
content packs or activate gameplay records.

Select one or more raw entity files. Paths are independent of their filenames:

```sh
cargo run --locked -p azeroth-cli -- stage-questiedb \
  --quest /local/QuestieDB/data/Forever/foreverQuestDB.lua \
  --npc /local/QuestieDB/data/Forever/foreverNpcDB.lua \
  --item /local/QuestieDB/data/Forever/foreverItemDB.lua \
  --object /local/QuestieDB/data/Forever/foreverObjectDB.lua
```

The example paths are placeholders for files already available locally. On
Windows, use quoted Windows paths as needed; the same options work in PowerShell
on one line. Redirect stdout with your shell to retain the JSON report. The
command itself creates no files. Input contents and local paths are omitted from
reports and errors.

Optional declarations apply to all selected files:

```sh
cargo run --locked -p azeroth-cli -- stage-questiedb \
  --quest /local/QuestieDB/data/Forever/foreverQuestDB.lua \
  --revision 9bc909a464ce0af80af83ce8a4f959b5f911fe88 \
  --edition wow_forever
```

`--revision` accepts forty lowercase hexadecimal characters. `--edition`, when
provided, must be `wow_forever`; `--build` accepts an explicit label of up to
64 ASCII letters, digits, dots, underscores or hyphens. Omitted revision,
edition and build remain JSON `null`. No directory name, filename or requested
release target supplies these values. Declarations are unverified: even a
revision matching the audited pin or a build declaration of `1.60.1.70374`
does not authenticate file contents or verify game compatibility. Compare the
reported exact bytes and SHA-256 with the pinned source audit independently.

The report includes the adapter version and audit revision, caller declarations,
each parsed raw entity's byte length, SHA-256 and row count, and missing entity
and correction inventories. Entity order is deterministic. Empty raw tables
remain empty; successful parsing does not establish a complete inventory.
Raw rows, names, coordinates and other gameplay values are never serialized.

The command accepts at most four raw files, each at most 8 MiB. The adapter also
limits nesting, strings, values and record counts. Every option may occur only
once; at least one raw file is required. Unknown options, positional arguments,
missing values, duplicate selections, nonexistent files, directories, symlinks,
Windows reparse points, special files and parent traversal (`..`) are rejected.
The reader checks file size and modification metadata around each read and
rejects observed changes. Rejected inputs produce no partial JSON.
Exit status is `0` for a completed staging report, `1` for rejected files/data
or invalid source declarations, and `2` for unsupported argument shapes.

All eighteen audited correction providers remain missing and unapplied through
this raw-only command. Correction coverage, derived passes, source-build
verification, target-inventory completeness, distribution eligibility and
playable-dataset flags remain `false`. Source redistribution permissions and
exact-build completeness are unresolved; a successful report grants neither.
Keep any locally available upstream files outside committed fixtures and release
assets unless their applicable permissions are independently established.

See the [source audit](../sources/v1-public-source-audit.md),
[staging parser handoff](../handoffs/2026-10-14-questiedb-staging.md) and
[CLI handoff](../handoffs/2026-10-14-questiedb-staging-cli.md) for pinned mappings,
actual verification and remaining work.
