# Stable source/tag hosted gate wiring

Owner/sole writer `/root/tooling_review`; isolated
fix/stable-hosted-workflow-wiring baselineab16316; claimb3b8752b.
Implementation2b76c259 is frozen for root independent review.

Baseline inspection corrected the initial audit scope: ab16316 already contains
actual Stable-Acceptance.ps1 invocation on source-declared1.0.0 and final1.0.0 tags,
and1200000ms startup for both generic native smoke paths (legacy jobs retain30000).
Those reviewed producer/timing changes were preserved rather than duplicated.
The actual remaining gap was non-tag stable source builds finishing after recording
hosted observations without invoking full draft/material readiness on that overlay.

The workflow now identifies the recorder step and requires both actual producer
and recorder success before invoking Verify-ReleaseArtifacts.ps1 with explicit
draft readiness and the exact hosted evidence overlay. It runs for every stable
source/tag build, using the same stable predicate, including when earlier steps
failed/skipped. Missing/failed/unknown outcomes refuse before the verifier. Missing
or failed rows, changed identity/source/installer and material failures remain
subject to the existing reviewed full verification. Only pristine pending owner
smoke may remain. No owner evidence is created. The tag draft job retains its
independent downloaded-artifact/evidence verification, needs package success and
creates drafts only. No new dispatch flag or bypass was introduced.

Actual checks on2b76:

-5 Node workflow guard tests pass: source/tag selection, both startup bounds,
  actual producer→overlay→gate ordering, exact identity inputs and draft-only flow.
-12 native Windows PowerShell5.1 mock checks execute the actual workflow gate body
  with an owned temporary verifier stub. Success invokes the exact draft parameters;
  verifier nonzero fails; absent/skipped/failed/cancelled/unknown producer or recorder
  refuses before artifact verification. No installer/adapter/registry is touched.
- Existing42 preparation/hosted-evidence and10 producer-helper tests pass under
  native PowerShell5.1.32 combined Node helper/workflow tests pass.
- actionlint1.7.12 and diff checks pass. Initial new guard assertions used a wrong
  existing step name and an overly strict spacing regex; corrected guards then
  passed. No production behavior was loosened to satisfy those assertions.

Release-Materials.ps1 and workload persistence remain separately owned and were
not edited. Requested bounded-reader workflow test steps await the independently
reviewed reader prerequisite. PowerShell7 execution, actual installed full bundle,
all eight hosted case results, exact draft owner smoke and public downloads remain
unverified here. No signing, network adapter change, publication, purchase, quota
expansion or push occurred. Root must review/integrate before protected checks.
