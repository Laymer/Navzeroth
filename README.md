# Navzeroth

Azeroth Navigator — a planned Rust-based companion for World of Warcraft: Forever.

The Rust and pnpm workspaces are initialized. Follow the current status record for implemented behavior and verification evidence.

## Start here

1. Read [AGENTS.md](AGENTS.md) for shared contributor instructions.
2. Read [docs/STATUS.md](docs/STATUS.md) for the current task and verified progress.
3. Use [AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md) as the implementation specification.
4. Open the repository in Claude Code and use the [starter prompt in section 23.2](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#232-prompt-to-start-claude-code). [CLAUDE.md](CLAUDE.md) is its project entry point.

For a ChatGPT/Codex review, use [section 23.3](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#233-prompt-to-request-a-chatgpt-review) and supply the relevant revision or diff, status, and handoff.

## Architecture

- Rust domain logic and services.
- Tauri 2 with React and TypeScript for the desktop interface.
- SQLite for local persistence.
- A Lua addon for permitted live observations and in-game guidance.
- Imported snapshots for desktop character state; optional official API integration only after its availability is verified.

## Development setup

The core provides validated domain records, observations/freshness, bounded
three-valued rule evaluation and versioned exchange contracts. See
[the domain reference](docs/DOMAIN.md) and
[the contract reference](schemas/v1/README.md). The commands below run today; see
[plan section 19](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#19-development-commands-and-ci-contract)
for the full target command surface and which milestone adds each remaining
command.

### Rust workspace (crates/core, crates/cli)

Prerequisites: the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml),
installed via [rustup](https://rustup.rs). `rustup` picks up the pin
automatically inside this repository. On Linux/WSL, open a new terminal after
installation or run `. "$HOME/.cargo/env"` to put Cargo on PATH.

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

`azeroth-cli` prints a capability banner with no arguments and rejects unsupported
arguments with exit code 2. Its only subcommand regenerates the exchange schema:

```bash
cargo run --locked -p azeroth-cli -- export-schemas --output schemas/v1
```

`doctor`, `validate-fixtures`, and the other subcommands in plan section 19.3
are planned, not implemented — do not run or report them as working.

### pnpm workspace

Tested setup: Node.js 24.15.0 (also pinned in `.node-version`) and
[Corepack](https://github.com/nodejs/corepack) 0.36.0. Use
`npm install --global corepack@0.36.0` followed by `corepack enable`.
Corepack reads the exact pnpm version and integrity from `package.json`.
Older Corepack 0.24.0 cannot launch this pnpm 12 binary layout.

On Linux/WSL without a writable global npm prefix, install with
`npm install --global corepack@0.36.0 --prefix "$HOME/.npm-global"`, put
`$HOME/.npm-global/bin` first on PATH, then run `corepack enable`.
Verify `node --version`, `corepack --version`, and `pnpm --version` before use.

```bash
pnpm install --frozen-lockfile
pnpm contracts:types   # regenerate schemas/v1/exchange.ts from the schema
pnpm contracts:check   # tsc type check, then Ajv + TypeScript fixture checks
```

No application package exists under `apps/` yet. `apps/desktop` and its
`typecheck`/`lint`/`test:unit`/`test:e2e`/`build`/`tauri dev` scripts
(plan section 19.2) are planned for task M1.4.

### Not yet available

- Tauri desktop build/dev commands (M1.4).
- SQLite migrations/storage adapter (M1.7).

## Content and imagery

`content/forever/atlas/` holds owner-supplied map imagery under Git LFS. Those
files are **not** covered by the BSD-3-Clause licence of this repository; see
`content/forever/atlas/NOTICE` and [docs/data](docs/data/SOURCES.md). Install
Git LFS (`git lfs install`) before cloning or pulling to materialize them; CI
never downloads them.

## Collaboration records

Use [STATUS](docs/STATUS.md) and the latest linked handoff for current ownership,
verification and next work. Historical handoffs remain available in docs/handoffs/.

## Continuous integration

The CI workflow runs Rust formatting on Ubuntu 24.04 and locked Clippy/tests
plus frozen pnpm installation and contract checks on Ubuntu 24.04 and Windows
2025. Contract drift fails the Rust test jobs (committed schema) and the pnpm
jobs (regenerated TypeScript). Tool versions come from the repository pins; Corepack is 0.36.0. All five checks run on draft
and ready PRs targeting main and on main pushes. Cache hits never skip checks.

Windows Rust uses `x86_64-pc-windows-msvc` and Visual Studio C++ build tools
with the Windows SDK (provided by the hosted runner). For a future local native
Tauri build, also install the WebView2 runtime following
[the current Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
No desktop package exists yet; these Windows jobs validate Rust and pnpm only.

M1.3 added schema/fixture checks; M1.5 extends fixtures and M2 adds Lua parity;
M1.4 adds TypeScript/UI and native desktop build jobs; data and packaging jobs follow their implementations.
See STATUS and the M1.6 handoff for actual run evidence and main protection.
