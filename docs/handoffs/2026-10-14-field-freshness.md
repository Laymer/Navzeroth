# Planning typed-field freshness publication handoff

Owner: Codex /root/atlas_dense, sole task writer. Claimed main
cb974d982c3d6c5dea33749246e6592cbdca7470 before implementation; normal merged
main 06b369e into this task after source commit 9d1e5c1. Separate corrective task
before the v0.5.0 release; root owns release integration/publication.

A fresh snapshot capture could previously hide an independently expiring field.
At frozen source 9fd604d, a synthetic known level 22 observation 50 ms from its
30-minute limit selected one eligible objective. Publication at supplied clock
+100ms succeeded, although recalculation selected none. The seven new integration
cases run unchanged against that baseline produced five failures: level, nested
item variant, nested quest accepted progress, declared travel preference and a
future-time uncertain observation becoming fresh. The two unchanged-class success
cases passed. These are synthetic static-clock checks, not live WoW evidence.

PublicationGuard now records bounded typed observation classes with the actual
initial request clock, covering level, role, location, preferences, outer equipment
slots and known item variant/instance/modifiers, quest accepted/completed/turned-in
and nested objectives, and snapshot objectives. This conservatively checks even
unused typed observations; an unused field changing class can require
recalculation. Imported x_* extension values never become planning facts and are
not reflected into the signature. Unavailable status remains distinct; already
stale/unavailable/uncertain inputs may publish their existing filtered reports if
those classes remain unchanged. Ordinary age increase within Fresh is accepted.

Existing capture-age, identity/content hash, membership, elapsed computation/store
wait and atomic publication checks remain. After hashing and typed comparisons,
a second advanced-clock sample checks each signature's retained inclusive UTC
class interval, including capture. This closes deadline crossing during the new
walk itself, including ClockReversed becoming Fresh. Bounds follow real trusted
observed UTC and the supplied fixed UTC/no-session request context; missing or
untrusted metadata and unavailable values infer no deadlines. Lower bounds also
reject a clock reversal after comparison. No second large traversal follows that
last clock sample. Typed count is additionally bounded by MAX_ENVELOPE_BYTES;
this is a count cap, not a separately guaranteed memory budget.

Changed app call sites pass the existing request clock into guard construction,
including the legacy solo/joint synchronous paths. No schema/native/frontend/core
source or release-version declaration changed in this task.

Actual integrated checks on92566cc (main 06b369e plus source 9d1e5c1):

- cargo test --locked -p azeroth-app: 118 passed, 0 failed (including new seven
  integration cases and two private late-clock/boundary regressions; existing five
  elapsed capture/identity service cases pass).
- cargo clippy --locked -p azeroth-app --all-targets -- -D warnings: passed.
- cargo fmt --all --check and git diff --check: passed.

Commands used --target-dir /tmp/navzeroth-peer-planning-a4c7a93/target to reuse
local compilation only; logs /tmp/navzeroth-field-final-app.log and
/tmp/navzeroth-field-final-clippy.log. Baseline failure log
/tmp/navzeroth-field-baseline-fail.log, independently compiled baseline target
/tmp/navzeroth-field-baseline-target. Private late-sample regression uses injected
clock samples NOW then NOW+100, not sleeps or huge fixtures. Native desktop and
live-client execution were not run for this app-only correction and remain
unverified; CI and root independent exact-source review pending at handoff.

Next: root review this separate PR, require all exact-head CI checks, merge, update
held release PR79 with corrected main, re-run release checks, and only then publish
v0.5.0. Do not claim the prior release candidate already includes this correction.
