# Parallel continuation review

Baseline for all four implementation tasks: `826aa21`. Each writer used a
separate worktree and branch. Existing PR #212 was excluded from this audit.

## Coverage inputs — PR #215

Desktop-locale agent reviewed `9bacb4e` read-only, ran 27 tests and reproduced a
blocking FIFO replacement at the bounded-reader open. Severity: important.
Expected refusal before an unbounded wait; actual blocking `O_RDONLY` open.
Root fixed with nonblocking open and a timeout-bounded POSIX regression.
Guide-validation agent independently reviewed corrected head `05fcf66`, ran
all 28 coverage-tool tests and checked the diff: no actionable findings.
The report-directory ancestor replacement limitation is documented explicitly.
Native Windows execution remains unverified locally.

## Desktop locales — PR #214

Root reviewed `7ee59f5` against parent selection/remount behavior and the relevant
locale boundaries. Saved choices are intersected with declared installation
locales without English fallback. Missing/duplicate/unknown choices and explicit
non-English reselection are covered. Independent affected-file run passed
21 tests in two files. No actionable findings. Broader V3.2 catalog UI remains
open; this is a bounded repair, not completion of that entire task.

## Guide objectives — PR #216

Root reviewed `3345e3a` source and regression diff. Checking each required u8
index fixes invalid-index substitution while preserving known zero and unknown
objective counts. Matrix/property tests cover missing, invalid and repeated
marks; filtered and abandon/reaccept cases preserve lifecycle semantics.
No actionable findings. Author-run checks and measured coverage are retained
in the guide task handoff; no additional independent Rust execution is claimed.

## Archive failures — PR #217

Root reviewed the implementation and final `688e716` compatibility test. The
v2 catalog/checkpoint envelopes admit distinct unknown/unsupported failures;
v1 historical variants remain readable, and future versions are refused.
Extraction failures retain no partial bytes; projection gaps remain incomplete.
Searched catalog consumers for additional version-one envelope assumptions and
found none requiring correction. Requested explicit coverage of every historical
disposition, which the final commit adds. No actionable finding remains from
this source review. Author-run affected checks are in the task handoff;
independent Rust execution and native/live evidence are not claimed here.
