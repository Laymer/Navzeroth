# Stored v2 decoded-graph validation

Owner: tooling_review. Baseline: 2466b7440fec64e346b7d0eba7e8fefcb1a1fa6e.
Isolated branch fix/storage-decoded-catalog-validation; one active writer.

Scope: private storage strict decode/canonical boundary avoids validating the
same unchanged graph twice. Arbitrary typed staging remains validated. All size,
source, conflict, hash, coverage and retained-history checks remain required.
No acceptance promotion, canonical release metadata edits or key access.
Separate subsequent commit may stream the existing canonical hash writer into
SHA-256, preserving exact bytes; no new typed serializer or cache waiver.

Read STATUS, V14 recovery-runtime handoff and implementation plan section8.4.
Actual tests, measured full-pack scope and remaining limits follow in handoff.
