# Schema 4 stable acceptance contract (ADR 0008)

Schema 4 inventory/evidence and readiness reports preserve schema 3's target,
accepted-origin policy, content, source, rights, coverage and deferral contracts.
They do not require live-client observations or a launch-build freeze. Schema 1,
2 and 3 retain their previous meaning; no observation is automatically migrated.
Current repository manifests remain schema 3 until an explicitly reviewed migration
records and retains historical evidence.

Schema 4 replaces the ten historical WIN rows with exactly these nine rows:

| ID | Required method |
| --- | --- |
| HOSTED-INSTALL | hosted_windows |
| HOSTED-FIRST-START | hosted_windows |
| HOSTED-UPGRADE-RC1 | hosted_windows |
| HOSTED-DOWNGRADE-REFUSAL | hosted_windows |
| HOSTED-PERSISTENCE-RECOVERY | hosted_windows |
| HOSTED-RESOURCES | hosted_windows |
| HOSTED-OFFLINE | hosted_windows |
| HOSTED-PERFORMANCE | hosted_windows |
| OWNER-INSTALLER-SMOKE | interactive_windows |

The upgrade row means upgrade from 1.0.0-rc.1. Performance evidence must record
full-pack installation, loading, activation, browsing, planning, peak memory,
workload, hardware and limitations. Hosted evidence cannot establish ordinary-PC
performance or interactive display/accessibility acceptance.

Evidence adds optional `candidate_installer_sha256`; each observation adds optional
`installer_sha256`. Every passed schema4 row must identify exactly that candidate
installer, with existing source revision / production fingerprint binding and
operator, date, environment and hashed artifact evidence. The owner row covers
installation/upgrade, first start, bundled activation, representative planning and
restart/persistence of the independently verified draft installer. Missing, failed,
wrong-method or mismatched-installer observations hold stable publication. No
installer executable is committed. Actual binary/hash verification remains the
independent download-verification boundary, not a claim made by this pure model.

Historical schemas reject populated installer bindings and hosted_windows methods.
A pristine unverified row retains unverified status. Existing source/hash/file and
independent-review boundaries remain unchanged. Relevant production changes revoke
the existing source/fingerprint binding and require new installer observations.

Artifact preparation is explicit and never authorizes publication:

- `release-build-readiness`: schema4 prebuild permits pristine pending hosted and
  owner rows. Source / production identity must already be bound. Initial missing
  installer identity is permitted only while no passed observation requires it.
- `release-draft-readiness`: all hosted rows must pass against one candidate
  installer; only pristine pending OWNER-INSTALLER-SMOKE may remain.
- `release-readiness`: unchanged final publication gate, including owner smoke.

Both preparation commands perform the same full source/content/file/receipt I/O
verification as release-readiness, then expose `phase`, `build_ready`, and
`publication_allowed:false`, retaining the detailed report with its own publication
flag forced false. Failure, partial unverified claims, wrong methods, stale source
or installer hashes, initial unbound source identity and other content blockers
cannot pass preparation. Historical schemas cannot use these preparation adapters.

Outstanding integration: packaging must use prebuild, draft verification must use
draft readiness and public verification/publication must retain full readiness.
Those workflow adapters remain unimplemented by this contract change. Current
stable packaging still uses full readiness, so draft preparation stays blocked
until that explicitly reviewed adaptation. Hosted evidence producers and actual
results, manifest migration and exact-draft owner smoke remain outstanding.
