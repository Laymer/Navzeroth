# RC lifecycle transaction repair

Owner and sole writer: Codex `/root`. Baseline: `38bbe4ab03ec48292b57cabfde15de940e99946c`.
Branch: `fix/rc-lifecycle-transaction`; isolated worktree `/tmp/navzeroth-rc-lifecycle`.
Claim recorded before implementation. Serialize identity validation and all additive
migrations in one immediate transaction; preserve schemas and refusal rollback.
Next: concurrent initialization/migration and rollback regression verification,
independent read-only review, exact-head checks, then the Windows and SemVer repairs.
Original checkout, private inputs and existing worktrees remain untouched.
