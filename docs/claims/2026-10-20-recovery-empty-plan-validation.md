# Avoid duplicate validation of an unchanged reconstructed database

Owner / sole writer `/root/world_review`. Baseline
`d271bba67e8fa274e44953c9f78198b1d8fc5f6d`, isolated branch
`fix/stable-recovery-empty-plan-validation`. Only move the second full validation
into the branch that actually updates an imported plan guard; retain first full
schema/history/semantic validation and every other recovery boundary. Add meaningful
corruption/recovered-plan regression evidence; no schema/cache/trust/budget changes,
canonical/receipt/key edits or root-worktree mutation. Timing remains unmeasured
until independently reviewed optimized benchmark execution.
