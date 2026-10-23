# Navzeroth

Azeroth Navigator is a Rust, Tauri/React desktop companion with a Lua addon
for World of Warcraft: Forever. Desktop character state comes from
explicitly imported snapshots; live game observations remain inside the permitted
addon environment. Synthetic fixtures are always labelled.

## Releases and prepared stable scope

Find the independently verified public download and its exact source/asset hashes
in the [download metadata](docs/site/release.json) and
[installation guide](docs/site/installation.html). Those versioned records, not
the version of this source checkout, identify the published release. See
[STATUS](docs/STATUS.md) for current candidate work and actual verification results.
Prepared 1.0.0 is not a publication claim; missing or failed required checks hold
publication. Historical release receipts retain their original identities.

The prepared dataset target is **Forever 1.60.1.70450 / interface 16001**.
[ADR 0007](docs/decisions/0007-world-data-automation.md),
[ADR 0008](docs/decisions/0008-unsigned-stable-release.md) and
[ADR 0009](docs/decisions/0009-automated-draft-release-acceptance.md) supersede the
earlier launch-build freeze and Human levels 1–20 slice. Historical
CMaNGOS 1.12.1 and Forever 1.60.1.70245 inputs retain their actual origins; admission does not establish
current-build gameplay validity. The prepared corpus has 260,113 records and 274,059
accepted unknown fields, source-backed partial quest plans, 54 authored heuristic
models across nine classes, 18 class-role combinations and three level bands,
18 reminder rules and 26 dungeon records (20 estimated entrances, six directly sourced historical ones).
Unknown eligibility, prerequisites, access, stale state and unsupported fields
remain explicit. Trainer-spell gameplay, talents, professions and enhancements
are excluded from promised coverage. See [1.0.0 notes](docs/releases/1.0.0.md).

The 1.0.0 contract explicitly permits an **unsigned installer and application**;
checksums/source binding and signed content receipts protect separate boundaries.
The content receipt is not an Authenticode publisher signature. Full protected
checks, independent review, source parity, stability, hosted Windows acceptance
and schema 5 automated UI/native smoke of the exact independently downloaded draft
installer remain required. Human smoke and the supplemental Windows 11 upgrade
are separate, unperformed observations, not invented substitutes. See the
[v1 release process](docs/releases/V1_RELEASE_PROCESS.md) and
[schema 5 contract](docs/releases/SCHEMA5_ACCEPTANCE.md).

### Use an explicitly verified release

1. Select the public release linked by the installation guide, retain its notes,
   SHA256SUMS.txt and SOURCE_COMMIT.txt, and verify hashes before installation.
2. Follow [Windows installation](docs/compatibility/WINDOWS_INSTALL.md) and the
   separate, explicit addon installation into the client-selected AddOns folder.
3. Record the actual client build using [addon setup](docs/compatibility/ADDON_INSTALL.md).
   The addon admits 70338/70374 and development-only unverified 70425/70450 with
   interface 16001. This experimental addon admission is distinct from the 70450
   content target; a matching TOC or historical observation is not live acceptance.
   See [the addon window guide](docs/compatibility/ADDON_GUI.md).
4. Imported profiles remain local. Synthetic browser demonstrations and reference
   imagery do not establish gameplay facts. Keep SavedVariables and backups private.

[Versioned release checks](release/v1/README.md) and the explicit source admission
registry preserve source rights, bounded facts and unresolved references. Historical
private reference imagery can remain distribution-ineligible even when separately
admitted world/authored release packs have verified resource/source closure.

[Repository identity gallery](docs/brand/README.md) — planned after verified release.

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

Prepared admitted content and authored heuristic models retain their documented
source and uncertainty limits; complete live duo/session acceptance is not claimed.
Party messaging, combat-log analysis and official API access remain deferred. See the
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
See [local reference pack format](docs/data/CONTENT_PACK.md). Historical reference imagery has a typed registry and can be inspected as a
`local_reference` pack; its recorded incomplete rights and
`distribution_eligible=false` remain explicit. This is distinct from the prepared
admitted world/authored release packs and their source/material closure.

## Windows installer

`scripts\windows\Build-Windows.ps1` builds the per-user NSIS installer and the addon
package on Windows; the `Windows package` workflow builds, smoke-installs and uploads
them, and drafts (never publishes) a GitHub release for a version tag. Installing,
updating and removing the application and addon:
[WINDOWS_INSTALL.md](docs/compatibility/WINDOWS_INSTALL.md).
The full-content v1 publication gates and required content/source assets are in the
[v1 release process](docs/releases/V1_RELEASE_PROCESS.md).

**Code signing policy:** the explicit unsigned 1.0.0 exception is governed by
ADR 0008/0009 above; exact unsigned status still requires verification. Optional
future Authenticode/SignPath setup is documented in
[CODE_SIGNING.md](docs/compatibility/CODE_SIGNING.md); no approved signing service
or signed Windows publisher identity is claimed. Content receipts remain required.

## Content and imagery

Owner-supplied imagery under `content/forever/atlas/` uses Git LFS and separate
usage terms in its NOTICE; it is not covered by the repository BSD-3-Clause license.
See [sources](docs/data/SOURCES.md). CI preserves committed LFS pointer identities. Jobs hydrate only explicitly
required, hash-verified resources; pointer checks remain distinct from hydrated
content and release-material checks. Its pointer check is:

```sh
cargo run --locked -p azeroth-cli -- prepare-atlas --manifest content/forever/atlas/eastern-kingdoms/atlas.manifest.json --content-root content/forever --check-pointer
```

This command deliberately fails on materialized images. To prepare owner-supplied
imagery, omit `--check-pointer`. Do not commit personal exports or secrets.

## Data credits

Generated world data under `data-generators/` comes from Wowhead, CMaNGOS
classic-db and wago.tools (Blizzard Entertainment client data), produced by data
generators based on work by NotInTheBand (TALOD), used with permission. Credits,
licences and terms are in [data-generators/NOTICE.md](data-generators/NOTICE.md);
the admission decision is [ADR 0006 §14](docs/decisions/0006-v1-program.md) and the
format is [WORLD_DATA.md](docs/data/WORLD_DATA.md). Navzeroth is not endorsed by
TALOD, Wowhead, CMaNGOS, wago.tools or Blizzard Entertainment.

## CI and evidence

CI checks Rust formatting, locked Rust and frozen pnpm/contracts on Linux/Windows,
Lua static/parity checks, explicit fixture/content validation, native Windows shell
compilation and mocked Chromium desktop workflows. Cache hits never skip checks.
PR CI targets main; stacked branches can dispatch `ci.yml` explicitly. Hand-offs
record actual local/hosted checks, checked revisions, reviews and limitations.
CI itself produces no release; native packages come from the `Windows package` workflow.
