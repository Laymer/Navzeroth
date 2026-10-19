# Windows verification repair

Owner/sole writer Codex `/root`; baseline `38bbe4a`, branch
`fix/rc-windows-verification`; claim recorded before implementation.

The research fixture now atomically reserves a process/counter directory, retries
existing names within a bound, and retains Unix private-directory permissions.
Native smoke waits at most 30 seconds for get_database_status=ready, including
bounding a hung IPC request; checking retries, recovery/unknown/error/timeout fail.
Failed readiness stops dependent smoke operations and remains a failed receipt.
Native reference and concurrent workflow reads serialize refusals inside the page:
only bounded error codes survive, never backend messages, paths, arguments or stacks.
All three owned picker cancellations and unchanged-state assertions remain.

Actual local checks: 15 native-smoke helper tests, five application research-boundary
tests (including 32 parallel directory reservations), affected Rust test-target
Clippy with warnings denied, formatting, JavaScript syntax and diff checks pass.
Delayed readiness, recovery, unknown status, IPC failure, polling timeout, hung
request and structured/unstructured sanitization are covered. Native Windows and
picker checks require hosted execution; independent review remains pending.

Next: final-head review/CI, integration after lifecycle repair, shared SemVer upgrade
checks. No ordinary Windows/live acceptance or publication claim. Preserve ten
required checks and strict coverage; no private game files accessed.
