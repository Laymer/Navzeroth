# Exact LF role-audit report on Windows checkout

Owner/sole writer `/root/tooling_review`; baseline2c3fce1c; claim5d2ad481.
Isolated fix/windows-role-audit-report-eol. Code68e8c60f frozen for root review.

Windows Rust run39390030412/job118519045423 failed in15seconds because strict
verified_json(REPORT) compared a CRLF checkout with the committed LF report.
The source-platform workflow had passed with core.autocrlf=false; ordinary Windows
CI used autocrlf=true. This was a report checkout-byte mismatch, not a source or
audit discrepancy. The failed run retains its failed status.

Added exactly `docs/data/role-coverage-v1.json text eol=lf` to .gitattributes.
No broad text rule/global Git setting, data rewrite, read-time normalization,
changed helper/hash expectation or weakened audit assertion. Report, helper,
all four original CLI tests and22retained generation code pins remain unchanged.

Actual verification:12 hydration tests pass. The new temporary-Git regression
sets core.autocrlf=true, forces report checkout, observes CRLF and proves the
unchanged strict helper refuses. It applies the exact actual repository attribute,
forces checkout again, proves byte-for-byte original canonical LF with no CR, then
successfully runs offline two-input hydration from local LFS objects. This local
Git configuration reproduction is not an actual hosted Windows test result.
Diff check passes. No broader suite repeated because executable behavior and all
prior input/provenance assertions are unchanged.

Next: root independently review the immutable correction and normally integrate
before fresh protected Windows/coverage checks. Actual hosted pass, pipeline,
installer/manual acceptance and release readiness remain unverified. No push,
signing, publication, upstream acquisition or quota change was performed.
