# COVERAGE-INPUTS — bounded coverage evidence reads

Owner/sole writer: Codex `/root`; baseline `826aa21`;
branch `fix/release-evidence-audit`. Claim:
[coverage inputs](../claims/2026-10-19-coverage-inputs.md).

## Implemented

The coverage gate now checks inventory type/size before its first byte read and
parses the same bytes it hashes. Inventory, receipt and raw-report reads use a
checked descriptor, compare file identity after opening, and read at most the
checked size plus one byte. Growth, truncation and replacement are refused.
Raw-report directories beneath the receipt directory must be real directories,
not symlinks or junctions. Legitimate nested reports still work. Coverage tool
versions, thresholds and evidence requirements are unchanged.

Seven added regression tests cover oversized inventory before any read, linked
inventory/receipt, oversized/nonregular/linked reports, nested report directories,
growth, truncation and file replacement. All use synthetic temporary inputs;
none claims product coverage or native acceptance.

## Actual validation

Node v24.15.0, pnpm v12.11.2, Linux/WSL:

- `pnpm install --frozen-lockfile`: passed, reused pinned packages.
- `node --test scripts/coverage/*.test.mjs`: 27 passed, zero failed, including
  16 gate tests (previously nine).
- `git diff --check`: passed.

The initial broad test attempt found missing worktree dependencies and an overly
strict assertion about requested versus returned read bytes in the new truncation
test. Installed locked dependencies and corrected that assertion; final suite
above passed. No failed run is counted as a pass.

## Limits and review focus

Native Windows execution is unverified locally. Parent-directory checks refuse
static linked report paths; they do not promise protection against an adversarial
concurrent replacement of directory ancestors. Descriptor identity and bounded
reads address the concrete file replacement/growth issues. No content, release,
Windows performance or live WoW gate is promoted.

## Next task

Run exact-head hosted checks and review before integration. Remaining program
work includes source inventory/validation, post-launch target freeze, signing
setup and candidate-bound operator evidence; draft implementation PRs do not
clear those gates.
