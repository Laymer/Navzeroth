# V22 app-data isolation continuation

Owner and sole integration writer: Codex `/root`.
Claim: `docs/claims/2026-10-24-codex-v22-continuation.md`.
Baseline: release candidate `b81e55c576df22d73f5d6ece3396a3b556376198`.

## Completed

Independently reviewed Claude Code's PR #286 at `2c443580`; no blocking findings.
Review: `docs/reviews/2026-10-24-appdata-isolation.md`. Integrated normally at
`2d33c2fd935c6b91754170862eda36b2d64561cc`. Original checkout and its unrelated
untracked work remain untouched. The release worktree's V19 claim is preserved.

Both failed V21 attempts from run 39990706776 are preserved byte-for-byte under
`release/history/2026-10-24-v21-attempts`, including original API responses, raw
case reports and overlays. The ledger pins downloaded ZIP identities; ZIPs remain
outside Git. Retained state.json is the disposable hosted synthetic engineering
state, not a personal export or SQLite database. Attempt 1 failed FIRST-START;
attempt 2 passed six cases, failed UPGRADE-RC1 and never ran DOWNGRADE-REFUSAL.
The earlier FIRST-START cause remains unknown. No old pass satisfies V22.

Seven exact V21 signed/functional inputs are retained under
`release/history/2026-10-24-v21-signed`. Codex independently checked all 287
FileRefs in the prior content review against actual files and Git/LFS identities:
zero mismatches. Runtime and content bytes are unchanged by PR #286.

## Actual checks

- Linux PowerShell 7.5.4: 57 helper checks passed.
- Native Windows PowerShell 5.1: 57 helper checks passed; native PS7.6.7: 58.
  Both native invocations used process-only ExecutionPolicy Bypass. The initial
  PS7 attempt lacked Node on PATH and failed; the successful rerun explicitly
  selected native Node 24.15.0. No persistent environment/policy changes.
- Node workflow tests 6/6, workflow-gate checks 12/12, actionlint exit 0.
- Preparation unittest discovery: 9 tests passed.
- Cargo --no-run and CLI build confirmed the existing cached optimized artifacts;
  no new compiler build claimed. Actual full-pack functional test passed in
  76.209 seconds, peak child RSS 6,205,496 KiB on shared Linux/WSL2. The first
  invocation omitted --ignored and executed no assertions; it is not evidence.
  The successful rerun explicitly used --ignored and produced the pinned proof.
- Regenerated inventory retains 260,113 records and 274,059 accepted unknown
  fields; coverage inventory is byte-identical to V21. Nine observations remain
  pristine pending. No hosted Windows or ordinary-PC performance claim.

Tested source: `2d33c2fd935c6b91754170862eda36b2d64561cc`.
Production fingerprint: `a947d2f092f9ed4921676c008745c87c837325102db04b106460121871295537`
over 3,160 entries, excluding only docs/ and release/ under the existing contract.

## Remaining

V22 unsigned material preview, current content/binding review, committed unsigned
preparation, two-key receipt and independent signed closure must finish before
one candidate push. The coordinator authored the new proof/binding metadata and
cannot describe its own audit as independent review of that work. A read-only
reviewer was requested under AGENTS.md's explicit-subagent-request rule.

Then require fresh protected checks, per-file coverage, two-platform source parity
and all eight real installed cases. Follow the existing 2026-10-23 continuation
and schema5 downloaded-draft recipe for resulting main, tag, draft verification,
publication, public verification and website deployment. No main merge, tag,
publication or acceptance promotion has occurred in this continuation.
