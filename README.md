# Navzeroth

Azeroth Navigator is a Rust, Tauri/React desktop companion with a Lua addon
foundation for World of Warcraft: Forever. Desktop character state comes from
explicitly imported snapshots; live game observations remain inside the permitted
addon environment. Synthetic fixtures are always labelled.

Read [AGENTS.md](AGENTS.md), [STATUS](docs/STATUS.md), the latest relevant handoff,
and [the implementation plan](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md) before
changing code. [DOMAIN](docs/DOMAIN.md) describes types and boundaries;
[exchange contracts](schemas/v1/README.md) describe the bounded data formats.

## Implemented foundations

SQLite character-snapshot storage and import ordering; bounded ANAV1/SavedVariables
parsing; optional watcher for one selected file; Today/Imports/Atlas screens;
reference pin search/layers/keyboard navigation; pure travel, eligibility, greedy
planning, gear/acquisition, reminder and session analytics libraries; immutable
context-qualified content revisions with activation/rollback/backup; Lua data
decoding, predicate evaluation, injected HUD and synthetic gear scoring parity.

Wave-2 work was delivered as separate PRs and merged in dependency order after
protected CI. See [the integration handoff](docs/handoffs/2026-10-12-wave2-integration.md)
and [merge audit](docs/handoffs/2026-10-12-wave2-merge.md).
Guarded manual capture, runtime addon plan controls, acquisition-aware Gear and
Sessions history/analytics, and Planner route inspection are implemented for the
v0.4.0 developer preview (release preparation). Actual client bindings remain experimental and native window
interaction remains unverified; hosted Windows packaging is revision-scoped.
See the [release notes](docs/releases/0.4.0.md) and
[consolidated live checklist](docs/compatibility/V040_LIVE_CHECKLIST.md).
Manual joint planning, local proposals/acceptance, quiet reminder controls and
conditional session comparison are added in the guidance batch. PartySnapshot
transport, joint addon export, verified real content and full live acceptance
remain later work. See the [task ledger](docs/IMPLEMENTATION_TASKS.md).

## Development checks

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

The CLI accepts `export-schemas`, `prepare-atlas`, `validate-fixtures`,
`validate-content`, `build-content` and `verify-pack`; other plan commands
remain separate tasks.
Content validation has bounded regular-file/directory discovery, deterministic
JSON/human reports and separate nonfatal gaps. See [coverage](docs/data/COVERAGE.md).
Content packs require a typed `sources.json` sidecar and registered evidence/image
sources. They preserve exact dataset identities and verified PNG bytes, refuse
overwrite, and report declared distribution eligibility separately from gaps.
See [local reference pack format](docs/data/CONTENT_PACK.md). Real Forever content
lacks the necessary registered source/coverage records; no real pack is claimed.

## Windows installer

`scripts\windows\Build-Windows.ps1` builds the per-user NSIS installer and the addon
package on Windows; the `Windows package` workflow builds, smoke-installs and uploads
them, and drafts (never publishes) a GitHub release for a version tag. Installing,
updating and removing the application and addon:
[WINDOWS_INSTALL.md](docs/compatibility/WINDOWS_INSTALL.md).

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
