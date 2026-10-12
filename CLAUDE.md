# Claude Code entry point

@AGENTS.md

AGENTS.md holds the rules shared by every contributor. If those rules (headed
"Azeroth Navigator — shared project instructions") are not in your context, the
import did not load: read AGENTS.md before doing anything else. Shared policy
changes go in AGENTS.md; this file keeps only Claude-specific notes.

## Starting a session

Read docs/STATUS.md and the latest relevant docs/handoffs/ record.
Use AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md as the implementation specification;
read the current task's sections rather than importing the entire plan here.
Check the existing worktree and claim a dependency-ready task before editing.

- Session workflow: the starter prompt in
  [plan §23.2](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#232-prompt-to-start-claude-code).
- Handoffs: the template in
  [plan §21.5](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#215-handoff-template).

## Environments

- The owner's checkout is in WSL (`\\wsl.localhost\kali-linux\…`). A Claude Code
  session started there works in that checkout directly.
- Cloud sessions cannot reach that checkout: the link to the owner's computer
  refuses `\\wsl.localhost` paths. Work on a clone of Laymer/Navzeroth and
  exchange changes through GitHub; after a push, the owner runs `git pull` in WSL.
- Linux/WSL checks cannot verify native Windows packaging, dialogs, selected
  WoW paths, or live-client behavior. A Windows runner can provide build evidence;
  interactive and live-client checks still require their actual environments
  ([plan §5.2](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#52-windows-and-wsl-development)).
  Record those checks as unverified.

## Commits and pushes

- Follow [plan §21.4](AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#214-branch-and-commit-convention):
  prefix the subject with the task ID (`M1.2: …`), or `docs:` for changes that
  belong to no task.
- Use the configured Git author unless the owner specifies another identity.
  Keep genuine Co-Authored-By and session trailers the session supplies.
- Follow AGENTS.md for publication authorization: routine feature-branch pushes
  and draft PRs are authorized; main pushes, merges, and releases are separate.
- Before pushing, fetch `origin/main`. Rebase unpublished work; incorporate later
  main changes without rewriting published history.
