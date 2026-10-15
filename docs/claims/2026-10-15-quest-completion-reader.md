# Quest-log completion binding

Owner and sole writer: Codex `/root`. Baseline main
`e45eed1b2bde95b9a565f144ec81d49758c05c90`; isolated worktree
`/tmp/navzeroth-quest-completion-reader`, branch `fix/quest-completion-reader`.
Previous capture-tooling implementation is frozen on branch5551ba5 pending CI;
this is the next active implementation task. Shared status/evidence updates
remain coordinator-owned and will integrate after PR148.

Read STATUS, foundation/capture handoffs, actual source-identified guided70374
observations and implementation plan7.4–7.5/14. User confirmed matching build,
level30 and no errors. Inert snapshot inspection found all30completion flags
unknown because the adapter assumes a GetInfo.isComplete boolean. Publicly
installed quest UI source references a separate IsComplete(questID) getter;
operator function-type/call observations are pending, not assumed verified.

Add an optional injected, guarded, bounded boolean completion getter to the
existing manual capture. Preserve unknown on missing/unreadable/failed calls,
false separately from unknown, and unknown turn-in/history/objective mappings.
Never scan quest identities or enable automatic advancement without content
bindings. Exercise strict failure and actual slash capture/import regressions;
require review/hosted checks and resulting-main verification before installation.
