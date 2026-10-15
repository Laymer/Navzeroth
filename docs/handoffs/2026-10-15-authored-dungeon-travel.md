# Authored dungeon travel consumption

Owner: source_admission worker. Baseline 872a814 on frozen consumer integration branch; claim b9d12ca. Root transferred these dungeon report/UI/test files following independent review of 557d86b.

The authored dungeon adapter had discarded `travel_edges`, so unavailable authored travel facts were absent from both report and gaps. Preparation now includes an optional authored-travel declaration preserving original observation status/metadata and dungeon evidence. Known IDs resolve to the exact active catalog's directed edge records, retaining endpoint, mode, cost/wait, requirement, uncertainty and edge evidence facts. Edge evidence also enters the report's evidence/source review. Unknown/restricted/unsupported/not-applicable lists produce a section gap; a known empty list remains explicitly empty. The UI displays availability, directed dependencies and complete facts/provenance.

The dependency list is not specified as an ordered path, so it does not replace graph routing or imply inferred geometry/current access. Existing member-conditioned shared entrance search remains separate. Local preparation and snapshot/plan exchange formats are unchanged; the optional report field is omitted for local packages, retaining their golden output.

Checks: app authored-content 5 tests and dungeon-preparation 8 tests pass; strict app all-target Clippy and Rust formatting pass. Desktop typecheck/lint pass and full desktop test run passed 200 tests (including three new availability display cases). Synthetic tests cover unknown/restricted/unsupported/not-applicable, known-empty and exact directed-record resolution; an absent origin still produces no route. No live/Windows check or gameplay corpus is claimed. Next: independent review, integration with pagination/Atlas, hosted checks.

Coordinator integration repeated all13 focused app tests successfully. Hosted
Chromium exposed obsolete dungeon copy and missing authored-catalog mocks in the
existing Gear/Dungeon browser procedures; those mocks now explicitly return an
empty authored catalog. Repeating the procedures then exposed real narrow-screen
fieldset overflow. The shared authored picker now permits shrinking and wraps its
controls. Both affected Chromium procedures pass at their existing viewport/zoom
assertions; no assertion was relaxed.
