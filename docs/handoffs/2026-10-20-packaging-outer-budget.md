# Packaging outer budget covers the same source and tag suite

Sole writer `/root/tooling_review`; baseline
`f1d8372694e22b02d46fc74719d32a303464c25e`; frozen workflow repair
`ad61fc25e57358b23b914e52991bc332a9a13ea6`.

The packaging job now has a finite170-minute outer limit on source/main/draft and
tag runs, using the already existing tag limit. Exactly one workflow line changed.
The per-workload20-minute bound, suite60-minute remaining-time guard, all workflow
steps, final producer/recorder outcome guard, readiness and90% coverage gates,
trust/signature/source/material checks and permissions are unchanged.

Actual V8 package39424036854 used candidatee5ecdf65 and merge checkout
5d9c89522241eff5285494503289c940a9849d1c. Setup/build/rehearsal/native/RC upgrade
occupied approximately51 minutes (12:40 job start to13:31 producer invocation).
Stable-Acceptance.ps1 runs Assert-ReleaseArtifacts before its suite stopwatch;
actual pre-suite source/material checks took approximately6 minutes. The declared
suite then permits60 minutes. Recording reruns prebuild verification (about3
minutes observed), and final draft readiness/material verification needs further
source/material work (roughly6 minutes from comparable actual checks). Thus the
observed setup plus allowed suite and verification composition is approximately
51+6+60+3+6=126 minutes before upload/cleanup overhead, exceeding the old110 limit.
These figures combine measured stages and an allowed suite envelope; they are
not a measured completed126-minute run or an actual observed timeout.

V8 actually failed the performance metric object validation, not the timeout.
Its raw cases and failed guard remain historical failures. The cache repair may
reduce repeat decoding, but this change makes no speedup or ordinary-PC claim.
170 is an outer fail-closed bound, not proof that every possible worst-case path
fits: the60-minute suite guard controls remaining bounded workloads, not every
synchronous source/resource check. A stalled job can run longer before outer
cancellation. No account quota, repository setting, purchases or concurrency were
changed; no request for expanded quota is made.

Actual checks: Actionlint validates workflow YAML/Actions syntax and expressions;
git diff --check passes. No mirror unit test or full functional rerun is warranted
for this single timeout line. Runtime, packs and current actual functional proof
retain their original source identities; this production workflow change requires
new production/review/receipt binding before fresh candidate checks. No keys,
signing, canonical edits, workflow dispatch, installer/owner acceptance promotion,
release publication or root-branch push occurred here.

Next: independent root/desktop review, routine focused draft repair PR, normal
integration and renewed release bindings. Keep every currently pending hosted or
owner observation unverified until its actual exact-installer evidence exists.
