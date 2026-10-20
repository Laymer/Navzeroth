# Paged dungeon provenance integration expectation

Owner/sole writer Codex `/root`, baselineee36ae02; isolated
fix/212-origin-page-expectation. Hosted original-consumer CI found the existing
OriginPlacement test assumed a single origin lookup. New24-row member pages
correctly perform separate revision-bound lookups; actual rendered badge checks
passed. Verify every request's subject/revision and the exact union of requested
records, preserving all marker assertions. No production behavior or gate changes.
Run full desktop unit/coverage, typecheck/lint and independent review. Prior failed
CI/Coverage stay failed; require fresh all-ten checks on final212 head.
