# Hosted recorder literal timestamps and artifact paths

Sole writer: `/root/tooling_review`; baseline
`7db6e76ae94c4e5492e5977bb02a049c187b84c0`. Production repair
`eb7ea89bc9df089e173bb0f3550406b4bfcb8c93`; standalone boundary regression
`36f2c45d2b321ddccb38b54ffc673a92e684fdec`.

Actual package run 39413135905 failed its recorder because PowerShell converted
an ISO JSON timestamp into DateTime before the existing metadata validation.
The parser now preserves literal timestamp strings (DateKind String where
available, raw-token restoration on older PowerShell7), including Base evidence
round trips. Date/operator/environment validation remains strict. The recorder
also normalizes one Windows path separator rather than the previous two-literal-
backslash replacement; traversal, root, source and artifact checks remain.

Actual author checks: PowerShell7.6.6 timestamp suite32, native Windows
PowerShell5.1 suite29; existing preparation42 and producer helper20 in each shell;
Actionlint and diff checks passed. Root independently repeated32/29 and static
review. Desktop independently reviewed the production and test-only commits and
ran the real recorder boundary3 checks in each shell. Native5.1 used an external
WSL Git bridge for the synthetic repository. Author's initial native boundary
attempt failed because local Windows PATH lacked Git; this was an environment
failure, not a passing check. Logs are retained at
`/tmp/navzeroth-timestamps-ps76-boundary.log`,
`/tmp/navzeroth-timestamps-ps51-boundary.log`,
`/tmp/navzeroth-desktop-recorder-boundary-ps7.log` and
`/tmp/navzeroth-desktop-recorder-boundary-ps51.log`.

The boundary test executes the actual recorder and checks written literal dates,
portable artifact path, bytes/hash, and missing cases remaining unverified. It
uses a synthetic temporary Git repository and fake installer bytes (never
executed), stubbing only the release-material adapter. The historical raw-case
fixture retains its JSON values; LF formatting is fixture-only. This is parser
and recorder engineering evidence, not installer or release acceptance.

Historical producer evidence contains only install/resources passes and a
first-start failure; five cases are absent. No owner result, missing case, or
release gate was promoted. Desktop separately repairs the installed catalog
consumer. Next: normally integrate reviewed repairs, renew source-bound release
metadata/receipts, then run fresh exact-installer hosted verification. No keys,
signing, installer execution, release publication or workflow retry occurred here.
