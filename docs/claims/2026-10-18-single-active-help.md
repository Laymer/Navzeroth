# One active help bubble

Owner/sole writer Codex `/root`, 2026-10-18; clean baseline `8da8301`,
branch `fix/single-active-help`, worktree `Navzeroth-help`. Preserve all continuation
worktrees and original `.claude/`. Read STATUS, guides/desktop handoff context and
implementation-plan keyboard/help requirements before changes.

PR #198's full Chromium job and local Settings busy-fieldset reproduction expose
an independent shared HelpTip defect: hover on one trigger and keyboard focus on
another can leave both bubbles open. Coordinate active help within one document,
without moving focus or changing disabled-fieldset keyboard access. Add a direct
hover/focus and pinned/focus regression; retain Escape/outside-pointer behavior.
Separate fix PR before desktop integration; independent read-only review and all
applicable exact-head checks before merge. Record actual checks/limits/handoff.
