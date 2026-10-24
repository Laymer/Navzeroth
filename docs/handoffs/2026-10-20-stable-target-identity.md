# Hosted exact target identity repair

Owner/sole writer: Codex `/root/desktop_review`. Baseline `aeb7fd8f`;
claim `e5375a52`; product code `b2e772ad`; isolated branch
`fix/stable-hosted-target-identity`, `/tmp/navzeroth-stable-hosted-target-identity`.

## Actual failure and cause

Preliminary Windows package39400861524 at40ea reached `Stable-Acceptance.ps1:37`
and failed before running acceptance cases. Its job conclusion was success because
of continue-on-error; the producer step outcome was failure. No eight-case pass
occurred and all eight hosted observations plus owner smoke remain unverified.
The pending baseline artifact is not an actual acceptance result.

Both40ea and currentaeb contain identical target values/types but different JSON
member order: release evidence has build/edition/interface/version, whereas build
identity has edition/version/build/interface. Comparing `ConvertTo-Json` text
therefore refused the correct identity. The regression explicitly checks that the
old textual comparison differs before exercising the corrected comparison.

## Repair and actual checks

Only the producer comparison, its pure helper and helper tests changed. The helper
requires JSON objects with exactly four correctly named fields, nonempty string
edition/version/build and an integer interface in1..4294967295. Values compare
case-sensitively; JSON member order and supported parser integer widths do not
change identity. Unknown/missing fields, wrong origin/build/version/interface,
non-object targets, malformed strings and bool/float/string/null interface values
refuse. Existing schema4, production hash, app version, source, exact installer,
public baseline, receipt/prebuild and hosted-runner requirements are unchanged.

Actual native Windows PowerShell5.1.26100.9711 and Linux PowerShell7.6.6 pure-helper
runs each passed20 checks. This includes bounded reading of the actual current
acceptance contract and committed build identity, reordered correct targets,
negative identity cases and the existing synthetic case/offline restoration tests.
The latter use mocked adapters only; no real network control or installation ran.
Initial new-test attempt failed because its loop variable collided with the test
helper's dynamically scoped Name parameter; renamed the local field variable and
both complete runs passed. Producer PowerShell7 parser and git diff --check passed.

Logs `/tmp/navzeroth-stable-target-identity-ps51.log` and `-ps7.log` are byte equal,
SHA256 `42ea6eeeeca680882716908338034515914a60fd8c888cefa6afd99f1f95bffb`.
No content/source bytes, keys, signatures, gate statuses, installer or publication
were changed. This product repair changes the production fingerprint and requires
independent review and renewed source-bound release preparation before publication.

Next: independent review, separate repair PR against the version branch, normal
integration, renewed production bindings and fresh exact-source hosted execution.
The actual eight-case installer acceptance remains unverified until it runs.
