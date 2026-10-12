# Navzeroth

Azeroth Navigator — a planned Rust-based companion for World of Warcraft: Forever.

The repository currently contains the implementation specification and collaboration documents. Application development has not started.

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

Application code does not exist yet beyond the M1.1 workspace skeleton. The
commands below are the ones that actually run today; see
[plan section 19](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#19-development-commands-and-ci-contract)
for the full target command surface and which milestone adds each remaining
command.

### Rust workspace (crates/core, crates/cli)

Prerequisites: the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml),
installed via [rustup](https://rustup.rs). `rustup` picks up the pin
automatically inside this repository.

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

`azeroth-cli` currently prints a placeholder banner; it has no subcommands.
`doctor`, `validate-fixtures`, and the other subcommands in plan section 19.3
are planned, not implemented — do not run or report them as working.

### pnpm workspace

Prerequisites: Node.js and [Corepack](https://nodejs.org/api/corepack.html)
(`corepack enable`), which installs the pnpm version pinned in this
repository's `package.json` `packageManager` field.

```bash
pnpm install
```

No application package exists under `apps/` yet. `apps/desktop` and its
`typecheck`/`lint`/`test:unit`/`test:e2e`/`build`/`tauri dev` scripts
(plan section 19.2) are planned for task M1.4.

### Not yet available

- Tauri desktop build/dev commands (M1.4).
- CI (M1.6): the commands above are currently run locally only, not
  automatically enforced.
- SQLite migrations/storage adapter (M1.7).

## Collaboration records

The [initial handoff](docs/handoffs/2026-10-12-project-setup.md) records the documentation setup and the next task. Contributors maintain status, task ownership, decisions, and handoffs in Git as described in the plan.
