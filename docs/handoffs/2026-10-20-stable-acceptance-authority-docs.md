# Stable acceptance authority documentation correction

Owner/sole writer `/root/tooling_review`; baseline66cf9ec8; claim3821a0ad.
Isolated docs/stable-acceptance-authority worktree; no edits to PR222's worktree.

V1_RELEASE_PROCESS.md and release/v1/README.md now lead with governing ADR0008 and
link the existing schema4 acceptance contract. Historical ADR6/7 narratives remain
intact under clearly historical headings. Summary preserves ten protected checks,
explicit unsigned stable1.0.0-only authority, signed content receipts, eight hosted
Windows cases plus exact-draft owner smoke, source/hash binding and distinct
build/draft/final gates. It explains fresh release-commit checks/tagging, verified
draft download, owner checkpoint, full publication gate, public verification and
website sequence. No live-game or ordinary-PC claim is made.

Actual documentation verification: five local link targets including the new
release-process section anchor resolve; complete historical bodies match baseline
exactly after their original titles. Diff whitespace check passes. An initial
external verification script had an incorrect substring assertion for heading
level; the corrected heading/anchor and historical-body checks pass. No code,
manifest, observation, release status or workflow behavior changed. No test suite
was run for this prose-only correction; no push or publication occurred.

Next: root independently review immutable documentation head, integrate normally
into acceptance PR222 and retain the actual outstanding release gates. This
summary supplies no readiness or acceptance result and migrates no manifests.
