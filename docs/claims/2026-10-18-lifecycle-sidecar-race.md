# Lifecycle sidecar observation race

Owner/sole writer Codex `/root`, baseline `ac114c8`, branch
`fix/lifecycle-sidecar-race`. Read STATUS, performance/desktop handoffs and plan
section18 before code. PR201's refreshed Windows Rust check exposed an existing
race: lifecycle::index statted a SQLite journal, then checked/canonicalized it
again while the detector's normal commit removed it, returning Selection is
inaccessible. Validate each optional sidecar using one no-follow metadata
observation. Preserve link/reparse, type and quota refusals and surface errors
other than NotFound. Add deterministic disappearance and negative-boundary tests.
Independent product fix gets its own PR before coverage integration. Preserve
all worktrees and `.claude/`; no release/acceptance claim or cache/game read.
