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

Follow-up audit also replaces timestamp-only reusable storage and local-data
sidecar fixture directories with bounded atomic reservation. All storage tests
passed, as did four focused sidecar tests and all-target storage/local-data Clippy
with warnings denied. The initial independent review found no actionable issues;
this additional fixture-only delta requires a refreshed exact-head review/checks.

## Integration receipt — 2026-10-19

PR #205 merged as `e9bb77042ebaf306c8d35c82fcb8bde46eaf092c` after independent read-only approval of
final head `8e4a59c6ef6de95e7e4b5730372daacfbe0ab106` and all ten required checks passing.
Successful runs: CI 39221168492, Coverage 39221168487, package 39221168540 and source/parity 39221168489.
Earlier failures/cancellations remain distinct from these passes.
Next: [development RC verification and preparation](2026-10-19-rc-release.md).
This repair does not establish stable/candidate readiness or release publication.
