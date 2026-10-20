# STABLE-WORKFLOW-ADAPTER handoff

Owner/sole writer: `/root/tooling_review`. Baseline `b39635b`, exact unsigned
verifier `e4ae08c` cherry-picked as prerequisite `153c4e3`. No push/signing/network
pipeline execution. Claim: docs/claims/2026-10-19-stable-workflow-adapter.md.

Implemented explicit prebuild/draft/public PowerShell and workflow adapters,
source-immutable evidence overlays, exact installer hash binding at draft/public,
external evidence artifact transport, reusable eight-row hosted record validation
and recording, and mandatory stable signed-bundle build inputs. Old candidate
transport and RC unsigned policy retained. See SCHEMA4_WINDOWS_PHASES.md.

Actual checks on the final implementation:
- Native Windows PowerShell 5.1 Test-StablePreparation.ps1: 42 checks passed;
  synthetic fixtures only. Covers phase dispatch/refusal, overlay/source mutation,
  actual installer bytes, missing/failing hosted cases, source/hash/method pins,
  baseline/downgrade/offline/metrics, absent/failed bundle verification, Tauri environment/config restoration on failure and scoped
  material catalog bounds.
- Test-ReleasePolicy.ps1: 17 passed. Test-Signing.ps1: 11 passed synthetic PE cases.
- All scripts/windows/*.ps1 parsed by native PS5.1 parser; git diff --check passed.
- Both changed workflow YAML files parsed with system Node yaml (unique keys).
- Test-ReleaseGate, Test-ReleaseMaterials, Test-AcceptanceCandidate were attempted
  under native PS5.1 but could not start fixtures because Windows PATH has no git.
  These remain unverified here; no git shim or product gate was substituted.
- PowerShell 7 unavailable here; Windows hosted runs/installer/resource mapping,
  bundle CLI integration, real signing, network evidence and manual smoke remain
  unverified. No action was dispatched.

Outstanding integration (must block release until done): root-owned bundle CLI,
loader, signed bundle assembly and explicit package inputs; actual hosted case
harnesses for the eight new cases and independent review of their reports; owner
smoke recording/review and explicit complete overlay; full publication readiness
and independently downloaded asset verification. Existing regression smoke files
are not automatically acceptance rows. Missing new cases leave draft readiness
false. No full usable release or ordinary-PC performance claim is made.
