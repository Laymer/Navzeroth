# Native acceptance tooling continuation

Owner and sole writer: Codex `/root`. Baseline `e45eed1b2bde95b9a565f144ec81d49758c05c90`.
Branch `fix/native-acceptance-tooling`; existing checkout was clean before claim.

Continue dependency-ready release work by running the acceptance/packaging helper
checks in actual Windows PowerShell 5.1, fixing verified portability failures and
recording exact checks and environment limits. Inspect only project addon exports
and installed addon identity when available; never publish personal SavedVariables.
Do not promote synthetic helper tests to interactive Windows or live acceptance.
No new agents. Read STATUS, pipeline/capture/acceptance handoffs and implementation
plan 7.3–7.5,14,26; existing release gates and target coverage remain mandatory.

After native checks passed, scope includes a read-only `inspect-capture` CLI for
bounded inert SavedVariables parsing and an allowlisted development diagnostic
summary. This supports the operator's newly authorized guided checks without
exposing raw character exports or implying final candidate acceptance. Add the
previously absent acceptance-kit checks to both hosted PowerShell versions.
