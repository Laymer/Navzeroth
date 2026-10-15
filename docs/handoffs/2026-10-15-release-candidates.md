# Candidate readiness and publication separation

Owner: Codex `/root`, baseline `d59aa91`, claim `36f6125`.
Branch `feat/release-candidate-contracts`; root owns shared release contracts,
existing packaging scripts/workflow. Three workers have separate task worktrees.

Implemented additive typed candidate readiness and explicit non-publishable
packaging described in [release contract](../../release/v1/README.md).
Stable gates remain intact, and stable artifact verification now independently
checks full readiness. No invented content, source rights, acceptance or versions.
The party support matrix portion of PR6 remains a separate content-contract task.

Actual checks: 14 core release tests and 14 CLI release-readiness tests pass;
strict core/CLI all-target Clippy, formatting and diff checks pass. PowerShell 7
on Linux passes 29 packaging gate checks and 19 material transport checks.
New tests cover pending vs failed/partial/automated observation claims, absent vs
malformed tested bindings, no rights/content waiver, candidate CLI exit behavior,
refusal before output mutation, and default marker rejection with rebuilt checksums.
Native Windows/client acceptance remains unverified. Hosted nine-check results
will be recorded after push. Initial shell lacked Cargo on PATH; the installed
`/home/laymer/.cargo/bin/cargo` was used; no failed check is treated as a pass.

Independent read-only review by `/root/source_admission` found no blocker in
core/CLI defer and I/O predicates and requested CLI/default-marker regressions;
both were added and pass. Packaging positive candidate transport still requires
synthetic integration and hosted review before merge.

Next: complete packaging review and nine checks; integrate independently completed
PR1–3; implement frozen v2 contracts/consumers, capacity and party evidence matrix.
Real source admission/population and actual acceptance remain external gates.
The user reports only unidentified verbal permission; no dataset or rights evidence
has been admitted. Published preview remains unchanged.
