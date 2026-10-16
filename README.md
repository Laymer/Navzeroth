# Navzeroth

Azeroth Navigator is a Rust, Tauri/React desktop companion with a Lua addon
for World of Warcraft: Forever. Desktop character state comes from
explicitly imported snapshots; live game observations remain inside the permitted
addon environment. Synthetic fixtures are always labelled.

The published build is the **[0.9.1 developer preview](https://github.com/Laymer/Navzeroth/releases/tag/v0.9.1)**.
It contains tested engineering foundations, but no verified playable quest, item,
acquisition or travel catalog. Interactive Windows and complete live-game acceptance
remain unverified. See the [release notes](docs/releases/0.9.1.md) and
[publication evidence](docs/handoffs/2026-10-16-v0.9.1-publication.md).

The authorized v1.0.0 target is full enumerated content for **Forever
1.60.1.70374**, with Windows desktop support. The user reports that build and
interface 16001 on Windows 11, out of combat, with pre-version-bump 0.8.1 diagnostics reporting no
errors; source-identified development captures now verify quest completion flags and one
opt-in turn-in/export flow. These limited observations do not establish full
compatibility or frozen-candidate acceptance. Missing mandatory content, source permissions or
real acceptance evidence blocks v1.0.0 publication. Acceptance candidates also
require complete licensed content and static validation; see the
[v1 release process](docs/releases/V1_RELEASE_PROCESS.md).

## Try the current preview

1. Download the Windows installer, `SHA256SUMS.txt`, `SOURCE_COMMIT.txt` and notes
   from the linked 0.9.1 release. Check the installer hash before running it.
2. Follow [Windows installation](docs/compatibility/WINDOWS_INSTALL.md), including
   the separate, explicit addon installation into the client-selected AddOns folder.
3. Use [addon setup](docs/compatibility/ADDON_INSTALL.md) to record the actual client
   build. The 0.9.1 snapshot, clock, position, plan and recording adapters admit
   `1.60.1.70338` and `1.60.1.70374`, both with interface `16001`, experimentally.
   Selectable exports and opt-in quest-turn-in recording are included. Full live
   acceptance remains required. A matching TOC interface alone does not establish compatibility.
4. Imported profiles remain local. The browser demo and bundled reference imagery
   are synthetic; they do not establish game-content coverage. Keep personal
   SavedVariables and backups private.

| Identity | Current evidence |
| --- | --- |
| Application/addon 0.9.1 | Published developer preview with verified release hashes and hosted Windows smoke checks |
| Forever 1.60.1.70338 / interface 16001 | Limited user-reported diagnostic, level and clock-age flows; other bindings and full live acceptance remain unverified |
| Reference atlas 1.60.1.70205 | Historical content identity; imagery rights incomplete and local pack distribution-ineligible |
| Forever 1.60.1.70374 / interface 16001 | Frozen v1 target; source-identified development capture/turn-in checks, with full acceptance still unverified |

Current source includes a bounded QuestieDB raw-table staging adapter and
[versioned release checks](release/v1/README.md). Staging preserves partial facts
and reports unsupported corrections. The [pinned source audit](docs/sources/v1-public-source-audit.md)
accepts neither QuestieDB nor historical CMaNGOS for shipped assets. Unknown
content inventory, source rights, real class/role models and live/Windows acceptance
continue to block v1.0.0.

[Claude Code 1.0.0 execution prompt](initial.md) — detailed continuation steps and release gates.

[Repository identity gallery](docs/brand/README.md) — planned after the verified 1.0.0 release.

## Development

Read [AGENTS.md](AGENTS.md), [STATUS](docs/STATUS.md), the latest relevant handoff,
and [the implementation plan](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md) before
changing code. [DOMAIN](docs/DOMAIN.md) describes types and boundaries;
[exchange contracts](schemas/v1/README.md) describe the bounded data formats.

### Implemented foundations

SQLite character-snapshot storage and import ordering; bounded ANAV1/SavedVariables
parsing; optional watcher for one selected file; Today/Imports/Atlas screens;
reference pin search/layers/keyboard navigation; travel, eligibility, clustering and
bounded planning; Gear/acquisition comparisons; local parties and guarded joint plan
export; reminders and local proposals; Sessions history/analytics; content revisions
with activation/rollback and local review dispositions; database backup/recovery;
Settings and a read-only desktop reference window. The addon has guarded manual
capture, bounded plan import, runtime HUD controls and opt-in manual session recording.
Rust/Lua synthetic tests cover predicate and gear parity.

Playable catalogs, documented real class/role models and complete duo/session
acceptance remain prerequisites. Party messaging, combat-log analysis, official API
access and hosted distribution remain explicitly deferred or blocked. See the
[task ledger](docs/IMPLEMENTATION_TASKS.md) for individual dispositions.

### Independent source builds

The [source pipeline](docs/data/SOURCE_PIPELINE.md) rebuilds a uniform,
provenance-bearing knowledge catalog offline from retained licensed inputs.
After provisioning the Rust toolchain/dependencies, run
`./scripts/Build-SourceKnowledge.ps1` in PowerShell 7. Reliability ranks before
trusted freshness; different builds and unresolved coverage remain explicit.
This research catalog does not establish complete Forever content.

### Development checks

Use the pinned Rust toolchain, Node version and pnpm version in the repository.
On Linux, source `~/.cargo/env` if Cargo is not on PATH (adapt for your
installation). The native Linux shell additionally needs Tauri GTK/WebKit packages;
the portable checks exclude that shell. Hosted CI compiles it on Windows.

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --exclude azeroth-desktop -- -D warnings
cargo test --locked --workspace --exclude azeroth-desktop
cargo run --locked -p azeroth-cli -- validate-fixtures
cargo run --locked -p azeroth-cli -- validate-content --input content/forever --json
cargo run --locked -p azeroth-cli -- export-schemas --output schemas/v1
pnpm install --frozen-lockfile
pnpm contracts:types
pnpm contracts:check
pnpm --dir apps/desktop typecheck
pnpm --dir apps/desktop lint
pnpm --dir apps/desktop test:unit
pnpm --dir apps/desktop build
pnpm --dir apps/desktop exec playwright install chromium
pnpm --dir apps/desktop test:e2e
```

The browser tests mock command adapters and verify UI behavior. For the labelled
browser demo run `pnpm --dir apps/desktop dev`; for the native shell run
`pnpm --dir apps/desktop tauri dev` with platform prerequisites installed.
Native Rust checks are `cargo clippy --locked -p azeroth-desktop --all-targets -- -D warnings`
and `cargo test --locked -p azeroth-desktop`.

The CLI accepts `doctor`, `export-schemas`, `prepare-atlas`, `validate-fixtures`,
`validate-content`, `build-content`, `verify-pack`, `release-readiness`,
[`stage-questiedb`](docs/data/QUESTIEDB_STAGING.md) and bounded synthetic
`replay --fixture <file>`. Replay format/examples/limits are documented in
[PLANNER_REPLAY.md](docs/data/PLANNER_REPLAY.md).
[The local-data workflow](docs/data/LOCAL_EXTRACTION.md) inspects explicitly selected
installations and extracts supported local archive tables into private research
catalogs. Compatibility > Local Data provides resource review, progress/cancellation
and paginated inventory inspection. Research decoding does not establish playable
availability; gameplay projections and the full private-release redesign remain
unfinished. [localdata.md](localdata.md) records the complete implementation contract.
`cargo run --locked -p azeroth-cli -- doctor --json` reports this binary's version,
OS/architecture, pinned development requirements and tool presence on PATH. Presence
is not runtime-version verification, and missing development tools do not prevent
using the packaged desktop. Optional `--saved-variables <file>`, `--addons <dir>`
and `--content <dir>` inspect only explicitly selected path kinds. Unselected paths
stay unselected; invalid selections return a nonzero exit code. The command does
not read their contents, scan account directories, run detected tools, or print
selected paths. Native prerequisites and live-client compatibility remain unverified.

Content validation has bounded regular-file/directory discovery, deterministic
JSON/human reports and separate nonfatal gaps. See [coverage](docs/data/COVERAGE.md).
Content packs require a typed `sources.json` sidecar and registered evidence/image
sources. They preserve exact dataset identities and verified PNG bytes, refuse
overwrite, and report declared distribution eligibility separately from gaps.
See [local reference pack format](docs/data/CONTENT_PACK.md). Real Forever content
has a typed registry and can be built and verified as a `local_reference` pack in
0.9.1, but its recorded source rights remain incomplete and
`distribution_eligible=false`. It is not a release asset or a playable-content pack.

## Windows installer

`scripts\windows\Build-Windows.ps1` builds the per-user NSIS installer and the addon
package on Windows; the `Windows package` workflow builds, smoke-installs and uploads
them, and drafts (never publishes) a GitHub release for a version tag. Installing,
updating and removing the application and addon:
[WINDOWS_INSTALL.md](docs/compatibility/WINDOWS_INSTALL.md).
The full-content v1 publication gates and required content/source assets are in the
[v1 release process](docs/releases/V1_RELEASE_PROCESS.md).

## Content and imagery

Owner-supplied imagery under `content/forever/atlas/` uses Git LFS and separate
usage terms in its NOTICE; it is not covered by the repository BSD-3-Clause license.
See [sources](docs/data/SOURCES.md). CI keeps committed LFS pointers and never
fetches imagery. Its pointer check is:

```sh
cargo run --locked -p azeroth-cli -- prepare-atlas --manifest content/forever/atlas/eastern-kingdoms/atlas.manifest.json --content-root content/forever --check-pointer
```

This command deliberately fails on materialized images. To prepare owner-supplied
imagery, omit `--check-pointer`. Do not commit personal exports or secrets.

## CI and evidence

CI checks Rust formatting, locked Rust and frozen pnpm/contracts on Linux/Windows,
Lua static/parity checks, explicit fixture/content validation, native Windows shell
compilation and mocked Chromium desktop workflows. Cache hits never skip checks.
PR CI targets main; stacked branches can dispatch `ci.yml` explicitly. Hand-offs
record actual local/hosted checks, checked revisions, reviews and limitations.
CI itself produces no release; native packages come from the `Windows package` workflow.
