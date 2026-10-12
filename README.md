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

## Collaboration records

The [initial handoff](docs/handoffs/2026-10-12-project-setup.md) records the documentation setup and the next task. Contributors maintain status, task ownership, decisions, and handoffs in Git as described in the plan.
