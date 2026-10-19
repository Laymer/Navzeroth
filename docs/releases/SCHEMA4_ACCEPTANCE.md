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

Outstanding integration: separate build/artifact readiness from final publication
so a pristine pending owner smoke can permit draft preparation while never permitting
publication. Current stable artifact packaging invokes full readiness, so it remains
blocked until that explicit two-phase adapter is implemented. This document does
not authorize bypassing that gate. Hosted harness producers and actual results,
manifest migration and exact-draft owner smoke remain outstanding.
