# Draft release verification access

Owner/sole writer: Codex `/root`; baseline `fcf48ea5b3c7ba79cc3603bd5005fe961bca60e3`.
Isolated branch `fix/rc-draft-verification`, worktree `/tmp/navzeroth-rc-draft-verifier`.

Verification run 39245077836 could not discover the unpublished RC with a
contents-read workflow token. Correct only the manual verifier job permission
needed to read draft releases. Preserve the immutable RC tag/assets and exact
source checkout. Independent read-only review and all applicable final-head
checks precede merge; actual draft/public verification remains required.
